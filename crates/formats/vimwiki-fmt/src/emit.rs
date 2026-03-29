//! VimWiki emitter.

use crate::ast::*;

/// Build a VimWiki string from a [`VimwikiDoc`].
pub fn build(doc: &VimwikiDoc) -> String {
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
            Inline::Bold(children, _)
            | Inline::Italic(children, _)
            | Inline::Strikethrough(children, _)
            | Inline::Superscript(children, _)
            | Inline::Subscript(children, _) => {
                s.push_str(&collect_inline_text(children));
            }
            Inline::Code(t, _) => s.push_str(t),
            Inline::Link { label, .. } => s.push_str(label),
            Inline::Image { alt, .. } => {
                if let Some(a) = alt {
                    s.push_str(a);
                }
            }
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

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines, .. } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Heading { level, inlines, .. } => {
            let marker: String = "=".repeat(*level);
            ctx.write(&marker);
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(" ");
            ctx.write(&marker);
            ctx.write("\n\n");
        }

        Block::CodeBlock { language, content, .. } => {
            ctx.write("{{{");
            if let Some(lang) = language {
                ctx.write(lang);
            }
            ctx.write("\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("}}}\n\n");
        }

        Block::Blockquote { inlines, .. } => {
            ctx.write("> ");
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::List { ordered, items, .. } => {
            let mut num = 1;
            for item in items {
                if *ordered {
                    ctx.write(&format!("{}. ", num));
                    num += 1;
                } else {
                    ctx.write("* ");
                }

                // Check for checkbox
                if let Some(checked) = item.checked {
                    if checked {
                        ctx.write("[X] ");
                    } else {
                        ctx.write("[ ] ");
                    }
                }

                build_inlines(&item.inlines, ctx);
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

        Block::DefinitionList { items, .. } => {
            for item in items {
                ctx.write("; ");
                build_inlines(&item.term, ctx);
                ctx.write("\n");
                ctx.write(": ");
                build_inlines(&item.desc, ctx);
                ctx.write("\n");
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
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Italic(children, _) => {
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
            ctx.write(",,");
            build_inlines(children, ctx);
            ctx.write(",,");
        }

        Inline::Code(s, _) => {
            ctx.write("`");
            ctx.write(s);
            ctx.write("`");
        }

        Inline::Link { url, label, .. } => {
            ctx.write("[[");
            ctx.write(url);
            if url != label {
                ctx.write("|");
                ctx.write(label);
            }
            ctx.write("]]");
        }

        Inline::Image { url, alt, style, .. } => {
            ctx.write("{{");
            ctx.write(url);
            if let Some(a) = alt {
                ctx.write("|");
                ctx.write(a);
            }
            if let Some(s) = style {
                if alt.is_none() {
                    ctx.write("|");
                }
                ctx.write("|");
                ctx.write(s);
            }
            ctx.write("}}");
        }
    }
}
