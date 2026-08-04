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

#[cfg(feature = "tables")]
use crate::options::ColumnAlignment;
#[cfg(feature = "frontmatter")]
use crate::options::FrontMatterKind;

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
    /// Corrects a list's tightness once it becomes fully known.
    ///
    /// CommonMark list tightness ("is any item blank-line-separated from
    /// its neighbor, or does any item directly contain two block-level
    /// children separated by a blank line") is a property of the *entire*
    /// list, uniform across every item — pulldown-cmark itself only wraps
    /// item content in real `Paragraph` tags when the whole list is loose,
    /// never per-item. Determining it can require seeing every item, which
    /// isn't bounded by a constant lookahead, so `EventIter` cannot always
    /// know it in time for `StartList`.
    ///
    /// `EventIter` therefore always emits `StartList { tight: true, .. }`
    /// optimistically (the common case: most lists are tight) and, only if
    /// the list turns out loose, emits this event exactly once, immediately
    /// before the matching `EndList`, to correct it. It is never emitted
    /// for a genuinely tight list — the optimistic default was already
    /// right, so there is nothing to correct. Applies to the most recently
    /// opened `StartList` that hasn't yet seen its `EndList` — the same
    /// nesting correlation `StartItem`/`EndItem` use with `StartList`.
    ListTightnessResolved {
        tight: bool,
    },
    /// `checked` is `Some` for GFM task-list items (`- [ ]`/`- [x]`), `None`
    /// for ordinary items. Always `None` when the `task-lists` feature is off.
    StartItem {
        #[cfg(feature = "task-lists")]
        checked: Option<bool>,
    },
    EndItem,

    // ── Table open/close (`tables` feature) ──────────────────────────────────
    #[cfg(feature = "tables")]
    StartTable {
        alignments: Vec<ColumnAlignment>,
    },
    #[cfg(feature = "tables")]
    EndTable,
    #[cfg(feature = "tables")]
    StartTableHead,
    #[cfg(feature = "tables")]
    EndTableHead,
    #[cfg(feature = "tables")]
    StartTableRow,
    #[cfg(feature = "tables")]
    EndTableRow,
    #[cfg(feature = "tables")]
    StartTableCell,
    #[cfg(feature = "tables")]
    EndTableCell,

    // ── Front matter (`frontmatter` feature) — single self-contained event ──
    #[cfg(feature = "frontmatter")]
    FrontMatter {
        kind: FrontMatterKind,
        content: Cow<'a, str>,
    },

    /// A reference-style link definition (`[label]: url "title"`), mirroring
    /// [`crate::ast::LinkDef`] field-for-field. pulldown-cmark's own event
    /// stream never surfaces these — it resolves reference links to their
    /// target inline, silently — so `EventIter` computes them the same way
    /// `parse()` does (via `Parser::reference_definitions()`) and emits one
    /// event per definition after the document body, before `EndDocument`
    /// (there is no footnote-defs section in CommonMark to place them
    /// before, unlike djot).
    LinkDef {
        label: Cow<'a, str>,
        url: Cow<'a, str>,
        title: Option<Cow<'a, str>>,
    },

    // ── Inline open/close ────────────────────────────────────────────────────
    StartEmphasis,
    EndEmphasis,
    StartStrong,
    EndStrong,
    #[cfg(feature = "strikethrough")]
    StartStrikethrough,
    #[cfg(feature = "strikethrough")]
    EndStrikethrough,
    StartLink {
        url: Cow<'a, str>,
        title: Option<Cow<'a, str>>,
    },
    EndLink,
    /// The alt text for the image is provided here as a convenience; it is
    /// also emitted as a single [`Event::Text`] event between `StartImage`
    /// and `EndImage` (omitted when alt text is empty), matching the AST
    /// projection.
    StartImage {
        url: Cow<'a, str>,
        title: Option<Cow<'a, str>>,
        alt: Cow<'a, str>,
    },
    EndImage,

    // ── Footnotes (`footnotes` feature) ──────────────────────────────────────
    /// A footnote definition (`[^label]: content`) — behaves exactly like
    /// `StartBlockquote`/`EndBlockquote` (content is always block-level, with
    /// no tight-inline shortcut; pulldown-cmark always wraps footnote
    /// definition content in real `Paragraph` tags).
    #[cfg(feature = "footnotes")]
    StartFootnoteDefinition {
        label: Cow<'a, str>,
    },
    #[cfg(feature = "footnotes")]
    EndFootnoteDefinition,
    /// A reference to a footnote (`[^label]`).
    #[cfg(feature = "footnotes")]
    FootnoteReference {
        label: Cow<'a, str>,
    },

    // ── Definition lists (`definition-lists` feature) ───────────────────────
    #[cfg(feature = "definition-lists")]
    StartDefinitionList,
    #[cfg(feature = "definition-lists")]
    EndDefinitionList,
    /// Corrects a definition list's tightness once it becomes fully known —
    /// same mechanism and same reason as [`Event::ListTightnessResolved`]
    /// (see its doc comment): `Tag::DefinitionList` carries no tightness bit
    /// of its own, so `EventIter` emits `StartDefinitionList` without a
    /// tightness field and, if the list turns out loose, emits this event
    /// exactly once, immediately before the matching `EndDefinitionList`.
    #[cfg(feature = "definition-lists")]
    DefinitionListTightnessResolved {
        tight: bool,
    },
    /// A definition list term (`dt`). Always inline-only content —
    /// pulldown-cmark never wraps a title in its own nested `Paragraph` tag,
    /// so (unlike `StartItem`/`StartDefinitionListDefinition`) this needs no
    /// synthetic-paragraph handling.
    #[cfg(feature = "definition-lists")]
    StartDefinitionListTitle,
    #[cfg(feature = "definition-lists")]
    EndDefinitionListTitle,
    /// A single definition (`dd`) body. Like `StartItem`, may be followed
    /// directly by bare inline events (tight) with no wrapping `Paragraph`
    /// tag — `EventIter` synthesizes `StartParagraph`/`EndParagraph` around
    /// such runs the same way it does for tight list items.
    #[cfg(feature = "definition-lists")]
    StartDefinitionListDefinition,
    #[cfg(feature = "definition-lists")]
    EndDefinitionListDefinition,

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
    /// Inline math (`$math$`, `math` feature).
    #[cfg(feature = "math")]
    InlineMath(Cow<'a, str>),
    /// Display math (`$$math$$`, `math` feature).
    #[cfg(feature = "math")]
    DisplayMath(Cow<'a, str>),
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
            Event::StartList {
                ordered,
                start,
                tight,
            } => Event::StartList {
                ordered,
                start,
                tight,
            },
            Event::EndList => Event::EndList,
            Event::ListTightnessResolved { tight } => Event::ListTightnessResolved { tight },
            #[cfg(feature = "task-lists")]
            Event::StartItem { checked } => Event::StartItem { checked },
            #[cfg(not(feature = "task-lists"))]
            Event::StartItem {} => Event::StartItem {},
            Event::EndItem => Event::EndItem,
            #[cfg(feature = "tables")]
            Event::StartTable { alignments } => Event::StartTable { alignments },
            #[cfg(feature = "tables")]
            Event::EndTable => Event::EndTable,
            #[cfg(feature = "tables")]
            Event::StartTableHead => Event::StartTableHead,
            #[cfg(feature = "tables")]
            Event::EndTableHead => Event::EndTableHead,
            #[cfg(feature = "tables")]
            Event::StartTableRow => Event::StartTableRow,
            #[cfg(feature = "tables")]
            Event::EndTableRow => Event::EndTableRow,
            #[cfg(feature = "tables")]
            Event::StartTableCell => Event::StartTableCell,
            #[cfg(feature = "tables")]
            Event::EndTableCell => Event::EndTableCell,
            #[cfg(feature = "frontmatter")]
            Event::FrontMatter { kind, content } => Event::FrontMatter {
                kind,
                content: Cow::Owned(content.into_owned()),
            },
            Event::LinkDef { label, url, title } => Event::LinkDef {
                label: Cow::Owned(label.into_owned()),
                url: Cow::Owned(url.into_owned()),
                title: title.map(|t| Cow::Owned(t.into_owned())),
            },
            Event::StartEmphasis => Event::StartEmphasis,
            Event::EndEmphasis => Event::EndEmphasis,
            Event::StartStrong => Event::StartStrong,
            Event::EndStrong => Event::EndStrong,
            #[cfg(feature = "strikethrough")]
            Event::StartStrikethrough => Event::StartStrikethrough,
            #[cfg(feature = "strikethrough")]
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
            #[cfg(feature = "footnotes")]
            Event::StartFootnoteDefinition { label } => Event::StartFootnoteDefinition {
                label: Cow::Owned(label.into_owned()),
            },
            #[cfg(feature = "footnotes")]
            Event::EndFootnoteDefinition => Event::EndFootnoteDefinition,
            #[cfg(feature = "footnotes")]
            Event::FootnoteReference { label } => Event::FootnoteReference {
                label: Cow::Owned(label.into_owned()),
            },
            #[cfg(feature = "definition-lists")]
            Event::StartDefinitionList => Event::StartDefinitionList,
            #[cfg(feature = "definition-lists")]
            Event::EndDefinitionList => Event::EndDefinitionList,
            #[cfg(feature = "definition-lists")]
            Event::DefinitionListTightnessResolved { tight } => {
                Event::DefinitionListTightnessResolved { tight }
            }
            #[cfg(feature = "definition-lists")]
            Event::StartDefinitionListTitle => Event::StartDefinitionListTitle,
            #[cfg(feature = "definition-lists")]
            Event::EndDefinitionListTitle => Event::EndDefinitionListTitle,
            #[cfg(feature = "definition-lists")]
            Event::StartDefinitionListDefinition => Event::StartDefinitionListDefinition,
            #[cfg(feature = "definition-lists")]
            Event::EndDefinitionListDefinition => Event::EndDefinitionListDefinition,
            #[cfg(feature = "math")]
            Event::InlineMath(cow) => Event::InlineMath(Cow::Owned(cow.into_owned())),
            #[cfg(feature = "math")]
            Event::DisplayMath(cow) => Event::DisplayMath(Cow::Owned(cow.into_owned())),
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
///
/// Starts `true` (mirroring `StartList`'s optimistic default) and flips to
/// `false` the moment a *real* `Paragraph` tag is seen as a direct child of
/// one of this list's items — never back to `true`, since pulldown-cmark
/// only emits real `Paragraph` tags for item content when the whole list is
/// loose (see `Event::ListTightnessResolved`'s doc comment).
struct ListState {
    tight: bool,
}

/// Which kind of tight-inline-shortcut container a [`TightFrame`] represents.
/// `Item` and (when the `definition-lists` feature is on)
/// `DefinitionListDefinition` are structurally identical from `EventIter`'s
/// point of view — both may receive bare inline content with no wrapping
/// `Paragraph` tag when their enclosing list/definition-list is tight — but
/// the tightness-correction signal they produce targets a different stack
/// (`list_stack` vs `def_list_stack`), so the frame remembers which.
enum TightFrameKind {
    Item,
    #[cfg(feature = "definition-lists")]
    DefinitionListDefinition,
}

/// State tracked per currently-open tight-inline-shortcut container (a list
/// item or, with `definition-lists`, a definition body).
struct TightFrame {
    kind: TightFrameKind,
    /// Whether a synthetic `StartParagraph` is currently open for this
    /// container (see `tight_stack`'s doc comment on `EventIter`).
    synthetic_open: bool,
    /// Nesting depth of blockquotes (and, with `footnotes`, footnote
    /// definitions — see `StartFootnoteDefinition`'s handling) opened since
    /// this container began (relative depth, not absolute document depth).
    /// A real `Paragraph` nested inside `Item > Blockquote > Paragraph` is
    /// never surgerized by pulldown-cmark regardless of list tightness (its
    /// own tight-list rewrite only touches a container's *direct* block
    /// children), so it must not be mistaken for the tight/loose signal —
    /// the signal only counts when `quote_depth == 0`.
    quote_depth: u32,
}

/// State tracked for definition-list tightness detection — mirrors
/// [`ListState`] exactly, one level for `list_stack`'s definition-list
/// counterpart.
#[cfg(feature = "definition-lists")]
struct DefListState {
    tight: bool,
}

/// Streaming event iterator over a CommonMark `&str`.
///
/// Constructed via [`events_str`] or indirectly via [`events`].
pub struct EventIter<'a> {
    inner: pulldown_cmark::OffsetIter<'a, pulldown_cmark::DefaultBrokenLinkCallback>,
    /// Pre-translated events not yet delivered to the caller.
    pending: VecDeque<Event<'a>>,
    /// Raw pulldown-cmark events peeked ahead (e.g. to check for a
    /// `TaskListMarker` following `Tag::Item`) and not yet consumed.
    pending_pd: VecDeque<(pulldown_cmark::Event<'a>, std::ops::Range<usize>)>,
    /// When `Some`, we are inside a `Tag::CodeBlock` and buffering content.
    code_block: Option<CodeBlockState>,
    /// When `Some`, we are inside a `Tag::Image` and buffering alt text.
    image: Option<ImageState>,
    /// Stack of list states for tightness tracking.
    list_stack: Vec<ListState>,
    /// Stack of definition-list states for tightness tracking — mirrors
    /// `list_stack`, one entry per currently open `Tag::DefinitionList`.
    #[cfg(feature = "definition-lists")]
    def_list_stack: Vec<DefListState>,
    /// Stack of per-container synthetic-paragraph state, one entry per
    /// currently open `Tag::Item` or (with `definition-lists`)
    /// `Tag::DefinitionListDefinition`. `true` means a synthetic
    /// `StartParagraph` is currently open for that container (see the
    /// pre-dispatch gate in `next()`). pulldown-cmark omits
    /// `Start/End(Paragraph)` around such a container's bare inline content
    /// when tight, but `parse()`'s AST always wraps that content in an
    /// implicit `Block::Paragraph` (`parse.rs`'s `flush_tight_inlines`) —
    /// this stack lets `EventIter` synthesize the matching
    /// `StartParagraph`/`EndParagraph` pair without buffering the
    /// container. Also carries the per-container state needed for
    /// tightness detection (see `TightFrame`).
    tight_stack: Vec<TightFrame>,
    /// Depth counter for "real" inline-bearing containers that already
    /// come with their own explicit open/close tags (`Paragraph`,
    /// `Heading`, table cells) — incremented/decremented alongside those
    /// events. The synthetic-paragraph gate only applies at depth 0;
    /// inline content inside a real `Paragraph` (loose list items) must
    /// never be double-wrapped.
    text_container_depth: u32,
    /// Whether `StartDocument` has been emitted yet.
    started: bool,
    /// Whether `EndDocument` has been emitted yet.
    ended: bool,
    /// Reference-style link definitions, computed eagerly at construction
    /// (mirrors `parse()`'s `collect_link_defs` — pulldown-cmark's event
    /// stream never surfaces them). Drained into `pending` as `LinkDef`
    /// events once the pulldown stream is exhausted, right before
    /// `EndDocument`. `None` once drained.
    link_defs: Option<Vec<crate::ast::LinkDef>>,
}

impl<'a> EventIter<'a> {
    /// Create an iterator over the given CommonMark string.
    pub fn new(input: &'a str) -> Self {
        use pulldown_cmark::Parser;
        let opts = crate::options::build_options();
        // Reference definitions must be read off `Parser` (via a shared
        // reference) before `into_offset_iter()` consumes it.
        let link_defs = crate::parse::collect_link_defs(input);
        let inner = Parser::new_ext(input, opts).into_offset_iter();
        EventIter {
            inner,
            pending: VecDeque::new(),
            pending_pd: VecDeque::new(),
            code_block: None,
            image: None,
            list_stack: Vec::new(),
            #[cfg(feature = "definition-lists")]
            def_list_stack: Vec::new(),
            tight_stack: Vec::new(),
            text_container_depth: 0,
            started: false,
            ended: false,
            link_defs: Some(link_defs),
        }
    }

    /// Pull the next raw pulldown-cmark event, preferring anything peeked
    /// ahead and buffered in `pending_pd`.
    fn next_pd(&mut self) -> Option<(pulldown_cmark::Event<'a>, std::ops::Range<usize>)> {
        self.pending_pd.pop_front().or_else(|| self.inner.next())
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
            let (pd_event, _range) = match self.next_pd() {
                Some(pair) => pair,
                None => {
                    // Pulldown stream exhausted. Drain link_defs (computed
                    // eagerly at construction) as their own leaf events
                    // before EndDocument, mirroring CmDoc.link_defs's
                    // placement outside doc.blocks.
                    if let Some(defs) = self.link_defs.take() {
                        for def in defs {
                            self.pending.push_back(Event::LinkDef {
                                label: Cow::Owned(def.label),
                                url: Cow::Owned(def.url),
                                title: def.title.map(Cow::Owned),
                            });
                        }
                        continue;
                    }
                    if !self.ended {
                        self.ended = true;
                        return Some(Event::EndDocument);
                    }
                    return None;
                }
            };

            use pulldown_cmark::{CodeBlockKind, Event as PdEvent, Tag, TagEnd};

            // Synthetic-paragraph gate for tight list items and (with
            // `definition-lists`) tight definition bodies — see
            // `tight_stack`'s doc comment. Only applies directly under such
            // a container — skipped while buffering a code block or image
            // (their internal `Text` events are not container-level
            // content) and while inside a real inline-bearing container
            // (`text_container_depth > 0`, e.g. a loose item's actual
            // `Paragraph`).
            if self.code_block.is_none()
                && self.image.is_none()
                && self.text_container_depth == 0
                && let Some(open) = self.tight_stack.last().map(|f| f.synthetic_open)
            {
                let is_inline_flow = is_inline_flow_event(&pd_event);
                if open && !is_inline_flow {
                    // Leaving the container's bare inline content (a nested
                    // block starts, or the container itself ends): close
                    // the synthetic paragraph before letting `pd_event`
                    // through.
                    self.tight_stack.last_mut().unwrap().synthetic_open = false;
                    self.pending_pd.push_front((pd_event, _range));
                    return Some(Event::EndParagraph);
                }
                if !open && is_inline_flow {
                    // Bare inline content arriving directly under the
                    // container, with no wrapping Paragraph from
                    // pulldown-cmark: open the synthetic paragraph before
                    // letting `pd_event` through.
                    self.tight_stack.last_mut().unwrap().synthetic_open = true;
                    self.pending_pd.push_front((pd_event, _range));
                    return Some(Event::StartParagraph);
                }
            }

            match pd_event {
                // ── Block opens ──────────────────────────────────────────────
                PdEvent::Start(Tag::Paragraph) => {
                    // A *real* Paragraph tag arriving as a direct child of
                    // the innermost open tight container (no intervening
                    // blockquote) is only ever emitted by pulldown-cmark
                    // when the whole enclosing list/definition-list is
                    // loose — see `ListState`'s and
                    // `Event::ListTightnessResolved`'s doc comments. Record
                    // the signal now; the correction event itself is only
                    // emitted later, at `EndList`/`EndDefinitionList`, once
                    // (see there).
                    if let Some(frame) = self.tight_stack.last()
                        && frame.quote_depth == 0
                    {
                        match frame.kind {
                            TightFrameKind::Item => {
                                if let Some(ls) = self.list_stack.last_mut() {
                                    ls.tight = false;
                                }
                            }
                            #[cfg(feature = "definition-lists")]
                            TightFrameKind::DefinitionListDefinition => {
                                if let Some(ds) = self.def_list_stack.last_mut() {
                                    ds.tight = false;
                                }
                            }
                        }
                    }
                    self.text_container_depth += 1;
                    return Some(Event::StartParagraph);
                }
                PdEvent::Start(Tag::Heading { level, .. }) => {
                    self.text_container_depth += 1;
                    return Some(Event::StartHeading {
                        level: heading_level_to_u8(level),
                    });
                }
                PdEvent::Start(Tag::BlockQuote(_)) => {
                    if let Some(frame) = self.tight_stack.last_mut() {
                        frame.quote_depth += 1;
                    }
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
                    self.code_block = Some(CodeBlockState {
                        language,
                        content: String::new(),
                    });
                    // Continue looping — we emit a single CodeBlock event on End.
                }
                PdEvent::Start(Tag::List(first)) => {
                    let (ordered, start) = match first {
                        None => (false, 1u64),
                        Some(n) => (true, n),
                    };
                    self.list_stack.push(ListState { tight: true });
                    // Tightness is unknown until we see the list's items;
                    // emit with tight=true optimistically. If this turns
                    // out wrong, `Event::ListTightnessResolved` corrects it
                    // right before the matching `EndList` (see there and
                    // that event's doc comment).
                    return Some(Event::StartList {
                        ordered,
                        start,
                        tight: true,
                    });
                }
                #[cfg(feature = "task-lists")]
                PdEvent::Start(Tag::Item) => {
                    self.tight_stack.push(TightFrame {
                        kind: TightFrameKind::Item,
                        synthetic_open: false,
                        quote_depth: 0,
                    });
                    // Peek ahead: a task-list item is immediately followed by
                    // a `TaskListMarker` event before any other content.
                    match self.next_pd() {
                        Some((PdEvent::TaskListMarker(checked), _)) => {
                            return Some(Event::StartItem {
                                checked: Some(checked),
                            });
                        }
                        Some(other) => {
                            self.pending_pd.push_front(other);
                            return Some(Event::StartItem { checked: None });
                        }
                        None => {
                            return Some(Event::StartItem { checked: None });
                        }
                    }
                }
                #[cfg(not(feature = "task-lists"))]
                PdEvent::Start(Tag::Item) => {
                    self.tight_stack.push(TightFrame {
                        kind: TightFrameKind::Item,
                        synthetic_open: false,
                        quote_depth: 0,
                    });
                    return Some(Event::StartItem {});
                }
                PdEvent::Start(Tag::HtmlBlock) => {
                    // Content arrives as PdEvent::Html; we accumulate inline until End.
                    // No sub-state needed — Html events go directly through.
                }
                #[cfg(feature = "frontmatter")]
                PdEvent::Start(Tag::MetadataBlock(kind)) => {
                    let kind = match kind {
                        pulldown_cmark::MetadataBlockKind::YamlStyle => FrontMatterKind::Yaml,
                        pulldown_cmark::MetadataBlockKind::PlusesStyle => FrontMatterKind::Toml,
                    };
                    let mut content = String::new();
                    loop {
                        match self.next_pd() {
                            Some((PdEvent::Text(t), _)) => content.push_str(&t),
                            Some((PdEvent::End(TagEnd::MetadataBlock(_)), _)) => break,
                            Some(_) => {} // shouldn't happen; ignore
                            None => break,
                        }
                    }
                    return Some(Event::FrontMatter {
                        kind,
                        content: Cow::Owned(content),
                    });
                }
                #[cfg(feature = "tables")]
                PdEvent::Start(Tag::Table(alignments)) => {
                    let alignments = alignments
                        .iter()
                        .map(crate::options::pd_alignment_to_ast)
                        .collect();
                    return Some(Event::StartTable { alignments });
                }
                #[cfg(feature = "tables")]
                PdEvent::Start(Tag::TableHead) => {
                    // pulldown-cmark does not emit a `Tag::TableRow` around
                    // the header cells (only body rows get one) — but
                    // parse()'s AST always synthesizes a `TableRow` wrapper
                    // for the head (parse.rs's `Tag::TableHead` pushes a
                    // `Frame::TableRow`). Queue the synthetic StartTableRow
                    // right after StartTableHead to match.
                    self.pending.push_back(Event::StartTableRow);
                    return Some(Event::StartTableHead);
                }
                #[cfg(feature = "tables")]
                PdEvent::Start(Tag::TableRow) => {
                    return Some(Event::StartTableRow);
                }
                #[cfg(feature = "tables")]
                PdEvent::Start(Tag::TableCell) => {
                    self.text_container_depth += 1;
                    return Some(Event::StartTableCell);
                }
                #[cfg(feature = "footnotes")]
                PdEvent::Start(Tag::FootnoteDefinition(label)) => {
                    // Behaves like BlockQuote: content is always block-level
                    // (see StartFootnoteDefinition's doc comment), so guard
                    // any enclosing tight container's quote_depth exactly as
                    // BlockQuote does.
                    if let Some(frame) = self.tight_stack.last_mut() {
                        frame.quote_depth += 1;
                    }
                    return Some(Event::StartFootnoteDefinition {
                        label: Cow::Owned(label.into_string()),
                    });
                }
                #[cfg(feature = "definition-lists")]
                PdEvent::Start(Tag::DefinitionList) => {
                    self.def_list_stack.push(DefListState { tight: true });
                    return Some(Event::StartDefinitionList);
                }
                #[cfg(feature = "definition-lists")]
                PdEvent::Start(Tag::DefinitionListTitle) => {
                    return Some(Event::StartDefinitionListTitle);
                }
                #[cfg(feature = "definition-lists")]
                PdEvent::Start(Tag::DefinitionListDefinition) => {
                    self.tight_stack.push(TightFrame {
                        kind: TightFrameKind::DefinitionListDefinition,
                        synthetic_open: false,
                        quote_depth: 0,
                    });
                    return Some(Event::StartDefinitionListDefinition);
                }

                // ── Inline opens ──────────────────────────────────────────────
                PdEvent::Start(Tag::Emphasis) => {
                    return Some(Event::StartEmphasis);
                }
                PdEvent::Start(Tag::Strong) => {
                    return Some(Event::StartStrong);
                }
                #[cfg(feature = "strikethrough")]
                PdEvent::Start(Tag::Strikethrough) => {
                    return Some(Event::StartStrikethrough);
                }
                PdEvent::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    ..
                }) => {
                    // Email autolinks (`<user@example.com>`) arrive from
                    // pulldown-cmark with a bare `dest_url`; parse()'s AST
                    // normalizes it to a `mailto:` URL (see parse.rs's own
                    // `Start(Tag::Link)` handling) — mirror that here.
                    let raw_url = dest_url.into_string();
                    let url = if link_type == pulldown_cmark::LinkType::Email
                        && !raw_url.starts_with("mailto:")
                    {
                        format!("mailto:{raw_url}")
                    } else {
                        raw_url
                    };
                    let url = Cow::Owned(url);
                    let title = if title.is_empty() {
                        None
                    } else {
                        Some(Cow::Owned(title.into_string()))
                    };
                    return Some(Event::StartLink { url, title });
                }
                PdEvent::Start(Tag::Image {
                    dest_url, title, ..
                }) => {
                    let url = dest_url.into_string();
                    let title = if title.is_empty() {
                        None
                    } else {
                        Some(title.into_string())
                    };
                    self.image = Some(ImageState {
                        url,
                        title,
                        alt: String::new(),
                    });
                    // We buffer alt text and emit StartImage on End.
                }

                // ── Block closes ──────────────────────────────────────────────
                PdEvent::End(TagEnd::Paragraph) => {
                    self.text_container_depth = self.text_container_depth.saturating_sub(1);
                    return Some(Event::EndParagraph);
                }
                PdEvent::End(TagEnd::Heading(level)) => {
                    self.text_container_depth = self.text_container_depth.saturating_sub(1);
                    return Some(Event::EndHeading {
                        level: heading_level_to_u8(level),
                    });
                }
                PdEvent::End(TagEnd::BlockQuote(_)) => {
                    if let Some(frame) = self.tight_stack.last_mut() {
                        frame.quote_depth = frame.quote_depth.saturating_sub(1);
                    }
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
                    if let Some(ls) = self.list_stack.pop()
                        && !ls.tight
                    {
                        // The optimistic StartList { tight: true } was
                        // wrong: correct it once, immediately before
                        // EndList (see Event::ListTightnessResolved's doc
                        // comment for why this is the one point in the
                        // stream where the correction is guaranteed known
                        // and doesn't require buffering the whole list).
                        self.pending.push_back(Event::EndList);
                        return Some(Event::ListTightnessResolved { tight: false });
                    }
                    return Some(Event::EndList);
                }
                PdEvent::End(TagEnd::Item) => {
                    self.tight_stack.pop();
                    return Some(Event::EndItem);
                }
                PdEvent::End(TagEnd::HtmlBlock) => {
                    // Nothing extra to emit — Html events already forwarded.
                }
                #[cfg(feature = "footnotes")]
                PdEvent::End(TagEnd::FootnoteDefinition) => {
                    if let Some(frame) = self.tight_stack.last_mut() {
                        frame.quote_depth = frame.quote_depth.saturating_sub(1);
                    }
                    return Some(Event::EndFootnoteDefinition);
                }
                #[cfg(feature = "definition-lists")]
                PdEvent::End(TagEnd::DefinitionList) => {
                    if let Some(ds) = self.def_list_stack.pop()
                        && !ds.tight
                    {
                        // Same one-shot correction pattern as
                        // TagEnd::List — see Event::ListTightnessResolved's
                        // doc comment.
                        self.pending.push_back(Event::EndDefinitionList);
                        return Some(Event::DefinitionListTightnessResolved { tight: false });
                    }
                    return Some(Event::EndDefinitionList);
                }
                #[cfg(feature = "definition-lists")]
                PdEvent::End(TagEnd::DefinitionListTitle) => {
                    return Some(Event::EndDefinitionListTitle);
                }
                #[cfg(feature = "definition-lists")]
                PdEvent::End(TagEnd::DefinitionListDefinition) => {
                    self.tight_stack.pop();
                    return Some(Event::EndDefinitionListDefinition);
                }
                #[cfg(feature = "tables")]
                PdEvent::End(TagEnd::Table) => {
                    return Some(Event::EndTable);
                }
                #[cfg(feature = "tables")]
                PdEvent::End(TagEnd::TableHead) => {
                    // Close the synthetic head row opened at Start(TableHead)
                    // before EndTableHead itself, matching the AST's
                    // StartTableRow/EndTableRow wrapper around head cells.
                    self.pending.push_back(Event::EndTableHead);
                    return Some(Event::EndTableRow);
                }
                #[cfg(feature = "tables")]
                PdEvent::End(TagEnd::TableRow) => {
                    return Some(Event::EndTableRow);
                }
                #[cfg(feature = "tables")]
                PdEvent::End(TagEnd::TableCell) => {
                    self.text_container_depth = self.text_container_depth.saturating_sub(1);
                    return Some(Event::EndTableCell);
                }

                // ── Inline closes ─────────────────────────────────────────────
                PdEvent::End(TagEnd::Emphasis) => {
                    return Some(Event::EndEmphasis);
                }
                PdEvent::End(TagEnd::Strong) => {
                    return Some(Event::EndStrong);
                }
                #[cfg(feature = "strikethrough")]
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
                        // Queue the alt text as a Text event (matching the AST
                        // projection, which only emits one when alt is
                        // non-empty) followed by EndImage; return StartImage
                        // now. Both queued events drain — in order — on
                        // subsequent next() calls, so the caller observes
                        // StartImage, Text(alt), EndImage: alt text lands
                        // strictly *between* the Start/End pair, never before
                        // StartImage.
                        if !state.alt.is_empty() {
                            self.pending
                                .push_back(Event::Text(Cow::Owned(state.alt.clone())));
                        }
                        self.pending.push_back(Event::EndImage);
                        return Some(Event::StartImage {
                            url,
                            title,
                            alt: Cow::Owned(state.alt),
                        });
                    }
                }

                // ── Leaf events ───────────────────────────────────────────────
                PdEvent::Text(text) => {
                    if let Some(state) = &mut self.code_block {
                        state.content.push_str(&text);
                        // Continue — no event emitted yet.
                    } else if let Some(state) = &mut self.image {
                        // Buffer into the alt-text accumulator only. The
                        // corresponding Text event is emitted from
                        // `TagEnd::Image`, strictly between StartImage and
                        // EndImage — never here, and never before StartImage
                        // has been returned to the caller.
                        state.alt.push_str(&text);
                    } else {
                        // Coalesce consecutive raw pulldown-cmark Text events
                        // into a single Event::Text, matching parse()'s AST
                        // (push_inline merges consecutive Inline::Text nodes
                        // because pulldown-cmark can split one logical text
                        // run into multiple Text events, e.g. backslash
                        // escapes). Peek ahead: as long as the next raw event
                        // is also Text with nothing else interleaved, merge
                        // it into this run instead of returning separately.
                        let mut merged = text.into_string();
                        loop {
                            match self.next_pd() {
                                Some((PdEvent::Text(more), _)) => merged.push_str(&more),
                                Some(other) => {
                                    self.pending_pd.push_front(other);
                                    break;
                                }
                                None => break,
                            }
                        }
                        return Some(Event::Text(Cow::Owned(merged)));
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
                #[cfg(feature = "footnotes")]
                PdEvent::FootnoteReference(label) => {
                    return Some(Event::FootnoteReference {
                        label: Cow::Owned(label.into_string()),
                    });
                }
                #[cfg(feature = "math")]
                PdEvent::InlineMath(math) => {
                    return Some(Event::InlineMath(Cow::Owned(math.into_string())));
                }
                #[cfg(feature = "math")]
                PdEvent::DisplayMath(math) => {
                    return Some(Event::DisplayMath(Cow::Owned(math.into_string())));
                }

                // ── Ignored (reached only when the corresponding construct
                // feature is off, so pulldown-cmark never produces the
                // event) ───────────────────────────────────────────────────
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

/// Whether a raw pulldown-cmark event is part of ongoing inline flow — text
/// content or an inline span open/close — as opposed to a block-level
/// construct or item-boundary event. Used by the tight-list-item synthetic
/// paragraph gate in `Iterator::next()`: this is the exact set of events
/// that keep a synthetic paragraph open (or open one), everything else
/// closes it.
fn is_inline_flow_event(ev: &pulldown_cmark::Event<'_>) -> bool {
    use pulldown_cmark::{Event as PdEvent, Tag, TagEnd};
    matches!(
        ev,
        PdEvent::Text(_)
            | PdEvent::Code(_)
            | PdEvent::InlineHtml(_)
            | PdEvent::SoftBreak
            | PdEvent::HardBreak
            | PdEvent::Start(Tag::Emphasis)
            | PdEvent::End(TagEnd::Emphasis)
            | PdEvent::Start(Tag::Strong)
            | PdEvent::End(TagEnd::Strong)
            | PdEvent::Start(Tag::Link { .. })
            | PdEvent::End(TagEnd::Link)
            | PdEvent::Start(Tag::Image { .. })
            | PdEvent::End(TagEnd::Image)
    ) || is_inline_flow_strikethrough(ev)
        || is_inline_flow_footnote_reference(ev)
        || is_inline_flow_math(ev)
}

#[cfg(feature = "footnotes")]
fn is_inline_flow_footnote_reference(ev: &pulldown_cmark::Event<'_>) -> bool {
    matches!(ev, pulldown_cmark::Event::FootnoteReference(_))
}

#[cfg(not(feature = "footnotes"))]
fn is_inline_flow_footnote_reference(_ev: &pulldown_cmark::Event<'_>) -> bool {
    false
}

#[cfg(feature = "math")]
fn is_inline_flow_math(ev: &pulldown_cmark::Event<'_>) -> bool {
    matches!(
        ev,
        pulldown_cmark::Event::InlineMath(_) | pulldown_cmark::Event::DisplayMath(_)
    )
}

#[cfg(not(feature = "math"))]
fn is_inline_flow_math(_ev: &pulldown_cmark::Event<'_>) -> bool {
    false
}

#[cfg(feature = "strikethrough")]
fn is_inline_flow_strikethrough(ev: &pulldown_cmark::Event<'_>) -> bool {
    use pulldown_cmark::{Event as PdEvent, Tag, TagEnd};
    matches!(
        ev,
        PdEvent::Start(Tag::Strikethrough) | PdEvent::End(TagEnd::Strikethrough)
    )
}

#[cfg(not(feature = "strikethrough"))]
fn is_inline_flow_strikethrough(_ev: &pulldown_cmark::Event<'_>) -> bool {
    false
}

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
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::Text(t) if t == "Hello"))
        );
        assert!(evs.iter().any(|e| matches!(e, Event::EndParagraph)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndDocument)));
    }

    #[test]
    fn test_events_heading() {
        let evs = collect("# Hello\n");
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::StartHeading { level: 1 }))
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::Text(t) if t == "Hello"))
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::EndHeading { level: 1 }))
        );
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
    #[cfg(feature = "strikethrough")]
    fn test_events_strikethrough() {
        let evs = collect("~~deleted~~\n");
        assert!(evs.iter().any(|e| matches!(e, Event::StartStrikethrough)));
        assert!(evs.iter().any(|e| matches!(e, Event::EndStrikethrough)));
    }

    #[test]
    #[cfg(feature = "frontmatter")]
    fn test_events_frontmatter() {
        let evs = collect("---\ntitle: X\n---\n\nbody\n");
        assert!(evs.iter().any(|e| matches!(
            e,
            Event::FrontMatter { kind: FrontMatterKind::Yaml, content }
            if content.trim() == "title: X"
        )));
    }

    #[test]
    #[cfg(feature = "tables")]
    fn test_events_table() {
        let evs = collect("| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(evs.iter().any(|e| matches!(e, Event::StartTable { .. })));
        assert!(evs.iter().any(|e| matches!(e, Event::StartTableHead)));
        assert_eq!(
            evs.iter()
                .filter(|e| matches!(e, Event::StartTableCell))
                .count(),
            4
        );
    }

    #[test]
    #[cfg(feature = "task-lists")]
    fn test_events_task_list() {
        let evs = collect("- [ ] todo\n- [x] done\n");
        let checked: Vec<_> = evs
            .iter()
            .filter_map(|e| match e {
                Event::StartItem { checked } => Some(*checked),
                _ => None,
            })
            .collect();
        assert_eq!(checked, vec![Some(false), Some(true)]);
    }

    #[test]
    #[cfg(feature = "footnotes")]
    fn test_events_footnote() {
        let evs = collect("Text.[^1]\n\n[^1]: A note.\n");
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::FootnoteReference { label } if label == "1"))
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::StartFootnoteDefinition { label } if label == "1"))
        );
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::EndFootnoteDefinition))
        );
    }

    #[test]
    #[cfg(feature = "definition-lists")]
    fn test_events_definition_list() {
        let evs = collect("apple\n:   red fruit\n\norange\n:   orange fruit\n");
        assert!(evs.iter().any(|e| matches!(e, Event::StartDefinitionList)));
        assert_eq!(
            evs.iter()
                .filter(|e| matches!(e, Event::StartDefinitionListTitle))
                .count(),
            2
        );
        assert_eq!(
            evs.iter()
                .filter(|e| matches!(e, Event::StartDefinitionListDefinition))
                .count(),
            2
        );
    }

    #[test]
    #[cfg(feature = "math")]
    fn test_events_inline_math() {
        let evs = collect("$x plus y$\n");
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::InlineMath(s) if s == "x plus y"))
        );
    }

    #[test]
    #[cfg(feature = "math")]
    fn test_events_display_math() {
        let evs = collect("$$x plus y$$\n");
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::DisplayMath(s) if s == "x plus y"))
        );
    }

    #[test]
    fn test_events_thematic_break() {
        let evs = collect("---\n");
        assert!(evs.iter().any(|e| matches!(e, Event::ThematicBreak)));
    }

    #[test]
    fn test_events_list() {
        let evs = collect("- one\n- two\n");
        assert!(
            evs.iter()
                .any(|e| matches!(e, Event::StartList { ordered: false, .. }))
        );
        assert_eq!(
            evs.iter()
                .filter(|e| matches!(e, Event::StartItem { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn test_events_ordered_list() {
        let evs = collect("1. first\n2. second\n");
        assert!(evs.iter().any(|e| matches!(
            e,
            Event::StartList {
                ordered: true,
                start: 1,
                ..
            }
        )));
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
        let direct: Vec<OwnedEvent> = events_str("# Hello\n\nA paragraph.\n")
            .map(|e| e.into_owned())
            .collect();

        // Collect via StreamingParser fed in two chunks.
        let mut collected = Vec::new();
        let mut p = StreamingParser::new(|ev: OwnedEvent| collected.push(ev));
        p.feed(b"# Hello\n\n");
        p.feed(b"A paragraph.\n");
        p.finish();

        assert_eq!(direct, collected);
    }
}
