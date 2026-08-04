//! Streaming event iterator over a MultiMarkdown document.
//!
//! `EventIter<'a>` wraps `commonmark_fmt::events::EventIter` (itself a true
//! streaming wrapper over pulldown-cmark's offset iterator) and layers MMD's
//! own recognition on top, without ever materializing an `MmdDoc`:
//!
//! - The leading metadata block is extracted once at construction (a bounded
//!   prefix scan — see `crate::metadata` — not a full-document buffer) and
//!   replayed as `Event::Metadata` events immediately after `StartDocument`.
//! - Every `Text` event is run through `crate::citation::scan` as it is
//!   pulled, splitting out `Event::Citation`/`Event::CrossReference` — O(one
//!   text token), no lookahead beyond the token already in hand.
//! - A citation-definition paragraph (`[#label]: ...`) is recognized with
//!   exactly one token of lookahead: on `StartParagraph`, the iterator pulls
//!   the *next* event to check whether it is a `Text` beginning with the
//!   definition prefix. If so, `StartParagraph`/`EndParagraph` are
//!   re-tagged as `StartCitationDefinition`/`EndCitationDefinition`; if not,
//!   the looked-ahead event is held in a single-slot `deferred` buffer and
//!   replayed first on the next `next()` call. Memory overhead is therefore
//!   O(1) event, not O(one paragraph) or O(document) — this is a true
//!   streaming iterator, not `parse()` wrapped in an adapter.

use std::borrow::Cow;
use std::collections::VecDeque;

use commonmark_fmt::events::Event as CmEvent;

use crate::ast::MmdInline;
use crate::citation;

/// A streaming event produced while iterating over a MultiMarkdown document.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    StartDocument,
    EndDocument,
    /// One `Key: value` metadata entry (see `crate::metadata`).
    Metadata {
        key: Cow<'a, str>,
        value: Cow<'a, str>,
    },
    /// Any event with no MMD-specific meaning, passed straight through from
    /// `commonmark_fmt::events::Event`.
    Cm(CmEvent<'a>),
    /// `[locator][#refname]` / `[][#refname]`.
    Citation {
        locator: Option<Cow<'a, str>>,
        label: Cow<'a, str>,
    },
    /// `[target]` / `[target][]`.
    CrossReference {
        target: Cow<'a, str>,
        collapsed: bool,
    },
    StartCitationDefinition {
        label: Cow<'a, str>,
    },
    EndCitationDefinition,
}

/// Owned counterpart of [`Event`] (`Event<'static>`), for use in batch/chunk
/// contexts (see [`crate::batch`]).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::StartDocument => Event::StartDocument,
            Event::EndDocument => Event::EndDocument,
            Event::Metadata { key, value } => Event::Metadata {
                key: Cow::Owned(key.into_owned()),
                value: Cow::Owned(value.into_owned()),
            },
            Event::Cm(ev) => Event::Cm(ev.into_owned()),
            Event::Citation { locator, label } => Event::Citation {
                locator: locator.map(|l| Cow::Owned(l.into_owned())),
                label: Cow::Owned(label.into_owned()),
            },
            Event::CrossReference { target, collapsed } => Event::CrossReference {
                target: Cow::Owned(target.into_owned()),
                collapsed,
            },
            Event::StartCitationDefinition { label } => Event::StartCitationDefinition {
                label: Cow::Owned(label.into_owned()),
            },
            Event::EndCitationDefinition => Event::EndCitationDefinition,
        }
    }
}

/// Return a streaming event iterator over a MultiMarkdown document, or
/// `None` if `input` is not valid UTF-8.
pub fn events(input: &[u8]) -> Option<EventIter<'_>> {
    std::str::from_utf8(input).ok().map(EventIter::new)
}

/// Return a streaming event iterator over a MultiMarkdown `&str`.
pub fn events_str(input: &str) -> EventIter<'_> {
    EventIter::new(input)
}

pub struct EventIter<'a> {
    inner: commonmark_fmt::events::EventIter<'a>,
    pending: VecDeque<Event<'a>>,
    deferred: Option<CmEvent<'a>>,
    finished: bool,
    in_citation_def: bool,
}

impl<'a> EventIter<'a> {
    pub fn new(input: &'a str) -> Self {
        let (metadata, _style, body) = crate::metadata::extract(input);
        let mut pending = VecDeque::new();
        pending.push_back(Event::StartDocument);
        for entry in metadata {
            pending.push_back(Event::Metadata {
                key: Cow::Owned(entry.key),
                value: Cow::Owned(entry.value),
            });
        }
        EventIter {
            inner: commonmark_fmt::events::events_str(body),
            pending,
            deferred: None,
            finished: false,
            in_citation_def: false,
        }
    }

    fn pull_inner(&mut self) -> Option<CmEvent<'a>> {
        self.deferred.take().or_else(|| self.inner.next())
    }

    /// Scan `text` for citation/cross-reference patterns and enqueue the
    /// resulting event(s). When nothing matches, the original `Cow` is
    /// forwarded unchanged (no allocation).
    fn enqueue_scanned_text(&mut self, text: Cow<'a, str>) {
        let pieces = citation::scan(&text);
        if let [MmdInline::Text { content, .. }] = pieces.as_slice()
            && content == text.as_ref()
        {
            self.pending.push_back(Event::Cm(CmEvent::Text(text)));
            return;
        }
        for piece in pieces {
            match piece {
                MmdInline::Text { content, .. } => self
                    .pending
                    .push_back(Event::Cm(CmEvent::Text(Cow::Owned(content)))),
                MmdInline::Citation { locator, label, .. } => {
                    self.pending.push_back(Event::Citation {
                        locator: locator.map(Cow::Owned),
                        label: Cow::Owned(label),
                    });
                }
                MmdInline::CrossReference {
                    target, collapsed, ..
                } => {
                    self.pending.push_back(Event::CrossReference {
                        target: Cow::Owned(target),
                        collapsed,
                    });
                }
                _ => unreachable!("citation::scan only produces Text/Citation/CrossReference"),
            }
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        loop {
            if let Some(ev) = self.pending.pop_front() {
                return Some(ev);
            }
            if self.finished {
                return None;
            }
            let Some(ev) = self.pull_inner() else {
                self.finished = true;
                return Some(Event::EndDocument);
            };
            match ev {
                CmEvent::StartDocument | CmEvent::EndDocument => continue,
                CmEvent::StartParagraph => match self.pull_inner() {
                    Some(CmEvent::Text(text)) => {
                        if let Some((label, remainder)) = citation::match_definition_prefix(&text) {
                            let label = label.to_string();
                            let remainder = remainder.to_string();
                            self.in_citation_def = true;
                            self.pending.push_back(Event::StartCitationDefinition {
                                label: Cow::Owned(label),
                            });
                            if !remainder.is_empty() {
                                self.enqueue_scanned_text(Cow::Owned(remainder));
                            }
                        } else {
                            self.deferred = Some(CmEvent::Text(text));
                            self.pending.push_back(Event::Cm(CmEvent::StartParagraph));
                        }
                    }
                    Some(other) => {
                        self.deferred = Some(other);
                        self.pending.push_back(Event::Cm(CmEvent::StartParagraph));
                    }
                    None => {
                        self.pending.push_back(Event::Cm(CmEvent::StartParagraph));
                    }
                },
                CmEvent::EndParagraph => {
                    if self.in_citation_def {
                        self.in_citation_def = false;
                        return Some(Event::EndCitationDefinition);
                    }
                    return Some(Event::Cm(CmEvent::EndParagraph));
                }
                CmEvent::Text(text) => self.enqueue_scanned_text(text),
                other => return Some(Event::Cm(other)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_events_precede_content() {
        let evs: Vec<_> = events_str("Title: Doc\n\nHello.\n").collect();
        assert_eq!(evs[0], Event::StartDocument);
        assert!(
            matches!(&evs[1], Event::Metadata { key, value } if key == "Title" && value == "Doc")
        );
    }

    #[test]
    fn citation_event() {
        let evs: Vec<_> = events_str("See[p. 23][#Doe:2006].\n").collect();
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::Citation { label, .. } if label == "Doe:2006"))
        );
    }

    #[test]
    fn citation_definition_events() {
        let evs: Vec<_> = events_str("[#Doe:2006]: John Doe.\n").collect();
        assert!(
            evs.iter().any(
                |e| matches!(e, Event::StartCitationDefinition { label } if label == "Doe:2006")
            )
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::EndCitationDefinition))
        );
    }

    #[test]
    fn ordinary_paragraph_unaffected() {
        let evs: Vec<_> = events_str("Just a paragraph.\n").collect();
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::Cm(CmEvent::StartParagraph)))
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::Cm(CmEvent::EndParagraph)))
        );
    }
}
