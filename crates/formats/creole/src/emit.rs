use crate::ast::*;

/// Build a Creole string from a [`CreoleDoc`].
pub fn build(doc: &CreoleDoc) -> String {
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
        Inline::Bold(children, _) | Inline::Italic(children, _) => {
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
            let level = (*level as usize).min(6);
            for _ in 0..level {
                ctx.write("=");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(" ");
            for _ in 0..level {
                ctx.write("=");
            }
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, .. } => {
            ctx.write("{{{\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("}}}\n\n");
        }

        Block::Blockquote { children, .. } => {
            for child in children {
                if matches!(child, Block::Paragraph { .. }) {
                    ctx.write("> ");
                    if let Block::Paragraph { inlines, .. } = child {
                        build_inlines(inlines, ctx);
                    }
                    ctx.write("\n");
                } else {
                    build_block(child, ctx);
                }
            }
            ctx.write("\n");
        }

        Block::List { ordered, items, .. } => {
            let marker = if *ordered { "#" } else { "*" };
            ctx.list_depth += 1;

            for item_blocks in items {
                for _ in 0..ctx.list_depth {
                    ctx.write(marker);
                }
                ctx.write(" ");

                for (i, item_child) in item_blocks.iter().enumerate() {
                    if i > 0 {
                        ctx.write("\n");
                    }
                    match item_child {
                        Block::Paragraph { inlines, .. } => {
                            build_inlines(inlines, ctx);
                        }
                        Block::List { .. } => {
                            build_block(item_child, ctx);
                        }
                        _ => {
                            build_block(item_child, ctx);
                        }
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
                        ctx.write("|=");
                    } else {
                        ctx.write("|");
                    }
                    build_inlines(&cell.inlines, ctx);
                }
                ctx.write("|\n");
            }
            ctx.write("\n");
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

        Inline::Code(s, _) => {
            ctx.write("{{{");
            ctx.write(s);
            ctx.write("}}}");
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

        Inline::Image { url, alt, .. } => {
            ctx.write("{{");
            ctx.write(url);
            if let Some(alt_text) = alt {
                ctx.write("|");
                ctx.write(alt_text);
            }
            ctx.write("}}");
        }

        Inline::LineBreak(_) => {
            ctx.write("\\\\");
        }
    }
}
