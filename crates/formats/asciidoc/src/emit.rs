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

        Block::MathBlock { content, flavor, .. } => {
            let macro_name = flavor.as_deref().unwrap_or("stem");
            ctx.write("[");
            ctx.write(macro_name);
            ctx.write("]\n++++\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("++++\n\n");
        }

        Block::Table { rows, .. } => {
            ctx.write("|===\n");
            for row in rows {
                for cell in &row.cells {
                    ctx.write("| ");
                    build_inlines(cell, ctx);
                    ctx.write(" ");
                }
                ctx.write("\n");

                // Blank line after a header row separates it from body rows.
                // Only emit if the row is explicitly marked as a header.
                if row.is_header {
                    ctx.write("\n");
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
    let mut i = 0;
    while i < inlines.len() {
        // FootnoteRef immediately followed by FootnoteDef with the same label
        // collapses back to footnote:id[text] (or footnote:[text] for auto ids).
        if let Inline::FootnoteRef { label: ref_label, .. } = &inlines[i] {
            if let Some(Inline::FootnoteDef { label: def_label, children, .. }) = inlines.get(i + 1) {
                if ref_label == def_label {
                    // Emit as single inline footnote macro.
                    // Auto-generated labels (fn<N>) are not user-visible IDs,
                    // so we emit anonymous form footnote:[text].
                    let is_auto = def_label.starts_with("fn")
                        && def_label[2..].chars().all(|c| c.is_ascii_digit());
                    if is_auto {
                        ctx.write("footnote:[");
                    } else {
                        ctx.write("footnote:");
                        ctx.write(def_label);
                        ctx.write("[");
                    }
                    build_inlines(children, ctx);
                    ctx.write("]");
                    i += 2;
                    continue;
                }
            }
        }
        build_inline(&inlines[i], ctx);
        i += 1;
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
            // Back-reference to a named footnote: footnote:id[]
            ctx.write("footnote:");
            ctx.write(label);
            ctx.write("[]");
        }

        Inline::FootnoteDef {
            label, children, ..
        } => {
            // Named footnote definition is emitted inline as footnote:id[text].
            // Anonymous footnotes use footnote:[text] (label is auto-generated
            // and starts with "fn", so we always include it as the id here to
            // preserve the ref/def pairing on round-trips).
            ctx.write("footnote:");
            ctx.write(label);
            ctx.write("[");
            build_inlines(children, ctx);
            ctx.write("]");
        }

        Inline::MathInline { content, flavor, .. } => {
            let macro_name = flavor.as_deref().unwrap_or("stem");
            ctx.write(macro_name);
            ctx.write(":[");
            ctx.write(content);
            ctx.write("]");
        }

        Inline::RawInline { format, content, .. } => {
            if format == "asciidoc" {
                ctx.write(content);
            }
        }

        Inline::Anchor { id, .. } => {
            ctx.write("[[");
            ctx.write(id);
            ctx.write("]]");
        }
    }
}
