//! Streaming MultiMarkdown writer — converts a stream of [`crate::events::Event`]
//! directly to MultiMarkdown bytes without ever materializing an [`crate::ast::MmdDoc`].
//!
//! Metadata entries are buffered (there are usually only a handful, and MMD
//! requires the metadata block to be the very first thing written) until
//! the first real content event forces the underlying
//! `commonmark_fmt::Writer` to be constructed and the metadata block
//! flushed to the sink ahead of it. All CommonMark-shaped events
//! (`Event::Cm`) are forwarded straight to `commonmark_fmt::Writer`, which
//! does the real incremental writing — this crate never hand-writes
//! Markdown syntax. MMD-unique events are translated to the literal bracket
//! text `commonmark_fmt::Writer` would treat as plain content, mirroring
//! `crate::transform::mmd_to_cm_inline`.
//!
//! # Limitation: metadata delimiter style
//!
//! [`crate::events::Event`] carries `Metadata { key, value }` entries but no
//! delimiter-style signal (unlike [`crate::ast::MmdDoc::metadata_style`] on
//! the AST path). A document whose metadata was originally `---`-delimited
//! and round-tripped through `events()` → this `Writer` comes back out
//! bare-style — same key/value content, different (but still valid MMD)
//! delimiter cosmetics. Nothing is lost; the AST path
//! ([`crate::parse::parse`] / [`crate::emit::emit`]) preserves the style
//! exactly. Use the AST path when delimiter-style fidelity matters.

use std::io::Write;

use commonmark_fmt::events::Event as CmEvent;

use crate::ast::MetadataStyle;
use crate::events::Event;

enum State<W: Write> {
    BeforeBody(Option<W>),
    Body(commonmark_fmt::Writer<W>),
}

/// Streaming MultiMarkdown writer.
pub struct Writer<W: Write> {
    state: State<W>,
    metadata: Vec<(String, String)>,
}

impl<W: Write> Writer<W> {
    /// Create a new writer that emits to `sink`.
    pub fn new(sink: W) -> Self {
        Writer {
            state: State::BeforeBody(Some(sink)),
            metadata: Vec::new(),
        }
    }

    fn ensure_body(&mut self) -> &mut commonmark_fmt::Writer<W> {
        if let State::BeforeBody(sink_slot) = &mut self.state {
            let mut sink = sink_slot.take().expect("ensure_body runs at most once");
            let mut prefix = String::new();
            let style = if self.metadata.is_empty() {
                MetadataStyle::None
            } else {
                MetadataStyle::Bare
            };
            let entries: Vec<crate::ast::MetadataEntry> = self
                .metadata
                .iter()
                .map(|(k, v)| crate::ast::MetadataEntry {
                    key: k.clone(),
                    value: v.clone(),
                })
                .collect();
            crate::metadata::emit(&entries, style, &mut prefix);
            let _ = sink.write_all(prefix.as_bytes());
            let mut inner = commonmark_fmt::Writer::new(sink);
            inner.write_event(CmEvent::StartDocument);
            self.state = State::Body(inner);
        }
        match &mut self.state {
            State::Body(inner) => inner,
            State::BeforeBody(_) => unreachable!("just transitioned to Body"),
        }
    }

    /// Feed one event to the writer.
    pub fn write_event(&mut self, event: Event<'_>) {
        match event {
            Event::StartDocument => {}
            Event::Metadata { key, value } => {
                self.metadata.push((key.into_owned(), value.into_owned()));
            }
            Event::EndDocument => {
                self.ensure_body().write_event(CmEvent::EndDocument);
            }
            Event::Cm(ev) => {
                self.ensure_body().write_event(ev);
            }
            Event::Citation { locator, label } => {
                let text = format!("[{}][#{}]", locator.as_deref().unwrap_or(""), label);
                self.ensure_body().write_event(CmEvent::Text(text.into()));
            }
            Event::CrossReference { target, collapsed } => {
                let text = if collapsed {
                    format!("[{target}][]")
                } else {
                    format!("[{target}]")
                };
                self.ensure_body().write_event(CmEvent::Text(text.into()));
            }
            Event::StartCitationDefinition { label } => {
                self.ensure_body().write_event(CmEvent::StartParagraph);
                let text = format!("[#{label}]: ");
                self.ensure_body().write_event(CmEvent::Text(text.into()));
            }
            Event::EndCitationDefinition => {
                self.ensure_body().write_event(CmEvent::EndParagraph);
            }
        }
    }

    /// Flush any remaining buffered bytes to the sink and recover it.
    pub fn finish(self) -> std::io::Result<W> {
        match self.state {
            State::Body(inner) => inner.finish(),
            State::BeforeBody(sink_slot) => {
                let mut sink = sink_slot.expect("sink always present until first ensure_body");
                let mut prefix = String::new();
                let style = if self.metadata.is_empty() {
                    MetadataStyle::None
                } else {
                    MetadataStyle::Bare
                };
                let entries: Vec<crate::ast::MetadataEntry> = self
                    .metadata
                    .into_iter()
                    .map(|(key, value)| crate::ast::MetadataEntry { key, value })
                    .collect();
                crate::metadata::emit(&entries, style, &mut prefix);
                sink.write_all(prefix.as_bytes())?;
                Ok(sink)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::events_str;
    use std::borrow::Cow;

    #[test]
    fn writer_round_trips_via_events() {
        let input = "Title: Doc\n\nSee[p. 23][#Doe:2006] and [Metadata][].\n";
        let evs: Vec<_> = events_str(input).map(|e| e.into_owned()).collect();

        let mut w = Writer::new(Vec::<u8>::new());
        for ev in evs {
            w.write_event(ev);
        }
        let bytes = w.finish().unwrap();
        let out = String::from_utf8(bytes).unwrap();
        assert!(out.contains("Title: Doc"));
        // See emit.rs's equivalent test: commonmark-fmt escapes literal `[`
        // in plain text, invisible after reparsing — check that instead of
        // literal substrings.
        let (doc, diags) = crate::parse::parse(out.as_bytes());
        assert!(diags.is_empty());
        assert!(doc.blocks.iter().any(|b| matches!(b, crate::ast::MmdBlock::Paragraph { inlines, .. }
            if inlines.iter().any(|i| matches!(i, crate::ast::MmdInline::Citation { label, .. } if label == "Doe:2006"))
            && inlines.iter().any(|i| matches!(i, crate::ast::MmdInline::CrossReference { target, collapsed: true, .. } if target == "Metadata")))));
    }

    #[test]
    fn writer_citation_definition() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(Event::StartCitationDefinition {
            label: Cow::Borrowed("Doe:2006"),
        });
        w.write_event(Event::Cm(CmEvent::Text(Cow::Borrowed("John Doe."))));
        w.write_event(Event::EndCitationDefinition);
        w.write_event(Event::EndDocument);
        let bytes = w.finish().unwrap();
        let out = String::from_utf8(bytes).unwrap();
        assert!(out.contains("[#Doe:2006]: John Doe."));
    }
}
