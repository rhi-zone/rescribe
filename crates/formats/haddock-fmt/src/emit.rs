use crate::ast::{Block, HaddockDoc, Inline};

/// Build a Haddock string from a [`HaddockDoc`].
pub fn build(doc: &HaddockDoc) -> String {
    let mut ctx = BuildContext::new();
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.output
}

/// Collect the plain text content of a slice of inlines.
pub fn collect_inline_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(s, _) => out.push_str(s),
            Inline::Code(s, _) => out.push_str(s),
            Inline::Strong(ch, _) | Inline::Emphasis(ch, _) => {
                out.push_str(&collect_inline_text(ch));
            }
            Inline::Link { text, url, .. } => {
                out.push_str(if text.is_empty() { url } else { text });
            }
        }
    }
    out
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
        Block::Heading { level, inlines, .. } => {
            for _ in 0..*level {
                ctx.write("=");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Paragraph { inlines, .. } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, .. } => {
            for line in content.lines() {
                ctx.write("> ");
                ctx.write(line);
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::UnorderedList { items, .. } => {
            for item_inlines in items {
                ctx.write("* ");
                build_inlines(item_inlines, ctx);
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::OrderedList { items, .. } => {
            for (i, item_inlines) in items.iter().enumerate() {
                ctx.write(&format!("({}) ", i + 1));
                build_inlines(item_inlines, ctx);
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::DefinitionList { items, .. } => {
            for (term_inlines, desc_inlines) in items {
                ctx.write("[");
                build_inlines(term_inlines, ctx);
                ctx.write("] ");
                build_inlines(desc_inlines, ctx);
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

        Inline::Code(s, _) => {
            ctx.write("@");
            ctx.write(s);
            ctx.write("@");
        }

        Inline::Strong(children, _) => {
            ctx.write("__");
            build_inlines(children, ctx);
            ctx.write("__");
        }

        Inline::Emphasis(children, _) => {
            ctx.write("/");
            build_inlines(children, ctx);
            ctx.write("/");
        }

        Inline::Link { url, text, .. } => {
            ctx.write("\"");
            ctx.write(text);
            ctx.write("\"<");
            ctx.write(url);
            ctx.write(">");
        }
    }
}
