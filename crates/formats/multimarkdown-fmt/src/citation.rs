//! Recognizing MultiMarkdown's two citation forms and its cross-reference
//! form inside already-parsed CommonMark inline content.
//!
//! Neither construct has CommonMark grammar of its own — both are spelled
//! using ordinary link/reference-link bracket syntax that, because no
//! matching link definition exists, CommonMark's own parser (pulldown-cmark,
//! via commonmark-fmt) already leaves as literal bracket text per the
//! CommonMark spec's "unresolved reference link" fallback. This module's job
//! is exactly that fallback text: given a single contiguous string (the
//! content of one `Inline::Text` node), find and split out:
//!
//! - `[locator][#refname]` / `[][#refname]` — [`super::ast::MmdInline::Citation`]
//! - `[target]` / `[target][]` — [`super::ast::MmdInline::CrossReference`]
//!
//! # Scope
//!
//! Detection operates within a single contiguous text run. If a locator or
//! cross-reference label contains nested Markdown markup (e.g.
//! `[*emph*][#Doe:2006]`), pulldown-cmark still parses the emphasis as its
//! own `Inline::Emphasis` node, splitting the construct across multiple
//! sibling inlines — this scanner does not attempt to reassemble it, so it
//! is left as plain inline content, byte-for-byte identical to how a
//! generic CommonMark reader would render it. Nothing is lost: the
//! construct simply isn't upgraded to a `Citation`/`CrossReference` node in
//! that case. See `fixtures/multimarkdown/COVERAGE.md` for what is and
//! isn't covered.
//!
//! The MMD "inline citation content" form (`text.[#Full citation text.]`,
//! defining a citation's content inline rather than via a separate
//! `[#refname]:` definition) is not yet implemented — tracked in TODO.md.

use crate::ast::{MmdInline, Span};

/// Find the first unescaped-nesting `]` matching the `[` at `chars[open_idx]`.
/// Bails (`None`) on encountering a nested `[` before the close, keeping
/// detection conservative — see module docs.
fn find_matching_bracket<'a>(
    s: &'a str,
    chars: &[(usize, char)],
    open_idx: usize,
) -> Option<(usize, &'a str)> {
    let start_byte = chars[open_idx].0 + 1;
    let mut i = open_idx + 1;
    while i < chars.len() {
        let (byte_pos, c) = chars[i];
        match c {
            '[' => return None,
            ']' => return Some((i, &s[start_byte..byte_pos])),
            _ => {}
        }
        i += 1;
    }
    None
}

fn flush(buf: &mut String, out: &mut Vec<MmdInline>) {
    if !buf.is_empty() {
        out.push(MmdInline::Text {
            content: std::mem::take(buf),
            span: Span::NONE,
        });
    }
}

/// Scan `s` for citation and cross-reference patterns, returning a sequence
/// of `Text`/`Citation`/`CrossReference` inlines that concatenate back to
/// exactly `s` (modulo the structured nodes carrying their own content).
pub fn scan(s: &str) -> Vec<MmdInline> {
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let n = chars.len();
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut idx = 0usize;

    while idx < n {
        let (_, c) = chars[idx];
        if c != '[' {
            buf.push(c);
            idx += 1;
            continue;
        }

        let Some((close1, content1)) = find_matching_bracket(s, &chars, idx) else {
            buf.push(c);
            idx += 1;
            continue;
        };
        let after_first = close1 + 1;

        // Two-bracket forms: `[content1][content2]`.
        if after_first < n
            && chars[after_first].1 == '['
            && let Some((close2, content2)) = find_matching_bracket(s, &chars, after_first)
        {
            if let Some(label) = content2.strip_prefix('#') {
                flush(&mut buf, &mut out);
                out.push(MmdInline::Citation {
                    locator: (!content1.is_empty()).then(|| content1.to_string()),
                    label: label.to_string(),
                    span: Span::NONE,
                });
                idx = close2 + 1;
                continue;
            }
            if content2.is_empty()
                && !content1.is_empty()
                && !content1.starts_with('#')
                && !content1.starts_with('^')
            {
                flush(&mut buf, &mut out);
                out.push(MmdInline::CrossReference {
                    target: content1.to_string(),
                    collapsed: true,
                    span: Span::NONE,
                });
                idx = close2 + 1;
                continue;
            }
            // Any other two-bracket combination (e.g. a genuine but
            // unresolved `[text][other-label]` reference link) is not an
            // MMD construct — fall through to single-bracket handling.
        }

        // Single-bracket shortcut cross-reference: `[target]`.
        if !content1.is_empty() && !content1.starts_with('#') && !content1.starts_with('^') {
            flush(&mut buf, &mut out);
            out.push(MmdInline::CrossReference {
                target: content1.to_string(),
                collapsed: false,
                span: Span::NONE,
            });
            idx = after_first;
            continue;
        }

        buf.push(c);
        idx += 1;
    }

    flush(&mut buf, &mut out);
    out
}

/// Match a citation-definition prefix (`[#label]: ` or `[#label]:`) at the
/// very start of `s`. Returns `(label, rest_of_s_after_the_prefix)`.
pub fn match_definition_prefix(s: &str) -> Option<(&str, &str)> {
    let rest = s.strip_prefix('[')?;
    let rest = rest.strip_prefix('#')?;
    let close = rest.find(']')?;
    let label = &rest[..close];
    if label.is_empty() {
        return None;
    }
    let after_bracket = &rest[close + 1..];
    let after_colon = after_bracket.strip_prefix(':')?;
    let content = after_colon.strip_prefix(' ').unwrap_or(after_colon);
    Some((label, content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citation_with_locator() {
        let out = scan("See[p. 23][#Doe:2006] for details.");
        assert_eq!(
            out,
            vec![
                MmdInline::Text {
                    content: "See".into(),
                    span: Span::NONE
                },
                MmdInline::Citation {
                    locator: Some("p. 23".into()),
                    label: "Doe:2006".into(),
                    span: Span::NONE
                },
                MmdInline::Text {
                    content: " for details.".into(),
                    span: Span::NONE
                },
            ]
        );
    }

    #[test]
    fn citation_without_locator() {
        let out = scan("See[][#Doe:2006].");
        assert_eq!(
            out,
            vec![
                MmdInline::Text {
                    content: "See".into(),
                    span: Span::NONE
                },
                MmdInline::Citation {
                    locator: None,
                    label: "Doe:2006".into(),
                    span: Span::NONE
                },
                MmdInline::Text {
                    content: ".".into(),
                    span: Span::NONE
                },
            ]
        );
    }

    #[test]
    fn cross_reference_shortcut() {
        let out = scan("See [MultiMarkdownOverview] for more.");
        assert_eq!(
            out,
            vec![
                MmdInline::Text {
                    content: "See ".into(),
                    span: Span::NONE
                },
                MmdInline::CrossReference {
                    target: "MultiMarkdownOverview".into(),
                    collapsed: false,
                    span: Span::NONE
                },
                MmdInline::Text {
                    content: " for more.".into(),
                    span: Span::NONE
                },
            ]
        );
    }

    #[test]
    fn cross_reference_collapsed() {
        let out = scan("[Metadata][]");
        assert_eq!(
            out,
            vec![MmdInline::CrossReference {
                target: "Metadata".into(),
                collapsed: true,
                span: Span::NONE
            }]
        );
    }

    #[test]
    fn definition_prefix() {
        assert_eq!(
            match_definition_prefix("[#Doe:2006]: John Doe."),
            Some(("Doe:2006", "John Doe."))
        );
        assert_eq!(match_definition_prefix("Not a citation def"), None);
    }

    #[test]
    fn footnote_and_empty_brackets_left_alone() {
        assert_eq!(
            scan("[^1] and []"),
            vec![MmdInline::Text {
                content: "[^1] and []".into(),
                span: Span::NONE
            }]
        );
    }
}
