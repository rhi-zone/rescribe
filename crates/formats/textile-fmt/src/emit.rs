//! Textile emitter — Document → String.

use crate::ast::{Block, Inline, TextileDoc};

/// Build a Textile string from a [`TextileDoc`].
pub fn emit(doc: &TextileDoc) -> String {
    let mut ctx = EmitContext::new();
    for block in &doc.blocks {
        emit_block(block, &mut ctx);
    }
    ctx.output
}

struct EmitContext {
    output: String,
    list_depth: usize,
}

impl EmitContext {
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

fn emit_block(block: &Block, ctx: &mut EmitContext) {
    match block {
        Block::Paragraph { inlines, .. } => {
            emit_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Heading { level, inlines, .. } => {
            ctx.write(&format!("h{}. ", level));
            emit_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, .. } => {
            ctx.write("bc. ");
            ctx.write(content);
            ctx.write("\n\n");
        }

        Block::Blockquote { inlines, .. } => {
            ctx.write("bq. ");
            emit_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::List { ordered, items, .. } => {
            let marker = if *ordered { "#" } else { "*" };
            ctx.list_depth += 1;

            for item_blocks in items {
                for _ in 0..ctx.list_depth {
                    ctx.write(marker);
                }
                ctx.write(" ");

                for item_child in item_blocks {
                    match item_child {
                        Block::Paragraph { inlines, .. } => {
                            emit_inlines(inlines, ctx);
                        }
                        Block::List { .. } => {
                            ctx.write("\n");
                            emit_block(item_child, ctx);
                            continue;
                        }
                        other => emit_block(other, ctx),
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
                    ctx.write("|");
                    if cell.is_header {
                        ctx.write("_. ");
                    }
                    emit_inlines(&cell.inlines, ctx);
                }
                ctx.write("|\n");
            }
            ctx.write("\n");
        }

        Block::FootnoteDef { label, inlines, .. } => {
            ctx.write(&format!("fn{}. ", label));
            emit_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::DefinitionList { items, .. } => {
            for (term, def) in items {
                ctx.write(";");
                emit_inlines(term, ctx);
                ctx.write("\n:");
                emit_inlines(def, ctx);
                ctx.write("\n");
            }
            ctx.write("\n");
        }
    }
}

fn emit_inlines(inlines: &[Inline], ctx: &mut EmitContext) {
    for inline in inlines {
        emit_inline(inline, ctx);
    }
}

fn emit_inline(inline: &Inline, ctx: &mut EmitContext) {
    match inline {
        Inline::Text(s, _) => ctx.write(s),

        Inline::Bold(children, _) => {
            ctx.write("*");
            emit_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Italic(children, _) => {
            ctx.write("_");
            emit_inlines(children, ctx);
            ctx.write("_");
        }

        Inline::Strikethrough(children, _) => {
            ctx.write("-");
            emit_inlines(children, ctx);
            ctx.write("-");
        }

        Inline::Underline(children, _) => {
            ctx.write("+");
            emit_inlines(children, ctx);
            ctx.write("+");
        }

        Inline::Superscript(children, _) => {
            ctx.write("^");
            emit_inlines(children, ctx);
            ctx.write("^");
        }

        Inline::Subscript(children, _) => {
            ctx.write("~");
            emit_inlines(children, ctx);
            ctx.write("~");
        }

        Inline::Code(s, _) => {
            ctx.write("@");
            ctx.write(s);
            ctx.write("@");
        }

        Inline::Link { url, children, .. } => {
            ctx.write("\"");
            emit_inlines(children, ctx);
            ctx.write("\":");
            ctx.write(url);
        }

        Inline::Image { url, alt, .. } => {
            ctx.write("!");
            ctx.write(url);
            if let Some(alt_text) = alt {
                ctx.write("(");
                ctx.write(alt_text);
                ctx.write(")");
            }
            ctx.write("!");
        }

        Inline::FootnoteRef { label, .. } => {
            ctx.write("[");
            ctx.write(label);
            ctx.write("]");
        }
    }
}
