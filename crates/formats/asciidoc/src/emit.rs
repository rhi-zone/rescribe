//! AsciiDoc emitter / builder.

use crate::ast::{AsciiDoc, Block, DefinitionItem, Inline, QuoteType};

// ── Public API ────────────────────────────────────────────────────────────────

/// Build an AsciiDoc string from an [`AsciiDoc`] document.
pub fn build(doc: &AsciiDoc) -> String {
    let mut ctx = BuildContext::new();
    build_blocks(&doc.blocks, &mut ctx);
    ctx.output
}

// ── Build context ─────────────────────────────────────────────────────────────

struct BuildContext {
    output: String,
    list_depth: usize,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
            list_depth: 0,
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }
}

// ── Block builders ────────────────────────────────────────────────────────────

fn build_blocks(blocks: &[Block], ctx: &mut BuildContext) {
    for block in blocks {
        build_block(block, ctx);
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines, .. } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Heading { level, inlines, .. } => {
            // AsciiDoc uses = for headings (more = means deeper level)
            for _ in 0..=*level {
                ctx.write("=");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock {
            content, language, ..
        } => {
            if let Some(lang) = language {
                ctx.write("[source,");
                ctx.write(lang);
                ctx.write("]\n");
            }
            ctx.write("----\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("----\n\n");
        }

        Block::Blockquote { children, .. } => {
            ctx.write("[quote]\n____\n");
            build_blocks(children, ctx);
            ctx.write("____\n\n");
        }

        Block::List { ordered, items, .. } => {
            ctx.list_depth += 1;
            for item_blocks in items {
                // AsciiDoc uses * for unordered, . for ordered (repeated for depth)
                if *ordered {
                    for _ in 0..ctx.list_depth {
                        ctx.write(".");
                    }
                } else {
                    for _ in 0..ctx.list_depth {
                        ctx.write("*");
                    }
                }
                ctx.write(" ");

                // Emit item content — inline paragraphs flatten their content
                for child_block in item_blocks {
                    match child_block {
                        Block::Paragraph { inlines, .. } => {
                            build_inlines(inlines, ctx);
                            ctx.write("\n");
                        }
                        other => build_block(other, ctx),
                    }
                }
            }
            ctx.list_depth -= 1;
            if ctx.list_depth == 0 {
                ctx.write("\n");
            }
        }

        Block::DefinitionList { items, .. } => {
            for item in items {
                build_definition_item(item, ctx);
            }
            ctx.write("\n");
        }

        Block::HorizontalRule { .. } => {
            ctx.write("'''\n\n");
        }

        Block::PageBreak { .. } => {
            ctx.write("<<<\n\n");
        }

        Block::Figure { image, .. } => {
            ctx.write("image::");
            ctx.write(&image.url);
            ctx.write("[");
            if let Some(alt) = &image.alt {
                ctx.write(alt);
            }
            ctx.write("]\n\n");
        }

        Block::Div { children, .. } => {
            build_blocks(children, ctx);
        }

        Block::RawBlock { format, content, .. } => {
            if format == "asciidoc" {
                ctx.write(content);
            }
        }

        Block::Table { rows, .. } => {
            ctx.write("|===\n");
            let mut first_row = true;
            for row in rows {
                for cell in &row.cells {
                    ctx.write("| ");
                    build_inlines(cell, ctx);
                    ctx.write(" ");
                }
                ctx.write("\n");

                // Add blank line after header row
                if first_row || row.is_header {
                    ctx.write("\n");
                    first_row = false;
                }
            }
            ctx.write("|===\n\n");
        }
    }
}

fn build_definition_item(item: &DefinitionItem, ctx: &mut BuildContext) {
    build_inlines(&item.term, ctx);
    ctx.write(":: ");
    build_inlines(&item.desc, ctx);
    ctx.write("\n");
}

// ── Inline builders ───────────────────────────────────────────────────────────

fn build_inlines(inlines: &[Inline], ctx: &mut BuildContext) {
    for inline in inlines {
        build_inline(inline, ctx);
    }
}

fn build_inline(inline: &Inline, ctx: &mut BuildContext) {
    match inline {
        Inline::Text { text, .. } => ctx.write(text),

        Inline::Strong(children, _) => {
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Emphasis(children, _) => {
            ctx.write("_");
            build_inlines(children, ctx);
            ctx.write("_");
        }

        Inline::Code(s, _) => {
            ctx.write("`");
            ctx.write(s);
            ctx.write("`");
        }

        Inline::Superscript(children, _) => {
            ctx.write("^");
            build_inlines(children, ctx);
            ctx.write("^");
        }

        Inline::Subscript(children, _) => {
            ctx.write("~");
            build_inlines(children, ctx);
            ctx.write("~");
        }

        Inline::Highlight(children, _) => {
            ctx.write("#");
            build_inlines(children, ctx);
            ctx.write("#");
        }

        Inline::Strikeout(children, _) => {
            ctx.write("[line-through]#");
            build_inlines(children, ctx);
            ctx.write("#");
        }

        Inline::Underline(children, _) => {
            ctx.write("[underline]#");
            build_inlines(children, ctx);
            ctx.write("#");
        }

        Inline::SmallCaps(children, _) => {
            ctx.write("[small-caps]#");
            build_inlines(children, ctx);
            ctx.write("#");
        }

        Inline::Quoted {
            quote_type,
            children,
            ..
        } => match quote_type {
            QuoteType::Single => {
                ctx.write("'`");
                build_inlines(children, ctx);
                ctx.write("`'");
            }
            QuoteType::Double => {
                ctx.write("\"`");
                build_inlines(children, ctx);
                ctx.write("`\"");
            }
        },

        Inline::Link { url, children, .. } => {
            ctx.write(url);
            ctx.write("[");
            build_inlines(children, ctx);
            ctx.write("]");
        }

        Inline::Image(img, _) => {
            ctx.write("image:");
            ctx.write(&img.url);
            ctx.write("[");
            if let Some(alt) = &img.alt {
                ctx.write(alt);
            }
            ctx.write("]");
        }

        Inline::LineBreak { .. } => ctx.write(" +\n"),

        Inline::SoftBreak { .. } => ctx.write("\n"),

        Inline::FootnoteRef { label, .. } => {
            ctx.write("footnoteref:[");
            ctx.write(label);
            ctx.write("]");
        }

        Inline::FootnoteDef {
            label, children, ..
        } => {
            ctx.write("footnotedef:[");
            ctx.write(label);
            ctx.write(",");
            build_inlines(children, ctx);
            ctx.write("]\n");
        }

        Inline::MathInline { source, .. } => {
            ctx.write("stem:[");
            ctx.write(source);
            ctx.write("]");
        }

        Inline::MathDisplay { source, .. } => {
            ctx.write("[stem]\n++++\n");
            ctx.write(source);
            ctx.write("\n++++\n\n");
        }

        Inline::RawInline { format, content, .. } => {
            if format == "asciidoc" {
                ctx.write(content);
            }
        }
    }
}
