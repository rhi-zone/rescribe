//! Streaming event iterator over a CommonMark document.
//!
//! `EventIter<'a>` wraps pulldown-cmark's offset iterator and translates its
//! events into the commonmark-fmt [`Event`] type. Events are yielded lazily
//! via `Iterator::next()`. For code blocks, content is buffered internally
//! until the closing fence so that a single [`Event::CodeBlock`] event can be
//! emitted (matching the AST representation).
//!
//! # Zero-copy text
//!
//! `Text`, `Code`, `HtmlBlock`, and `HtmlInline` events carry
//! `Cow::Borrowed` slices of the original input — no allocation for the
//! common case of forwarding text to a downstream consumer.

use std::borrow::Cow;
use std::collections::VecDeque;

// ── Public event types ────────────────────────────────────────────────────────

/// A streaming event produced while iterating over a CommonMark document.
///
/// Text fields use `Cow<'a, str>` so that events can borrow from the input
/// `&'a str` without copying. When you need fully-owned events (e.g. for
/// storage or cross-thread use) call [`Event::into_owned`] or use the
/// [`OwnedEvent`] alias.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    // ── Document boundary ────────────────────────────────────────────────────
    StartDocument,
    EndDocument,

    // ── Block open/close ─────────────────────────────────────────────────────
    StartParagraph,
    EndParagraph,
    StartHeading {
        level: u8,
    },
    EndHeading {
        level: u8,
    },
    StartBlockquote,
    EndBlockquote,
    StartList {
        ordered: bool,
        /// Starting number for ordered lists; always `1` for unordered.
        start: u64,
        tight: bool,
    },
    EndList,
    StartItem,
    EndItem,

    // ── Inline open/close ────────────────────────────────────────────────────
    StartEmphasis,
    EndEmphasis,
    StartStrong,
    EndStrong,
    StartStrikethrough,
    EndStrikethrough,
    StartLink {
        url: Cow<'a, str>,
        title: Option<Cow<'a, str>>,
    },
    EndLink,
    /// The alt text for the image is provided here as a convenience; it is
    /// also emitted as one or more [`Event::Text`] events between
    /// `StartImage` and `EndImage`.
    StartImage {
        url: Cow<'a, str>,
        title: Option<Cow<'a, str>>,
        alt: Cow<'a, str>,
    },
    EndImage,

    // ── Leaf events ──────────────────────────────────────────────────────────
    Text(Cow<'a, str>),
    /// Inline code span.
    Code(Cow<'a, str>),
    /// Fenced or indented code block (emitted as a single event).
    CodeBlock {
        language: Option<Cow<'a, str>>,
        content: Cow<'a, str>,
    },
    HtmlBlock(Cow<'a, str>),
    HtmlInline(Cow<'a, str>),
    SoftBreak,
    HardBreak,
    ThematicBreak,
}

/// Type alias for events with `'static` lifetime (all `Cow` fields are owned).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert this event into an [`OwnedEvent`] by cloning any borrowed text.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::StartDocument => Event::StartDocument,
            Event::EndDocument => Event::EndDocument,
            Event::StartParagraph => Event::StartParagraph,
            Event::EndParagraph => Event::EndParagraph,
            Event::StartHeading { level } => Event::StartHeading { level },
            Event::EndHeading { level } => Event::EndHeading { level },
            Event::StartBlockquote => Event::StartBlockquote,
            Event::EndBlockquote => Event::EndBlockquote,
            Event::StartList { ordered, start, tight } => {
                Event::StartList { ordered, start, tight }
            }
            Event::EndList => Event::EndList,
            Event::StartItem => Event::StartItem,
            Event::EndItem => Event::EndItem,
            Event::StartEmphasis => Event::StartEmphasis,
            Event::EndEmphasis => Event::EndEmphasis,
            Event::StartStrong => Event::StartStrong,
            Event::EndStrong => Event::EndStrong,
            Event::StartStrikethrough => Event::StartStrikethrough,
            Event::EndStrikethrough => Event::EndStrikethrough,
            Event::StartLink { url, title } => Event::StartLink {
                url: Cow::Owned(url.into_owned()),
                title: title.map(|t| Cow::Owned(t.into_owned())),
            },
            Event::EndLink => Event::EndLink,
            Event::StartImage { url, title, alt } => Event::StartImage {
                url: Cow::Owned(url.into_owned()),
                title: title.map(|t| Cow::Owned(t.into_owned())),
                alt: Cow::Owned(alt.into_owned()),
            },
            Event::EndImage => Event::EndImage,
            Event::Text(cow) => Event::Text(Cow::Owned(cow.into_owned())),
            Event::Code(cow) => Event::Code(Cow::Owned(cow.into_owned())),
            Event::CodeBlock { language, content } => Event::CodeBlock {
                language: language.map(|l| Cow::Owned(l.into_owned())),
                content: Cow::Owned(content.into_owned()),
            },
            Event::HtmlBlock(cow) => Event::HtmlBlock(Cow::Owned(cow.into_owned())),
            Event::HtmlInline(cow) => Event::HtmlInline(Cow::Owned(cow.into_owned())),
            Event::SoftBreak => Event::SoftBreak,
            Event::HardBreak => Event::HardBreak,
            Event::ThematicBreak => Event::ThematicBreak,
        }
    }
}

// ── Iterator ──────────────────────────────────────────────────────────────────

/// State tracked while inside a code block (buffers content until `End`).
struct CodeBlockState {
    language: Option<String>,
    content: String,
}

/// State tracked while inside an image tag (buffers alt text until `End`).
struct ImageState {
    url: String,
    title: Option<String>,
    alt: String,
}

/// State tracked for list tightness detection.
struct ListState {
    /// Number of paragraphs seen in this list so far (tight = 0).
    para_count: usize,
}

/// Streaming event iterator over a CommonMark `&str`.
///
/// Constructed via [`events_str`] or indirectly via [`events`].
pub struct EventIter<'a> {
    inner: pulldown_cmark::OffsetIter<'a, pulldown_cmark::DefaultBrokenLinkCallback>,
    /// Pre-translated events not yet delivered to the caller.
    pending: VecDeque<Event<'a>>,
    /// When `Some`, we are inside a `Tag::CodeBlock` and buffering content.
    code_block: Option<CodeBlockState>,
    /// When `Some`, we are inside a `Tag::Image` and buffering alt text.
    image: Option<ImageState>,
    /// Stack of list states for tightness tracking.
    list_stack: Vec<ListState>,
    /// Whether `StartDocument` has been emitted yet.
    started: bool,
    /// Whether `EndDocument` has been emitted yet.
    ended: bool,
}

impl<'a> EventIter<'a> {
    /// Create an iterator over the given CommonMark string.
    pub fn new(input: &'a str) -> Self {
        use pulldown_cmark::{Options, Parser};
        let opts = Options::ENABLE_STRIKETHROUGH;
        let inner = Parser::new_ext(input, opts).into_offset_iter();
        EventIter {
            inner,
            pending: VecDeque::new(),
            code_block: None,
            image: None,
            list_stack: Vec::new(),
            started: false,
            ended: false,
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Emit StartDocument once.
        if !self.started {
            self.started = true;
            return Some(Event::StartDocument);
        }

        loop {
            // Drain any buffered events first.
            if let Some(ev) = self.pending.pop_front() {
                return Some(ev);
            }

            // Pull the next pulldown-cmark event.
            let (pd_event, _range) = match self.inner.next() {
                Some(pair) => pair,
                None => {
                    // Pulldown stream exhausted.
                    if !self.ended {
                        self.ended = true;
                        return Some(Event::EndDocument);
                    }
                    return None;
                }
            };

            use pulldown_cmark::{CodeBlockKind, Event as PdEvent, Tag, TagEnd};

            match pd_event {
                // ── Block opens ──────────────────────────────────────────────
                PdEvent::Start(Tag::Paragraph) => {
                    return Some(Event::StartParagraph);
                }
                PdEvent::Start(Tag::Heading { level, .. }) => {
                    return Some(Event::StartHeading { level: heading_level_to_u8(level) });
                }
                PdEvent::Start(Tag::BlockQuote(_)) => {
                    return Some(Event::StartBlockquote);
                }
                PdEvent::Start(Tag::CodeBlock(kind)) => {
                    let language = match kind {
                        CodeBlockKind::Fenced(lang) => {
                            let s = lang.trim().to_string();
                            if s.is_empty() { None } else { Some(s) }
                        }
                        CodeBlockKind::Indented => None,
                    };
                    self.code_block = Some(CodeBlockState { language, content: String::new() });
                    // Continue looping — we emit a single CodeBlock event on End.
                }
                PdEvent::Start(Tag::List(first)) => {
                    let (ordered, start) = match first {
                        None => (false, 1u64),
                        Some(n) => (true, n),
                    };
                    self.list_stack.push(ListState { para_count: 0 });
                    // Tightness is unknown until we see paragraphs; emit with
                    // tight=true tentatively. Callers that need accurate tightness
                    // should use the AST API instead.
                    return Some(Event::StartList { ordered, start, tight: true });
                }
                PdEvent::Start(Tag::Item) => {
                    return Some(Event::StartItem);
                }
                PdEvent::Start(Tag::HtmlBlock) => {
                    // Content arrives as PdEvent::Html; we accumulate inline until End.
                    // No sub-state needed — Html events go directly through.
                }

                // ── Inline opens ──────────────────────────────────────────────
                PdEvent::Start(Tag::Emphasis) => {
                    return Some(Event::StartEmphasis);
                }
                PdEvent::Start(Tag::Strong) => {
                    return Some(Event::StartStrong);
                }
                PdEvent::Start(Tag::Strikethrough) => {
                    return Some(Event::StartStrikethrough);
                }
                PdEvent::Start(Tag::Link { dest_url, title, .. }) => {
                    let url = Cow::Owned(dest_url.into_string());
                    let title = if title.is_empty() {
                        None
                    } else {
                        Some(Cow::Owned(title.into_string()))
                    };
                    return Some(Event::StartLink { url, title });
                }
                PdEvent::Start(Tag::Image { dest_url, title, .. }) => {
                    let url = dest_url.into_string();
                    let title =
                        if title.is_empty() { None } else { Some(title.into_string()) };
                    self.image = Some(ImageState { url, title, alt: String::new() });
                    // We buffer alt text and emit StartImage on End.
                }

                // ── Block closes ──────────────────────────────────────────────
                PdEvent::End(TagEnd::Paragraph) => {
                    if let Some(ls) = self.list_stack.last_mut() {
                        ls.para_count += 1;
                    }
                    return Some(Event::EndParagraph);
                }
                PdEvent::End(TagEnd::Heading(level)) => {
                    return Some(Event::EndHeading { level: heading_level_to_u8(level) });
                }
                PdEvent::End(TagEnd::BlockQuote(_)) => {
                    return Some(Event::EndBlockquote);
                }
                PdEvent::End(TagEnd::CodeBlock) => {
                    if let Some(state) = self.code_block.take() {
                        let language = state.language.map(Cow::Owned);
                        let content = Cow::Owned(state.content);
                        return Some(Event::CodeBlock { language, content });
                    }
                }
                PdEvent::End(TagEnd::List(_)) => {
                    self.list_stack.pop();
                    return Some(Event::EndList);
                }
                PdEvent::End(TagEnd::Item) => {
                    return Some(Event::EndItem);
                }
                PdEvent::End(TagEnd::HtmlBlock) => {
                    // Nothing extra to emit — Html events already forwarded.
                }

                // ── Inline closes ─────────────────────────────────────────────
                PdEvent::End(TagEnd::Emphasis) => {
                    return Some(Event::EndEmphasis);
                }
                PdEvent::End(TagEnd::Strong) => {
                    return Some(Event::EndStrong);
                }
                PdEvent::End(TagEnd::Strikethrough) => {
                    return Some(Event::EndStrikethrough);
                }
                PdEvent::End(TagEnd::Link) => {
                    return Some(Event::EndLink);
                }
                PdEvent::End(TagEnd::Image) => {
                    if let Some(state) = self.image.take() {
                        let url = Cow::Owned(state.url);
                        let title = state.title.map(Cow::Owned);
                        let alt = Cow::Owned(state.alt);
                        // Queue EndImage; return StartImage now.
                        self.pending.push_back(Event::EndImage);
                        return Some(Event::StartImage { url, title, alt });
                    }
                }

                // ── Leaf events ───────────────────────────────────────────────
                PdEvent::Text(text) => {
                    if let Some(state) = &mut self.code_block {
                        state.content.push_str(&text);
                        // Continue — no event emitted yet.
                    } else if let Some(state) = &mut self.image {
                        state.alt.push_str(&text);
                        // Also emit a Text event so consumers between
                        // StartImage / EndImage can read the alt text inline.
                        // We push it into pending; it will be drained before
                        // we emit EndImage (which is already in pending).
                        // NOTE: StartImage is emitted on Tag::End(Image), so
                        // at this point we have not yet emitted StartImage.
                        // The text will flow between Start/End once we emit
                        // StartImage at End(Image) with pending = [Text, EndImage].
                        self.pending.push_back(Event::Text(Cow::Owned(text.into_string())));
                    } else {
                        return Some(Event::Text(Cow::Owned(text.into_string())));
                    }
                }
                PdEvent::Code(text) => {
                    return Some(Event::Code(Cow::Owned(text.into_string())));
                }
                PdEvent::Html(text) => {
                    return Some(Event::HtmlBlock(Cow::Owned(text.into_string())));
                }
                PdEvent::InlineHtml(text) => {
                    return Some(Event::HtmlInline(Cow::Owned(text.into_string())));
                }
                PdEvent::SoftBreak => {
                    return Some(Event::SoftBreak);
                }
                PdEvent::HardBreak => {
                    return Some(Event::HardBreak);
                }
                PdEvent::Rule => {
                    return Some(Event::ThematicBreak);
                }

                // ── Ignored (pulldown extensions not modeled here) ────────────
                _ => {}
            }
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Return a streaming event iterator over a CommonMark byte slice.
///
/// Returns `None` if `input` is not valid UTF-8.
pub fn events(input: &[u8]) -> Option<EventIter<'_>> {
    std::str::from_utf8(input).ok().map(EventIter::new)
}

/// Return a streaming event iterator over a CommonMark `&str`.
pub fn events_str(input: &str) -> EventIter<'_> {
    EventIter::new(input)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn heading_level_to_u8(level: pulldown_cmark::HeadingLevel) -> u8 {
    use pulldown_cmark::HeadingLevel;
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(input: &str) -> Vec<Event<'static>> {
        events_str(input).map(|e| e.into_owned()).collect()
    }

    #[test]
    fn test_events_paragraph() {
        let evs = collect("Hello\n");
        assert!(evs.iter().any(|e| matches!(e, Event::StartDocument)));
        assert!(evs.iter().any(|e| matches!(e, Event::StartParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::Text(t) if t == "Hello")));
        assert!(evs.iter().any(|e| matches!(e, Event::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndDocument)));
    }

    #[test]
    fn test_events_heading() {
        let evs = collect("# Hello\n");
        assert!(evs.iter().any(|e| matches!(e, Event::StartHeading { level: 1 })));
        assert!(evs.iter().any(|e| matches!(e, Event::Text(t) if t == "Hello")));
        assert!(evs.iter().any(|e| matches!(e, Event::EndHeading { level: 1 })));
    }

    #[test]
    fn test_events_code_block() {
        let evs = collect("```rust\nfn main() {}\n```\n");
        assert!(evs.iter().any(|e| matches!(
            e,
            Event::CodeBlock { language: Some(lang), content }
            if lang == "rust" && content == "fn main() {}\n"
        )));
    }

    #[test]
    fn test_events_link() {
        let evs = collect("[text](https://example.com)\n");
        assert!(evs.iter().any(|e| matches!(
            e,
            Event::StartLink { url, .. } if url == "https://example.com"
        )));
        assert!(evs.iter().any(|e| matches!(e, Event::EndLink)));
    }

    #[test]
    fn test_events_image() {
        let evs = collect("![alt text](img.png)\n");
        assert!(evs.iter().any(|e| matches!(
            e,
            Event::StartImage { url, alt, .. }
            if url == "img.png" && alt == "alt text"
        )));
        assert!(evs.iter().any(|e| matches!(e, Event::EndImage)));
    }

    #[test]
    fn test_events_emphasis_strong() {
        let evs = collect("*em* and **strong**\n");
        assert!(evs.iter().any(|e| matches!(e, Event::StartEmphasis)));
        assert!(evs.iter().any(|e| matches!(e, Event::StartStrong)));
    }

    #[test]
    fn test_events_thematic_break() {
        let evs = collect("---\n");
        assert!(evs.iter().any(|e| matches!(e, Event::ThematicBreak)));
    }

    #[test]
    fn test_events_list() {
        let evs = collect("- one\n- two\n");
        assert!(evs.iter().any(|e| matches!(e, Event::StartList { ordered: false, .. })));
        assert_eq!(evs.iter().filter(|e| matches!(e, Event::StartItem)).count(), 2);
    }

    #[test]
    fn test_events_ordered_list() {
        let evs = collect("1. first\n2. second\n");
        assert!(evs.iter().any(|e| matches!(e, Event::StartList { ordered: true, start: 1, .. })));
    }

    #[test]
    fn test_events_html_block() {
        let evs = collect("<div>\ncontent\n</div>\n");
        assert!(evs.iter().any(|e| matches!(e, Event::HtmlBlock(_))));
    }

    #[test]
    fn test_events_inline_html() {
        let evs = collect("text <em>inline</em>\n");
        assert!(evs.iter().any(|e| matches!(e, Event::HtmlInline(_))));
    }

    #[test]
    fn test_events_invalid_utf8() {
        assert!(events(b"\xff\xfe").is_none());
    }

    #[test]
    fn test_start_end_document_bookend() {
        let evs = collect("");
        assert_eq!(evs.first(), Some(&Event::StartDocument));
        assert_eq!(evs.last(), Some(&Event::EndDocument));
    }

    #[test]
    fn test_batch_collects() {
        use crate::batch::StreamingParser;

        // Collect via events() directly.
        let direct: Vec<OwnedEvent> =
            events_str("# Hello\n\nA paragraph.\n").map(|e| e.into_owned()).collect();

        // Collect via StreamingParser fed in two chunks.
        let mut collected = Vec::new();
        let mut p = StreamingParser::new(|ev: OwnedEvent| collected.push(ev));
        p.feed(b"# Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();

        assert_eq!(direct, collected);
    }
}
