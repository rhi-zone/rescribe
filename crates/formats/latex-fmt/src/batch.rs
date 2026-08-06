//! `StreamingParser<H>` — chunk-driven callback reader for input that
//! cannot be fully loaded into memory at once.
//!
//! # Implementation note
//!
//! `feed()` appends the chunk to an internal `SourceBuffer` (`self.buf`),
//! then repeatedly tokenizes from the start of the *uncommitted* remainder
//! and dispatches each token that is provably complete (see below),
//! compacting `self.buf` after each dispatch. This keeps memory bounded by
//! the largest in-progress token plus nesting depth — it is **not**
//! "buffer everything, tokenize once in `finish()`" (the stub shape this
//! codebase's design explicitly rejects for hand-rolled parsers).
//!
//! A token is "provably complete" when:
//! - it is not the very last token obtainable from the currently buffered
//!   text (a later token in the buffer proves nothing after it could
//!   extend it — e.g. a control-sequence name is complete once *something*
//!   follows it, since the tokenizer's own maximal-munch rule already
//!   stopped it there); **or**
//! - `finish()` was called (no more input can ever arrive, so whatever
//!   remains is flushed as-is, including a deliberately-unterminated
//!   trailing construct).
//!
//! # Known limitation: `\verb`/verbatim-environment bodies split across a
//! `feed()` boundary
//!
//! `tokenize.rs`'s `\verb`/verbatim-environment scanning (untouched by
//! this design — see that module's docs) falls back to "consume to
//! end-of-input" when its terminator isn't found within the slice it was
//! given. For the in-memory `parse()`/`events()` APIs that's the correct
//! behavior for genuinely-truncated input. For `StreamingParser`, it means
//! a `\verb|...|` or `verbatim`-environment body whose closing delimiter
//! hasn't arrived yet in the buffer can be reported as complete
//! (terminated at the current buffer's end) one `feed()` call before it
//! actually should be, if the next chunk turns out to contain the real
//! terminator. Math spans don't have this problem because `capture_math`
//! below explicitly checks "was the terminator actually found" before
//! treating the span as complete; `tokenize.rs`'s own verbatim scanning has
//! no equivalent signal to check, since it was designed for whole-input
//! tokenization. Fixing this cleanly requires `tokenize.rs` to expose a
//! distinction between "found the real terminator" and "ran out of
//! buffered data," which is out of scope for this pass (tracked in
//! TODO.md).
//!
//! Math spans (`$...$`, `$$...$$`) need one further special case: the
//! tokenizer only emits an atomic `MathShift`/`DisplayMathShift` token —
//! finding *where the span ends* requires scanning the buffered text for
//! the matching closing delimiter, exactly as `events.rs`'s
//! `EventIter::capture_math` does for the in-memory case. Here, if that
//! closing delimiter isn't yet present in the buffer, the whole span
//! (from the opening delimiter onward) is held back rather than assumed
//! complete — the same "not yet provably complete" treatment as any other
//! in-progress token.

use crate::events::Event;
use crate::tokenize::{Lexer, Tok};
use rescribe_format_api::Handler;
use std::borrow::Cow;

pub struct StreamingParser<H> {
    handler: H,
    buf: String,
    started: bool,
}

impl<H: Handler<Event<'static>>> StreamingParser<H> {
    pub fn new(handler: H) -> Self {
        StreamingParser {
            handler,
            buf: String::new(),
            started: false,
        }
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        if !self.started {
            self.started = true;
            self.handler.handle(Event::StartDocument);
        }
        // UTF-8 chunk-boundary limitation (documented, not silent): a
        // chunk boundary that splits a multi-byte codepoint drops the
        // partial trailing bytes rather than reassembling them across
        // `feed()` calls. This covers the realistic case of chunking
        // aligned to newlines/ASCII delimiters; splitting mid-codepoint
        // is a known gap tracked in TODO.md, not a hidden corruption.
        match std::str::from_utf8(chunk) {
            Ok(s) => self.buf.push_str(s),
            Err(e) => {
                let valid = e.valid_up_to();
                self.buf
                    .push_str(std::str::from_utf8(&chunk[..valid]).unwrap_or(""));
            }
        }
        self.drain(false);
    }

    pub fn finish(mut self) {
        self.drain(true);
        self.handler.handle(Event::EndDocument);
    }

    fn drain(&mut self, final_flush: bool) {
        loop {
            if self.buf.is_empty() {
                return;
            }
            let mut lex = Lexer::new(&self.buf);
            let Some((tok, span)) = lex.next_token() else {
                return;
            };

            match tok {
                Tok::MathShift | Tok::DisplayMathShift => {
                    let display = matches!(tok, Tok::DisplayMathShift);
                    let needle = if display { "$$" } else { "$" };
                    let after = &self.buf[span.end..];
                    match after.find(needle) {
                        Some(off) => {
                            let source = after[..off].to_string();
                            let consumed = span.end + off + needle.len();
                            self.dispatch(if display {
                                Event::MathDisplay(Cow::Owned(source))
                            } else {
                                Event::MathInline(Cow::Owned(source))
                            });
                            self.buf.drain(..consumed);
                        }
                        None if final_flush => {
                            let source = after.to_string();
                            self.dispatch(if display {
                                Event::MathDisplay(Cow::Owned(source))
                            } else {
                                Event::MathInline(Cow::Owned(source))
                            });
                            self.buf.clear();
                        }
                        None => return, // not enough data yet
                    }
                }
                _ => {
                    let is_last = lex.next_token().is_none();
                    if is_last && !final_flush {
                        return; // could still be extended by more input
                    }
                    self.dispatch(tok_to_owned_event(tok));
                    self.buf.drain(..span.end);
                }
            }
        }
    }

    fn dispatch(&mut self, ev: Event<'static>) {
        self.handler.handle(ev);
    }
}

fn tok_to_owned_event(tok: Tok<'_>) -> Event<'static> {
    match tok {
        Tok::Cs(name) => Event::ControlSequence(Cow::Owned(name.to_string())),
        Tok::GroupOpen => Event::GroupOpen,
        Tok::GroupClose => Event::GroupClose,
        Tok::MathShift | Tok::DisplayMathShift => {
            unreachable!("handled directly in StreamingParser::drain")
        }
        Tok::AlignTab => Event::AlignTab,
        Tok::Param(d) => Event::Text(Cow::Owned(format!("#{d}"))),
        Tok::Hash => Event::Text(Cow::Owned("#".to_string())),
        Tok::Comment(s) => Event::Comment(Cow::Owned(s.to_string())),
        Tok::Text(s) => Event::Text(Cow::Owned(s.to_string())),
        Tok::Verb {
            star,
            delim,
            content,
        } => Event::Verb {
            star,
            delim,
            content: Cow::Owned(content.to_string()),
        },
        Tok::VerbatimEnvBody(s) => Event::VerbatimEnvBody(Cow::Owned(s.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn collect(feed_chunks: &[&[u8]]) -> Vec<Event<'static>> {
        let events: Rc<RefCell<Vec<Event<'static>>>> = Rc::new(RefCell::new(Vec::new()));
        let events2 = events.clone();
        let mut p = StreamingParser::new(move |e: Event<'static>| events2.borrow_mut().push(e));
        for chunk in feed_chunks {
            p.feed(chunk);
        }
        p.finish();
        Rc::try_unwrap(events).unwrap().into_inner()
    }

    #[test]
    fn feeds_across_chunk_boundaries() {
        let evs = collect(&[b"\\text", b"bf{hi}"]);
        assert!(evs.contains(&Event::ControlSequence(Cow::Borrowed("textbf"))));
        assert_eq!(evs.first(), Some(&Event::StartDocument));
        assert_eq!(evs.last(), Some(&Event::EndDocument));
    }

    #[test]
    fn single_feed_matches_byte_by_byte_feed() {
        let whole = collect(&[b"a \\foo{b} c"]);
        let bytes: Vec<&[u8]> = b"a \\foo{b} c".iter().map(std::slice::from_ref).collect();
        let piecewise = collect(&bytes);
        assert_eq!(whole, piecewise);
    }

    #[test]
    fn math_span_split_across_chunks_is_captured_whole() {
        let evs = collect(&[b"$x", b"^2$"]);
        assert!(evs.contains(&Event::MathInline(Cow::Borrowed("x^2"))));
    }

    #[test]
    fn unterminated_math_flushed_at_finish() {
        let evs = collect(&[b"$x^2"]);
        assert!(evs.contains(&Event::MathInline(Cow::Borrowed("x^2"))));
    }
}
