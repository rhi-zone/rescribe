//! Man page emitter.

use crate::ast::{Block, Inline, ManDoc};

// ── Public entry point ────────────────────────────────────────────────────────

/// Build a man page string from a [`ManDoc`].
pub fn build(doc: &ManDoc) -> String {
    let mut ctx = BuildContext::new();

    // Write title header
    let title = doc.title.as_deref().unwrap_or("UNTITLED");
    let section = doc.section.as_deref().unwrap_or("1");
    let date = doc.date.as_deref().unwrap_or("");
    let source = doc.source.as_deref().unwrap_or("");
    let manual = doc.manual.as_deref().unwrap_or("");
    ctx.write(&format!(
        ".TH {} {} \"{}\" \"{}\" \"{}\"\n",
        title.to_uppercase(),
        section,
        date,
        source,
        manual,
    ));

    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }

    ctx.output
}

// ── Internal ──────────────────────────────────────────────────────────────────

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

    fn newline(&mut self) {
        if !self.output.ends_with('\n') {
            self.write("\n");
        }
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Heading { level, inlines, .. } => {
            ctx.newline();

            // Level 1 is document title (already handled), 2 is .SH, 3+ is .SS
            let macro_name = if *level <= 2 { ".SH" } else { ".SS" };
            ctx.write(macro_name);
            ctx.write(" ");

            // Emit text in uppercase for sections
            let text = extract_text(inlines);
            ctx.write(&text.to_uppercase());
            ctx.write("\n");
        }

        Block::Paragraph { inlines, .. } => {
            ctx.newline();
            ctx.write(".PP\n");
            build_inlines(inlines, ctx);
            ctx.write("\n");
        }

        Block::IndentedParagraph { inlines, .. } => {
            ctx.newline();
            ctx.write(".IP\n");
            build_inlines(inlines, ctx);
            ctx.write("\n");
        }

        Block::CodeBlock { content, .. } => {
            ctx.newline();
            ctx.write(".nf\n");
            for line in content.lines() {
                // Lines starting with . need escaping
                if line.starts_with('.') {
                    ctx.write("\\&");
                }
                ctx.write(line);
                ctx.write("\n");
            }
            ctx.write(".fi\n");
        }

        Block::ExampleBlock { content, .. } => {
            ctx.newline();
            ctx.write(".EX\n");
            for line in content.lines() {
                ctx.write(line);
                ctx.write("\n");
            }
            ctx.write(".EE\n");
        }

        Block::List { ordered, items, .. } => {
            ctx.newline();
            for (i, item_blocks) in items.iter().enumerate() {
                if *ordered {
                    ctx.write(&format!(".IP {}.\n", i + 1));
                } else {
                    ctx.write(".IP \\(bu\n");
                }
                for block in item_blocks {
                    if let Block::Paragraph { inlines, .. } = block {
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    } else {
                        build_block(block, ctx);
                    }
                }
            }
        }

        Block::DefinitionList { items, .. } => {
            for (term_inlines, content_blocks) in items {
                ctx.newline();
                ctx.write(".TP\n");
                build_inlines(term_inlines, ctx);
                ctx.write("\n");
                for content_block in content_blocks {
                    if let Block::Paragraph { inlines, .. } = content_block {
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    } else {
                        build_block(content_block, ctx);
                    }
                }
            }
        }

        Block::HorizontalRule { .. } => {
            ctx.newline();
            ctx.write(".sp\n");
        }

        Block::Comment { text, .. } => {
            ctx.newline();
            ctx.write(".\\\" ");
            ctx.write(text);
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
        Inline::Text(s, _) => {
            let escaped = escape_man(s);
            ctx.write(&escaped);
        }

        Inline::Bold(children, _) => {
            ctx.write("\\fB");
            build_inlines(children, ctx);
            ctx.write("\\fR");
        }

        Inline::Italic(children, _) => {
            ctx.write("\\fI");
            build_inlines(children, ctx);
            ctx.write("\\fR");
        }

        Inline::Code(text, _) => {
            ctx.write("\\f(CW");
            ctx.write(&escape_man(text));
            ctx.write("\\fR");
        }

        Inline::Superscript(children, _) => {
            ctx.write("^{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Subscript(children, _) => {
            ctx.write("_{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Link { url, children, .. } => {
            build_inlines(children, ctx);
            ctx.write(" (");
            ctx.write(&escape_man(url));
            ctx.write(")");
        }
    }
}

pub fn escape_man(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '-' => result.push_str("\\-"),
            _ => result.push(c),
        }
    }
    result
}

pub fn extract_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(s, _) => text.push_str(s),
            Inline::Code(s, _) => text.push_str(s),
            Inline::Bold(children, _)
            | Inline::Italic(children, _)
            | Inline::Superscript(children, _)
            | Inline::Subscript(children, _)
            | Inline::Link { children, .. } => {
                text.push_str(&extract_text(children));
            }
        }
    }
    text
}
