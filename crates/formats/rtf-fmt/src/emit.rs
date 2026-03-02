/// Emit an [`RtfDoc`] to an RTF string.
use crate::ast::*;

/// Collect all unique RGB colors referenced in an `RtfDoc` (excluding black/default).
fn collect_colors(doc: &RtfDoc) -> Vec<(u8, u8, u8)> {
    let mut colors: Vec<(u8, u8, u8)> = Vec::new();
    for block in &doc.blocks {
        collect_colors_in_block(block, &mut colors);
    }
    colors
}

fn collect_colors_in_block(block: &Block, out: &mut Vec<(u8, u8, u8)>) {
    match block {
        Block::Paragraph { inlines, .. } | Block::Heading { inlines, .. } => {
            for inline in inlines {
                collect_colors_in_inline(inline, out);
            }
        }
        Block::Blockquote { children, .. } => {
            for child in children {
                collect_colors_in_block(child, out);
            }
        }
        Block::List { items, .. } => {
            for item in items {
                for block in item {
                    collect_colors_in_block(block, out);
                }
            }
        }
        Block::Table { rows, .. } => {
            for row in rows {
                for cell in &row.cells {
                    for inline in cell {
                        collect_colors_in_inline(inline, out);
                    }
                }
            }
        }
        Block::CodeBlock { .. } | Block::HorizontalRule { .. } => {}
    }
}

fn collect_colors_in_inline(inline: &Inline, out: &mut Vec<(u8, u8, u8)>) {
    match inline {
        Inline::Color {
            r, g, b, children, ..
        } => {
            let rgb = (*r, *g, *b);
            if !out.contains(&rgb) {
                out.push(rgb);
            }
            for child in children {
                collect_colors_in_inline(child, out);
            }
        }
        Inline::Bold { children, .. }
        | Inline::Italic { children, .. }
        | Inline::Underline { children, .. }
        | Inline::Strikethrough { children, .. }
        | Inline::Superscript { children, .. }
        | Inline::Subscript { children, .. }
        | Inline::FontSize { children, .. }
        | Inline::AllCaps { children, .. }
        | Inline::SmallCaps { children, .. }
        | Inline::Hidden { children, .. }
        | Inline::CharSpan { children, .. }
        | Inline::Link { children, .. }
        | Inline::Font { children, .. }
        | Inline::Lang { children, .. } => {
            for child in children {
                collect_colors_in_inline(child, out);
            }
        }
        Inline::BgColor {
            r, g, b, children, ..
        } => {
            let rgb = (*r, *g, *b);
            if !out.contains(&rgb) {
                out.push(rgb);
            }
            for child in children {
                collect_colors_in_inline(child, out);
            }
        }
        Inline::Text { .. }
        | Inline::Code { .. }
        | Inline::Image { .. }
        | Inline::LineBreak { .. }
        | Inline::SoftBreak { .. } => {}
    }
}

/// Collect all unique font names referenced in an `RtfDoc`.
fn collect_fonts(doc: &RtfDoc) -> Vec<String> {
    let mut fonts: Vec<String> = Vec::new();
    for block in &doc.blocks {
        collect_fonts_in_block(block, &mut fonts);
    }
    fonts
}

fn collect_fonts_in_block(block: &Block, out: &mut Vec<String>) {
    match block {
        Block::Paragraph { inlines, .. } | Block::Heading { inlines, .. } => {
            for inline in inlines {
                collect_fonts_in_inline(inline, out);
            }
        }
        Block::Blockquote { children, .. } => {
            for child in children {
                collect_fonts_in_block(child, out);
            }
        }
        Block::List { items, .. } => {
            for item in items {
                for block in item {
                    collect_fonts_in_block(block, out);
                }
            }
        }
        Block::Table { rows, .. } => {
            for row in rows {
                for cell in &row.cells {
                    for inline in cell {
                        collect_fonts_in_inline(inline, out);
                    }
                }
            }
        }
        Block::CodeBlock { .. } | Block::HorizontalRule { .. } => {}
    }
}

fn collect_fonts_in_inline(inline: &Inline, out: &mut Vec<String>) {
    match inline {
        Inline::Font { name, children, .. } => {
            if !out.contains(name) {
                out.push(name.clone());
            }
            for child in children {
                collect_fonts_in_inline(child, out);
            }
        }
        Inline::Bold { children, .. }
        | Inline::Italic { children, .. }
        | Inline::Underline { children, .. }
        | Inline::Strikethrough { children, .. }
        | Inline::Superscript { children, .. }
        | Inline::Subscript { children, .. }
        | Inline::FontSize { children, .. }
        | Inline::AllCaps { children, .. }
        | Inline::SmallCaps { children, .. }
        | Inline::Hidden { children, .. }
        | Inline::CharSpan { children, .. }
        | Inline::Color { children, .. }
        | Inline::BgColor { children, .. }
        | Inline::Link { children, .. }
        | Inline::Lang { children, .. } => {
            for child in children {
                collect_fonts_in_inline(child, out);
            }
        }
        Inline::Text { .. }
        | Inline::Code { .. }
        | Inline::Image { .. }
        | Inline::LineBreak { .. }
        | Inline::SoftBreak { .. } => {}
    }
}

/// Emit an RTF document to a [`String`].
///
/// # Round-trip guarantee
///
/// For any document `doc` in canonical AST form (i.e. the kind the parser
/// produces), `parse(emit(doc)).strip_spans() == doc.strip_spans()`.
///
/// "Canonical form" means inline formatting wrappers are nested in the fixed
/// order the parser always produces: strikethrough → underline → italic →
/// bold → superscript|subscript → font_size → color (outermost last).
/// Non-canonical nesting is not a valid parser output and does not carry a
/// roundtrip guarantee.
pub fn emit(doc: &RtfDoc) -> String {
    let color_map = collect_colors(doc);
    // font_map: index 0 = default ("Times New Roman"), then extra fonts
    let extra_fonts = collect_fonts(doc);
    let font_map: Vec<String> = {
        let mut v = vec!["Times New Roman".to_string()];
        v.extend(extra_fonts);
        v
    };
    let mut ctx = Ctx::new(color_map.clone(), font_map.clone());
    ctx.push(r"{\rtf1\ansi\deff0");
    ctx.push("{\\fonttbl");
    for (i, name) in font_map.iter().enumerate() {
        ctx.push(&format!("{{\\f{i} {name};}}"));
    }
    ctx.push("}");
    if !color_map.is_empty() {
        ctx.push("{\\colortbl;");
        for (r, g, b) in &color_map {
            ctx.push(&format!("\\red{r}\\green{g}\\blue{b};"));
        }
        ctx.push("}");
    }
    ctx.push("\n");
    for block in &doc.blocks {
        emit_block(block, &mut ctx);
    }
    ctx.push("}");
    ctx.out
}

struct Ctx {
    out: String,
    /// RGB colors in the color table (1-indexed; index 0 = auto).
    color_map: Vec<(u8, u8, u8)>,
    /// Font names (index 0 = default font).
    font_map: Vec<String>,
}

impl Ctx {
    fn new(color_map: Vec<(u8, u8, u8)>, font_map: Vec<String>) -> Self {
        Self {
            out: String::new(),
            color_map,
            font_map,
        }
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
        Block::Paragraph {
            inlines,
            align,
            para_props,
            ..
        } => {
            ctx.push("\\pard");
            match align {
                Align::Left => ctx.push("\\ql"),
                Align::Right => ctx.push("\\qr"),
                Align::Center => ctx.push("\\qc"),
                Align::Justify => ctx.push("\\qj"),
                Align::Default => {}
            }
            if !para_props.is_empty() {
                ctx.push(para_props);
            }
            ctx.push(" ");
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

fn emit_wrapped(keyword: &str, children: &[Inline], ctx: &mut Ctx) {
    ctx.push(&format!("{{\\{keyword} "));
    emit_inlines(children, ctx);
    ctx.push("}");
}

fn emit_inline(inline: &Inline, ctx: &mut Ctx) {
    match inline {
        Inline::Text { text, .. } => ctx.push_escaped(text),

        Inline::Bold { children, .. } => emit_wrapped("b", children, ctx),
        Inline::Italic { children, .. } => emit_wrapped("i", children, ctx),
        Inline::Underline { children, .. } => emit_wrapped("ul", children, ctx),
        Inline::Strikethrough { children, .. } => emit_wrapped("strike", children, ctx),
        Inline::Superscript { children, .. } => emit_wrapped("super", children, ctx),
        Inline::Subscript { children, .. } => emit_wrapped("sub", children, ctx),
        Inline::AllCaps { children, .. } => emit_wrapped("caps", children, ctx),
        Inline::SmallCaps { children, .. } => emit_wrapped("scaps", children, ctx),
        Inline::Hidden { children, .. } => emit_wrapped("v", children, ctx),

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

        Inline::FontSize { size, children, .. } => {
            ctx.push(&format!("{{\\fs{size} "));
            emit_inlines(children, ctx);
            ctx.push("}");
        }

        Inline::Color {
            r, g, b, children, ..
        } => {
            let idx = ctx
                .color_map
                .iter()
                .position(|c| *c == (*r, *g, *b))
                .map(|i| i + 1)
                .unwrap_or(0);
            ctx.push(&format!("{{\\cf{idx} "));
            emit_inlines(children, ctx);
            ctx.push("}");
        }

        Inline::CharSpan {
            char_props,
            children,
            ..
        } => {
            ctx.push(&format!("{{{char_props} "));
            emit_inlines(children, ctx);
            ctx.push("}");
        }

        Inline::BgColor {
            r, g, b, children, ..
        } => {
            let idx = ctx
                .color_map
                .iter()
                .position(|c| *c == (*r, *g, *b))
                .map(|i| i + 1)
                .unwrap_or(0);
            ctx.push(&format!("{{\\cb{idx} "));
            emit_inlines(children, ctx);
            ctx.push("}");
        }

        Inline::Font { name, children, .. } => {
            let idx = ctx.font_map.iter().position(|f| f == name).unwrap_or(0);
            ctx.push(&format!("{{\\f{idx} "));
            emit_inlines(children, ctx);
            ctx.push("}");
        }
        Inline::Lang { lcid, children, .. } => {
            ctx.push(&format!("{{\\lang{lcid} "));
            emit_inlines(children, ctx);
            ctx.push("}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_str as parse;

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
                align: Align::Default,
                para_props: String::new(),
                span: Span::NONE,
            }],
            color_table: vec![],
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
                align: Align::Default,
                para_props: String::new(),
                span: Span::NONE,
            }],
            color_table: vec![],
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
                align: Align::Default,
                para_props: String::new(),
                span: Span::NONE,
            }],
            color_table: vec![],
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
                align: Align::Default,
                para_props: String::new(),
                span: Span::NONE,
            }],
            color_table: vec![],
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

    #[test]
    fn test_emit_alignment() {
        let doc = RtfDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text {
                    text: "centered".into(),
                    span: Span::NONE,
                }],
                align: Align::Center,
                para_props: String::new(),
                span: Span::NONE,
            }],
            color_table: vec![],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("\\qc"));
    }

    #[test]
    fn test_emit_font_size() {
        let doc = RtfDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::FontSize {
                    size: 48,
                    children: vec![Inline::Text {
                        text: "big".into(),
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                }],
                align: Align::Default,
                para_props: String::new(),
                span: Span::NONE,
            }],
            color_table: vec![],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("\\fs48"));
    }

    #[test]
    fn test_emit_color() {
        let doc = RtfDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Color {
                    r: 255,
                    g: 0,
                    b: 0,
                    children: vec![Inline::Text {
                        text: "red".into(),
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
                }],
                align: Align::Default,
                para_props: String::new(),
                span: Span::NONE,
            }],
            color_table: vec![],
            span: Span::NONE,
        };
        let out = emit(&doc);
        assert!(out.contains("\\colortbl"));
        assert!(out.contains("\\red255"));
        assert!(out.contains("\\cf1"));
    }

    #[test]
    fn test_roundtrip_alignment() {
        let input = r"{\rtf1 \qr right aligned\par}";
        let (doc1, _) = parse(input);
        let emitted = emit(&doc1);
        let (doc2, _) = parse(&emitted);
        assert_eq!(doc1.strip_spans(), doc2.strip_spans());
    }

    #[test]
    fn test_roundtrip_font_size() {
        let input = r"{\rtf1 \fs36 medium text\par}";
        let (doc1, _) = parse(input);
        let emitted = emit(&doc1);
        let (doc2, _) = parse(&emitted);
        assert_eq!(doc1.strip_spans(), doc2.strip_spans());
    }

    #[test]
    fn test_roundtrip_color() {
        let input = r"{\rtf1{\colortbl ;\red0\green128\blue0;}\cf1 green text\par}";
        let (doc1, _) = parse(input);
        let emitted = emit(&doc1);
        let (doc2, _) = parse(&emitted);
        assert_eq!(doc1.strip_spans(), doc2.strip_spans());
    }

    #[test]
    fn test_roundtrip_para_props() {
        // Paragraph with indent and space-after: raw props must survive emit → parse.
        let input = r"{\rtf1\pard\li720\sa200 indented\par}";
        let (doc1, _) = parse(input);
        let Block::Paragraph { para_props, .. } = &doc1.blocks[0] else {
            panic!("expected paragraph");
        };
        assert_eq!(
            para_props, "\\li720\\sa200",
            "para_props captured incorrectly"
        );
        let emitted = emit(&doc1);
        let (doc2, _) = parse(&emitted);
        assert_eq!(doc1.strip_spans(), doc2.strip_spans());
    }

    #[test]
    fn test_roundtrip_char_props() {
        // Paragraph with baseline-down char-prop: raw char_props must survive emit → parse.
        let input = r"{\rtf1{\dn3 lowered}\par}";
        let (doc1, diags) = parse(input);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let Block::Paragraph { inlines, .. } = &doc1.blocks[0] else {
            panic!("expected paragraph");
        };
        assert!(
            matches!(&inlines[0], Inline::CharSpan { char_props, .. } if char_props == "\\dn3"),
            "expected CharSpan with \\dn3, got: {:?}",
            inlines[0]
        );
        let emitted = emit(&doc1);
        let (doc2, _) = parse(&emitted);
        assert_eq!(doc1.strip_spans(), doc2.strip_spans());
    }

    #[test]
    fn test_roundtrip_char_props_shad() {
        let input = r"{\rtf1{\shad shadowed}\par}";
        let (doc1, diags) = parse(input);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        let emitted = emit(&doc1);
        let (doc2, _) = parse(&emitted);
        assert_eq!(doc1.strip_spans(), doc2.strip_spans());
    }

    /// Regression: paragraph with only LineBreak + non-default alignment
    /// used to fail because parse_color_table always stored a (0,0,0) sentinel
    /// at index 0, causing the reparsed color_table to differ from the original.
    #[test]
    fn test_roundtrip_linebreak_center() {
        let doc = RtfDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::LineBreak { span: Span::NONE }],
                align: Align::Center,
                para_props: String::new(),
                span: Span::NONE,
            }],
            color_table: vec![],
            span: Span::NONE,
        }
        .normalize();
        let emitted = emit(&doc);
        let (reparsed, _) = parse(&emitted);
        assert_eq!(doc.strip_spans(), reparsed.strip_spans());
    }
}
