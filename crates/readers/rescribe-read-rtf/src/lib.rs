//! RTF (Rich Text Format) reader for rescribe.
//!
//! Thin adapter over [`rtf_fmt`]: parses RTF into the `rtf_fmt` AST,
//! then maps it to the rescribe document model.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};
use rtf_fmt::{Align, Block, Inline, RtfDoc};

/// Parse an RTF document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse an RTF document with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (rtf, _diagnostics) = rtf_fmt::parse(input.as_bytes());
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
                        .map(|cell| Node::new(node::TABLE_CELL).children(inlines_to_nodes(cell)))
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
        let span = para.children.iter().find(|n| n.kind.as_str() == node::SPAN);
        assert!(span.is_some());
        let span = span.unwrap();
        assert_eq!(span.props.get_str(prop::STYLE_SIZE), Some("24pt"));
    }

    #[test]
    fn test_parse_color() {
        let doc = parse_str(r"{\rtf1{\colortbl ;\red255\green0\blue0;}\cf1 red\par}");
        let para = &doc.content.children[0];
        let span = para.children.iter().find(|n| n.kind.as_str() == node::SPAN);
        assert!(span.is_some());
        let span = span.unwrap();
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
