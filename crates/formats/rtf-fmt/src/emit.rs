/// Emit an [`RtfDoc`] to an RTF string.
use crate::ast::*;

/// Emit an RTF document to a [`String`].
///
/// # Round-trip guarantee
///
/// `parse(emit(&parse(input).0)).0.strip_spans()` is identical to
/// `parse(input).0.strip_spans()` for any valid RTF input.
pub fn emit(doc: &RtfDoc) -> String {
    let mut ctx = Ctx::new();
    ctx.push(r"{\rtf1\ansi\deff0");
    ctx.push(r"{\fonttbl{\f0 Times New Roman;}}");
    ctx.push("\n");
    for block in &doc.blocks {
        emit_block(block, &mut ctx);
    }
    ctx.push("}");
    ctx.out
}

struct Ctx {
    out: String,
}

impl Ctx {
    fn new() -> Self {
        Self { out: String::new() }
    }

    fn push(&mut self, s: &str) {
        self.out.push_str(s);
    }

    /// Write `s` with RTF special-character escaping.
    fn push_escaped(&mut self, s: &str) {
        for ch in s.chars() {
            match ch {
                '\\' => self.push("\\\\"),
                '{' => self.push("\\{"),
                '}' => self.push("\\}"),
                '\t' => self.push("\\tab "),
                // Emit newline/CR as hex escapes so they survive a parse round-trip.
                // Bare \n/\r in RTF source are silently stripped by the parser;
                // \'0a/\'0d are decoded back to the original character.
                '\n' => self.push("\\'0a"),
                '\r' => self.push("\\'0d"),
                '\u{00A0}' => self.push("\\~"),
                '\u{2002}' => self.push("\\enspace "),
                '\u{2003}' => self.push("\\emspace "),
                '\u{2014}' => self.push("\\emdash "),
                '\u{2013}' => self.push("\\endash "),
                '\u{2018}' => self.push("\\lquote "),
                '\u{2019}' => self.push("\\rquote "),
                '\u{201C}' => self.push("\\ldblquote "),
                '\u{201D}' => self.push("\\rdblquote "),
                '\u{2022}' => self.push("\\bullet "),
                c if c.is_ascii() => self.out.push(c),
                c => {
                    // Use \uN? Unicode escape
                    self.push(&format!("\\u{}?", c as u32));
                }
            }
        }
    }
}

fn emit_block(block: &Block, ctx: &mut Ctx) {
    match block {
        Block::Paragraph { inlines, .. } => {
            ctx.push("\\pard\\fs24 ");
            emit_inlines(inlines, ctx);
            ctx.push("\\par\n");
        }

        Block::Heading { level, inlines, .. } => {
            let size = match level {
                1 => 48,
                2 => 40,
                3 => 32,
                4 => 28,
                _ => 24,
            };
            ctx.push(&format!("\\pard\\fs{size} \\b "));
            emit_inlines(inlines, ctx);
            ctx.push("\\b0\\par\n");
        }

        Block::CodeBlock { content, .. } => {
            ctx.push("\\pard\\f1\\fs20 ");
            for line in content.lines() {
                ctx.push_escaped(line);
                ctx.push("\\line ");
            }
            ctx.push("\\f0\\par\n");
        }

        Block::Blockquote { children, .. } => {
            ctx.push("\\pard\\li720 ");
            for child in children {
                match child {
                    Block::Paragraph { inlines, .. } => emit_inlines(inlines, ctx),
                    other => emit_block(other, ctx),
                }
            }
            ctx.push("\\par\n");
        }

        Block::List { ordered, items, .. } => {
            let mut num = 1u32;
            for item_blocks in items {
                ctx.push("\\pard\\li720\\fi-360 ");
                if *ordered {
                    ctx.push(&format!("{num}. "));
                    num += 1;
                } else {
                    ctx.push("\\bullet  ");
                }
                for block in item_blocks {
                    match block {
                        Block::Paragraph { inlines, .. } => emit_inlines(inlines, ctx),
                        other => emit_block(other, ctx),
                    }
                }
                ctx.push("\\par\n");
            }
        }

        Block::Table { rows, .. } => {
            for row in rows {
                ctx.push("\\trowd ");
                for (i, _) in row.cells.iter().enumerate() {
                    let right = (i + 1) * 2000;
                    ctx.push(&format!("\\cellx{right}"));
                }
                for cell in &row.cells {
                    ctx.push("\\pard\\intbl ");
                    emit_inlines(cell, ctx);
                    ctx.push("\\cell ");
                }
                ctx.push("\\row\n");
            }
        }

        Block::HorizontalRule { .. } => {
            ctx.push("\\pard\\brdrb\\brdrs\\brdrw10\\brsp20 \\par\n");
        }
    }
}

fn emit_inlines(inlines: &[Inline], ctx: &mut Ctx) {
    for inline in inlines {
        emit_inline(inline, ctx);
    }
}

fn emit_inline(inline: &Inline, ctx: &mut Ctx) {
    match inline {
        Inline::Text { text, .. } => ctx.push_escaped(text),

        Inline::Bold { children, .. } => {
            ctx.push("{\\b ");
            emit_inlines(children, ctx);
            ctx.push("}");
        }

        Inline::Italic { children, .. } => {
            ctx.push("{\\i ");
            emit_inlines(children, ctx);
            ctx.push("}");
        }

        Inline::Underline { children, .. } => {
            ctx.push("{\\ul ");
            emit_inlines(children, ctx);
            ctx.push("}");
        }

        Inline::Strikethrough { children, .. } => {
            ctx.push("{\\strike ");
            emit_inlines(children, ctx);
            ctx.push("}");
        }

        Inline::Code { text, .. } => {
            ctx.push("{\\f1 ");
            ctx.push_escaped(text);
            ctx.push("}");
        }

        Inline::Link { url, children, .. } => {
            ctx.push("{\\field{\\*\\fldinst HYPERLINK \"");
            ctx.push(url);
            ctx.push("\"}{\\fldrslt ");
            if children.is_empty() {
                ctx.push_escaped(url);
            } else {
                emit_inlines(children, ctx);
            }
            ctx.push("}}");
        }

        Inline::Image { url, alt, .. } => {
            let label = if !alt.is_empty() { alt } else { url };
            ctx.push("[Image: ");
            ctx.push_escaped(label);
            ctx.push("]");
        }

        Inline::LineBreak { .. } => ctx.push("\\line "),

        Inline::SoftBreak { .. } => ctx.push(" "),

        Inline::Superscript { children, .. } => {
            ctx.push("{\\super ");
            emit_inlines(children, ctx);
            ctx.push("}");
        }

        Inline::Subscript { children, .. } => {
            ctx.push("{\\sub ");
            emit_inlines(children, ctx);
            ctx.push("}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn test_emit_header() {
        let (doc, _) = parse(r"{\rtf1 Hello\par}");
        let out = emit(&doc);
        assert!(out.starts_with("{\\rtf1"));
        assert!(out.ends_with('}'));
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = RtfDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text {
                    text: "Hello, world!".into(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("Hello, world!"));
        assert!(out.contains("\\par"));
    }

    #[test]
    fn test_emit_bold() {
        let doc = RtfDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold {
                    children: vec![Inline::Text {
                        text: "bold".into(),
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("{\\b bold}"));
    }

    #[test]
    fn test_emit_escaped_chars() {
        let doc = RtfDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text {
                    text: "Open { and close }".into(),
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("\\{"));
        assert!(out.contains("\\}"));
    }

    #[test]
    fn test_emit_link() {
        let doc = RtfDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "http://example.com".into(),
                    children: vec![Inline::Text {
                        text: "click".into(),
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                }],
                span: Span::NONE,
            }],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("HYPERLINK"));
        assert!(out.contains("http://example.com"));
        assert!(out.contains("click"));
    }

    #[test]
    fn test_roundtrip_paragraph() {
        let input = r"{\rtf1 Hello world\par}";
        let (doc1, _) = parse(input);
        let emitted = emit(&doc1);
        let (doc2, _) = parse(&emitted);
        assert_eq!(doc1.strip_spans(), doc2.strip_spans());
    }

    /// Regression: `\'0d` (hex CR) parsed to Text{"\r"}, which was emitted as
    /// a literal `\r` (stripped on re-parse → empty AST).  Now emitted as
    /// `\'0d` so it survives the round-trip.
    #[test]
    fn test_roundtrip_hex_cr() {
        let input = "\\'0d";
        let (ast1, _) = parse(input);
        let emitted = emit(&ast1);
        let (ast2, _) = parse(&emitted);
        assert_eq!(ast1.strip_spans(), ast2.strip_spans());
    }

    #[test]
    fn test_roundtrip_bold() {
        let input = r"{\rtf1 {\b bold text} normal\par}";
        let (doc1, _) = parse(input);
        let emitted = emit(&doc1);
        let (doc2, _) = parse(&emitted);
        assert_eq!(doc1.strip_spans(), doc2.strip_spans());
    }
}
