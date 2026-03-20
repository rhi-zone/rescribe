pub use crate::ast::*;

/// Build a POD string from a [`PodDoc`].
pub fn build(doc: &PodDoc) -> String {
    let mut ctx = BuildContext::new();
    ctx.write("=pod\n\n");
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.write("=cut\n");
    ctx.output
}

/// Collect all text content from a slice of inlines as a plain string.
pub fn collect_inline_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(s, _) => out.push_str(s),
            Inline::Bold(ch, _) | Inline::Italic(ch, _) | Inline::Underline(ch, _) => {
                out.push_str(&collect_inline_text(ch));
            }
            Inline::Code(s, _) => out.push_str(s),
            Inline::Link { label, url, .. } => {
                out.push_str(if label.is_empty() { url } else { label });
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
        Self { output: String::new() }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Heading { level, inlines, .. } => {
            ctx.write(&format!("=head{} ", level));
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Paragraph { inlines, .. } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content, .. } => {
            // Verbatim paragraphs need 4-space indentation
            for line in content.lines() {
                ctx.write("    ");
                ctx.write(line);
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::List { ordered, items, .. } => {
            ctx.write("=over 4\n\n");

            let mut num = 1;
            for item_blocks in items {
                if *ordered {
                    ctx.write(&format!("=item {}. ", num));
                    num += 1;
                } else {
                    ctx.write("=item * ");
                }

                // Emit first paragraph inline with =item
                let mut first = true;
                for item_block in item_blocks {
                    if first && matches!(item_block, Block::Paragraph { .. }) {
                        if let Block::Paragraph { inlines, .. } = item_block {
                            build_inlines(inlines, ctx);
                            ctx.write("\n\n");
                        }
                        first = false;
                    } else {
                        build_block(item_block, ctx);
                    }
                }
            }

            ctx.write("=back\n\n");
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
            // Escape < and > in plain text
            let escaped = s.replace('<', "E<lt>").replace('>', "E<gt>");
            ctx.write(&escaped);
        }

        Inline::Bold(children, _) => {
            ctx.write("B<");
            build_inlines(children, ctx);
            ctx.write(">");
        }

        Inline::Italic(children, _) => {
            ctx.write("I<");
            build_inlines(children, ctx);
            ctx.write(">");
        }

        Inline::Underline(children, _) => {
            ctx.write("U<");
            build_inlines(children, ctx);
            ctx.write(">");
        }

        Inline::Code(content, _) => {
            // Use double brackets if content contains > or <
            if content.contains('>') || content.contains('<') {
                ctx.write("C<< ");
                ctx.write(content);
                ctx.write(" >>");
            } else {
                ctx.write("C<");
                ctx.write(content);
                ctx.write(">");
            }
        }

        Inline::Link { url, label, .. } => {
            if label.is_empty() || label == url {
                ctx.write("L<");
                ctx.write(url);
                ctx.write(">");
            } else {
                ctx.write("L<");
                ctx.write(label);
                ctx.write("|");
                ctx.write(url);
                ctx.write(">");
            }
        }
    }
}
