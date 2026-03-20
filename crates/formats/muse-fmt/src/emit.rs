//! Muse emitter.

use crate::ast::{Block, Inline, MuseDoc};

// ── Public API ────────────────────────────────────────────────────────────────

/// Build a Muse string from a [`MuseDoc`].
pub fn build(doc: &MuseDoc) -> String {
    let mut ctx = BuildContext::new();
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.output
}

// ── Builder internals ─────────────────────────────────────────────────────────

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
            let level_capped = (*level as usize).min(5);
            for _ in 0..level_capped {
                ctx.write("*");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, .. } => {
            ctx.write("<example>\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("</example>\n\n");
        }

        Block::Blockquote { children, .. } => {
            ctx.write("<quote>\n");
            for child in children {
                match child {
                    Block::Paragraph { inlines, .. } => {
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    }
                    other => build_block(other, ctx),
                }
            }
            ctx.write("</quote>\n\n");
        }

        Block::List { ordered, items, .. } => {
            let mut num = 1;
            for item_blocks in items {
                if *ordered {
                    ctx.write(&format!(" {}. ", num));
                    num += 1;
                } else {
                    ctx.write(" - ");
                }
                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines, .. } => build_inlines(inlines, ctx),
                        other => build_block(other, ctx),
                    }
                }
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::DefinitionList { items, .. } => {
            for (term_inlines, desc_blocks) in items {
                build_inlines(term_inlines, ctx);
                ctx.write(" :: ");
                for block in desc_blocks {
                    match block {
                        Block::Paragraph { inlines, .. } => build_inlines(inlines, ctx),
                        other => build_block(other, ctx),
                    }
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
            ctx.write("*");
            build_inlines(children, ctx);
            ctx.write("*");
        }

        Inline::Code(s, _) => {
            ctx.write("=");
            ctx.write(s);
            ctx.write("=");
        }

        Inline::Link { url, children, .. } => {
            ctx.write("[[");
            ctx.write(url);
            ctx.write("][");
            build_inlines(children, ctx);
            ctx.write("]]");
        }
    }
}
