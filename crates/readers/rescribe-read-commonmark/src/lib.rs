//! CommonMark reader for rescribe.
//!
//! Parses CommonMark (with GFM strikethrough) into rescribe's document IR
//! using the `commonmark-fmt` crate.

use commonmark_fmt::{Block, CmDoc, Inline, ListItem, ListKind};
use rescribe_core::{ConversionResult, Document, FidelityWarning, ParseError, ParseOptions, Properties, Severity, Span, WarningKind};
use rescribe_std::{Node, node, prop};

/// Parse CommonMark input into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse CommonMark input into a document with options.
pub fn parse_with_options(
    input: &str,
    opts: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    Ok(parse_bytes(input.as_bytes(), opts))
}

fn parse_bytes(input: &[u8], _opts: &ParseOptions) -> ConversionResult<Document> {
    let (cm_doc, diags) = commonmark_fmt::parse(input);

    let mut warnings: Vec<FidelityWarning> = diags
        .into_iter()
        .map(|d| FidelityWarning::new(
            Severity::Minor,
            WarningKind::FeatureLost(d.code.to_string()),
            d.message,
        ))
        .collect();

    let children = convert_doc(&cm_doc, &mut warnings);
    let root = Node::new(node::DOCUMENT).children(children);
    let doc = Document::new()
        .with_content(root)
        .with_metadata(Properties::new());
    ConversionResult::with_warnings(doc, warnings)
}

fn convert_doc(doc: &CmDoc, warnings: &mut Vec<FidelityWarning>) -> Vec<Node> {
    doc.blocks.iter().map(|b| convert_block(b, warnings)).collect()
}

fn convert_block(block: &Block, warnings: &mut Vec<FidelityWarning>) -> Node {
    match block {
        Block::Paragraph { inlines, span } => {
            let children = convert_inlines(inlines, warnings);
            span_node(Node::new(node::PARAGRAPH).children(children), span)
        }
        Block::Heading { level, inlines, span } => {
            let children = convert_inlines(inlines, warnings);
            span_node(
                Node::new(node::HEADING).prop(prop::LEVEL, *level as i64).children(children),
                span,
            )
        }
        Block::CodeBlock { language, content, span } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            span_node(n, span)
        }
        Block::HtmlBlock { content, span } => {
            span_node(
                Node::new(node::RAW_BLOCK)
                    .prop(prop::FORMAT, "html")
                    .prop(prop::CONTENT, content.clone()),
                span,
            )
        }
        Block::Blockquote { blocks, span } => {
            let children: Vec<Node> = blocks.iter().map(|b| convert_block(b, warnings)).collect();
            span_node(Node::new(node::BLOCKQUOTE).children(children), span)
        }
        Block::List { kind, items, tight, span } => {
            let (ordered, start) = match kind {
                ListKind::Unordered { .. } => (false, None::<u64>),
                ListKind::Ordered { start, .. } => (true, Some(*start)),
            };
            let item_nodes: Vec<Node> =
                items.iter().map(|item| convert_list_item(item, *tight, warnings)).collect();
            let mut list = Node::new(node::LIST)
                .prop(prop::ORDERED, ordered)
                .prop(prop::TIGHT, *tight)
                .children(item_nodes);
            if let Some(s) = start {
                list = list.prop(prop::START, s as i64);
            }
            span_node(list, span)
        }
        Block::ThematicBreak { span } => span_node(Node::new(node::HORIZONTAL_RULE), span),
    }
}

fn convert_list_item(item: &ListItem, tight: bool, warnings: &mut Vec<FidelityWarning>) -> Node {
    let children: Vec<Node> = if tight
        && item.blocks.len() == 1
        && matches!(&item.blocks[0], Block::Paragraph { .. })
    {
        if let Block::Paragraph { inlines, .. } = &item.blocks[0] {
            convert_inlines(inlines, warnings)
        } else {
            unreachable!()
        }
    } else {
        item.blocks.iter().map(|b| convert_block(b, warnings)).collect()
    };
    span_node(Node::new(node::LIST_ITEM).children(children), &item.span)
}

fn convert_inlines(inlines: &[Inline], warnings: &mut Vec<FidelityWarning>) -> Vec<Node> {
    inlines.iter().map(|i| convert_inline(i, warnings)).collect()
}

fn convert_inline(inline: &Inline, warnings: &mut Vec<FidelityWarning>) -> Node {
    match inline {
        Inline::Text { content, span } => {
            span_node(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()), span)
        }
        Inline::SoftBreak { span } => span_node(Node::new(node::SOFT_BREAK), span),
        Inline::HardBreak { span } => span_node(Node::new(node::LINE_BREAK), span),
        Inline::Emphasis { inlines, span } => {
            span_node(Node::new(node::EMPHASIS).children(convert_inlines(inlines, warnings)), span)
        }
        Inline::Strong { inlines, span } => {
            span_node(Node::new(node::STRONG).children(convert_inlines(inlines, warnings)), span)
        }
        Inline::Strikethrough { inlines, span } => {
            span_node(Node::new(node::STRIKEOUT).children(convert_inlines(inlines, warnings)), span)
        }
        Inline::Code { content, span } => {
            span_node(Node::new(node::CODE).prop(prop::CONTENT, content.clone()), span)
        }
        Inline::HtmlInline { content, span } => span_node(
            Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "html")
                .prop(prop::CONTENT, content.clone()),
            span,
        ),
        Inline::Link { inlines, url, title, span } => {
            let mut n = Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(convert_inlines(inlines, warnings));
            if let Some(t) = title {
                n = n.prop(prop::TITLE, t.clone());
            }
            span_node(n, span)
        }
        Inline::Image { alt, url, title, span } => {
            let mut n = Node::new(node::IMAGE)
                .prop(prop::URL, url.clone())
                .prop(prop::ALT, alt.clone());
            if let Some(t) = title {
                n = n.prop(prop::TITLE, t.clone());
            }
            span_node(n, span)
        }
    }
}

fn span_node(mut node: Node, span: &commonmark_fmt::Span) -> Node {
    if span.start != 0 || span.end != 0 {
        node.span = Some(Span { start: span.start, end: span.end });
    }
    node
}
