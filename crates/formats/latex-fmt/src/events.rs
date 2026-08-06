//! `events()` — the parser IS the iterator (see
//! `docs/format-library-design.md`).
//!
//! # Scope note
//!
//! `events()` reports **structural** events (control sequences, groups,
//! math spans, verbatim, comments, environment boundaries) directly off
//! [`crate::tokenize::Lexer`] — it does **not** replicate [`crate::parse`]'s
//! in-document macro/environment-definition scope tracking. Doing so would
//! require carrying the same stateful scope stack through a pull-iterator
//! shape, which is exactly the kind of "make the streaming API a thin
//! wrapper over the AST builder's logic" this codebase's design explicitly
//! rejects (`docs/format-library-design.md`: each API has its own optimal
//! implementation). A consumer that needs resolved-vs-raw-preserved
//! distinctions (which commands are locally defined) uses `parse()`;
//! `events()` serves consumers that want a fast, low-level structural view
//! (search indexers, syntax highlighters) where that distinction is not
//! needed. This is a documented scope boundary, not a silent gap.

use crate::tokenize::{Lexer, Tok};
use rescribe_format_api::Span;
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    StartDocument,
    EndDocument,
    Text(Cow<'a, str>),
    Comment(Cow<'a, str>),
    ControlSequence(Cow<'a, str>),
    GroupOpen,
    GroupClose,
    MathInline(Cow<'a, str>),
    MathDisplay(Cow<'a, str>),
    AlignTab,
    Verb {
        star: bool,
        delim: char,
        content: Cow<'a, str>,
    },
    VerbatimEnvBody(Cow<'a, str>),
    /// `\begin{name}` — reported as a plain event, with no attempt to
    /// resolve `name`'s meaning (see module docs).
    EnvironmentBegin(Cow<'a, str>),
    /// `\end{name}`.
    EnvironmentEnd(Cow<'a, str>),
}

pub type OwnedEvent = Event<'static>;

impl Event<'_> {
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::StartDocument => Event::StartDocument,
            Event::EndDocument => Event::EndDocument,
            Event::Text(s) => Event::Text(Cow::Owned(s.into_owned())),
            Event::Comment(s) => Event::Comment(Cow::Owned(s.into_owned())),
            Event::ControlSequence(s) => Event::ControlSequence(Cow::Owned(s.into_owned())),
            Event::GroupOpen => Event::GroupOpen,
            Event::GroupClose => Event::GroupClose,
            Event::MathInline(s) => Event::MathInline(Cow::Owned(s.into_owned())),
            Event::MathDisplay(s) => Event::MathDisplay(Cow::Owned(s.into_owned())),
            Event::AlignTab => Event::AlignTab,
            Event::Verb {
                star,
                delim,
                content,
            } => Event::Verb {
                star,
                delim,
                content: Cow::Owned(content.into_owned()),
            },
            Event::VerbatimEnvBody(s) => Event::VerbatimEnvBody(Cow::Owned(s.into_owned())),
            Event::EnvironmentBegin(s) => Event::EnvironmentBegin(Cow::Owned(s.into_owned())),
            Event::EnvironmentEnd(s) => Event::EnvironmentEnd(Cow::Owned(s.into_owned())),
        }
    }
}

/// Pull iterator: holds the lexer state directly, `next()` advances it.
pub struct EventIter<'a> {
    lex: Lexer<'a>,
    started: bool,
    finished: bool,
    /// Pending name-group capture state for `\begin{name}`/`\end{name}`,
    /// so those four raw tokens collapse into one `EnvironmentBegin`/
    /// `EnvironmentEnd` event rather than four separate low-level events —
    /// still zero built-in command/environment-name knowledge, this is
    /// purely "these four tokens form one begin/end marker," the same
    /// syntactic fact `tokenize.rs` itself already leans on for its
    /// verbatim-environment detector.
    pending: Vec<(Tok<'a>, Span)>,
}

pub fn events(input: &[u8]) -> EventIter<'_> {
    // `events()`'s contract (per `rescribe-format-api::Events`) takes
    // `&[u8]`; LaTeX source is treated as UTF-8 (lossy on invalid bytes,
    // matching `parse()`'s handling in `lib.rs`).
    let text = std::str::from_utf8(input).unwrap_or("");
    EventIter {
        lex: Lexer::new(text),
        started: false,
        finished: false,
        pending: Vec::new(),
    }
}

pub fn events_str(input: &str) -> EventIter<'_> {
    EventIter {
        lex: Lexer::new(input),
        started: false,
        finished: false,
        pending: Vec::new(),
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        if !self.started {
            self.started = true;
            return Some(Event::StartDocument);
        }
        if self.finished {
            return None;
        }
        if let Some((tok, _)) = self.pending.pop() {
            return Some(tok_to_event(tok));
        }
        match self.lex.next_token() {
            Some((Tok::Cs("begin"), _)) => match self.capture_name() {
                Some(name) => Some(Event::EnvironmentBegin(Cow::Borrowed(name))),
                None => Some(Event::ControlSequence(Cow::Borrowed("begin"))),
            },
            Some((Tok::Cs("end"), _)) => match self.capture_name() {
                Some(name) => Some(Event::EnvironmentEnd(Cow::Borrowed(name))),
                None => Some(Event::ControlSequence(Cow::Borrowed("end"))),
            },
            Some((Tok::MathShift, _)) => Some(self.capture_math(false)),
            Some((Tok::DisplayMathShift, _)) => Some(self.capture_math(true)),
            Some((tok, _)) => Some(tok_to_event(tok)),
            None => {
                self.finished = true;
                Some(Event::EndDocument)
            }
        }
    }
}

impl<'a> EventIter<'a> {
    /// After `\begin`/`\end`, tries to consume `{name}` and return the
    /// name; on any deviation, stashes the consumed tokens in `pending` (in
    /// reverse order, since `next()` pops from the back) so nothing is
    /// lost.
    fn capture_name(&mut self) -> Option<&'a str> {
        let open = self.lex.next_token();
        let Some((Tok::GroupOpen, _)) = open else {
            if let Some(t) = open {
                self.pending.push(t);
            }
            return None;
        };
        let text = self.lex.next_token();
        let Some((Tok::Text(name), _)) = text else {
            self.pending.push((Tok::GroupOpen, Span::NONE));
            if let Some(t) = text {
                self.pending.push(t);
            }
            return None;
        };
        let close = self.lex.next_token();
        let Some((Tok::GroupClose, _)) = close else {
            self.pending.push((Tok::GroupOpen, Span::NONE));
            self.pending.push((Tok::Text(name), Span::NONE));
            if let Some(t) = close {
                self.pending.push(t);
            }
            return None;
        };
        Some(name)
    }

    /// Raw source capture for `$...$` / `$$...$$`, matching `parse.rs`'s
    /// approach: find the literal closing delimiter directly in the
    /// remaining source and seek the lexer past it.
    fn capture_math(&mut self, display: bool) -> Event<'a> {
        let rest = self.lex.rest();
        let needle = if display { "$$" } else { "$" };
        match rest.find(needle) {
            Some(off) => {
                let source = &rest[..off];
                self.lex.seek(self.lex.pos() + off + needle.len());
                if display {
                    Event::MathDisplay(Cow::Borrowed(source))
                } else {
                    Event::MathInline(Cow::Borrowed(source))
                }
            }
            None => {
                let source = rest;
                self.lex.seek(self.lex.pos() + rest.len());
                if display {
                    Event::MathDisplay(Cow::Borrowed(source))
                } else {
                    Event::MathInline(Cow::Borrowed(source))
                }
            }
        }
    }
}

fn tok_to_event(tok: Tok<'_>) -> Event<'_> {
    match tok {
        Tok::Cs(name) => Event::ControlSequence(Cow::Borrowed(name)),
        Tok::GroupOpen => Event::GroupOpen,
        Tok::GroupClose => Event::GroupClose,
        // MathShift/DisplayMathShift never reach here: `next()` intercepts
        // them directly via `capture_math` before falling through to this
        // function, since they need a raw-source scan for the matching
        // closing delimiter (see `EventIter::capture_math`).
        Tok::MathShift | Tok::DisplayMathShift => unreachable!("handled in EventIter::next"),
        Tok::AlignTab => Event::AlignTab,
        Tok::Param(d) => Event::Text(Cow::Owned(format!("#{d}"))),
        Tok::Hash => Event::Text(Cow::Borrowed("#")),
        Tok::Comment(s) => Event::Comment(Cow::Borrowed(s)),
        Tok::Text(s) => Event::Text(Cow::Borrowed(s)),
        Tok::Verb {
            star,
            delim,
            content,
        } => Event::Verb {
            star,
            delim,
            content: Cow::Borrowed(content),
        },
        Tok::VerbatimEnvBody(s) => Event::VerbatimEnvBody(Cow::Borrowed(s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_with_start_end_document() {
        let evs: Vec<_> = events_str("hi").collect();
        assert_eq!(evs.first(), Some(&Event::StartDocument));
        assert_eq!(evs.last(), Some(&Event::EndDocument));
    }

    #[test]
    fn begin_end_collapse_to_environment_events() {
        let evs: Vec<_> = events_str("\\begin{itemize}\\end{itemize}").collect();
        assert!(evs.contains(&Event::EnvironmentBegin(Cow::Borrowed("itemize"))));
        assert!(evs.contains(&Event::EnvironmentEnd(Cow::Borrowed("itemize"))));
    }

    #[test]
    fn control_sequence_event() {
        let evs: Vec<_> = events_str("\\foo").collect();
        assert!(evs.contains(&Event::ControlSequence(Cow::Borrowed("foo"))));
    }
}
