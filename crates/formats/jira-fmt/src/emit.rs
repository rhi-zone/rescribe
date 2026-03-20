//! Jira wiki markup emitter.

use crate::ast::*;

/// Build a Jira string from a [`JiraDoc`].
pub fn build(doc: &JiraDoc) -> String {
    let mut ctx = BuildContext::new();
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.output
}

/// Collect plain text from a slice of inlines.
pub fn collect_inline_text(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(t, _) => s.push_str(t),
            Inline::Bold(c, _)
            | Inline::Italic(c, _)
            | Inline::Underline(c, _)
            | Inline::Strikethrough(c, _)
            | Inline::Superscript(c, _)
            | Inline::Subscript(c, _) => s.push_str(&collect_inline_text(c)),
            Inline::Code(t, _) => s.push_str(t),
            Inline::Link { children, .. } => s.push_str(&collect_inline_text(children)),
            Inline::Image { url, .. } => s.push_str(url),
        }
    }
    s
}

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

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines, .. } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Heading { level, inlines, .. } => {
            ctx.write(&format!("h{}. ", level));
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, language, .. } => {
            if let Some(lang) = language {
                ctx.write(&format!("{{code:{}}}\n", lang));
            } else {
                ctx.write("{code}\n");
            }
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("{code}\n\n");
        }

        Block::Blockquote { children, .. } => {
            ctx.write("{quote}\n");
            for child in children {
                build_block(child, ctx);
            }
            ctx.write("{quote}\n\n");
        }

        Block::Panel { children, .. } => {
            ctx.write("{panel}\n");
            for child in children {
                build_block(child, ctx);
            }
            ctx.write("{panel}\n\n");
        }

        Block::List { ordered, items, .. } => {
            ctx.list_depth += 1;
            for item_blocks in items {
                let marker = if *ordered { "#" } else { "*" };
                for _ in 0..ctx.list_depth {
                    ctx.write(marker);
                }
                ctx.write(" ");
                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines, .. } => build_inlines(inlines, ctx),
                        _ => build_block(block, ctx),
                    }
                }
                ctx.write("\n");
            }
            ctx.list_depth -= 1;
            if ctx.list_depth == 0 {
                ctx.write("\n");
            }
        }

        Block::Table { rows, .. } => {
            for row in rows {
                for cell in &row.cells {
                    if cell.is_header {
                        ctx.write("||");
                    } else {
                        ctx.write("|");
                    }
                    build_inlines(&cell.inlines, ctx);
                }
                if rows
                    .first()
                    .and_then(|r| r.cells.first().map(|c| c.is_header))
                    == Some(true)
                {
                    ctx.write("||\n");
                } else {
                    ctx.write("|\n");
                }
            }
            ctx.write("\n");
        }

        Block::HorizontalRule { .. } => {
            ctx.write("----\n\n");
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
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Italic(children, _) => {
            ctx.write("_");
            build_inlines(children, ctx);
            ctx.write("_");
        }

        Inline::Underline(children, _) => {
            ctx.write("+");
            build_inlines(children, ctx);
            ctx.write("+");
        }

        Inline::Strikethrough(children, _) => {
            ctx.write("-");
            build_inlines(children, ctx);
            ctx.write("-");
        }

        Inline::Code(s, _) => {
            ctx.write("{{");
            ctx.write(s);
            ctx.write("}}");
        }

        Inline::Link { url, children, .. } => {
            ctx.write("[");
            build_inlines(children, ctx);
            ctx.write("|");
            ctx.write(url);
            ctx.write("]");
        }

        Inline::Image { url, alt, .. } => {
            ctx.write("!");
            ctx.write(url);
            if let Some(alt) = alt {
                ctx.write("|");
                ctx.write(alt);
            }
            ctx.write("!");
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
    }
}
