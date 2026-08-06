//! `rescribe::Document` → HTML.
//!
//! Translates rescribe's document IR to `crate::HtmlDoc`, then emits via
//! `crate::emit_with_options`/`crate::emit_fragment`. All HTML
//! serialization lives in the rest of this crate; this module is a thin
//! translator.

use crate::Node as HtmlNode;
use crate::ast::Span;
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, ResourceId,
    ResourceMap, Severity, WarningKind,
};
use rescribe_std::{node, prop};

/// Emit a document as HTML.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as HTML with custom options.
pub fn emit_with_options(
    doc: &Document,
    options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut ctx = ConvertContext::new(&doc.resources);

    let nodes = convert_nodes(&doc.content.children, &mut ctx);
    let html_doc = crate::HtmlDoc { nodes };

    let emit_opts = crate::EmitOptions {
        pretty: options.pretty,
    };
    let bytes = crate::emit_with_options(&html_doc, &emit_opts);

    Ok(ConversionResult::with_warnings(bytes, ctx.warnings))
}

/// Emit a document as a complete HTML document with doctype.
pub fn emit_full_document(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_full_document_with_options(doc, &EmitOptions::default())
}

/// Emit a document as a complete HTML document with doctype and options.
pub fn emit_full_document_with_options(
    doc: &Document,
    options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut ctx = ConvertContext::new(&doc.resources);

    let body_nodes = convert_nodes(&doc.content.children, &mut ctx);

    let meta_charset = HtmlNode::Element {
        tag: "meta".into(),
        attrs: vec![("charset".into(), "utf-8".into())],
        children: vec![],
        self_closing: true,
        span: Span::NONE,
    };
    let head = HtmlNode::Element {
        tag: "head".into(),
        attrs: vec![],
        children: vec![meta_charset],
        self_closing: false,
        span: Span::NONE,
    };
    let body = HtmlNode::Element {
        tag: "body".into(),
        attrs: vec![],
        children: body_nodes,
        self_closing: false,
        span: Span::NONE,
    };
    let html_el = HtmlNode::Element {
        tag: "html".into(),
        attrs: vec![],
        children: vec![head, body],
        self_closing: false,
        span: Span::NONE,
    };
    let doctype = HtmlNode::Doctype {
        name: "html".into(),
        public_id: String::new(),
        system_id: String::new(),
        span: Span::NONE,
    };

    let html_doc = crate::HtmlDoc {
        nodes: vec![doctype, html_el],
    };

    let emit_opts = crate::EmitOptions {
        pretty: options.pretty,
    };
    let bytes = crate::emit_with_options(&html_doc, &emit_opts);

    Ok(ConversionResult::with_warnings(bytes, ctx.warnings))
}

/// Context for Document → HtmlDoc conversion.
struct ConvertContext<'a> {
    warnings: Vec<FidelityWarning>,
    resources: &'a ResourceMap,
}

impl<'a> ConvertContext<'a> {
    fn new(resources: &'a ResourceMap) -> Self {
        Self {
            warnings: Vec::new(),
            resources,
        }
    }
}

/// Convert a sequence of rescribe nodes to html-fmt nodes.
fn convert_nodes(nodes: &[Node], ctx: &mut ConvertContext) -> Vec<HtmlNode> {
    let mut result = Vec::new();
    for node in nodes {
        result.extend(convert_node(node, ctx));
    }
    result
}

/// Convert a single rescribe node to html-fmt node(s).
fn convert_node(node: &Node, ctx: &mut ConvertContext) -> Vec<HtmlNode> {
    match node.kind.as_str() {
        node::DOCUMENT => convert_nodes(&node.children, ctx),
        node::PARAGRAPH => vec![element(
            "p",
            common_attrs(node),
            convert_nodes(&node.children, ctx),
        )],
        node::HEADING => vec![convert_heading(node, ctx)],
        node::CODE_BLOCK => vec![convert_code_block(node)],
        node::BLOCKQUOTE => vec![element(
            "blockquote",
            common_attrs(node),
            convert_nodes(&node.children, ctx),
        )],
        node::LIST => vec![convert_list(node, ctx)],
        node::LIST_ITEM => vec![element(
            "li",
            common_attrs(node),
            convert_nodes(&node.children, ctx),
        )],
        node::TABLE => vec![element("table", vec![], convert_nodes(&node.children, ctx))],
        node::TABLE_HEAD => vec![element("thead", vec![], convert_nodes(&node.children, ctx))],
        node::TABLE_BODY => vec![element("tbody", vec![], convert_nodes(&node.children, ctx))],
        node::TABLE_FOOT => vec![element("tfoot", vec![], convert_nodes(&node.children, ctx))],
        node::TABLE_ROW => vec![element("tr", vec![], convert_nodes(&node.children, ctx))],
        node::TABLE_CELL => vec![convert_table_cell(node, "td", ctx)],
        node::TABLE_HEADER => vec![convert_table_cell(node, "th", ctx)],
        node::FIGURE => vec![element(
            "figure",
            vec![],
            convert_nodes(&node.children, ctx),
        )],
        node::CAPTION => vec![element(
            "figcaption",
            vec![],
            convert_nodes(&node.children, ctx),
        )],
        node::HORIZONTAL_RULE => vec![void_element("hr", vec![])],
        node::DIV => vec![convert_div(node, ctx)],
        node::RAW_BLOCK => convert_raw(node),
        node::DEFINITION_LIST => vec![element("dl", vec![], convert_nodes(&node.children, ctx))],
        node::DEFINITION_TERM => vec![element("dt", vec![], convert_nodes(&node.children, ctx))],
        node::DEFINITION_DESC => vec![element("dd", vec![], convert_nodes(&node.children, ctx))],
        node::TEXT => convert_text(node),
        node::EMPHASIS => vec![element(
            "em",
            common_attrs(node),
            convert_nodes(&node.children, ctx),
        )],
        node::STRONG => vec![element(
            "strong",
            common_attrs(node),
            convert_nodes(&node.children, ctx),
        )],
        node::STRIKEOUT => vec![element(
            "del",
            common_attrs(node),
            convert_nodes(&node.children, ctx),
        )],
        node::UNDERLINE => vec![element(
            "u",
            common_attrs(node),
            convert_nodes(&node.children, ctx),
        )],
        node::SUBSCRIPT => vec![element(
            "sub",
            common_attrs(node),
            convert_nodes(&node.children, ctx),
        )],
        node::SUPERSCRIPT => vec![element(
            "sup",
            common_attrs(node),
            convert_nodes(&node.children, ctx),
        )],
        node::CODE => vec![convert_inline_code(node)],
        node::LINK => vec![convert_link(node, ctx)],
        node::IMAGE => vec![convert_image(node, ctx)],
        node::LINE_BREAK => vec![void_element("br", vec![])],
        node::SOFT_BREAK => vec![HtmlNode::Text {
            content: "\n".into(),
            span: Span::NONE,
        }],
        node::SPAN => vec![convert_span(node, ctx)],
        node::RAW_INLINE => convert_raw(node),
        node::FOOTNOTE_REF => vec![convert_footnote_ref(node)],
        node::FOOTNOTE_DEF => vec![convert_footnote_def(node, ctx)],
        node::SMALL_CAPS => vec![element(
            "small",
            common_attrs(node),
            convert_nodes(&node.children, ctx),
        )],
        node::QUOTED => vec![element("q", vec![], convert_nodes(&node.children, ctx))],
        "math_inline" => convert_math_inline(node),
        "math_display" => convert_math_display(node),
        _ => {
            ctx.warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
                format!("Unknown node type: {}", node.kind.as_str()),
            ));
            convert_nodes(&node.children, ctx)
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build an html-fmt element node.
fn element(tag: &str, attrs: Vec<(String, String)>, children: Vec<HtmlNode>) -> HtmlNode {
    HtmlNode::Element {
        tag: tag.into(),
        attrs,
        children,
        self_closing: false,
        span: Span::NONE,
    }
}

/// Build a void (self-closing) html-fmt element.
fn void_element(tag: &str, attrs: Vec<(String, String)>) -> HtmlNode {
    HtmlNode::Element {
        tag: tag.into(),
        attrs,
        children: vec![],
        self_closing: true,
        span: Span::NONE,
    }
}

/// Extract common HTML attributes (id, class, lang, dir, style) from a rescribe node.
fn common_attrs(node: &Node) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    if let Some(id) = node.props.get_str(prop::ID) {
        attrs.push(("id".into(), id.into()));
    }
    if let Some(classes) = node.props.get_str(prop::CLASSES) {
        attrs.push(("class".into(), classes.into()));
    }
    if let Some(lang) = node.props.get_str("html:lang") {
        attrs.push(("lang".into(), lang.into()));
    }
    if let Some(dir) = node.props.get_str("html:dir") {
        attrs.push(("dir".into(), dir.into()));
    }
    if let Some(style) = node.props.get_str("html:style") {
        attrs.push(("style".into(), style.into()));
    }
    attrs
}

// ── Node-specific converters ────────────────────────────────────────────────

fn convert_heading(node: &Node, ctx: &mut ConvertContext) -> HtmlNode {
    let level = node.props.get_int(prop::LEVEL).unwrap_or(1);
    let tag = match level {
        1 => "h1",
        2 => "h2",
        3 => "h3",
        4 => "h4",
        5 => "h5",
        _ => "h6",
    };
    element(tag, common_attrs(node), convert_nodes(&node.children, ctx))
}

fn convert_code_block(node: &Node) -> HtmlNode {
    let mut code_attrs = Vec::new();
    if let Some(lang) = node.props.get_str(prop::LANGUAGE) {
        code_attrs.push(("class".into(), format!("language-{lang}")));
    }

    let content = node.props.get_str(prop::CONTENT).unwrap_or("");
    let code = element(
        "code",
        code_attrs,
        vec![HtmlNode::Text {
            content: content.into(),
            span: Span::NONE,
        }],
    );
    element("pre", vec![], vec![code])
}

fn convert_list(node: &Node, ctx: &mut ConvertContext) -> HtmlNode {
    let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
    let tag = if ordered { "ol" } else { "ul" };
    let mut attrs = Vec::new();
    if ordered
        && let Some(start) = node.props.get_int(prop::START)
        && start != 1
    {
        attrs.push(("start".into(), start.to_string()));
    }
    element(tag, attrs, convert_nodes(&node.children, ctx))
}

fn convert_table_cell(node: &Node, tag: &str, ctx: &mut ConvertContext) -> HtmlNode {
    let mut attrs = Vec::new();
    if let Some(colspan) = node.props.get_int(prop::COLSPAN)
        && colspan > 1
    {
        attrs.push(("colspan".into(), colspan.to_string()));
    }
    if let Some(rowspan) = node.props.get_int(prop::ROWSPAN)
        && rowspan > 1
    {
        attrs.push(("rowspan".into(), rowspan.to_string()));
    }
    element(tag, attrs, convert_nodes(&node.children, ctx))
}

fn convert_div(node: &Node, ctx: &mut ConvertContext) -> HtmlNode {
    let tag = node.props.get_str("html:tag").unwrap_or("div");
    element(tag, common_attrs(node), convert_nodes(&node.children, ctx))
}

fn convert_raw(node: &Node) -> Vec<HtmlNode> {
    let format = node.props.get_str(prop::FORMAT).unwrap_or("html");
    if format == "html"
        && let Some(content) = node.props.get_str(prop::CONTENT)
    {
        return vec![HtmlNode::Raw {
            content: content.into(),
            span: Span::NONE,
        }];
    }
    vec![]
}

fn convert_text(node: &Node) -> Vec<HtmlNode> {
    if let Some(content) = node.props.get_str(prop::CONTENT) {
        vec![HtmlNode::Text {
            content: content.into(),
            span: Span::NONE,
        }]
    } else {
        vec![]
    }
}

fn convert_inline_code(node: &Node) -> HtmlNode {
    let content = node.props.get_str(prop::CONTENT).unwrap_or("");
    element(
        "code",
        vec![],
        vec![HtmlNode::Text {
            content: content.into(),
            span: Span::NONE,
        }],
    )
}

fn convert_link(node: &Node, ctx: &mut ConvertContext) -> HtmlNode {
    let mut attrs = Vec::new();
    if let Some(url) = node.props.get_str(prop::URL) {
        attrs.push(("href".into(), url.into()));
    }
    if let Some(title) = node.props.get_str(prop::TITLE) {
        attrs.push(("title".into(), title.into()));
    }
    element("a", attrs, convert_nodes(&node.children, ctx))
}

fn convert_image(node: &Node, ctx: &mut ConvertContext) -> HtmlNode {
    let mut attrs = Vec::new();

    // Check for embedded resource first
    if let Some(resource_id_str) = node.props.get_str(prop::RESOURCE_ID) {
        let resource_id = ResourceId::from_string(resource_id_str);
        if let Some(resource) = ctx.resources.get(&resource_id) {
            let data_uri = format!(
                "data:{};base64,{}",
                resource.mime_type,
                base64_encode(&resource.data)
            );
            attrs.push(("src".into(), data_uri));
        }
    } else if let Some(url) = node.props.get_str(prop::URL) {
        attrs.push(("src".into(), url.into()));
    }

    if let Some(alt) = node.props.get_str(prop::ALT) {
        attrs.push(("alt".into(), alt.into()));
    }
    if let Some(title) = node.props.get_str(prop::TITLE) {
        attrs.push(("title".into(), title.into()));
    }

    void_element("img", attrs)
}

fn convert_span(node: &Node, ctx: &mut ConvertContext) -> HtmlNode {
    let tag = node.props.get_str("html:tag").unwrap_or("span");
    let mut attrs = common_attrs(node);
    // <abbr> carries its expansion in the title attribute.
    if tag == "abbr"
        && let Some(title) = node.props.get_str(prop::TITLE)
    {
        attrs.push(("title".into(), title.into()));
    }
    element(tag, attrs, convert_nodes(&node.children, ctx))
}

/// Footnote reference convention: `<sup class="footnote-ref"><a href="#fn-{label}">{label}</a></sup>`.
/// The `footnote-ref` class name and `#fn-{label}` href are the anchor the
/// reader (`rescribe::read`) pattern-matches on to reconstruct
/// `footnote_ref`; see `convert_element`'s "sup" arm there.
fn convert_footnote_ref(node: &Node) -> HtmlNode {
    let label = node.props.get_str(prop::LABEL).unwrap_or("?");
    element(
        "sup",
        vec![("class".into(), "footnote-ref".into())],
        vec![element(
            "a",
            vec![
                ("href".into(), format!("#fn-{label}")),
                ("id".into(), format!("fnref-{label}")),
            ],
            vec![HtmlNode::Text {
                content: label.into(),
                span: Span::NONE,
            }],
        )],
    )
}

/// Footnote definition convention: a `<div id="fn-{label}" class="footnote">`
/// wrapping a visible `<sup class="footnote-label">` marker, the real content
/// in a `<span class="footnote-content">` (the part the reader extracts
/// back out), and a `<a class="footnote-back">` backlink to the reference.
/// The marker and backlink are purely derived from `label` and are not
/// read back — only the content span round-trips — so this stays lossless
/// while still rendering like the common CMS "backlink" convention.
fn convert_footnote_def(node: &Node, ctx: &mut ConvertContext) -> HtmlNode {
    let label = node.props.get_str(prop::LABEL).unwrap_or("?");
    let marker = element(
        "sup",
        vec![("class".into(), "footnote-label".into())],
        vec![HtmlNode::Text {
            content: label.into(),
            span: Span::NONE,
        }],
    );
    let content = element(
        "span",
        vec![("class".into(), "footnote-content".into())],
        convert_nodes(&node.children, ctx),
    );
    let backlink = element(
        "a",
        vec![
            ("href".into(), format!("#fnref-{label}")),
            ("class".into(), "footnote-back".into()),
        ],
        vec![HtmlNode::Text {
            content: "\u{21a9}".into(),
            span: Span::NONE,
        }],
    );

    element(
        "div",
        vec![
            ("id".into(), format!("fn-{label}")),
            ("class".into(), "footnote".into()),
        ],
        vec![marker, content, backlink],
    )
}

fn convert_math_inline(node: &Node) -> Vec<HtmlNode> {
    convert_math(node, false)
}

fn convert_math_display(node: &Node) -> Vec<HtmlNode> {
    convert_math(node, true)
}

/// Convert a math_inline/math_display node to HTML.
///
/// If `math:format` is `mathml`, `math:source` already holds a verbatim
/// `<math>…</math>` fragment (captured raw by the reader — see
/// `convert_mathml` in `rescribe::read`) and is spliced back in unescaped.
/// Otherwise `math:source` is treated as LaTeX and wrapped in the
/// `\(…\)` / `\[…\]` convention.
fn convert_math(node: &Node, display: bool) -> Vec<HtmlNode> {
    let Some(source) = node.props.get_str("math:source") else {
        return vec![];
    };

    if node.props.get_str("math:format") == Some("mathml") {
        return vec![HtmlNode::Raw {
            content: source.into(),
            span: Span::NONE,
        }];
    }

    let (tag, content) = if display {
        ("div", format!("\\[{source}\\]"))
    } else {
        ("span", format!("\\({source}\\)"))
    };
    let class = if display {
        "math math-display"
    } else {
        "math math-inline"
    };
    vec![element(
        tag,
        vec![("class".into(), class.into())],
        vec![HtmlNode::Text {
            content,
            span: Span::NONE,
        }],
    )]
}

/// Base64 encode bytes.
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);

        result.push(ALPHABET[(b0 >> 2) as usize] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4 | b1 >> 4) as usize] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2 | b2 >> 6) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[(b2 & 0x3f) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use builder::html;

    fn emit_str(doc: &Document) -> String {
        let result = emit(doc).unwrap();
        String::from_utf8(result.value).unwrap()
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = html(|d| d.p(|i| i.text("Hello, world!")));
        let output = emit_str(&doc);
        assert_eq!(output, "<p>Hello, world!</p>");
    }

    #[test]
    fn test_emit_heading() {
        let doc = html(|d| d.h2(|i| i.text("Title")));
        let output = emit_str(&doc);
        assert_eq!(output, "<h2>Title</h2>");
    }

    #[test]
    fn test_emit_emphasis() {
        let doc = html(|d| d.p(|i| i.em(|i| i.text("italic"))));
        let output = emit_str(&doc);
        assert_eq!(output, "<p><em>italic</em></p>");
    }

    #[test]
    fn test_emit_link() {
        let doc = html(|d| d.p(|i| i.a("https://example.com", |i| i.text("link"))));
        let output = emit_str(&doc);
        assert_eq!(output, "<p><a href=\"https://example.com\">link</a></p>");
    }

    #[test]
    fn test_emit_code_block() {
        let doc = html(|d| d.pre_lang("rust", "fn main() {}"));
        let output = emit_str(&doc);
        assert_eq!(
            output,
            "<pre><code class=\"language-rust\">fn main() {}</code></pre>"
        );
    }

    #[test]
    fn test_emit_list() {
        let doc = html(|d| d.ul(|l| l.li(|i| i.text("item 1")).li(|i| i.text("item 2"))));
        let output = emit_str(&doc);
        assert!(output.contains("<ul>"));
        assert!(output.contains("<li>"));
        assert!(output.contains("item 1"));
        assert!(output.contains("item 2"));
    }

    #[test]
    fn test_emit_image() {
        let doc = html(|d| d.p(|i| i.img("test.png", "Test image")));
        let output = emit_str(&doc);
        assert!(output.contains("<img src=\"test.png\" alt=\"Test image\">"));
    }

    #[test]
    fn test_escape_html() {
        let doc = html(|d| d.p(|i| i.text("<script>alert('xss')</script>")));
        let output = emit_str(&doc);
        assert!(output.contains("&lt;script&gt;"));
        assert!(!output.contains("<script>"));
    }

    #[test]
    fn test_pretty_print_list() {
        let doc = html(|d| d.ul(|l| l.li(|i| i.text("item 1")).li(|i| i.text("item 2"))));

        // Default (not pretty)
        let output = emit_str(&doc);
        assert!(!output.contains('\n'));

        // Pretty mode
        let options = EmitOptions {
            pretty: true,
            ..Default::default()
        };
        let result = emit_with_options(&doc, &options).unwrap();
        let pretty_output = String::from_utf8(result.value).unwrap();

        // Should have newlines and indentation
        assert!(pretty_output.contains('\n'));
        assert!(pretty_output.contains("  <li>"));
    }

    #[test]
    fn test_emit_footnote() {
        let footnote_ref = Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, "1");
        let footnote_def = Node::new(node::FOOTNOTE_DEF)
            .prop(prop::LABEL, "1")
            .children(vec![Node::new(node::TEXT).prop(prop::CONTENT, "A note.")]);
        let para = Node::new(node::PARAGRAPH).children(vec![
            Node::new(node::TEXT).prop(prop::CONTENT, "Text."),
            footnote_ref,
        ]);
        let root = Node::new(node::DOCUMENT).children(vec![para, footnote_def]);
        let doc = Document::new().with_content(root);

        let output = emit_str(&doc);
        assert_eq!(
            output,
            "<p>Text.<sup class=\"footnote-ref\"><a href=\"#fn-1\" id=\"fnref-1\">1</a></sup></p>\
             <div id=\"fn-1\" class=\"footnote\">\
             <sup class=\"footnote-label\">1</sup>\
             <span class=\"footnote-content\">A note.</span>\
             <a href=\"#fnref-1\" class=\"footnote-back\">\u{21a9}</a></div>"
        );
    }

    #[test]
    fn test_emit_math_inline_latex() {
        let math = Node::new("math_inline").prop("math:source", "x^2");
        let doc = Document::new().with_content(Node::new(node::DOCUMENT).children(vec![math]));
        let output = emit_str(&doc);
        assert_eq!(output, "<span class=\"math math-inline\">\\(x^2\\)</span>");
    }

    #[test]
    fn test_emit_math_inline_mathml_raw() {
        let source = "<math><mi>x</mi></math>";
        let math = Node::new("math_inline")
            .prop("math:format", "mathml")
            .prop("math:source", source);
        let doc = Document::new().with_content(Node::new(node::DOCUMENT).children(vec![math]));
        let output = emit_str(&doc);
        assert_eq!(output, source);
    }

    #[test]
    fn test_pretty_print_nested() {
        // Build a nested structure: blockquote with paragraphs
        let doc = html(|d| d.blockquote(|bq| bq.p(|p| p.text("first")).p(|p| p.text("second"))));

        let options = EmitOptions {
            pretty: true,
            ..Default::default()
        };
        let result = emit_with_options(&doc, &options).unwrap();
        let output = String::from_utf8(result.value).unwrap();

        // Should have proper nesting with increased indentation
        assert!(output.contains('\n'));
        assert!(output.contains("  <p>"));
    }
}

/// Type-safe HTML document builder, used only by this module's own tests to
/// construct rescribe `Document`s without hand-writing `Node` trees.
///
/// This is test-only scaffolding (ported from the legacy
/// `rescribe-write-html` crate's `builder` module, which was public API
/// there but had no consumers outside its own tests — see git history);
/// it is not part of this crate's public surface.
#[cfg(test)]
mod builder {
    use rescribe_core::{Document, Node};
    use rescribe_std::{node, prop};

    /// Build an HTML document with type-safe structure.
    pub fn html<F>(f: F) -> Document
    where
        F: FnOnce(HtmlBuilder) -> HtmlBuilder,
    {
        let builder = f(HtmlBuilder::new());
        Document::new().with_content(builder.build())
    }

    /// Builder for HTML document structure.
    #[derive(Default)]
    pub struct HtmlBuilder {
        children: Vec<Node>,
    }

    impl HtmlBuilder {
        fn new() -> Self {
            Self::default()
        }

        /// Add an h2 heading.
        pub fn h2<F>(mut self, f: F) -> Self
        where
            F: FnOnce(HtmlInline) -> HtmlInline,
        {
            let inner = f(HtmlInline::new());
            self.children.push(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, 2i64)
                    .children(inner.children),
            );
            self
        }

        /// Add a paragraph.
        pub fn p<F>(mut self, f: F) -> Self
        where
            F: FnOnce(HtmlInline) -> HtmlInline,
        {
            let inner = f(HtmlInline::new());
            self.children
                .push(Node::new(node::PARAGRAPH).children(inner.children));
            self
        }

        /// Add an unordered list.
        pub fn ul<F>(mut self, f: F) -> Self
        where
            F: FnOnce(HtmlList) -> HtmlList,
        {
            let list = f(HtmlList::new());
            self.children.push(
                Node::new(node::LIST)
                    .prop(prop::ORDERED, false)
                    .children(list.items),
            );
            self
        }

        /// Add a blockquote.
        pub fn blockquote<F>(mut self, f: F) -> Self
        where
            F: FnOnce(HtmlBuilder) -> HtmlBuilder,
        {
            let inner = f(HtmlBuilder::new());
            self.children
                .push(Node::new(node::BLOCKQUOTE).children(inner.children));
            self
        }

        /// Add a code block with language class.
        pub fn pre_lang(mut self, language: impl Into<String>, code: impl Into<String>) -> Self {
            self.children.push(
                Node::new(node::CODE_BLOCK)
                    .prop(prop::LANGUAGE, language.into())
                    .prop(prop::CONTENT, code.into()),
            );
            self
        }

        fn build(self) -> Node {
            Node::new(node::DOCUMENT).children(self.children)
        }
    }

    /// Builder for HTML inline content.
    #[derive(Default)]
    pub struct HtmlInline {
        children: Vec<Node>,
    }

    impl HtmlInline {
        fn new() -> Self {
            Self::default()
        }

        /// Add plain text.
        pub fn text(mut self, content: impl Into<String>) -> Self {
            self.children
                .push(Node::new(node::TEXT).prop(prop::CONTENT, content.into()));
            self
        }

        /// Add emphasized text (<em>).
        pub fn em<F>(mut self, f: F) -> Self
        where
            F: FnOnce(HtmlInline) -> HtmlInline,
        {
            let inner = f(HtmlInline::new());
            self.children
                .push(Node::new(node::EMPHASIS).children(inner.children));
            self
        }

        /// Add a link (<a>).
        pub fn a<F>(mut self, href: impl Into<String>, f: F) -> Self
        where
            F: FnOnce(HtmlInline) -> HtmlInline,
        {
            let inner = f(HtmlInline::new());
            self.children.push(
                Node::new(node::LINK)
                    .prop(prop::URL, href.into())
                    .children(inner.children),
            );
            self
        }

        /// Add an image (<img>).
        pub fn img(mut self, src: impl Into<String>, alt: impl Into<String>) -> Self {
            self.children.push(
                Node::new(node::IMAGE)
                    .prop(prop::URL, src.into())
                    .prop(prop::ALT, alt.into()),
            );
            self
        }
    }

    /// Builder for HTML lists.
    #[derive(Default)]
    pub struct HtmlList {
        items: Vec<Node>,
    }

    impl HtmlList {
        fn new() -> Self {
            Self::default()
        }

        /// Add a list item with inline content.
        pub fn li<F>(mut self, f: F) -> Self
        where
            F: FnOnce(HtmlInline) -> HtmlInline,
        {
            let inline = f(HtmlInline::new());
            let item = Node::new(node::LIST_ITEM)
                .children(vec![Node::new(node::PARAGRAPH).children(inline.children)]);
            self.items.push(item);
            self
        }
    }
}
