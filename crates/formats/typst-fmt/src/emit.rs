//! Builder writer: [`emit`] serializes a [`TypstDoc`] to Typst markup bytes.
//!
//! This is the real markup-generation logic that used to live directly in
//! `rescribe-write-typst` (a CLAUDE.md "adapter must not contain writing
//! logic" violation) — same construct mapping, now consuming this crate's
//! own [`Block`]/[`Inline`] instead of rescribe `Node`s.

use crate::ast::{Block, Inline, TypstDoc};

/// Emit a [`TypstDoc`] as Typst markup.
///
/// This is the implementation behind
/// [`rescribe_format_api::Emit::emit`](crate::TypstDoc)'s trait method (see
/// `lib.rs`'s trait impl block); kept `pub(crate)` — no separate public
/// free function duplicating the trait method.
pub(crate) fn emit(doc: &TypstDoc) -> Vec<u8> {
    let mut ctx = EmitContext::default();
    emit_blocks(&doc.blocks, &mut ctx);
    ctx.output.into_bytes()
}

#[derive(Default)]
struct EmitContext {
    output: String,
    list_depth: usize,
}

impl EmitContext {
    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }
}

fn emit_blocks(blocks: &[Block], ctx: &mut EmitContext) {
    for block in blocks {
        emit_block(block, ctx);
    }
}

fn emit_block(block: &Block, ctx: &mut EmitContext) {
    match block {
        Block::Paragraph(inlines) => {
            emit_inlines(inlines, ctx);
            ctx.write("\n\n");
        }
        Block::Heading { level, body } => {
            for _ in 0..*level {
                ctx.write("=");
            }
            ctx.write(" ");
            emit_inlines(body, ctx);
            ctx.write("\n\n");
        }
        Block::CodeBlock { lang, content } => {
            ctx.write("```");
            if let Some(lang) = lang {
                ctx.write(lang);
            }
            ctx.write("\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("```\n\n");
        }
        Block::List { ordered, items } => {
            ctx.list_depth += 1;
            for item in items {
                for _ in 1..ctx.list_depth {
                    ctx.write("  ");
                }
                ctx.write(if *ordered { "+ " } else { "- " });
                for (i, b) in item.iter().enumerate() {
                    if i > 0 {
                        ctx.write("  ");
                    }
                    match b {
                        Block::Paragraph(inlines) => {
                            emit_inlines(inlines, ctx);
                            ctx.write("\n");
                        }
                        other => emit_block(other, ctx),
                    }
                }
            }
            ctx.list_depth -= 1;
            if ctx.list_depth == 0 {
                ctx.write("\n");
            }
        }
        Block::DefinitionList(entries) => {
            // Term-list markup (`/ term: desc`) is valid directly in Typst
            // markup, one line per entry — it is not a `#terms(...)`
            // function call (that was a pre-existing bug in this crate's
            // predecessor, `rescribe-write-typst`'s hand-rolled emitter:
            // `#terms(...)` is not valid Typst syntax at all, so no
            // roundtrip test ever caught it — see this crate's own
            // `roundtrip_covers_every_construct` test, which does).
            for (term, desc) in entries {
                ctx.write("/ ");
                emit_inlines(term, ctx);
                ctx.write(": ");
                emit_inlines(desc, ctx);
                ctx.write("\n");
            }
            ctx.write("\n");
        }
        Block::Quote(body) => {
            ctx.write("#quote[");
            emit_blocks(body, ctx);
            ctx.write("]\n\n");
        }
        Block::Table { columns, rows } => {
            ctx.write("#table(\n  columns: ");
            ctx.write(&columns.to_string());
            ctx.write(",\n");
            for row in rows {
                for cell in row {
                    ctx.write("  [");
                    emit_inlines(cell, ctx);
                    ctx.write("],\n");
                }
            }
            ctx.write(")\n\n");
        }
        Block::Figure { body, caption } => {
            ctx.write("#figure(\n");
            match body.as_deref() {
                Some(Block::Image { url }) => {
                    ctx.write("  image(\"");
                    ctx.write(url);
                    ctx.write("\"),\n");
                }
                Some(other) => {
                    ctx.write("  [");
                    emit_block(other, ctx);
                    ctx.write("  ],\n");
                }
                None => {}
            }
            if let Some(cap) = caption {
                ctx.write("  caption: [");
                emit_inlines(cap, ctx);
                ctx.write("],\n");
            }
            ctx.write(")\n\n");
        }
        Block::HorizontalRule => {
            ctx.write("#line(length: 100%)\n\n");
        }
        Block::MathDisplay(source) => {
            ctx.write("$ ");
            ctx.write(source);
            ctx.write(" $\n\n");
        }
        Block::Image { url } => {
            ctx.write("#image(\"");
            ctx.write(url);
            ctx.write("\")\n\n");
        }
        Block::Raw(text) => {
            ctx.write(text);
            ctx.write("\n\n");
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
        Inline::Text(t) => ctx.write(t),
        Inline::Strong(body) => {
            ctx.write("*");
            emit_inlines(body, ctx);
            ctx.write("*");
        }
        Inline::Emph(body) => {
            ctx.write("_");
            emit_inlines(body, ctx);
            ctx.write("_");
        }
        Inline::Underline(body) => {
            ctx.write("#underline[");
            emit_inlines(body, ctx);
            ctx.write("]");
        }
        Inline::Strike(body) => {
            ctx.write("#strike[");
            emit_inlines(body, ctx);
            ctx.write("]");
        }
        Inline::Subscript(body) => {
            ctx.write("#sub[");
            emit_inlines(body, ctx);
            ctx.write("]");
        }
        Inline::Superscript(body) => {
            ctx.write("#super[");
            emit_inlines(body, ctx);
            ctx.write("]");
        }
        Inline::Code(content) => {
            ctx.write("`");
            ctx.write(content);
            ctx.write("`");
        }
        Inline::Link { url, body } => {
            ctx.write("#link(\"");
            ctx.write(url);
            ctx.write("\")[");
            emit_inlines(body, ctx);
            ctx.write("]");
        }
        Inline::Image { url } => {
            ctx.write("#image(\"");
            ctx.write(url);
            ctx.write("\")");
        }
        Inline::LineBreak => ctx.write("\\ \n"),
        Inline::MathInline(source) => {
            ctx.write("$");
            ctx.write(source);
            ctx.write("$");
        }
        Inline::MathDisplay(source) => {
            ctx.write("$ ");
            ctx.write(source);
            ctx.write(" $");
        }
        Inline::Footnote(body) => {
            ctx.write("#footnote[");
            emit_inlines(body, ctx);
            ctx.write("]");
        }
        Inline::SmallCaps(body) => {
            ctx.write("#smallcaps[");
            emit_inlines(body, ctx);
            ctx.write("]");
        }
        Inline::Quoted { double, body } => {
            let q = if *double { "\"" } else { "'" };
            ctx.write(q);
            emit_inlines(body, ctx);
            ctx.write(q);
        }
        Inline::Raw(text) => ctx.write(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Span;

    fn doc(blocks: Vec<Block>) -> TypstDoc {
        TypstDoc {
            blocks,
            span: Span::NONE,
        }
    }

    #[test]
    fn emit_heading_and_paragraph() {
        let d = doc(vec![
            Block::Heading {
                level: 1,
                body: vec![Inline::Text("Title".into())],
            },
            Block::Paragraph(vec![Inline::Text("Hello".into())]),
        ]);
        let out = String::from_utf8(emit(&d)).unwrap();
        assert!(out.contains("= Title"));
        assert!(out.contains("Hello"));
    }

    #[test]
    fn emit_strong_and_emph() {
        let d = doc(vec![Block::Paragraph(vec![
            Inline::Strong(vec![Inline::Text("bold".into())]),
            Inline::Text(" ".into()),
            Inline::Emph(vec![Inline::Text("em".into())]),
        ])]);
        let out = String::from_utf8(emit(&d)).unwrap();
        assert!(out.contains("*bold*"));
        assert!(out.contains("_em_"));
    }
}
