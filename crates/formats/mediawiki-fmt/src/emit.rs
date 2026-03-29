//! MediaWiki emitter -- renders a [`MediawikiDoc`] back to MediaWiki markup.

use crate::ast::{Block, Inline, MediawikiDoc};

/// Emit a [`MediawikiDoc`] as a MediaWiki string.
pub fn emit(doc: &MediawikiDoc) -> String {
    let mut ctx = BuildContext::new();
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.output.trim_end().to_string() + "\n"
}

struct BuildContext {
    output: String,
    list_depth: usize,
    list_markers: Vec<char>,
}

impl BuildContext {
    fn new() -> Self {
        Self { output: String::new(), list_depth: 0, list_markers: Vec::new() }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn newline(&mut self) {
        self.output.push('\n');
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines, .. } => {
            build_inlines(inlines, ctx);
            ctx.newline();
            ctx.newline();
        }

        Block::Heading { level, inlines, .. } => {
            let markers = "=".repeat(*level as usize);
            ctx.write(&markers);
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(" ");
            ctx.writeln(&markers);
            ctx.newline();
        }

        Block::CodeBlock { language, content, .. } => {
            if let Some(lang) = language {
                ctx.writeln(&format!("<syntaxhighlight lang=\"{}\">", lang));
            } else {
                // Indented code block
                for line in content.lines() {
                    ctx.write(" ");
                    ctx.writeln(line);
                }
                ctx.newline();
                return;
            }
            ctx.writeln(content);
            ctx.writeln("</syntaxhighlight>");
            ctx.newline();
        }

        Block::List { ordered, items, .. } => {
            let marker = if *ordered { '#' } else { '*' };
            ctx.list_markers.push(marker);
            ctx.list_depth += 1;

            for item_blocks in items {
                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines, .. } => {
                            let markers: String = ctx.list_markers.iter().collect();
                            ctx.write(&markers);
                            ctx.write(" ");
                            build_inlines(inlines, ctx);
                            ctx.newline();
                        }
                        other => build_block(other, ctx),
                    }
                }
            }

            ctx.list_depth -= 1;
            ctx.list_markers.pop();

            if ctx.list_depth == 0 {
                ctx.newline();
            }
        }

        Block::DefinitionList { items, .. } => {
            for item in items {
                ctx.write("; ");
                build_inlines(&item.term, ctx);
                ctx.newline();
                if !item.desc.is_empty() {
                    ctx.write(": ");
                    build_inlines(&item.desc, ctx);
                    ctx.newline();
                }
            }
            ctx.newline();
        }

        Block::HorizontalRule => {
            ctx.writeln("----");
            ctx.newline();
        }

        Block::Table { rows, caption, .. } => {
            ctx.writeln("{|");
            if let Some(cap) = caption {
                ctx.write("|+ ");
                build_inlines(cap, ctx);
                ctx.newline();
            }
            for (i, row) in rows.iter().enumerate() {
                if i > 0 {
                    ctx.writeln("|-");
                }
                for cell in &row.cells {
                    let marker = if cell.is_header { "!" } else { "|" };
                    ctx.write(marker);
                    ctx.write(" ");
                    build_inlines(&cell.inlines, ctx);
                    ctx.newline();
                }
            }
            ctx.writeln("|}");
            ctx.newline();
        }

        Block::Blockquote { children, .. } => {
            ctx.writeln("<blockquote>");
            for child in children {
                build_block(child, ctx);
            }
            // Remove trailing blank line inside blockquote
            if ctx.output.ends_with("\n\n") {
                ctx.output.pop();
            }
            ctx.writeln("</blockquote>");
            ctx.newline();
        }

        Block::PreBlock { content, .. } => {
            ctx.write("<pre>");
            ctx.write(content);
            ctx.writeln("</pre>");
            ctx.newline();
        }

        Block::RawBlock { content, .. } => {
            ctx.writeln(content);
            ctx.newline();
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
        Inline::Text(s) => ctx.write(s),

        Inline::Bold(children) => {
            ctx.write("'''");
            build_inlines(children, ctx);
            ctx.write("'''");
        }

        Inline::Italic(children) => {
            ctx.write("''");
            build_inlines(children, ctx);
            ctx.write("''");
        }

        Inline::Code(s) => {
            ctx.write("<code>");
            ctx.write(s);
            ctx.write("</code>");
        }

        Inline::Link { url, text } => {
            if url.starts_with("http://") || url.starts_with("https://") {
                // External link
                if text == url {
                    ctx.write(&format!("[{}]", url));
                } else {
                    ctx.write(&format!("[{} {}]", url, text));
                }
            } else {
                // Internal link
                if text == url {
                    ctx.write(&format!("[[{}]]", url));
                } else {
                    ctx.write(&format!("[[{}|{}]]", url, text));
                }
            }
        }

        Inline::Image { url, alt } => {
            if alt.is_empty() {
                ctx.write(&format!("[[File:{}]]", url));
            } else {
                ctx.write(&format!("[[File:{}|{}]]", url, alt));
            }
        }

        Inline::LineBreak => {
            ctx.write("<br/>");
        }

        Inline::Strikeout(children) => {
            ctx.write("<s>");
            build_inlines(children, ctx);
            ctx.write("</s>");
        }

        Inline::Underline(children) => {
            ctx.write("<u>");
            build_inlines(children, ctx);
            ctx.write("</u>");
        }

        Inline::Subscript(children) => {
            ctx.write("<sub>");
            build_inlines(children, ctx);
            ctx.write("</sub>");
        }

        Inline::Superscript(children) => {
            ctx.write("<sup>");
            build_inlines(children, ctx);
            ctx.write("</sup>");
        }

        Inline::FootnoteRef { label, content } => {
            if label.is_empty() {
                if let Some(c) = content {
                    ctx.write(&format!("<ref>{}</ref>", c));
                } else {
                    ctx.write("<ref/>");
                }
            } else if let Some(c) = content {
                ctx.write(&format!("<ref name=\"{}\">{}</ref>", label, c));
            } else {
                ctx.write(&format!("<ref name=\"{}\" />", label));
            }
        }

        Inline::MathInline { source } => {
            ctx.write("<math>");
            ctx.write(source);
            ctx.write("</math>");
        }

        Inline::Template { content } => {
            ctx.write("{{");
            ctx.write(content);
            ctx.write("}}");
        }

        Inline::Nowiki { content } => {
            ctx.write("<nowiki>");
            ctx.write(content);
            ctx.write("</nowiki>");
        }
    }
}
