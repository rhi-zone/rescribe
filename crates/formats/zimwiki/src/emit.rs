//! ZimWiki emitter.

use crate::ast::*;

/// Build a ZimWiki string from a [`ZimwikiDoc`].
pub fn build(doc: &ZimwikiDoc) -> String {
    let mut ctx = BuildContext::new();
    build_blocks(&doc.blocks, &mut ctx, 0);
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
            | Inline::Subscript(c, _)
            | Inline::Superscript(c, _) => s.push_str(&collect_inline_text(c)),
            Inline::Code(t, _) => s.push_str(t),
            Inline::Link { children, .. } => s.push_str(&collect_inline_text(children)),
            Inline::Image { url, .. } => s.push_str(url),
            Inline::LineBreak { .. } => s.push('\n'),
            Inline::SoftBreak { .. } => s.push(' '),
        }
    }
    s
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

fn build_blocks(blocks: &[Block], ctx: &mut BuildContext, _depth: usize) {
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
            let level = (*level as usize).clamp(1, 5);
            // ZimWiki uses inverted levels: level 1 = 6 equals signs, level 2 = 5, etc.
            let eq_count = 7 - level;
            let marker: String = "=".repeat(eq_count);
            ctx.write(&marker);
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(" ");
            ctx.write(&marker);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, .. } => {
            ctx.write("'''\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("'''\n\n");
        }

        Block::Blockquote { children, .. } => {
            for child in children {
                if let Block::Paragraph { inlines, .. } = child {
                    ctx.write("> ");
                    build_inlines(inlines, ctx);
                    ctx.write("\n");
                } else {
                    build_block(child, ctx);
                }
            }
            ctx.write("\n");
        }

        Block::List { ordered, items, .. } => {
            let mut num = 1;

            for item in items {
                // Check for checkbox first
                if let Some(checked) = item.checked {
                    if checked {
                        ctx.write("[*] ");
                    } else {
                        ctx.write("[ ] ");
                    }
                } else if *ordered {
                    ctx.write(&format!("{}. ", num));
                    num += 1;
                } else {
                    ctx.write("* ");
                }

                for child in &item.children {
                    if let Block::Paragraph { inlines, .. } = child {
                        build_inlines(inlines, ctx);
                    } else {
                        build_block(child, ctx);
                    }
                }
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::Table { rows, .. } => {
            for row in rows {
                ctx.write("|");
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
            ctx.write("~~");
            build_inlines(children, ctx);
            ctx.write("~~");
        }

        Inline::Code(s, _) => {
            ctx.write("''");
            ctx.write(s);
            ctx.write("''");
        }

        Inline::Subscript(children, _) => {
            ctx.write("_{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Superscript(children, _) => {
            ctx.write("^{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Link { url, children, .. } => {
            ctx.write("[[");
            ctx.write(url);
            if !children.is_empty() {
                ctx.write("|");
                build_inlines(children, ctx);
            }
            ctx.write("]]");
        }

        Inline::Image { url, .. } => {
            ctx.write("{{");
            ctx.write(url);
            ctx.write("}}");
        }

        Inline::LineBreak { .. } => {
            ctx.write("\n");
        }

        Inline::SoftBreak { .. } => {
            ctx.write(" ");
        }
    }
}
