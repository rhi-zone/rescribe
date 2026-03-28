//! Textile emitter — Document → String.

use crate::ast::{Block, BlockAttrs, Inline, TextileDoc};

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
        Block::Paragraph { inlines, align, attrs, .. } => {
            // Emit "p" + block attrs only if there's something to write
            let has_align = align.is_some();
            let has_attrs = !attrs.is_empty();
            if has_align || has_attrs {
                ctx.write("p");
                emit_block_attrs(attrs, ctx);
                if let Some(a) = align {
                    let align_str = match a.as_str() {
                        "left" => "<",
                        "right" => ">",
                        "center" => "=",
                        "justify" => "<>",
                        _ => "",
                    };
                    ctx.write(align_str);
                }
                ctx.write(". ");
            }
            emit_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Heading { level, inlines, attrs, .. } => {
            ctx.write(&format!("h{}", level));
            emit_block_attrs(attrs, ctx);
            ctx.write(". ");
            emit_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, language, .. } => {
            if let Some(lang) = language {
                ctx.write(&format!("bc({}). ", lang));
            } else {
                ctx.write("bc. ");
            }
            ctx.write(content);
            ctx.write("\n\n");
        }

        Block::Blockquote { blocks, attrs, .. } => {
            let prefix = if !attrs.is_empty() {
                let mut p = String::from("bq");
                let mut tmp = EmitContext::new();
                emit_block_attrs(attrs, &mut tmp);
                p.push_str(&tmp.output);
                p.push_str(". ");
                p
            } else {
                "bq. ".to_string()
            };
            for block in blocks {
                match block {
                    Block::Paragraph { inlines, .. } => {
                        ctx.write(&prefix);
                        emit_inlines(inlines, ctx);
                        ctx.write("\n\n");
                    }
                    other => emit_block(other, ctx),
                }
            }
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
                // Emit row-level attributes if present
                if !row.attrs.is_empty() {
                    emit_block_attrs(&row.attrs, ctx);
                    ctx.write(". ");
                }
                for cell in &row.cells {
                    ctx.write("|");
                    let align_str = cell.align.as_deref().map(|a| match a {
                        "left" => "<.",
                        "right" => ">.",
                        "center" => "=.",
                        "justify" => "<>.",
                        _ => "",
                    });
                    if cell.is_header {
                        ctx.write("_");
                        if let Some(a) = align_str.filter(|s| !s.is_empty()) {
                            ctx.write(a);
                        } else {
                            ctx.write(".");
                        }
                        ctx.write(" ");
                    } else if let Some(a) = align_str.filter(|s| !s.is_empty()) {
                        ctx.write(a);
                        ctx.write(" ");
                    }
                    emit_inlines(&cell.inlines, ctx);
                }
                ctx.write("|\n");
            }
            ctx.write("\n");
        }

        Block::HorizontalRule { .. } => {
            ctx.write("---\n\n");
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

        Block::Raw { content, .. } => {
            ctx.write("notextile. ");
            ctx.write(content);
            ctx.write("\n\n");
        }
    }
}

/// Emit inline span attributes (`{style}(class)[lang]`) — no alignment or indent.
fn emit_inline_attrs(attrs: &BlockAttrs, ctx: &mut EmitContext) {
    if let Some(style) = &attrs.style {
        ctx.write("{");
        ctx.write(style);
        ctx.write("}");
    }
    if let Some(class) = &attrs.class {
        ctx.write("(");
        ctx.write(class);
        if let Some(id) = &attrs.id {
            ctx.write("#");
            ctx.write(id);
        }
        ctx.write(")");
    } else if let Some(id) = &attrs.id {
        ctx.write("(#");
        ctx.write(id);
        ctx.write(")");
    }
    if let Some(lang) = &attrs.lang {
        ctx.write("[");
        ctx.write(lang);
        ctx.write("]");
    }
}

fn emit_block_attrs(attrs: &BlockAttrs, ctx: &mut EmitContext) {
    if let Some(class) = &attrs.class {
        ctx.write("(");
        ctx.write(class);
        if let Some(id) = &attrs.id {
            ctx.write("#");
            ctx.write(id);
        }
        ctx.write(")");
    } else if let Some(id) = &attrs.id {
        ctx.write("(#");
        ctx.write(id);
        ctx.write(")");
    }
    if let Some(style) = &attrs.style {
        ctx.write("{");
        ctx.write(style);
        ctx.write("}");
    }
    if let Some(lang) = &attrs.lang {
        ctx.write("[");
        ctx.write(lang);
        ctx.write("]");
    }
    for _ in 0..attrs.indent_left {
        ctx.write("(");
    }
    for _ in 0..attrs.indent_right {
        ctx.write(")");
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

        Inline::Link { url, title, children, .. } => {
            ctx.write("\"");
            emit_inlines(children, ctx);
            if let Some(t) = title {
                ctx.write("(");
                ctx.write(t);
                ctx.write(")");
            }
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

        Inline::LineBreak(_) => {
            ctx.write("\n");
        }

        Inline::Raw(content, _) => {
            ctx.write("==");
            ctx.write(content);
            ctx.write("==");
        }

        Inline::Citation(children, _) => {
            ctx.write("??");
            emit_inlines(children, ctx);
            ctx.write("??");
        }

        Inline::GenericSpan { attrs, children, .. } => {
            ctx.write("%");
            if !attrs.is_empty() {
                emit_inline_attrs(attrs, ctx);
            }
            emit_inlines(children, ctx);
            ctx.write("%");
        }

        Inline::Acronym { text, title, .. } => {
            ctx.write(text);
            ctx.write("(");
            ctx.write(title);
            ctx.write(")");
        }
    }
}
