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
            Inline::Bold(ch, _)
            | Inline::Italic(ch, _)
            | Inline::Underline(ch, _)
            | Inline::Filename(ch, _)
            | Inline::NonBreaking(ch, _) => {
                out.push_str(&collect_inline_text(ch));
            }
            Inline::Code(s, _) => out.push_str(s),
            Inline::Link { label, url, .. } => {
                out.push_str(if label.is_empty() { url } else { label });
            }
            Inline::IndexEntry(s, _) => out.push_str(s),
            Inline::Entity(s, _) => out.push_str(s),
            Inline::Null(_) => {}
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
                    ctx.write(&format!("=item {}.\n\n", num));
                    num += 1;
                } else {
                    ctx.write("=item *\n\n");
                }

                for item_block in item_blocks {
                    build_block(item_block, ctx);
                }
            }

            ctx.write("=back\n\n");
        }

        Block::DefinitionList { items, .. } => {
            ctx.write("=over 4\n\n");

            for item in items {
                ctx.write("=item ");
                build_inlines(&item.term, ctx);
                ctx.write("\n\n");

                for desc_block in &item.desc {
                    build_block(desc_block, ctx);
                }
            }

            ctx.write("=back\n\n");
        }

        Block::RawBlock { format, content, .. } => {
            ctx.write(&format!("=begin {}\n", format));
            if !content.is_empty() {
                ctx.write(content);
                ctx.write("\n");
            }
            ctx.write(&format!("=end {}\n\n", format));
        }

        Block::ForBlock { format, content, .. } => {
            ctx.write(&format!("=for {} {}\n\n", format, content));
        }

        Block::Encoding { encoding, .. } => {
            ctx.write(&format!("=encoding {}\n\n", encoding));
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

        Inline::Filename(children, _) => {
            ctx.write("F<");
            build_inlines(children, ctx);
            ctx.write(">");
        }

        Inline::NonBreaking(children, _) => {
            ctx.write("S<");
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

        Inline::IndexEntry(entry, _) => {
            ctx.write("X<");
            ctx.write(entry);
            ctx.write(">");
        }

        Inline::Null(_) => {
            ctx.write("Z<>");
        }

        Inline::Entity(s, _) => {
            // Entity already resolved to a string; emit as text
            ctx.write(s);
        }
    }
}
