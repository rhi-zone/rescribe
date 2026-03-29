//! txt2tags emitter / builder.

use crate::ast::{Block, Inline, T2tDoc};

/// Build a txt2tags string from a [`T2tDoc`].
pub fn emit(doc: &T2tDoc) -> String {
    let mut ctx = BuildContext::new();

    // Emit header if present
    if let Some(ref title) = doc.title {
        ctx.write(title);
        ctx.write("\n");
        if let Some(ref author) = doc.author {
            ctx.write(author);
        }
        ctx.write("\n");
        if let Some(ref date) = doc.date {
            ctx.write(date);
        }
        ctx.write("\n\n");
    }

    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.output
}

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

        Block::Heading {
            level,
            numbered,
            inlines,
            ..
        } => {
            let marker = if *numbered { '+' } else { '=' };
            let level_capped = (*level as usize).min(5);

            for _ in 0..level_capped {
                ctx.write(&marker.to_string());
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(" ");
            for _ in 0..level_capped {
                ctx.write(&marker.to_string());
            }
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, .. } => {
            ctx.write("```\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("```\n\n");
        }

        Block::RawBlock { content, .. } => {
            ctx.write("\"\"\"\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("\"\"\"\n\n");
        }

        Block::Blockquote { children, .. } => {
            for child in children {
                if let Block::Paragraph { inlines, .. } = child {
                    ctx.write("\t");
                    build_inlines(inlines, ctx);
                    ctx.write("\n");
                } else {
                    build_block(child, ctx);
                }
            }
            ctx.write("\n");
        }

        Block::List { ordered, items, .. } => {
            let marker = if *ordered { "+ " } else { "- " };

            for item_blocks in items {
                ctx.write(marker);
                for block in item_blocks {
                    if let Block::Paragraph { inlines, .. } = block {
                        build_inlines(inlines, ctx);
                    } else {
                        build_block(block, ctx);
                    }
                }
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::Table { rows, .. } => {
            for row in rows {
                if row.is_header {
                    ctx.write("||");
                } else {
                    ctx.write("|");
                }

                for cell in &row.cells {
                    ctx.write(" ");
                    build_inlines(cell, ctx);
                    ctx.write(" |");
                }
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::HorizontalRule { .. } => {
            ctx.write("--------------------\n\n");
        }

        Block::DefinitionList { items, .. } => {
            for (term, desc) in items {
                ctx.write(": ");
                build_inlines(term, ctx);
                ctx.write("\n");
                for block in desc {
                    if let Block::Paragraph { inlines, .. } = block {
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    } else {
                        build_block(block, ctx);
                    }
                }
            }
            ctx.write("\n");
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
            ctx.write("//");
            build_inlines(children, ctx);
            ctx.write("//");
        }

        Inline::Underline(children, _) => {
            ctx.write("__");
            build_inlines(children, ctx);
            ctx.write("__");
        }

        Inline::Strikethrough(children, _) => {
            ctx.write("--");
            build_inlines(children, ctx);
            ctx.write("--");
        }

        Inline::Code(s, _) => {
            ctx.write("``");
            ctx.write(s);
            ctx.write("``");
        }

        Inline::Link { url, children, .. } => {
            ctx.write("[");
            if !children.is_empty() {
                build_inlines(children, ctx);
                ctx.write(" ");
            }
            ctx.write(url);
            ctx.write("]");
        }

        Inline::Image { url, .. } => {
            ctx.write("[");
            ctx.write(url);
            ctx.write("]");
        }

        Inline::LineBreak(_) => {
            ctx.write("\n");
        }

        Inline::SoftBreak(_) => {
            ctx.write(" ");
        }

        Inline::Verbatim(s, _) => {
            ctx.write("\"\"");
            ctx.write(s);
            ctx.write("\"\"");
        }

        Inline::Tagged(s, _) => {
            ctx.write("''");
            ctx.write(s);
            ctx.write("''");
        }
    }
}
