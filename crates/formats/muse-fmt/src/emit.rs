//! Muse emitter.

use crate::ast::{Block, Inline, MuseDoc};

// ── Public API ────────────────────────────────────────────────────────────────

/// Build a Muse string from a [`MuseDoc`].
pub fn build(doc: &MuseDoc) -> String {
    let mut ctx = BuildContext::new();

    // Emit document header directives
    if let Some(ref title) = doc.title {
        ctx.write("#title ");
        ctx.write(title);
        ctx.write("\n");
    }
    if let Some(ref author) = doc.author {
        ctx.write("#author ");
        ctx.write(author);
        ctx.write("\n");
    }
    if let Some(ref date) = doc.date {
        ctx.write("#date ");
        ctx.write(date);
        ctx.write("\n");
    }
    if let Some(ref desc) = doc.description {
        ctx.write("#desc ");
        ctx.write(desc);
        ctx.write("\n");
    }
    if let Some(ref kw) = doc.keywords {
        ctx.write("#keywords ");
        ctx.write(kw);
        ctx.write("\n");
    }

    // Add blank line after directives if any were emitted
    let has_directives = doc.title.is_some()
        || doc.author.is_some()
        || doc.date.is_some()
        || doc.description.is_some()
        || doc.keywords.is_some();
    if has_directives && !doc.blocks.is_empty() {
        ctx.write("\n");
    }

    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.output
}

// ── Builder internals ─────────────────────────────────────────────────────────

struct BuildContext {
    output: String,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines, .. } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Heading { level, inlines, .. } => {
            let level_capped = (*level as usize).min(5);
            for _ in 0..level_capped {
                ctx.write("*");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, .. } => {
            ctx.write("<example>\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("</example>\n\n");
        }

        Block::Blockquote { children, .. } => {
            ctx.write("<quote>\n");
            for child in children {
                match child {
                    Block::Paragraph { inlines, .. } => {
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    }
                    other => build_block(other, ctx),
                }
            }
            ctx.write("</quote>\n\n");
        }

        Block::List { ordered, items, .. } => {
            let mut num = 1;
            for item_blocks in items {
                if *ordered {
                    ctx.write(&format!(" {}. ", num));
                    num += 1;
                } else {
                    ctx.write(" - ");
                }
                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines, .. } => build_inlines(inlines, ctx),
                        other => build_block(other, ctx),
                    }
                }
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::DefinitionList { items, .. } => {
            for (term_inlines, desc_blocks) in items {
                build_inlines(term_inlines, ctx);
                ctx.write(" :: ");
                for block in desc_blocks {
                    match block {
                        Block::Paragraph { inlines, .. } => build_inlines(inlines, ctx),
                        other => build_block(other, ctx),
                    }
                }
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::HorizontalRule { .. } => {
            ctx.write("----\n\n");
        }

        Block::Verse { children, .. } => {
            ctx.write("<verse>\n");
            for child in children {
                match child {
                    Block::Paragraph { inlines, .. } => {
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    }
                    other => build_block(other, ctx),
                }
            }
            ctx.write("</verse>\n\n");
        }

        Block::CenteredBlock { children, .. } => {
            ctx.write("<center>\n");
            for child in children {
                match child {
                    Block::Paragraph { inlines, .. } => {
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    }
                    other => build_block(other, ctx),
                }
            }
            ctx.write("</center>\n\n");
        }

        Block::RightBlock { children, .. } => {
            ctx.write("<right>\n");
            for child in children {
                match child {
                    Block::Paragraph { inlines, .. } => {
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    }
                    other => build_block(other, ctx),
                }
            }
            ctx.write("</right>\n\n");
        }

        Block::LiteralBlock { content, .. } => {
            ctx.write("<literal>\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("</literal>\n\n");
        }

        Block::SrcBlock { lang, content, .. } => {
            if let Some(lang) = lang {
                ctx.write(&format!("<src lang=\"{}\">\n", lang));
            } else {
                ctx.write("<src>\n");
            }
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("</src>\n\n");
        }

        Block::Comment { content, .. } => {
            ctx.write(";; ");
            ctx.write(content);
            ctx.write("\n\n");
        }

        Block::Table { rows, .. } => {
            for row in rows {
                if row.header {
                    ctx.write("|| ");
                    for (i, cell) in row.cells.iter().enumerate() {
                        if i > 0 {
                            ctx.write(" || ");
                        }
                        build_inlines(cell, ctx);
                    }
                    ctx.write(" ||");
                } else {
                    ctx.write("| ");
                    for (i, cell) in row.cells.iter().enumerate() {
                        if i > 0 {
                            ctx.write(" | ");
                        }
                        build_inlines(cell, ctx);
                    }
                    ctx.write(" |");
                }
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::FootnoteDef {
            label, content, ..
        } => {
            ctx.write("[");
            ctx.write(label);
            ctx.write("] ");
            build_inlines(content, ctx);
            ctx.write("\n\n");
        }
    }
}

fn build_inlines(inlines: &[Inline], ctx: &mut BuildContext) {
    for inline in inlines {
        build_inline(inline, ctx);
    }
}

fn build_inline(inline: &Inline, ctx: &mut BuildContext) {
    match inline {
        Inline::Text(s, _) => ctx.write(s),

        Inline::Bold(children, _) => {
            ctx.write("**");
            build_inlines(children, ctx);
            ctx.write("**");
        }

        Inline::Italic(children, _) => {
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Code(s, _) => {
            ctx.write("=");
            ctx.write(s);
            ctx.write("=");
        }

        Inline::Link { url, children, .. } => {
            ctx.write("[[");
            ctx.write(url);
            ctx.write("][");
            build_inlines(children, ctx);
            ctx.write("]]");
        }

        Inline::Underline(children, _) => {
            ctx.write("_");
            build_inlines(children, ctx);
            ctx.write("_");
        }

        Inline::Strikethrough(children, _) => {
            ctx.write("~~");
            build_inlines(children, ctx);
            ctx.write("~~");
        }

        Inline::Superscript(children, _) => {
            ctx.write("^");
            build_inlines(children, ctx);
            ctx.write("^");
        }

        Inline::Subscript(children, _) => {
            ctx.write("<sub>");
            build_inlines(children, ctx);
            ctx.write("</sub>");
        }

        Inline::FootnoteRef { label, .. } => {
            ctx.write("[");
            ctx.write(label);
            ctx.write("]");
        }

        Inline::LineBreak(_) => {
            ctx.write("<br>");
        }

        Inline::Anchor { name, .. } => {
            ctx.write("<anchor ");
            ctx.write(name);
            ctx.write(">");
        }

        Inline::Image { src, alt, .. } => {
            ctx.write("[[");
            ctx.write(src);
            if let Some(alt_text) = alt {
                ctx.write("][");
                ctx.write(alt_text);
            }
            ctx.write("]]");
        }
    }
}
