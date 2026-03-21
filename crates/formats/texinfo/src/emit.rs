//! Texinfo emitter — converts a [`TexinfoDoc`] to a Texinfo string.

use crate::ast::{Block, Inline, TexinfoDoc};

/// Emit a [`TexinfoDoc`] to a Texinfo string.
pub fn emit(doc: &TexinfoDoc) -> String {
    let mut ctx = EmitContext::new();

    // Write header
    ctx.write("\\input texinfo\n");
    ctx.write("@setfilename output.info\n");

    // Write title if present
    if let Some(title) = &doc.title {
        ctx.write("@settitle ");
        ctx.write(title);
        ctx.write("\n");
    }

    ctx.write("\n@node Top\n");

    if let Some(title) = &doc.title {
        ctx.write("@top ");
        ctx.write(title);
        ctx.write("\n\n");
    }

    // Write blocks
    for block in &doc.blocks {
        emit_block(block, &mut ctx);
    }

    // Write footer
    ctx.write("\n@bye\n");

    ctx.output
}

struct EmitContext {
    output: String,
}

impl EmitContext {
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn write_escaped(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '@' => self.write("@@"),
                '{' => self.write("@{"),
                '}' => self.write("@}"),
                _ => self.output.push(c),
            }
        }
    }
}

fn emit_block(block: &Block, ctx: &mut EmitContext) {
    match block {
        Block::Heading { level, inlines, .. } => {
            let command = match level {
                1 => "@chapter",
                2 => "@section",
                3 => "@subsection",
                4 => "@subsubsection",
                _ => "@subsubsection",
            };

            ctx.write(command);
            ctx.write(" ");
            emit_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Paragraph { inlines, .. } => {
            emit_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, .. } => {
            ctx.write("@example\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("@end example\n\n");
        }

        Block::Blockquote { children, .. } => {
            ctx.write("@quotation\n");
            for child in children {
                if let Block::Paragraph { inlines, .. } = child {
                    emit_inlines(inlines, ctx);
                    ctx.write("\n");
                } else {
                    emit_block(child, ctx);
                }
            }
            ctx.write("@end quotation\n\n");
        }

        Block::List { ordered, items, .. } => {
            if *ordered {
                ctx.write("@enumerate\n");
            } else {
                ctx.write("@itemize @bullet\n");
            }

            for item in items {
                ctx.write("@item ");
                emit_inlines(item, ctx);
                ctx.write("\n");
            }

            if *ordered {
                ctx.write("@end enumerate\n\n");
            } else {
                ctx.write("@end itemize\n\n");
            }
        }

        Block::DefinitionList { items, .. } => {
            ctx.write("@table @asis\n");

            for (term, desc_blocks) in items {
                ctx.write("@item ");
                emit_inlines(term, ctx);
                ctx.write("\n");

                for desc_block in desc_blocks {
                    if let Block::Paragraph { inlines, .. } = desc_block {
                        emit_inlines(inlines, ctx);
                        ctx.write("\n");
                    } else {
                        emit_block(desc_block, ctx);
                    }
                }
            }

            ctx.write("@end table\n\n");
        }

        Block::HorizontalRule { .. } => {
            ctx.write("\n@sp 1\n@noindent\n@center * * *\n@sp 1\n\n");
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
        Inline::Text(s, _) => ctx.write_escaped(s),

        Inline::Strong(children, _) => {
            ctx.write("@strong{");
            emit_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Emphasis(children, _) => {
            ctx.write("@emph{");
            emit_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Code(s, _) => {
            ctx.write("@code{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Link { url, children, .. } => {
            if url.starts_with("mailto:") {
                let email = url.strip_prefix("mailto:").unwrap_or(url);
                ctx.write("@email{");
                ctx.write(email);
                if !children.is_empty() {
                    ctx.write(", ");
                    emit_inlines(children, ctx);
                }
                ctx.write("}");
            } else if url.starts_with('#') {
                // Internal reference
                let node_name = url.strip_prefix('#').unwrap_or(url);
                ctx.write("@ref{");
                ctx.write(node_name);
                ctx.write("}");
            } else {
                ctx.write("@uref{");
                ctx.write(url);
                if !children.is_empty() {
                    ctx.write(", ");
                    emit_inlines(children, ctx);
                }
                ctx.write("}");
            }
        }

        Inline::Superscript(children, _) => {
            ctx.write("@sup{");
            emit_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Subscript(children, _) => {
            ctx.write("@sub{");
            emit_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::LineBreak { .. } => {
            ctx.write("@*\n");
        }

        Inline::SoftBreak { .. } => {
            ctx.write(" ");
        }

        Inline::FootnoteDef { content, .. } => {
            ctx.write("@footnote{");
            emit_inlines(content, ctx);
            ctx.write("}");
        }
    }
}
