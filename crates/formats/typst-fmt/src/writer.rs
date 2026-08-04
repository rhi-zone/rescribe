//! Streaming writer: [`Writer`] consumes [`Event`]s one at a time and
//! writes Typst markup to a sink incrementally — no internal buffering of
//! the whole document, unlike [`crate::emit::emit`] which needs the full
//! [`crate::ast::TypstDoc`] up front. Byte-for-byte, feeding it
//! `events(input)`'s output and calling [`Writer::finish`] must reproduce
//! what `emit(&parse(input).0)` would have produced (see this crate's own
//! `writer_matches_builder_over_constructs` test).

use std::io::Write;

use crate::events::Event;

/// Marker for what's currently open, tracked as a stack so `write_event`
/// knows what closing text to emit and (for lists/figures) what state to
/// carry between events.
enum OpenKind {
    Document,
    Paragraph,
    Heading,
    CodeBlock {
        wrote_newline_terminated: bool,
    },
    List {
        ordered: bool,
    },
    ListItem,
    DefinitionList,
    DefinitionTerm,
    DefinitionDesc,
    Quote,
    Table,
    TableRow,
    TableCell,
    /// `body_seen`: has the (at most one) body construct been observed yet.
    /// `wrapped`: was it written as `[\n ... \n]` (anything but a bare
    /// image), meaning a closing `],\n` is still owed.
    /// `body_closed`: has that closing `],\n` already been written.
    Figure {
        body_seen: bool,
        wrapped: bool,
        body_closed: bool,
    },
    Caption,
    Strong,
    Emph,
    Underline,
    Strike,
    Subscript,
    Superscript,
    Link,
    Footnote,
}

/// Event-driven Typst writer.
pub struct Writer<W: Write> {
    sink: W,
    stack: Vec<OpenKind>,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            stack: Vec::new(),
        }
    }

    fn write(&mut self, s: &str) {
        let _ = self.sink.write_all(s.as_bytes());
    }

    fn list_depth(&self) -> usize {
        self.stack
            .iter()
            .filter(|k| matches!(k, OpenKind::List { .. }))
            .count()
    }

    fn current_list_ordered(&self) -> bool {
        for k in self.stack.iter().rev() {
            if let OpenKind::List { ordered } = k {
                return *ordered;
            }
        }
        false
    }

    fn in_inline_context(&self) -> bool {
        matches!(
            self.stack.last(),
            Some(
                OpenKind::Paragraph
                    | OpenKind::Heading
                    | OpenKind::DefinitionTerm
                    | OpenKind::DefinitionDesc
                    | OpenKind::TableCell
                    | OpenKind::Caption
                    | OpenKind::Strong
                    | OpenKind::Emph
                    | OpenKind::Underline
                    | OpenKind::Strike
                    | OpenKind::Subscript
                    | OpenKind::Superscript
                    | OpenKind::Link
                    | OpenKind::Footnote
            )
        )
    }

    /// Called before writing any block-level construct's opening text.
    /// If we are the (first) body of an open `Figure`, write the
    /// image-shorthand or `[`-wrap opener that Figure body needs.
    fn enter_block_slot(&mut self, is_image_leaf: bool, image_url: Option<&str>) -> bool {
        if let Some(OpenKind::Figure {
            body_seen: false, ..
        }) = self.stack.last()
        {
            if is_image_leaf {
                let url = image_url.unwrap_or_default().to_owned();
                self.write("  image(\"");
                self.write(&url);
                self.write("\"),\n");
                if let Some(OpenKind::Figure { body_seen, .. }) = self.stack.last_mut() {
                    *body_seen = true;
                }
                return true; // fully handled, caller should not also emit generic Image text
            }
            self.write("  [\n");
            if let Some(OpenKind::Figure {
                body_seen, wrapped, ..
            }) = self.stack.last_mut()
            {
                *body_seen = true;
                *wrapped = true;
            }
        }
        false
    }

    /// Called after popping a frame off the stack, to close a Figure's
    /// `[`-wrapped body if the pop just exposed the Figure frame again.
    fn after_pop(&mut self) {
        if let Some(OpenKind::Figure {
            wrapped: true,
            body_closed: false,
            ..
        }) = self.stack.last_mut()
        {
            self.write("  ],\n");
            if let Some(OpenKind::Figure { body_closed, .. }) = self.stack.last_mut() {
                *body_closed = true;
            }
        }
    }

    /// Process one event, writing to the sink.
    pub fn write_event(&mut self, event: Event) {
        match event {
            Event::StartDocument => self.stack.push(OpenKind::Document),
            Event::EndDocument => {
                self.stack.pop();
            }

            Event::StartParagraph => {
                self.enter_block_slot(false, None);
                self.stack.push(OpenKind::Paragraph);
            }
            Event::EndParagraph => {
                self.stack.pop();
                if matches!(self.stack.last(), Some(OpenKind::ListItem)) {
                    self.write("\n");
                } else {
                    self.write("\n\n");
                }
                self.after_pop();
            }

            Event::StartHeading { level } => {
                self.enter_block_slot(false, None);
                for _ in 0..level {
                    self.write("=");
                }
                self.write(" ");
                self.stack.push(OpenKind::Heading);
            }
            Event::EndHeading => {
                self.stack.pop();
                self.write("\n\n");
                self.after_pop();
            }

            Event::StartCodeBlock { lang } => {
                self.enter_block_slot(false, None);
                self.write("```");
                if let Some(l) = &lang {
                    self.write(l);
                }
                self.write("\n");
                self.stack.push(OpenKind::CodeBlock {
                    wrote_newline_terminated: true,
                });
            }
            Event::CodeBlockContent(c) => {
                self.write(&c);
                if let Some(OpenKind::CodeBlock {
                    wrote_newline_terminated,
                }) = self.stack.last_mut()
                {
                    *wrote_newline_terminated = c.ends_with('\n');
                }
            }
            Event::EndCodeBlock => {
                let terminated = matches!(
                    self.stack.pop(),
                    Some(OpenKind::CodeBlock {
                        wrote_newline_terminated: true
                    })
                );
                if !terminated {
                    self.write("\n");
                }
                self.write("```\n\n");
                self.after_pop();
            }

            Event::StartList { ordered } => {
                self.enter_block_slot(false, None);
                self.stack.push(OpenKind::List { ordered });
            }
            Event::EndList => {
                self.stack.pop();
                if self.list_depth() == 0 {
                    self.write("\n");
                }
                self.after_pop();
            }
            Event::StartListItem => {
                let depth = self.list_depth();
                for _ in 1..depth {
                    self.write("  ");
                }
                self.write(if self.current_list_ordered() {
                    "+ "
                } else {
                    "- "
                });
                self.stack.push(OpenKind::ListItem);
            }
            Event::EndListItem => {
                self.stack.pop();
            }

            Event::StartDefinitionList => {
                self.enter_block_slot(false, None);
                self.write("#terms(\n");
                self.stack.push(OpenKind::DefinitionList);
            }
            Event::EndDefinitionList => {
                self.stack.pop();
                self.write(")\n\n");
                self.after_pop();
            }
            Event::StartDefinitionTerm => {
                self.write("  / ");
                self.stack.push(OpenKind::DefinitionTerm);
            }
            Event::EndDefinitionTerm => {
                self.stack.pop();
                self.write(": ");
            }
            Event::StartDefinitionDesc => {
                self.stack.push(OpenKind::DefinitionDesc);
            }
            Event::EndDefinitionDesc => {
                self.stack.pop();
                self.write(",\n");
            }

            Event::StartQuote => {
                self.enter_block_slot(false, None);
                self.write("#quote[\n");
                self.stack.push(OpenKind::Quote);
            }
            Event::EndQuote => {
                self.stack.pop();
                self.write("]\n\n");
                self.after_pop();
            }

            Event::StartTable { columns } => {
                self.enter_block_slot(false, None);
                self.write("#table(\n  columns: ");
                self.write(&columns.to_string());
                self.write(",\n");
                self.stack.push(OpenKind::Table);
            }
            Event::EndTable => {
                self.stack.pop();
                self.write(")\n\n");
                self.after_pop();
            }
            Event::StartTableRow => self.stack.push(OpenKind::TableRow),
            Event::EndTableRow => {
                self.stack.pop();
            }
            Event::StartTableCell => {
                self.write("  [");
                self.stack.push(OpenKind::TableCell);
            }
            Event::EndTableCell => {
                self.stack.pop();
                self.write("],\n");
            }

            Event::StartFigure => {
                self.enter_block_slot(false, None);
                self.write("#figure(\n");
                self.stack.push(OpenKind::Figure {
                    body_seen: false,
                    wrapped: false,
                    body_closed: false,
                });
            }
            Event::EndFigure => {
                self.stack.pop();
                self.write(")\n\n");
                self.after_pop();
            }
            Event::StartCaption => {
                self.write("  caption: [");
                self.stack.push(OpenKind::Caption);
            }
            Event::EndCaption => {
                self.stack.pop();
                self.write("],\n");
            }

            Event::HorizontalRule => {
                self.enter_block_slot(false, None);
                self.write("#line(length: 100%)\n\n");
            }
            Event::MathDisplay(source) => {
                self.enter_block_slot(false, None);
                self.write("$ ");
                self.write(&source);
                self.write(" $\n\n");
            }

            Event::Image { url } => {
                if self.in_inline_context() {
                    self.write("#image(\"");
                    self.write(&url);
                    self.write("\")");
                } else if !self.enter_block_slot(true, Some(&url)) {
                    self.write("#image(\"");
                    self.write(&url);
                    self.write("\")\n\n");
                }
            }

            Event::Text(t) => self.write(&t),

            Event::StartStrong => {
                self.write("*");
                self.stack.push(OpenKind::Strong);
            }
            Event::EndStrong => {
                self.stack.pop();
                self.write("*");
            }
            Event::StartEmph => {
                self.write("_");
                self.stack.push(OpenKind::Emph);
            }
            Event::EndEmph => {
                self.stack.pop();
                self.write("_");
            }
            Event::StartUnderline => {
                self.write("#underline[");
                self.stack.push(OpenKind::Underline);
            }
            Event::EndUnderline => {
                self.stack.pop();
                self.write("]");
            }
            Event::StartStrike => {
                self.write("#strike[");
                self.stack.push(OpenKind::Strike);
            }
            Event::EndStrike => {
                self.stack.pop();
                self.write("]");
            }
            Event::StartSubscript => {
                self.write("#sub[");
                self.stack.push(OpenKind::Subscript);
            }
            Event::EndSubscript => {
                self.stack.pop();
                self.write("]");
            }
            Event::StartSuperscript => {
                self.write("#super[");
                self.stack.push(OpenKind::Superscript);
            }
            Event::EndSuperscript => {
                self.stack.pop();
                self.write("]");
            }
            Event::Code(content) => {
                self.write("`");
                self.write(&content);
                self.write("`");
            }
            Event::StartLink { url } => {
                self.write("#link(\"");
                self.write(&url);
                self.write("\")[");
                self.stack.push(OpenKind::Link);
            }
            Event::EndLink => {
                self.stack.pop();
                self.write("]");
            }
            Event::LineBreak => self.write("\\ \n"),
            Event::MathInline(source) => {
                self.write("$");
                self.write(&source);
                self.write("$");
            }
            Event::StartFootnote => {
                self.write("#footnote[");
                self.stack.push(OpenKind::Footnote);
            }
            Event::EndFootnote => {
                self.stack.pop();
                self.write("]");
            }
            Event::Raw(text) => {
                if self.in_inline_context() {
                    self.write(&text);
                } else {
                    self.enter_block_slot(false, None);
                    self.write(&text);
                    self.write("\n\n");
                }
            }
        }
    }

    /// Flush and return the sink.
    pub fn finish(self) -> W {
        self.sink
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::events;

    fn write_all(input: &str) -> String {
        let mut w = Writer::new(Vec::<u8>::new());
        for ev in events(input) {
            w.write_event(ev);
        }
        String::from_utf8(w.finish()).unwrap()
    }

    #[test]
    fn writer_matches_builder_over_constructs() {
        let cases = [
            "= Title\n\nHello world.\n",
            "Hello *bold* and _em_ and `code`.\n",
            "- one\n- two\n",
            "+ first\n+ second\n",
            "/ Term: Description\n",
            "#quote[Quoted text]\n",
            "#table(columns: 2, [A], [B], [C], [D])\n",
            "#figure(image(\"cat.png\"), caption: [A cat])\n",
            "#footnote[note] and #sub[2] and #super[3]\n",
            "#underline[u] and #strike[s]\n",
            "Here is $x^2$ math.\n",
            "#image(\"photo.png\")\n",
            "Visit #link(\"https://example.com\")[here].\n",
        ];
        for input in cases {
            let (doc, diags) = crate::parse::parse(input);
            assert!(diags.is_empty(), "{input:?}: {diags:?}");
            let built = String::from_utf8(crate::emit::emit(&doc)).unwrap();
            let streamed = write_all(input);
            assert_eq!(built, streamed, "mismatch for input {input:?}");
        }
    }
}
