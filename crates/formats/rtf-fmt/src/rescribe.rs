//! AST↔`rescribe::Document` translation, gated behind the `rescribe` feature.
//!
//! This module only ever calls into `crate::RtfDoc::parse`/`RtfDoc::emit` —
//! it never tokenizes, parses, or emits RTF bytes itself. See CLAUDE.md's
//! "The `rescribe` feature module must never contain parsing or writing
//! logic".

//! ## RTF `\shp` shape sourcing (ADR 0015's `positioned_container`)
//!
//! The `Inline::Shape` ↔ `positioned_container` translation below rests on
//! evidence gathered by directly reading two independent, real-world
//! Word-generated RTF files rather than the RTF spec text (an extensive web
//! search for the spec's own "Word 97-2000 RTF for Drawing Objects (Shapes)"
//! section came up empty — the section is referenced by every index/TOC
//! mirror found, but no fetch surfaced its actual body text):
//!
//! - `~/git/pandoc/test/command/10145.md` (Pandoc's own test corpus).
//! - `bitfocus/rtf2text`'s `sample.rtf` (a public GitHub fixture),
//!   specifically `{\shp{\*\shpinst\shpleft0\shptop0\shpright3150\shpbottom1575
//!   ...\shpz0\shplid1026{\sp{\sn shapeType}{\sv 75}}...}{\shptxt ...}}`
//!   plus a nested `{\shprslt{\*\do\dobxcolumn\dobypara\dodhgt8192\dptxbx
//!   {\dptxbxtext ...}\dpx0\dpy0\dpxsize3150\dpysize1575...}}`.
//!
//! **Confirmed by direct reading of those files** (not inferred): `\shpleft`/
//! `\shptop`/`\shpright`/`\shpbottom` are absolute corner coordinates
//! (`width = shpright - shpleft`, `height = shpbottom - shptop`); `\shptxt`
//! is a sibling destination group holding the shape's real text; `\shprslt`
//! is a sibling destination group holding old-reader fallback content
//! (typically a legacy `\do` object); `\shpz` is an explicit stacking index
//! (matches ADR 0015 Decision 6 exactly); legacy `\do`'s `\dpx`/`\dpy` are an
//! offset pair and `\dpxsize`/`\dpysize` a size pair (not corner-based like
//! `\shp`), and `\do` *can* wrap real text content via `\dptxbxtext`.
//!
//! **Corroborated but not spec-confirmed**: the twips unit. ADR 0015 already
//! flagged `\shpleft` et al. as "inferred by RTF's uniform-twips convention,
//! not independently spec-quoted." This session added two more data points,
//! neither a primary-source quote naming twips for `\shpleft` specifically:
//! (1) both real files' numeric values are only plausible as twips (e.g. a
//! shape `9638` twips wide ≈ 6.7in, sane for a Letter-page text width; `3150`
//! twips ≈ 2.19in, sane for an embedded-object thumbnail); (2) ReactOS's
//! `riched20` RTF reader (`rtf.h`) defines the sibling legacy words —
//! `rtfDrawOffsetX`/`rtfDrawSizeX`/`rtfDrawOffsetY`/`rtfDrawSizeY` for `\dpx`/
//! `\dpxsize`/`\dpy`/`\dpysize` — as "Drawing Attributes" under that same
//! header's general `rtfTpi = 1440` (twips-per-inch) convention. Treat
//! "twips" as strongly corroborated, not textually spec-confirmed, for
//! `\shpleft`/`\shptop`/`\shpright`/`\shpbottom` specifically.
//!
//! **No rotation property found.** Per ADR 0015 Decision 5, RTF's legacy
//! `\shp` format predates rotation support and no dedicated control word
//! exists. This session searched for a genuine real-world `{\sp{\sn ...}}`
//! rotation convention and found none: the real corpus's own named
//! properties are `shapeType`/`wzDescription`/`wzName`/`fFlipH`/`fFlipV`/
//! `pib` — no rotation-shaped key anywhere. `position:rotation` is therefore
//! never set on the RTF side, in either direction — there is nothing to
//! project, not a dropped value.
//!
//! **`\do` is not modeled as its own `positioned_container` producer.** Both
//! real files show `\do` only nested inside a `\shp` group's `\shprslt`
//! fallback, never standalone at the top level — no real-world evidence of
//! a standalone legacy `\do` was found to design against. The legacy
//! `\dodhgt` word (present in both samples) is speculated in some secondary
//! sources to be a z-order-like "depth", but no primary source confirms
//! this, so it is not projected to `position:z_order` — the entire
//! `\shprslt{...}` group (whatever it contains) is instead captured
//! verbatim as a raw `rtf:shprslt` property (see [`crate::ast::Inline::Shape`]),
//! preserving it losslessly without asserting an unconfirmed semantic.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub mod read {
    use crate::{Align, Block, Inline, RtfDoc};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions, PropValue};
    use rescribe_format_api::Parse as _;
    use rescribe_std::{node, prop};
    use std::collections::HashMap;

    /// EMU per twip, exact (914,400 EMU/inch ÷ 1,440 twips/inch = 635).
    /// See ADR 0015 Decision 4.
    const EMU_PER_TWIP: i64 = 635;

    /// Parse an RTF document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse an RTF document with custom options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (rtf, _diagnostics) = RtfDoc::parse(input.as_bytes());
        let nodes = doc_to_nodes(&rtf);
        let root = Node::new(node::DOCUMENT).children(nodes);
        let doc = Document::new().with_content(root);
        Ok(ConversionResult::ok(doc))
    }

    fn doc_to_nodes(rtf: &RtfDoc) -> Vec<Node> {
        rtf.blocks.iter().map(block_to_node).collect()
    }

    fn block_to_node(block: &Block) -> Node {
        match block {
            Block::Paragraph {
                inlines,
                align,
                para_props,
                ..
            } => {
                let mut node = Node::new(node::PARAGRAPH).children(inlines_to_nodes(inlines));
                if *align != Align::Default {
                    let align_str = match align {
                        Align::Left => "left",
                        Align::Right => "right",
                        Align::Center => "center",
                        Align::Justify => "justify",
                        Align::Default => unreachable!(),
                    };
                    node = node.prop(prop::STYLE_ALIGN, align_str);
                }
                if !para_props.is_empty() {
                    node = node.prop("rtf:para-props", para_props.clone());
                }
                node
            }

            Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(inlines_to_nodes(inlines)),

            Block::CodeBlock { content, .. } => {
                Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone())
            }

            Block::Blockquote { children, .. } => {
                Node::new(node::BLOCKQUOTE).children(children.iter().map(block_to_node))
            }

            Block::List { ordered, items, .. } => {
                let list_items: Vec<Node> = items
                    .iter()
                    .map(|item_blocks| {
                        Node::new(node::LIST_ITEM).children(item_blocks.iter().map(block_to_node))
                    })
                    .collect();
                Node::new(node::LIST)
                    .prop(prop::ORDERED, *ordered)
                    .children(list_items)
            }

            Block::Table { rows, .. } => {
                let row_nodes: Vec<Node> = rows
                    .iter()
                    .map(|row| {
                        let cells: Vec<Node> = row
                            .cells
                            .iter()
                            .map(|cell| {
                                Node::new(node::TABLE_CELL).children(inlines_to_nodes(cell))
                            })
                            .collect();
                        Node::new(node::TABLE_ROW).children(cells)
                    })
                    .collect();
                Node::new(node::TABLE).children(row_nodes)
            }

            Block::HorizontalRule { .. } => Node::new(node::HORIZONTAL_RULE),
        }
    }

    fn inlines_to_nodes(inlines: &[Inline]) -> Vec<Node> {
        inlines.iter().map(inline_to_node).collect()
    }

    fn inline_to_node(inline: &Inline) -> Node {
        match inline {
            Inline::Text { text, .. } => Node::new(node::TEXT).prop(prop::CONTENT, text.clone()),

            Inline::Bold { children, .. } => {
                Node::new(node::STRONG).children(inlines_to_nodes(children))
            }

            Inline::Italic { children, .. } => {
                Node::new(node::EMPHASIS).children(inlines_to_nodes(children))
            }

            Inline::Underline { children, .. } => {
                Node::new(node::UNDERLINE).children(inlines_to_nodes(children))
            }

            Inline::Strikethrough { children, .. } => {
                Node::new(node::STRIKEOUT).children(inlines_to_nodes(children))
            }

            Inline::Code { text, .. } => Node::new(node::CODE).prop(prop::CONTENT, text.clone()),

            Inline::Link { url, children, .. } => Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(inlines_to_nodes(children)),

            Inline::Image { url, alt, .. } => Node::new(node::IMAGE)
                .prop(prop::URL, url.clone())
                .prop(prop::ALT, alt.clone()),

            Inline::LineBreak { .. } => Node::new(node::LINE_BREAK),

            Inline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),

            Inline::Superscript { children, .. } => {
                Node::new(node::SUPERSCRIPT).children(inlines_to_nodes(children))
            }

            Inline::Subscript { children, .. } => {
                Node::new(node::SUBSCRIPT).children(inlines_to_nodes(children))
            }

            Inline::FontSize { size, children, .. } => {
                // size is in half-points; convert to points as string
                let pts = *size as f64 / 2.0;
                Node::new(node::SPAN)
                    .prop(prop::STYLE_SIZE, format!("{pts}pt"))
                    .children(inlines_to_nodes(children))
            }

            Inline::Color {
                r, g, b, children, ..
            } => Node::new(node::SPAN)
                .prop(prop::STYLE_COLOR, format!("#{r:02x}{g:02x}{b:02x}"))
                .children(inlines_to_nodes(children)),

            Inline::AllCaps { children, .. } => {
                Node::new(node::ALL_CAPS).children(inlines_to_nodes(children))
            }

            Inline::SmallCaps { children, .. } => {
                Node::new(node::SMALL_CAPS).children(inlines_to_nodes(children))
            }

            Inline::Hidden { children, .. } => {
                Node::new(node::HIDDEN).children(inlines_to_nodes(children))
            }

            Inline::CharSpan {
                char_props,
                children,
                ..
            } => Node::new("rtf:char-span")
                .prop("rtf:char-props", char_props.clone())
                .children(inlines_to_nodes(children)),

            Inline::Font { name, children, .. } => Node::new(node::SPAN)
                .prop(prop::STYLE_FONT, name.clone())
                .children(inlines_to_nodes(children)),

            Inline::BgColor {
                r, g, b, children, ..
            } => Node::new(node::SPAN)
                .prop("style:background", format!("#{r:02x}{g:02x}{b:02x}"))
                .children(inlines_to_nodes(children)),

            Inline::Lang { lcid, children, .. } => Node::new(node::SPAN)
                .prop("lang", lcid_to_bcp47(*lcid))
                .children(inlines_to_nodes(children)),

            Inline::Footnote { content, .. } => {
                Node::new(node::FOOTNOTE_REF).children(content.iter().map(block_to_node))
            }

            Inline::Shape {
                x,
                y,
                width,
                height,
                z_order,
                shape_props,
                named_props,
                text,
                fallback_raw,
                ..
            } => {
                let mut n = Node::new(node::POSITIONED_CONTAINER)
                    .prop(prop::POSITION_X, x * EMU_PER_TWIP)
                    .prop(prop::POSITION_Y, y * EMU_PER_TWIP)
                    .prop(prop::POSITION_WIDTH, width * EMU_PER_TWIP)
                    .prop(prop::POSITION_HEIGHT, height * EMU_PER_TWIP)
                    // ADR 0015 Decision 6: RTF's \shpz is explicit, so
                    // position:z_order is always set directly from it (no
                    // document-order fallback needed, unlike OOXML).
                    .prop(prop::POSITION_Z_ORDER, *z_order);
                // No position:rotation: see this module's top-level doc
                // comment ("No rotation property found") — RTF's \shp
                // format has no rotation concept to project, confirmed by
                // searching real-world \sp named properties, not omitted
                // for lack of trying.
                if !shape_props.is_empty() {
                    n = n.prop("rtf:shpinst-props", shape_props.clone());
                }
                if !fallback_raw.is_empty() {
                    n = n.prop("rtf:shprslt", fallback_raw.clone());
                }
                if !named_props.is_empty() {
                    let list = named_props
                        .iter()
                        .map(|(name, value)| {
                            let mut map = HashMap::new();
                            map.insert("name".to_string(), PropValue::String(name.clone()));
                            map.insert("value".to_string(), PropValue::String(value.clone()));
                            PropValue::Map(map)
                        })
                        .collect();
                    n = n.prop("rtf:sp", PropValue::List(list));
                }
                n.children(text.iter().map(block_to_node))
            }
        }
    }

    /// Convert a Windows LCID to a BCP-47 language tag.
    ///
    /// Covers the most common LCIDs; uncommon ones are represented as `und-LCID`.
    fn lcid_to_bcp47(lcid: u16) -> String {
        match lcid {
            1025 => "ar-SA",
            1026 => "bg-BG",
            1027 => "ca-ES",
            1028 => "zh-TW",
            1029 => "cs-CZ",
            1030 => "da-DK",
            1031 => "de-DE",
            1032 => "el-GR",
            1033 => "en-US",
            1034 => "es-ES",
            1035 => "fi-FI",
            1036 => "fr-FR",
            1037 => "he-IL",
            1038 => "hu-HU",
            1040 => "it-IT",
            1041 => "ja-JP",
            1042 => "ko-KR",
            1043 => "nl-NL",
            1044 => "nb-NO",
            1045 => "pl-PL",
            1046 => "pt-BR",
            1048 => "ro-RO",
            1049 => "ru-RU",
            1050 => "hr-HR",
            1051 => "sk-SK",
            1052 => "sq-AL",
            1053 => "sv-SE",
            1054 => "th-TH",
            1055 => "tr-TR",
            1058 => "uk-UA",
            1060 => "sl-SI",
            1061 => "et-EE",
            1062 => "lv-LV",
            1063 => "lt-LT",
            1066 => "vi-VN",
            2052 => "zh-CN",
            2055 => "de-CH",
            2057 => "en-GB",
            2058 => "es-MX",
            2060 => "fr-BE",
            2064 => "it-CH",
            2067 => "nl-BE",
            2068 => "nn-NO",
            2070 => "pt-PT",
            2074 => "sr-Latn-CS",
            2077 => "sv-FI",
            3076 => "zh-HK",
            3079 => "de-AT",
            3081 => "en-AU",
            3082 => "es-ES",
            3084 => "fr-CA",
            _ => return format!("und-x-lcid{lcid}"),
        }
        .to_string()
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_core::Document;

        fn parse_str(input: &str) -> Document {
            parse(input).unwrap().value
        }

        #[test]
        fn test_parse_simple_text() {
            let doc = parse_str(r"{\rtf1 Hello world\par}");
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
        }

        #[test]
        fn test_parse_bold() {
            let doc = parse_str(r"{\rtf1 \b bold text\b0 normal\par}");
            let para = &doc.content.children[0];
            assert!(
                para.children
                    .iter()
                    .any(|n| n.kind.as_str() == node::STRONG)
            );
        }

        #[test]
        fn test_parse_italic() {
            let doc = parse_str(r"{\rtf1 \i italic\i0\par}");
            let para = &doc.content.children[0];
            assert!(
                para.children
                    .iter()
                    .any(|n| n.kind.as_str() == node::EMPHASIS)
            );
        }

        #[test]
        fn test_parse_underline() {
            let doc = parse_str(r"{\rtf1 \ul underlined\ulnone\par}");
            let para = &doc.content.children[0];
            assert!(
                para.children
                    .iter()
                    .any(|n| n.kind.as_str() == node::UNDERLINE)
            );
        }

        #[test]
        fn test_parse_multiple_paragraphs() {
            let doc = parse_str(r"{\rtf1 First paragraph\par Second paragraph\par}");
            assert_eq!(doc.content.children.len(), 2);
        }

        #[test]
        fn test_parse_escaped_chars() {
            let doc = parse_str(r"{\rtf1 Open \{ and close \}\par}");
            let para = &doc.content.children[0];
            let text = get_all_text(para);
            assert!(text.contains('{'));
            assert!(text.contains('}'));
        }

        #[test]
        fn test_parse_special_chars() {
            let doc = parse_str(r"{\rtf1 Em\emdash dash\par}");
            let para = &doc.content.children[0];
            let text = get_all_text(para);
            assert!(text.contains('\u{2014}'));
        }

        #[test]
        fn test_parse_alignment() {
            let doc = parse_str(r"{\rtf1 \qc centered\par}");
            let para = &doc.content.children[0];
            assert_eq!(para.props.get_str(prop::STYLE_ALIGN), Some("center"));
        }

        #[test]
        fn test_parse_font_size() {
            let doc = parse_str(r"{\rtf1 \fs48 big\par}");
            let para = &doc.content.children[0];
            // FontSize node should become a SPAN with style:size
            let span = para
                .children
                .iter()
                .find(|n| n.kind.as_str() == node::SPAN)
                .expect("parsed paragraph should contain a span node");
            assert_eq!(span.props.get_str(prop::STYLE_SIZE), Some("24pt"));
        }

        #[test]
        fn test_parse_color() {
            let doc = parse_str(r"{\rtf1{\colortbl ;\red255\green0\blue0;}\cf1 red\par}");
            let para = &doc.content.children[0];
            let span = para
                .children
                .iter()
                .find(|n| n.kind.as_str() == node::SPAN)
                .expect("parsed paragraph should contain a span node");
            assert_eq!(span.props.get_str(prop::STYLE_COLOR), Some("#ff0000"));
        }

        fn get_all_text(node: &Node) -> String {
            let mut text = String::new();
            if let Some(content) = node.props.get_str(prop::CONTENT) {
                text.push_str(content);
            }
            for child in &node.children {
                text.push_str(&get_all_text(child));
            }
            text
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub mod write {
    use crate::{Align, Block, Inline, RtfDoc, Span, TableRow};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node, PropValue};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// EMU per twip, exact (914,400 EMU/inch ÷ 1,440 twips/inch = 635).
    /// See ADR 0015 Decision 4.
    const EMU_PER_TWIP: i64 = 635;

    /// Emit a document as RTF.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as RTF with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let rtf = doc_to_rtf(doc);
        let output = rtf.emit();
        Ok(ConversionResult::ok(output))
    }

    fn doc_to_rtf(doc: &Document) -> RtfDoc {
        RtfDoc {
            blocks: nodes_to_blocks(&doc.content.children),
            color_table: vec![],
            span: Span::NONE,
        }
    }

    fn nodes_to_blocks(nodes: &[Node]) -> Vec<Block> {
        nodes.iter().flat_map(node_to_blocks).collect()
    }

    /// Convert a rescribe node to zero or more `Block`s.
    fn node_to_blocks(node: &Node) -> Vec<Block> {
        match node.kind.as_str() {
            node::DOCUMENT => nodes_to_blocks(&node.children),

            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as u8;
                vec![Block::Heading {
                    level,
                    inlines: nodes_to_inlines(&node.children),
                    span: Span::NONE,
                }]
            }

            node::PARAGRAPH => {
                let align = parse_align(node.props.get_str(prop::STYLE_ALIGN));
                let para_props = node
                    .props
                    .get_str("rtf:para-props")
                    .unwrap_or("")
                    .to_string();
                vec![Block::Paragraph {
                    inlines: nodes_to_inlines(&node.children),
                    align,
                    para_props,
                    span: Span::NONE,
                }]
            }

            node::CODE_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                vec![Block::CodeBlock {
                    content,
                    span: Span::NONE,
                }]
            }

            node::BLOCKQUOTE => vec![Block::Blockquote {
                children: nodes_to_blocks(&node.children),
                span: Span::NONE,
            }],

            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let items: Vec<Vec<Block>> = node
                    .children
                    .iter()
                    .filter(|c| c.kind.as_str() == node::LIST_ITEM)
                    .map(|item| nodes_to_blocks(&item.children))
                    .collect();
                vec![Block::List {
                    ordered,
                    items,
                    span: Span::NONE,
                }]
            }

            node::LIST_ITEM => nodes_to_blocks(&node.children),

            node::TABLE => {
                let rows: Vec<TableRow> = node
                    .children
                    .iter()
                    .filter(|r| {
                        r.kind.as_str() == node::TABLE_ROW || r.kind.as_str() == node::TABLE_HEADER
                    })
                    .map(|row| TableRow {
                        cells: row
                            .children
                            .iter()
                            .map(|cell| nodes_to_inlines(&cell.children))
                            .collect(),
                        span: Span::NONE,
                    })
                    .collect();
                vec![Block::Table {
                    rows,
                    span: Span::NONE,
                }]
            }

            node::HORIZONTAL_RULE => vec![Block::HorizontalRule { span: Span::NONE }],

            node::DIV | node::FIGURE => nodes_to_blocks(&node.children),

            node::SPAN => {
                // A SPAN at block level: treat as a paragraph wrapping inline content
                vec![Block::Paragraph {
                    inlines: nodes_to_inlines(std::slice::from_ref(node)),
                    align: Align::Default,
                    para_props: String::new(),
                    span: Span::NONE,
                }]
            }

            // Inline nodes at block level: wrap in a paragraph
            node::TEXT | node::STRONG | node::EMPHASIS | node::CODE | node::LINK => {
                vec![Block::Paragraph {
                    inlines: nodes_to_inlines(std::slice::from_ref(node)),
                    align: Align::Default,
                    para_props: String::new(),
                    span: Span::NONE,
                }]
            }

            // positioned_container at block level: RTF only ever anchors a
            // shape inline within a paragraph's content stream (see
            // rescribe.rs's module doc comment — confirmed from real
            // Word-generated files), so wrap it in one, same as SPAN above.
            node::POSITIONED_CONTAINER => {
                vec![Block::Paragraph {
                    inlines: nodes_to_inlines(std::slice::from_ref(node)),
                    align: Align::Default,
                    para_props: String::new(),
                    span: Span::NONE,
                }]
            }

            _ => nodes_to_blocks(&node.children),
        }
    }

    /// Parse a CSS-style alignment string to an `Align` value.
    fn parse_align(s: Option<&str>) -> Align {
        match s {
            Some("left") => Align::Left,
            Some("right") => Align::Right,
            Some("center") => Align::Center,
            Some("justify") => Align::Justify,
            _ => Align::Default,
        }
    }

    fn nodes_to_inlines(nodes: &[Node]) -> Vec<Inline> {
        nodes.iter().map(node_to_inline).collect()
    }

    fn node_to_inline(node: &Node) -> Inline {
        match node.kind.as_str() {
            node::TEXT => {
                let text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Text {
                    text,
                    span: Span::NONE,
                }
            }

            node::STRONG => Inline::Bold {
                children: nodes_to_inlines(&node.children),
                span: Span::NONE,
            },

            node::EMPHASIS => Inline::Italic {
                children: nodes_to_inlines(&node.children),
                span: Span::NONE,
            },

            node::UNDERLINE => Inline::Underline {
                children: nodes_to_inlines(&node.children),
                span: Span::NONE,
            },

            node::STRIKEOUT => Inline::Strikethrough {
                children: nodes_to_inlines(&node.children),
                span: Span::NONE,
            },

            node::CODE => {
                let text = node.props.get_str(prop::CONTENT).unwrap_or("").to_string();
                Inline::Code {
                    text,
                    span: Span::NONE,
                }
            }

            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                Inline::Link {
                    url,
                    children: nodes_to_inlines(&node.children),
                    span: Span::NONE,
                }
            }

            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("").to_string();
                let alt = node.props.get_str(prop::ALT).unwrap_or("").to_string();
                Inline::Image {
                    url,
                    alt,
                    span: Span::NONE,
                }
            }

            node::LINE_BREAK => Inline::LineBreak { span: Span::NONE },

            node::SOFT_BREAK => Inline::SoftBreak { span: Span::NONE },

            node::SUPERSCRIPT => Inline::Superscript {
                children: nodes_to_inlines(&node.children),
                span: Span::NONE,
            },

            node::SUBSCRIPT => Inline::Subscript {
                children: nodes_to_inlines(&node.children),
                span: Span::NONE,
            },

            node::ALL_CAPS => Inline::AllCaps {
                children: nodes_to_inlines(&node.children),
                span: Span::NONE,
            },

            node::SMALL_CAPS => Inline::SmallCaps {
                children: nodes_to_inlines(&node.children),
                span: Span::NONE,
            },

            node::HIDDEN => Inline::Hidden {
                children: nodes_to_inlines(&node.children),
                span: Span::NONE,
            },

            "rtf:char-span" => {
                let char_props = node
                    .props
                    .get_str("rtf:char-props")
                    .unwrap_or("")
                    .to_string();
                Inline::CharSpan {
                    char_props,
                    children: nodes_to_inlines(&node.children),
                    span: Span::NONE,
                }
            }

            node::POSITIONED_CONTAINER => {
                let x = node.props.get_int(prop::POSITION_X).unwrap_or(0) / EMU_PER_TWIP;
                let y = node.props.get_int(prop::POSITION_Y).unwrap_or(0) / EMU_PER_TWIP;
                let width = node.props.get_int(prop::POSITION_WIDTH).unwrap_or(0) / EMU_PER_TWIP;
                let height = node.props.get_int(prop::POSITION_HEIGHT).unwrap_or(0) / EMU_PER_TWIP;
                let z_order = node.props.get_int(prop::POSITION_Z_ORDER).unwrap_or(0);
                // position:rotation is intentionally never read back: RTF's
                // \shp format has no rotation control word to emit it as
                // (see this module's top-level doc comment). A document
                // carrying rotation on a shape bound for RTF loses it
                // silently here — RTF simply cannot represent it, so there
                // is no raw fallback to fall back to either.
                let shape_props = node
                    .props
                    .get_str("rtf:shpinst-props")
                    .unwrap_or("")
                    .to_string();
                let fallback_raw = node.props.get_str("rtf:shprslt").unwrap_or("").to_string();
                let named_props = match node.props.get("rtf:sp") {
                    Some(PropValue::List(items)) => items
                        .iter()
                        .filter_map(|item| {
                            let PropValue::Map(map) = item else {
                                return None;
                            };
                            let name = match map.get("name") {
                                Some(PropValue::String(s)) => s.clone(),
                                _ => String::new(),
                            };
                            let value = match map.get("value") {
                                Some(PropValue::String(s)) => s.clone(),
                                _ => String::new(),
                            };
                            Some((name, value))
                        })
                        .collect(),
                    _ => Vec::new(),
                };
                Inline::Shape {
                    x,
                    y,
                    width,
                    height,
                    z_order,
                    shape_props,
                    named_props,
                    text: nodes_to_blocks(&node.children),
                    fallback_raw,
                    span: Span::NONE,
                }
            }

            node::SPAN => {
                // Check for style:size → FontSize
                if let Some(size_str) = node.props.get_str(prop::STYLE_SIZE)
                    && let Some(size) = parse_half_points(size_str)
                {
                    return Inline::FontSize {
                        size,
                        children: nodes_to_inlines(&node.children),
                        span: Span::NONE,
                    };
                }
                // Check for style:color → Color
                if let Some(color_str) = node.props.get_str(prop::STYLE_COLOR)
                    && let Some((r, g, b)) = parse_hex_color(color_str)
                {
                    return Inline::Color {
                        r,
                        g,
                        b,
                        children: nodes_to_inlines(&node.children),
                        span: Span::NONE,
                    };
                }
                // Plain span: pass children through
                let children = nodes_to_inlines(&node.children);
                match <[_; 1]>::try_from(children) {
                    Ok([only]) => only,
                    Err(children) if children.is_empty() => Inline::Text {
                        text: String::new(),
                        span: Span::NONE,
                    },
                    // Wrap in bold as a neutral container (best we can do without a generic span)
                    Err(children) => Inline::Bold {
                        children,
                        span: Span::NONE,
                    },
                }
            }

            _ => {
                let children = nodes_to_inlines(&node.children);
                match <[_; 1]>::try_from(children) {
                    Ok([only]) => only,
                    Err(children) if children.is_empty() => Inline::Text {
                        text: String::new(),
                        span: Span::NONE,
                    },
                    Err(children) => Inline::Bold {
                        children,
                        span: Span::NONE,
                    },
                }
            }
        }
    }

    /// Parse a font size string like `"12pt"` or `"12.5pt"` to half-points.
    /// Returns `None` if parsing fails.
    fn parse_half_points(s: &str) -> Option<u16> {
        let s = s.trim();
        if let Some(pt_str) = s.strip_suffix("pt") {
            let pts: f64 = pt_str.trim().parse().ok()?;
            Some((pts * 2.0).round() as u16)
        } else {
            None
        }
    }

    /// Parse a `#rrggbb` hex color string to `(r, g, b)`.
    /// Returns `None` if the format is wrong.
    fn parse_hex_color(s: &str) -> Option<(u8, u8, u8)> {
        let s = s.trim().strip_prefix('#')?;
        if s.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some((r, g, b))
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_std::builder::*;

        fn emit_str(doc: &Document) -> String {
            let result = emit(doc).unwrap();
            String::from_utf8(result.value).unwrap()
        }

        #[test]
        fn test_emit_rtf_header() {
            let doc = doc(|d| d.para(|p| p.text("Hello")));
            let output = emit_str(&doc);
            assert!(output.starts_with("{\\rtf1"));
            assert!(output.ends_with('}'));
        }

        #[test]
        fn test_emit_paragraph() {
            let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
            let output = emit_str(&doc);
            assert!(output.contains("Hello, world!"));
            assert!(output.contains("\\par"));
        }

        #[test]
        fn test_emit_bold() {
            let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
            let output = emit_str(&doc);
            assert!(output.contains("{\\b bold}"));
        }

        #[test]
        fn test_emit_italic() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            let output = emit_str(&doc);
            assert!(output.contains("{\\i italic}"));
        }

        #[test]
        fn test_emit_underline() {
            let doc = doc(|d| d.para(|p| p.underline(|u| u.text("underlined"))));
            let output = emit_str(&doc);
            assert!(output.contains("{\\ul underlined}"));
        }

        #[test]
        fn test_emit_heading() {
            let doc = doc(|d| d.heading(1, |h| h.text("Title")));
            let output = emit_str(&doc);
            assert!(output.contains("\\b "));
            assert!(output.contains("Title"));
        }

        #[test]
        fn test_emit_escaped_chars() {
            let doc = doc(|d| d.para(|p| p.text("Open { and close }")));
            let output = emit_str(&doc);
            assert!(output.contains("\\{"));
            assert!(output.contains("\\}"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("http://example.com", |l| l.text("click"))));
            let output = emit_str(&doc);
            assert!(output.contains("HYPERLINK"));
            assert!(output.contains("http://example.com"));
            assert!(output.contains("click"));
        }

        #[test]
        fn test_emit_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains("\\pnlvlblt"));
            assert!(output.contains("one"));
            assert!(output.contains("two"));
        }

        #[test]
        fn test_parse_half_points() {
            assert_eq!(parse_half_points("12pt"), Some(24));
            assert_eq!(parse_half_points("12.5pt"), Some(25));
            assert_eq!(parse_half_points("24pt"), Some(48));
            assert_eq!(parse_half_points("bad"), None);
        }

        #[test]
        fn test_parse_hex_color() {
            assert_eq!(parse_hex_color("#ff0000"), Some((255, 0, 0)));
            assert_eq!(parse_hex_color("#00ff00"), Some((0, 255, 0)));
            assert_eq!(parse_hex_color("#0080ff"), Some((0, 128, 255)));
            assert_eq!(parse_hex_color("bad"), None);
        }

        #[test]
        fn test_parse_align() {
            assert_eq!(parse_align(Some("left")), Align::Left);
            assert_eq!(parse_align(Some("right")), Align::Right);
            assert_eq!(parse_align(Some("center")), Align::Center);
            assert_eq!(parse_align(Some("justify")), Align::Justify);
            assert_eq!(parse_align(None), Align::Default);
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
