//! Markdown → rescribe IR adapter using the `commonmark-fmt` crate.
//!
//! Translates [`commonmark_fmt::CmDoc`] into a rescribe [`Document`].
//! All IR construction happens here; `commonmark-fmt` has no rescribe dependency.

use commonmark_fmt::{Block, CmDoc, Inline, ListItem, ListKind};
use rescribe_core::{ConversionResult, Document, FidelityWarning, ParseOptions, Properties, Severity, Span, WarningKind};
use rescribe_std::{Node, node, prop};

/// Parse markdown bytes into a rescribe Document.
pub fn parse_with_options(
    input: &[u8],
    _opts: &ParseOptions,
) -> ConversionResult<Document> {
    let (cm_doc, diags) = commonmark_fmt::parse(input);

    let mut warnings: Vec<FidelityWarning> = diags
        .into_iter()
        .map(|d| {
            FidelityWarning::new(
                Severity::Minor,
                WarningKind::FeatureLost(d.code.to_string()),
                d.message,
            )
        })
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
            maybe_span(Node::new(node::PARAGRAPH).children(children), span)
        }
        Block::Heading { level, inlines, span } => {
            let children = convert_inlines(inlines, warnings);
            let n = Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(children);
            maybe_span(n, span)
        }
        Block::CodeBlock { language, content, span } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            maybe_span(n, span)
        }
        Block::HtmlBlock { content, span } => {
            let n = Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, "html")
                .prop(prop::CONTENT, content.clone());
            maybe_span(n, span)
        }
        Block::Blockquote { blocks, span } => {
            let children: Vec<Node> = blocks.iter().map(|b| convert_block(b, warnings)).collect();
            maybe_span(Node::new(node::BLOCKQUOTE).children(children), span)
        }
        Block::List { kind, items, tight, span } => {
            let (ordered, start) = match kind {
                ListKind::Unordered { .. } => (false, None::<u64>),
                ListKind::Ordered { start, .. } => (true, Some(*start)),
            };
            let item_nodes: Vec<Node> = items.iter().map(|item| convert_list_item(item, *tight, warnings)).collect();
            let mut list = Node::new(node::LIST)
                .prop(prop::ORDERED, ordered)
                .prop(prop::TIGHT, *tight)
                .children(item_nodes);
            if let Some(s) = start {
                list = list.prop(prop::START, s as i64);
            }
            maybe_span(list, span)
        }
        Block::ThematicBreak { span } => {
            maybe_span(Node::new(node::HORIZONTAL_RULE), span)
        }
    }
}

/// Convert a list item's blocks.
///
/// For **tight** list items (no blank lines between items), `commonmark-fmt` still
/// wraps content in `Block::Paragraph` internally, but the IR spec says tight items
/// contain inline nodes directly (not wrapped in a paragraph). For loose items the
/// paragraph wrapper is preserved.
///
/// Unwrapping rule: if `tight` is true AND the item has exactly one `Block::Paragraph`
/// AND that paragraph's content consists only of inline nodes (no nested blocks), emit
/// the paragraph's inlines as the item's direct children.
fn convert_list_item(
    item: &ListItem,
    tight: bool,
    warnings: &mut Vec<FidelityWarning>,
) -> Node {
    let children: Vec<Node> = if tight
        && item.blocks.len() == 1
        && let Block::Paragraph { inlines, .. } = &item.blocks[0]
    {
        // Tight item: unwrap the implicit paragraph — emit inlines directly.
        convert_inlines(inlines, warnings)
    } else {
        item.blocks.iter().map(|b| convert_block(b, warnings)).collect()
    };
    maybe_span(Node::new(node::LIST_ITEM).children(children), &item.span)
}

fn convert_inlines(inlines: &[Inline], warnings: &mut Vec<FidelityWarning>) -> Vec<Node> {
    inlines.iter().map(|i| convert_inline(i, warnings)).collect()
}

fn convert_inline(inline: &Inline, warnings: &mut Vec<FidelityWarning>) -> Node {
    match inline {
        Inline::Text { content, span } => {
            maybe_span(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()), span)
        }
        Inline::SoftBreak { span } => maybe_span(Node::new(node::SOFT_BREAK), span),
        Inline::HardBreak { span } => maybe_span(Node::new(node::LINE_BREAK), span),
        Inline::Emphasis { inlines, span } => {
            let children = convert_inlines(inlines, warnings);
            maybe_span(Node::new(node::EMPHASIS).children(children), span)
        }
        Inline::Strong { inlines, span } => {
            let children = convert_inlines(inlines, warnings);
            maybe_span(Node::new(node::STRONG).children(children), span)
        }
        Inline::Strikethrough { inlines, span } => {
            let children = convert_inlines(inlines, warnings);
            maybe_span(Node::new(node::STRIKEOUT).children(children), span)
        }
        Inline::Code { content, span } => {
            maybe_span(Node::new(node::CODE).prop(prop::CONTENT, content.clone()), span)
        }
        Inline::HtmlInline { content, span } => {
            let n = Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "html")
                .prop(prop::CONTENT, content.clone());
            maybe_span(n, span)
        }
        Inline::Link { inlines, url, title, span } => {
            let children = convert_inlines(inlines, warnings);
            let mut n = Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(children);
            if let Some(t) = title {
                n = n.prop(prop::TITLE, t.clone());
            }
            maybe_span(n, span)
        }
        Inline::Image { alt, url, title, span } => {
            let mut n = Node::new(node::IMAGE)
                .prop(prop::URL, url.clone())
                .prop(prop::ALT, alt.clone());
            if let Some(t) = title {
                n = n.prop(prop::TITLE, t.clone());
            }
            maybe_span(n, span)
        }
    }
}

/// Attach a [`Span`] to a node if the span is non-zero (i.e. not `Span::NONE`).
///
/// `commonmark-fmt` always records spans; we store them unconditionally since
/// rescribe-read-markdown's public API doesn't yet thread `preserve_source_info`
/// through to this adapter. Callers that want no spans can strip them afterward.
fn maybe_span(mut node: Node, span: &commonmark_fmt::Span) -> Node {
    if span.start != 0 || span.end != 0 {
        node.span = Some(Span { start: span.start, end: span.end });
    }
    node
}
