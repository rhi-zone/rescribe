use crate::ast::*;

/// Build a DokuWiki string from a [`DokuwikiDoc`].
pub fn build(doc: &DokuwikiDoc) -> String {
    let mut ctx = BuildContext::new();
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.output
}

/// Collect all text content from a slice of inlines into a String.
pub fn collect_inline_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        collect_inline_text_one(inline, &mut out);
    }
    out
}

fn collect_inline_text_one(inline: &Inline, out: &mut String) {
    match inline {
        Inline::Text(s, _) => out.push_str(s),
        Inline::Bold(children, _) | Inline::Italic(children, _) | Inline::Underline(children, _) => {
            for child in children {
                collect_inline_text_one(child, out);
            }
        }
        Inline::Code(s, _) => out.push_str(s),
        Inline::Link { children, .. } => {
            for child in children {
                collect_inline_text_one(child, out);
            }
        }
        Inline::Image { alt, .. } => {
            if let Some(a) = alt {
                out.push_str(a);
            }
        }
        Inline::LineBreak(_) => out.push('\n'),
        Inline::SoftBreak(_) => out.push(' '),
    }
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
            let equals_count = 7 - (*level as usize).min(6);
            for _ in 0..equals_count {
                ctx.write("=");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(" ");
            for _ in 0..equals_count {
                ctx.write("=");
            }
            ctx.write("\n\n");
        }

        Block::CodeBlock { language, content, .. } => {
            ctx.write("<code");
            if let Some(lang) = language {
                ctx.write(" ");
                ctx.write(lang);
            }
            ctx.write(">\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("</code>\n\n");
        }

        Block::Blockquote { children, .. } => {
            for child in children {
                match child {
                    Block::Paragraph { inlines, .. } => {
                        ctx.write("> ");
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    }
                    _ => build_block(child, ctx),
                }
            }
            ctx.write("\n");
        }

        Block::List { ordered, items, .. } => {
            ctx.list_depth += 1;
            for item_blocks in items {
                for _ in 0..ctx.list_depth {
                    ctx.write("  ");
                }
                if *ordered {
                    ctx.write("- ");
                } else {
                    ctx.write("* ");
                }
                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines, .. } => {
                            build_inlines(inlines, ctx);
                            ctx.write("\n");
                        }
                        _ => build_block(block, ctx),
                    }
                }
            }
            ctx.list_depth -= 1;
            if ctx.list_depth == 0 {
                ctx.write("\n");
            }
        }

        Block::HorizontalRule(_) => {
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

        Inline::Code(s, _) => {
            ctx.write("''");
            ctx.write(s);
            ctx.write("''");
        }

        Inline::Link { url, children, .. } => {
            ctx.write("[[");
            ctx.write(url);
            ctx.write("|");
            build_inlines(children, ctx);
            ctx.write("]]");
        }

        Inline::Image { url, alt, .. } => {
            ctx.write("{{");
            ctx.write(url);
            if let Some(alt_text) = alt {
                ctx.write("|");
                ctx.write(alt_text);
            }
            ctx.write("}}");
        }

        Inline::LineBreak(_) => ctx.write("\\\\\n"),
        Inline::SoftBreak(_) => ctx.write(" "),
    }
}
