//! Byte-level scanning shared by the two chunk-boundary-detection passes
//! `batch::StreamingParser` needs: "has the RTF header finished arriving"
//! and "is there a safe body increment to hand to `parse::Parser` yet".
//!
//! Both are built on [`next_token`], a tokenizer that mirrors the exact
//! byte-level dispatch `parse::Parser::run_body_step` itself uses (the `\`,
//! `{`, `}`, plain-byte branches, `\binN`'s raw-byte skip, control-word
//! letters/param/optional-trailing-space reading) closely enough that a
//! misclassification here can only ever change *when* a boundary is found —
//! never what gets parsed once a boundary is found, since the actual content
//! is always handed to the real `Parser` (the exact same method the
//! whole-document `parse()` uses) for the genuine parse. The one place this
//! module's precision *does* matter for output correctness is the exact cut
//! position after a `\par`/`\pard` control word (see
//! [`find_next_par_cut`]'s doc comment) — everywhere else, a boundary found
//! a few bytes "late" just means slightly more was buffered, not incorrect
//! output.
//!
//! Every scan here is chunk-boundary-tolerant: instead of assuming
//! end-of-buffer means end-of-input (true for the whole-document parser,
//! false for a partial chunk), running out of bytes before a token/boundary
//! is unambiguously resolved returns `Err(resume_pos)` — "buffer more and
//! retry from `resume_pos`" — rather than silently guessing.

use crate::parse::{is_footnote_group_prefix, is_skip_group_prefix};

/// One RTF token, as far as boundary-scanning cares.
enum Token {
    GroupOpen,
    GroupClose,
    ControlWord(String),
    /// A text byte, control symbol (`\~`, `\\`, …), or hex escape (`\'XX`) —
    /// none of these open/close a group or match "par"/"pard", so boundary
    /// scanning never needs to distinguish them further.
    Other,
}

/// Advance past exactly one token starting at `buf[pos]`.
///
/// Returns `(new_pos, token)`, or `None` if `buf` doesn't (yet) contain a
/// complete, unambiguous token starting at `pos` — the caller should buffer
/// more bytes and retry from the same `pos`. A `\binN` control word's `N`
/// raw payload bytes are consumed as part of that one token (matching
/// `handle_control_word`'s `"bin"` arm exactly), so a chunk boundary landing
/// inside binary picture/object data is handled the same way as any other
/// mid-token split — buffered until the whole token (word *and* payload) is
/// present. For a large embedded `\binN` blob this means the increment
/// containing it is bounded by that blob's own size, not by the surrounding
/// document — see `batch.rs`'s module doc for why this is a real, narrower
/// (but still bounded) exception rather than a full return to
/// O(document size).
fn next_token(buf: &[u8], pos: usize) -> Option<(usize, Token)> {
    let byte = *buf.get(pos)?;
    match byte {
        b'{' => Some((pos + 1, Token::GroupOpen)),
        b'}' => Some((pos + 1, Token::GroupClose)),
        b'\\' => {
            let next = *buf.get(pos + 1)?;
            if next.is_ascii_lowercase() {
                // Control word: letters, optional '-', digits, optional
                // trailing space — exactly `Parser::read_control_word`.
                let mut i = pos + 1;
                while i < buf.len() && buf[i].is_ascii_alphabetic() {
                    i += 1;
                }
                if i >= buf.len() {
                    return None; // word may continue in the next chunk
                }
                let word = String::from_utf8_lossy(&buf[pos + 1..i]).into_owned();

                let mut j = i;
                let negative = buf.get(j) == Some(&b'-');
                if negative {
                    j += 1;
                    if j >= buf.len() {
                        return None; // don't yet know if digits follow the '-'
                    }
                }
                let digits_start = j;
                while j < buf.len() && buf[j].is_ascii_digit() {
                    j += 1;
                }
                if j >= buf.len() {
                    return None; // digit run may continue in the next chunk
                }
                let param = if j > digits_start {
                    std::str::from_utf8(&buf[digits_start..j])
                        .ok()
                        .and_then(|s| s.parse::<i32>().ok())
                        .map(|n| if negative { -n } else { n })
                } else {
                    None
                };

                // Optional single trailing space delimiter. `j < buf.len()`
                // is already guaranteed above, so `buf[j]` is safe to
                // inspect without further ambiguity.
                let mut end = j;
                if buf[end] == b' ' {
                    end += 1;
                }

                if word == "bin" {
                    let n = param.unwrap_or(0).max(0) as usize;
                    if buf.len() < end + n {
                        return None; // binary payload not fully buffered yet
                    }
                    end += n;
                }

                Some((end, Token::ControlWord(word)))
            } else if next == b'\'' {
                // \'XX hex-encoded byte — always exactly 2 bytes after `\'`,
                // consumed regardless of whether they're valid hex digits
                // (matches `Parser::run_body_step`'s `\'` branch exactly).
                if pos + 4 > buf.len() {
                    return None;
                }
                Some((pos + 4, Token::Other))
            } else {
                // Control symbol: `\` plus exactly one more byte.
                Some((pos + 2, Token::Other))
            }
        }
        _ => Some((pos + 1, Token::Other)),
    }
}

/// How a `{` (already consumed) classifies, for boundary-scanning purposes.
enum GroupKind {
    /// `is_skip_group_prefix`/`is_footnote_group_prefix` matched: skip the
    /// whole group atomically (its contents never affect the outer scan).
    Opaque,
    /// Neither matched: a real, transparent group.
    Transparent,
}

/// Classify the group whose `{` was just consumed (`pos` is the position
/// right after it in `buf`).
///
/// Rather than requiring some arbitrary fixed look-ahead before committing
/// to "not a recognized destination group" (which would make any short,
/// complete, well-formed transparent group at the end of a buffer look
/// permanently ambiguous), this reads exactly one bounded token — via
/// [`next_token`], which is itself chunk-boundary-tolerant — to learn the
/// group's opening control word (or lack of one), then checks that word
/// against [`is_skip_group_prefix`]/[`is_footnote_group_prefix`] using a
/// synthetic `\<word>` probe with nothing following it. This gives the
/// *identical* answer those two functions would give on the real
/// unbounded remainder: neither pattern set depends on what comes after the
/// matched word (no upper-bound check), so truncating to just the parsed
/// word cannot change the result — only `None` (need more data) is possible
/// while the word itself is still arriving, never a wrong answer.
fn classify_group(buf: &[u8], pos: usize) -> Option<GroupKind> {
    match (buf.get(pos), buf.get(pos + 1)) {
        // `\*` control symbol — `is_skip_group_prefix`'s destination-group
        // marker; decisive with just these 2 bytes, no need to read further.
        (Some(b'\\'), Some(b'*')) => Some(GroupKind::Opaque),
        (Some(b'\\'), Some(_)) => match next_token(buf, pos) {
            None => None, // control word/symbol not fully buffered yet
            Some((_, Token::ControlWord(word))) => {
                let probe = format!("\\{word}");
                if is_footnote_group_prefix(probe.as_bytes())
                    || is_skip_group_prefix(probe.as_bytes())
                {
                    Some(GroupKind::Opaque)
                } else {
                    Some(GroupKind::Transparent)
                }
            }
            // Any other control symbol/hex escape, or (degenerate) an
            // immediately-nested `{`/`}` — none of these open a recognized
            // destination group.
            Some((_, Token::Other | Token::GroupOpen | Token::GroupClose)) => {
                Some(GroupKind::Transparent)
            }
        },
        // `\` is the last buffered byte — ambiguous (could be `\*` or the
        // start of a control word), not "doesn't start with `\`". This arm
        // must come before the `Some(_)` catch-all below, which would
        // otherwise wrongly match `Some(b'\\')` here and commit to
        // `Transparent` on incomplete data.
        (Some(b'\\'), None) => None,
        // Doesn't start with `\` at all — can't be any recognized
        // destination group (all of them open with a control word/symbol).
        (Some(_), _) => Some(GroupKind::Transparent),
        (None, _) => None, // need at least 1 byte
    }
}

/// Skip a balanced `{...}` group whose opening `{` has already been consumed
/// (`pos` is the position right after it). Mirrors
/// `Parser::skip_balanced_group` byte-for-byte (via the same [`next_token`]),
/// but returns `None` instead of silently stopping at end-of-buffer, since
/// here `buf` may be a truncated prefix of a much longer document.
fn skip_balanced(buf: &[u8], pos: usize) -> Option<usize> {
    let mut depth = 1u32;
    let mut pos = pos;
    while depth > 0 {
        let (new_pos, tok) = next_token(buf, pos)?;
        match tok {
            Token::GroupOpen => depth += 1,
            Token::GroupClose => depth -= 1,
            Token::ControlWord(_) | Token::Other => {}
        }
        pos = new_pos;
    }
    Some(pos)
}

/// Find the position in `buf` where the RTF header ends and body content
/// begins — the earliest point, scanning forward, where either:
/// - the whole buffered input runs out before the header is confirmed
///   complete (`Err(resume_pos)`: buffer more and retry from `resume_pos`),
///   or
/// - a definitive body-start byte is found (`Ok(pos)`).
///
/// "Definitive body start" is deliberately coarse — the very first thing at
/// the top level (i.e. not inside a `\fonttbl`/`\colortbl`/`\stylesheet`/
/// `\info`/`\*`-destination/footnote group) that isn't itself a bare control
/// word: a literal byte, a control symbol, a hex escape, an unmatched `}`,
/// or a transparent `{...}` group. Precision beyond that isn't needed: any
/// leading control words this boundary ends up including in the *body* slice
/// instead of being consumed during header-table computation are control
/// words `Parser::run_body_step` already treats as no-ops when encountered
/// standalone (they're in `handle_control_word`'s giant ignored-word arm —
/// `\ansi`, `\deff`, `\deflang`, etc.), so misclassifying "still header" vs.
/// "already body" here changes *which pass* consumes a leading preamble
/// word, never what it does — see the module doc.
///
/// Mirrors `Parser::skip_rtf_header`'s "jump to the first `\rtf` pattern"
/// behavior: bytes before that pattern (if any) are never visited by the
/// real parser either, so they aren't visited here.
pub(crate) fn find_header_boundary(buf: &[u8], scan_from: usize) -> Result<usize, usize> {
    let mut pos = if scan_from == 0 {
        const PATTERN: &[u8] = b"\\rtf";
        match buf.windows(PATTERN.len()).position(|w| w == PATTERN) {
            Some(p) => p,
            None => return Err(0),
        }
    } else {
        scan_from
    };

    loop {
        // A bare `\n`/`\r` (not preceded by `\`) is silently ignored by
        // `Parser::run_body_step`'s main dispatch (its own `b'\n' | b'\r'`
        // arm — advance only, never even considered text) — it must not
        // count as "body has begun" here either, or a header whose control
        // words happen to be separated by real newlines (common when RTF is
        // hand-formatted or line-wrapped) would end the header scan before
        // reaching `\fonttbl`/`\colortbl`.
        if matches!(buf.get(pos), Some(b'\n') | Some(b'\r')) {
            pos += 1;
            continue;
        }
        let Some((new_pos, tok)) = next_token(buf, pos) else {
            return Err(pos);
        };
        match tok {
            Token::ControlWord(_) => pos = new_pos,
            Token::GroupOpen => match classify_group(buf, new_pos) {
                None => return Err(pos),
                Some(GroupKind::Opaque) => match skip_balanced(buf, new_pos) {
                    Some(after) => pos = after,
                    None => return Err(pos),
                },
                Some(GroupKind::Transparent) => return Ok(pos),
            },
            Token::GroupClose | Token::Other => return Ok(pos),
        }
    }
}

/// Find the next safe body-increment boundary in `buf`: the position right
/// after the next top-level `\par`/`\pard` control word — "top-level"
/// meaning not nested inside a skip-destination or footnote/endnote group
/// (which `Parser::run_body_step` never lets a nested `\par` affect either —
/// those groups are consumed atomically by `skip_balanced_group`/
/// `parse_footnote_group`, so any `\par`-looking bytes inside them never
/// reach `handle_control_word` in the real parser). A `\par` nested inside
/// an ordinary transparent `{...}` run-formatting group *does* still count
/// (RTF producers don't normally nest paragraph breaks inside a formatting
/// run, but `Parser::run_body_step` doesn't special-case that — it processes
/// `\par` wherever it's dispatched — so this scanner must not either).
///
/// Precision here matters more than in [`find_header_boundary`]: the
/// returned position becomes an actual slice boundary fed to
/// `Parser::run_body_step` as one bounded, independently-parsed increment,
/// so it must land exactly where `Parser`'s own `read_control_word` would
/// have stopped for that `\par`/`\pard` — including whether it consumed a
/// trailing space delimiter — or a stray leading space would leak into (or
/// vanish from) the next increment's text. [`next_token`] replicates that
/// consumption exactly.
///
/// Returns `Err(resume_pos)` when `buf` runs out before a `\par`/`\pard` is
/// found — the caller should buffer more and retry from `resume_pos`. Since
/// a single RTF paragraph with no `\par` control word can only be considered
/// complete when [`Handler`](crate::batch::Handler)'s owner calls `finish()`
/// (there is no earlier general-purpose signal that it won't grow further),
/// this makes a StreamingParser's true increment granularity "one paragraph
/// (or one still-open table/list spanning several paragraphs)", not
/// "one token" — see `batch.rs`'s module doc for the memory-bound
/// consequence.
pub(crate) fn find_next_par_cut(buf: &[u8], scan_from: usize) -> Result<usize, usize> {
    let mut pos = scan_from;
    let mut opaque_depth: u32 = 0;

    loop {
        let Some((new_pos, tok)) = next_token(buf, pos) else {
            return Err(pos);
        };
        match tok {
            Token::GroupOpen => {
                if opaque_depth > 0 {
                    // Already inside an opaque region: any nested `{` just
                    // needs a matching `}` to get back out, whatever it is
                    // (matches `skip_balanced_group`'s flat brace counting —
                    // it never reclassifies nested groups).
                    opaque_depth += 1;
                } else {
                    match classify_group(buf, new_pos) {
                        None => return Err(pos),
                        Some(GroupKind::Opaque) => opaque_depth = 1,
                        Some(GroupKind::Transparent) => {} // scan continues inside it
                    }
                }
                pos = new_pos;
            }
            Token::GroupClose => {
                opaque_depth = opaque_depth.saturating_sub(1);
                pos = new_pos;
            }
            Token::ControlWord(word) => {
                if opaque_depth == 0 && (word == "par" || word == "pard") {
                    return Ok(new_pos);
                }
                pos = new_pos;
            }
            Token::Other => pos = new_pos,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_boundary_simple() {
        let input = br"{\rtf1\ansi\deff0{\fonttbl{\f0 Times;}}Hello\par}";
        let boundary = find_header_boundary(input, 0).expect("boundary found");
        assert_eq!(&input[boundary..boundary + 5], b"Hello");
    }

    #[test]
    fn header_boundary_empty_doc() {
        let input = br"{\rtf1}";
        let boundary = find_header_boundary(input, 0).expect("boundary found");
        assert_eq!(&input[boundary..], b"}");
    }

    #[test]
    fn header_boundary_needs_more_mid_group() {
        let input = br"{\rtf1{\fonttbl{\f0 Times";
        assert!(find_header_boundary(input, 0).is_err());
    }

    #[test]
    fn par_cut_basic() {
        let input = b"Hello\\par World";
        let cut = find_next_par_cut(input, 0).expect("cut found");
        assert_eq!(&input[..cut], b"Hello\\par ");
    }

    #[test]
    fn par_cut_suppressed_in_skip_group() {
        // Trailing " end" disambiguates the final `\par` (a control word
        // ending exactly at the buffer's end is legitimately ambiguous —
        // see `par_cut_needs_more_at_trailing_word_boundary` below).
        let input = b"{\\*\\fake \\par}More\\par end";
        let cut = find_next_par_cut(input, 0).expect("cut found");
        // The \par inside the `{\*...}` destination group must not count —
        // only the one after "More" should.
        assert_eq!(&input[..cut], b"{\\*\\fake \\par}More\\par ");
    }

    #[test]
    fn par_cut_needs_more_at_trailing_word_boundary() {
        // A control word running right up to the end of the buffer is
        // genuinely ambiguous (could `\par` continue as `\party` in the next
        // chunk?) — must defer, not guess.
        let input = b"Hello\\par";
        assert!(find_next_par_cut(input, 0).is_err());
    }

    #[test]
    fn par_cut_fires_inside_transparent_group() {
        let input = b"{\\b Bold \\par}Tail";
        let cut = find_next_par_cut(input, 0).expect("cut found");
        assert_eq!(&input[..cut], b"{\\b Bold \\par");
    }

    #[test]
    fn par_cut_needs_more() {
        let input = b"Hello world, no par yet";
        assert!(find_next_par_cut(input, 0).is_err());
    }

    #[test]
    fn next_token_bin_skips_payload() {
        let input = b"\\bin3\x7b\x7d\x00rest";
        let (pos, tok) = next_token(input, 0).expect("token");
        assert!(matches!(tok, Token::ControlWord(w) if w == "bin"));
        assert_eq!(&input[pos..], b"rest");
    }
}
