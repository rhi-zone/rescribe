//! Texinfo emitter — converts a [`TexinfoDoc`] to a Texinfo string.

use crate::ast::{
    Block, CodeBlockVariant, CrossRefKind, HeadingKind, Inline, SymbolKind, TexinfoDoc,
};

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
        Block::Heading { level, kind, inlines, .. } => {
            let command = match (level, kind) {
                (1, HeadingKind::Numbered) => "@chapter",
                (1, HeadingKind::Unnumbered) => "@unnumbered",
                (1, HeadingKind::Appendix) => "@appendix",
                (2, HeadingKind::Numbered) => "@section",
                (2, HeadingKind::Unnumbered) => "@unnumberedsec",
                (2, HeadingKind::Appendix) => "@appendixsec",
                (3, HeadingKind::Numbered) => "@subsection",
                (3, HeadingKind::Unnumbered) => "@unnumberedsubsec",
                (3, HeadingKind::Appendix) => "@appendixsubsec",
                (4, HeadingKind::Numbered) => "@subsubsection",
                (4, HeadingKind::Unnumbered) => "@unnumberedsubsubsec",
                (4, HeadingKind::Appendix) => "@appendixsubsubsec",
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

        Block::CodeBlock { variant, content, .. } => {
            let (start, end) = match variant {
                CodeBlockVariant::Example => ("@example", "@end example"),
                CodeBlockVariant::SmallExample => ("@smallexample", "@end smallexample"),
                CodeBlockVariant::Verbatim => ("@verbatim", "@end verbatim"),
                CodeBlockVariant::Lisp => ("@lisp", "@end lisp"),
                CodeBlockVariant::Display => ("@display", "@end display"),
                CodeBlockVariant::Format => ("@format", "@end format"),
            };
            ctx.write(start);
            ctx.write("\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write(end);
            ctx.write("\n\n");
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

        Block::Table { rows, .. } => {
            ctx.write("@multitable\n");
            for row in rows {
                if row.is_header {
                    ctx.write("@headitem ");
                } else {
                    ctx.write("@item ");
                }
                for (j, cell) in row.cells.iter().enumerate() {
                    if j > 0 {
                        ctx.write(" @tab ");
                    }
                    emit_inlines(cell, ctx);
                }
                ctx.write("\n");
            }
            ctx.write("@end multitable\n\n");
        }

        Block::Menu { entries, .. } => {
            ctx.write("@menu\n");
            for entry in entries {
                ctx.write("* ");
                ctx.write(&entry.node);
                ctx.write("::");
                if let Some(desc) = &entry.description {
                    ctx.write(" ");
                    ctx.write(desc);
                }
                ctx.write("\n");
            }
            ctx.write("@end menu\n\n");
        }

        Block::HorizontalRule { .. } => {
            ctx.write("\n@sp 1\n@noindent\n@center * * *\n@sp 1\n\n");
        }

        Block::RawBlock { environment, content, .. } => {
            ctx.write("@");
            ctx.write(environment);
            ctx.write("\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("@end ");
            ctx.write(environment);
            ctx.write("\n\n");
        }

        Block::Float { float_type, label, children, .. } => {
            ctx.write("@float");
            if let Some(ft) = float_type {
                ctx.write(" ");
                ctx.write(ft);
                if let Some(lb) = label {
                    ctx.write(",");
                    ctx.write(lb);
                }
            }
            ctx.write("\n");
            for child in children {
                emit_block(child, ctx);
            }
            ctx.write("@end float\n\n");
        }

        Block::NoIndent { .. } => {
            ctx.write("@noindent\n");
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

        Inline::Var(children, _) => {
            ctx.write("@var{");
            emit_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::File(s, _) => {
            ctx.write("@file{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Command(s, _) => {
            ctx.write("@command{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Option(s, _) => {
            ctx.write("@option{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Env(s, _) => {
            ctx.write("@env{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Samp(s, _) => {
            ctx.write("@samp{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Kbd(s, _) => {
            ctx.write("@kbd{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Key(s, _) => {
            ctx.write("@key{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Dfn(children, _) => {
            ctx.write("@dfn{");
            emit_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Cite(s, _) => {
            ctx.write("@cite{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Acronym { abbrev, expansion, .. } => {
            ctx.write("@acronym{");
            ctx.write(abbrev);
            if let Some(exp) = expansion {
                ctx.write(", ");
                ctx.write(exp);
            }
            ctx.write("}");
        }

        Inline::Abbr { abbrev, expansion, .. } => {
            ctx.write("@abbr{");
            ctx.write(abbrev);
            if let Some(exp) = expansion {
                ctx.write(", ");
                ctx.write(exp);
            }
            ctx.write("}");
        }

        Inline::Roman(s, _) => {
            ctx.write("@r{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::SmallCaps(s, _) => {
            ctx.write("@sc{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::DirectItalic(children, _) => {
            ctx.write("@i{");
            emit_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::DirectBold(children, _) => {
            ctx.write("@b{");
            emit_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::DirectTypewriter(s, _) => {
            ctx.write("@t{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Link { url, children, .. } => {
            ctx.write("@uref{");
            ctx.write(url);
            if !children.is_empty() {
                ctx.write(", ");
                emit_inlines(children, ctx);
            }
            ctx.write("}");
        }

        Inline::Image { file, width, height, alt, extension, .. } => {
            ctx.write("@image{");
            ctx.write(file);
            // Always emit all five comma-separated fields (even if empty)
            // when any optional field is set
            if width.is_some() || height.is_some() || alt.is_some() || extension.is_some() {
                ctx.write(",");
                if let Some(w) = width {
                    ctx.write(w);
                }
                ctx.write(",");
                if let Some(h) = height {
                    ctx.write(h);
                }
                ctx.write(",");
                if let Some(a) = alt {
                    ctx.write(a);
                }
                ctx.write(",");
                if let Some(e) = extension {
                    ctx.write(e);
                }
            }
            ctx.write("}");
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

        Inline::CrossRef { kind, node, text, .. } => {
            let cmd = match kind {
                CrossRefKind::Xref => "@xref",
                CrossRefKind::Ref => "@ref",
                CrossRefKind::Pxref => "@pxref",
            };
            ctx.write(cmd);
            ctx.write("{");
            ctx.write(node);
            if let Some(t) = text {
                ctx.write(", ");
                ctx.write(t);
            }
            ctx.write("}");
        }

        Inline::Anchor { name, .. } => {
            ctx.write("@anchor{");
            ctx.write(name);
            ctx.write("}");
        }

        Inline::NoBreak(s, _) => {
            ctx.write("@w{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Email { address, text, .. } => {
            ctx.write("@email{");
            ctx.write(address);
            if let Some(t) = text {
                ctx.write(", ");
                ctx.write(t);
            }
            ctx.write("}");
        }

        Inline::Symbol(kind, _) => {
            let cmd = match kind {
                SymbolKind::Dots => "@dots{}",
                SymbolKind::EndDots => "@enddots{}",
                SymbolKind::Minus => "@minus{}",
                SymbolKind::Copyright => "@copyright{}",
                SymbolKind::Registered => "@registeredsymbol{}",
                SymbolKind::LaTeX => "@LaTeX{}",
                SymbolKind::TeX => "@TeX{}",
                SymbolKind::Tie => "@tie{}",
            };
            ctx.write(cmd);
        }
    }
}
