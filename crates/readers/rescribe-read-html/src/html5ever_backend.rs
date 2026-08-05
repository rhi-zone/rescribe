//! HTML parser using html-fmt (wraps html5ever).

use html_fmt::HtmlDoc;
use html_fmt::Node as HtmlNode;
use html_fmt::ast::Span;
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, ParseError, ParseOptions, Properties, Resource,
    ResourceId, ResourceMap, Severity, WarningKind,
};
use rescribe_format_api::Parse as _;
use rescribe_std::{Node, node, prop};

use crate::{
    extract_text_content, get_code_language, is_block_element, merge_text_nodes, parse_data_uri,
};

/// Parse HTML text into a rescribe Document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse HTML with custom options.
pub fn parse_with_options(
    input: &str,
    options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let mut warnings = Vec::new();
    let mut metadata = Properties::new();
    let mut resources = ResourceMap::new();

    // Parse HTML using html-fmt (which wraps html5ever)
    let (doc, _diagnostics) = HtmlDoc::parse(input.as_bytes());

    // Extract metadata from <head> (and <html> lang attribute)
    for node in &doc.nodes {
        extract_metadata(node, &mut metadata);
    }

    // Convert html-fmt AST to rescribe nodes
    let mut children = Vec::new();
    for node in &doc.nodes {
        children.extend(convert_node(node, &mut warnings, &mut resources, options));
    }
    merge_text_nodes(&mut children);

    let root = Node::new(node::DOCUMENT).children(children);
    let mut doc = Document::new().with_content(root).with_metadata(metadata);
    doc.resources = resources;

    Ok(ConversionResult::with_warnings(doc, warnings))
}

/// Extract metadata from HTML head element.
fn extract_metadata(node: &HtmlNode, metadata: &mut Properties) {
    let HtmlNode::Element {
        tag,
        attrs,
        children,
        ..
    } = node
    else {
        return;
    };

    match tag.as_str() {
        "html" => {
            if let Some(lang) = get_attr(attrs, "lang") {
                metadata.set("lang", lang);
            }
        }
        "title" => {
            let title = extract_element_text(node);
            if !title.is_empty() {
                metadata.set("title", title);
            }
        }
        "meta" => {
            // charset declaration: <meta charset="utf-8">
            if let Some(charset) = get_attr(attrs, "charset") {
                metadata.set("charset", charset);
            }
            // http-equiv content-type
            if get_attr(attrs, "http-equiv")
                .as_deref()
                .map(|v| v.eq_ignore_ascii_case("content-type"))
                == Some(true)
                && let Some(content) = get_attr(attrs, "content")
            {
                metadata.set("content-type", content);
            }
            // Standard name/content pairs
            if let Some(name) = get_attr(attrs, "name")
                && let Some(content) = get_attr(attrs, "content")
            {
                metadata.set(&name, content);
            }
            // Open Graph properties (og: prefix stripped)
            if let Some(property) = get_attr(attrs, "property")
                && let Some(content) = get_attr(attrs, "content")
            {
                let key = property.strip_prefix("og:").unwrap_or(&property);
                metadata.set(key, content);
            }
        }
        "link" => {
            if get_attr(attrs, "rel").as_deref() == Some("stylesheet")
                && let Some(href) = get_attr(attrs, "href")
            {
                metadata.set("stylesheet", href);
            }
        }
        "base" => {
            if let Some(href) = get_attr(attrs, "href") {
                metadata.set("base", href);
            }
        }
        _ => {}
    }

    for child in children {
        extract_metadata(child, metadata);
    }
}

/// Extract text content from an html-fmt element.
fn extract_element_text(node: &HtmlNode) -> String {
    let mut text = String::new();
    match node {
        HtmlNode::Text { content, .. } => text.push_str(content),
        HtmlNode::Element { children, .. } => {
            for child in children {
                text.push_str(&extract_element_text(child));
            }
        }
        _ => {}
    }
    text
}

/// Apply global HTML attributes (id, class, lang, dir, style) to a node.
fn apply_global_attrs(mut node: Node, attrs: &[(String, String)]) -> Node {
    if let Some(id) = get_attr(attrs, "id") {
        node = node.prop(prop::ID, id);
    }
    if let Some(class) = get_attr(attrs, "class") {
        node = node.prop(prop::CLASSES, class);
    }
    if let Some(lang) = get_attr(attrs, "lang") {
        node = node.prop("html:lang", lang);
    }
    if let Some(dir) = get_attr(attrs, "dir") {
        node = node.prop("html:dir", dir);
    }
    if let Some(style) = get_attr(attrs, "style") {
        node = node.prop("html:style", style);
    }
    node
}

/// Convert child nodes of an html-fmt element.
fn convert_children(
    children: &[HtmlNode],
    warnings: &mut Vec<FidelityWarning>,
    resources: &mut ResourceMap,
    options: &ParseOptions,
) -> Vec<Node> {
    let mut nodes = Vec::new();
    for child in children {
        nodes.extend(convert_node(child, warnings, resources, options));
    }
    merge_text_nodes(&mut nodes);
    nodes
}

/// Convert a single html-fmt node to rescribe Node(s).
fn convert_node(
    html_node: &HtmlNode,
    warnings: &mut Vec<FidelityWarning>,
    resources: &mut ResourceMap,
    options: &ParseOptions,
) -> Vec<Node> {
    match html_node {
        HtmlNode::Text { content, .. } => {
            let text = content.to_string();
            if text.trim().is_empty() {
                return vec![];
            }
            vec![Node::new(node::TEXT).prop(prop::CONTENT, text)]
        }
        HtmlNode::Element {
            tag,
            attrs,
            children,
            ..
        } => convert_element(tag, attrs, children, warnings, resources, options),
        // Skip doctype, comments
        _ => vec![],
    }
}

/// Convert an HTML element to a rescribe Node.
fn convert_element(
    tag: &str,
    attrs: &[(String, String)],
    children_nodes: &[HtmlNode],
    warnings: &mut Vec<FidelityWarning>,
    resources: &mut ResourceMap,
    options: &ParseOptions,
) -> Vec<Node> {
    // MathML subtrees are captured verbatim (see the "math" arm below) —
    // their children are foreign-namespace elements (`<mi>`, `<mo>`, …) that
    // must not be recursively converted (and warned about) as HTML.
    if tag == "math" {
        return vec![convert_mathml(tag, attrs, children_nodes)];
    }

    let children = convert_children(children_nodes, warnings, resources, options);

    let node = match tag {
        "html" | "body" => return children,

        "head" | "script" | "style" | "meta" | "link" | "title" | "base" => return vec![],

        // Layout-only table elements — no semantic content, skip silently.
        "colgroup" | "col" => return vec![],

        "p" => apply_global_attrs(Node::new(node::PARAGRAPH).children(children), attrs),

        "h1" => apply_global_attrs(
            Node::new(node::HEADING)
                .prop(prop::LEVEL, 1i64)
                .children(children),
            attrs,
        ),
        "h2" => apply_global_attrs(
            Node::new(node::HEADING)
                .prop(prop::LEVEL, 2i64)
                .children(children),
            attrs,
        ),
        "h3" => apply_global_attrs(
            Node::new(node::HEADING)
                .prop(prop::LEVEL, 3i64)
                .children(children),
            attrs,
        ),
        "h4" => apply_global_attrs(
            Node::new(node::HEADING)
                .prop(prop::LEVEL, 4i64)
                .children(children),
            attrs,
        ),
        "h5" => apply_global_attrs(
            Node::new(node::HEADING)
                .prop(prop::LEVEL, 5i64)
                .children(children),
            attrs,
        ),
        "h6" => apply_global_attrs(
            Node::new(node::HEADING)
                .prop(prop::LEVEL, 6i64)
                .children(children),
            attrs,
        ),

        "pre" => {
            let content = extract_text_content(&children);
            let lang = get_code_language(&children);
            let mut node = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content);
            if let Some(l) = lang {
                node = node.prop(prop::LANGUAGE, l);
            }
            node
        }

        "blockquote" => apply_global_attrs(Node::new(node::BLOCKQUOTE).children(children), attrs),

        "ul" => apply_global_attrs(
            Node::new(node::LIST)
                .prop(prop::ORDERED, false)
                .children(children),
            attrs,
        ),

        "ol" => {
            let mut list = Node::new(node::LIST).prop(prop::ORDERED, true);
            if let Some(start) = get_attr(attrs, "start")
                && let Ok(n) = start.parse::<i64>()
            {
                list = list.prop(prop::START, n);
            }
            apply_global_attrs(list.children(children), attrs)
        }

        "li" => apply_global_attrs(Node::new(node::LIST_ITEM).children(children), attrs),

        "dl" => Node::new(node::DEFINITION_LIST).children(children),
        "dt" => Node::new(node::DEFINITION_TERM).children(children),
        "dd" => Node::new(node::DEFINITION_DESC).children(children),

        "table" => Node::new(node::TABLE).children(children),
        "thead" => Node::new(node::TABLE_HEAD).children(children),
        "tbody" => Node::new(node::TABLE_BODY).children(children),
        "tfoot" => Node::new(node::TABLE_FOOT).children(children),
        "tr" => Node::new(node::TABLE_ROW).children(children),
        "th" => {
            let mut cell = Node::new(node::TABLE_HEADER).children(children);
            if let Some(colspan) = get_attr(attrs, "colspan")
                && let Ok(n) = colspan.parse::<i64>()
            {
                cell = cell.prop(prop::COLSPAN, n);
            }
            if let Some(rowspan) = get_attr(attrs, "rowspan")
                && let Ok(n) = rowspan.parse::<i64>()
            {
                cell = cell.prop(prop::ROWSPAN, n);
            }
            cell
        }
        "td" => {
            let mut cell = Node::new(node::TABLE_CELL).children(children);
            if let Some(colspan) = get_attr(attrs, "colspan")
                && let Ok(n) = colspan.parse::<i64>()
            {
                cell = cell.prop(prop::COLSPAN, n);
            }
            if let Some(rowspan) = get_attr(attrs, "rowspan")
                && let Ok(n) = rowspan.parse::<i64>()
            {
                cell = cell.prop(prop::ROWSPAN, n);
            }
            cell
        }

        "figure" => Node::new(node::FIGURE).children(children),
        "figcaption" => Node::new(node::CAPTION).children(children),

        "hr" => Node::new(node::HORIZONTAL_RULE),

        // Footnote definition convention (see rescribe-write-html's
        // convert_footnote_def): `<div id="fn-{label}" class="footnote">`
        // wrapping a `<sup class="footnote-label">` marker, a
        // `<span class="footnote-content">` holding the real content, and a
        // `<a class="footnote-back">` backlink. The marker and backlink are
        // regenerated from the label on write, so only the content span's
        // children need to survive the round trip.
        "div"
            if has_class(attrs, "footnote")
                && get_attr(attrs, "id")
                    .as_deref()
                    .is_some_and(|id| id.starts_with("fn-")) =>
        {
            let label = get_attr(attrs, "id").unwrap()["fn-".len()..].to_string();
            let content = children.iter().find(|c| {
                c.kind.as_str() == node::SPAN
                    && c.props.get_str(prop::CLASSES).is_some_and(|classes| {
                        classes.split_whitespace().any(|c| c == "footnote-content")
                    })
            });
            match content {
                Some(content_span) => Node::new(node::FOOTNOTE_DEF)
                    .prop(prop::LABEL, label)
                    .children(content_span.children.clone()),
                None => apply_global_attrs(Node::new(node::DIV).children(children), attrs),
            }
        }

        // Generic block container — no html:tag prop (it IS a div).
        "div" => apply_global_attrs(Node::new(node::DIV).children(children), attrs),

        // Semantic HTML5 section-like elements: preserved as div with html:tag.
        "section" | "article" | "main" | "aside" | "nav" | "header" | "footer" | "address" => {
            apply_global_attrs(
                Node::new(node::DIV)
                    .prop("html:tag", tag.to_string())
                    .children(children),
                attrs,
            )
        }

        // Interactive/disclosure elements.
        "details" | "summary" => apply_global_attrs(
            Node::new(node::DIV)
                .prop("html:tag", tag.to_string())
                .children(children),
            attrs,
        ),

        "em" | "i" => Node::new(node::EMPHASIS).children(children),
        "strong" | "b" => Node::new(node::STRONG).children(children),
        "s" | "strike" | "del" => Node::new(node::STRIKEOUT).children(children),
        "u" => Node::new(node::UNDERLINE).children(children),
        // <ins> is "inserted text" (tracked change), not just underline.
        "ins" => apply_global_attrs(
            Node::new(node::SPAN)
                .prop("html:tag", "ins")
                .children(children),
            attrs,
        ),
        "sub" => Node::new(node::SUBSCRIPT).children(children),

        // Footnote reference convention (see rescribe-write-html's
        // convert_footnote_ref): `<sup class="footnote-ref"><a href="#fn-{label}">…</a></sup>`.
        // Recognize it structurally and reconstruct footnote_ref; the label
        // is derived from the href, not any id attribute, so hand-authored
        // markup without an `id` still round-trips.
        "sup" if has_class(attrs, "footnote-ref") => match children.as_slice() {
            [link] if link.kind.as_str() == node::LINK => {
                match link
                    .props
                    .get_str(prop::URL)
                    .and_then(|u| u.strip_prefix("#fn-"))
                {
                    Some(label) => {
                        Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.to_string())
                    }
                    None => Node::new(node::SUPERSCRIPT).children(children),
                }
            }
            _ => Node::new(node::SUPERSCRIPT).children(children),
        },

        "sup" => Node::new(node::SUPERSCRIPT).children(children),

        "code" => {
            let content = extract_text_content(&children);
            let mut code = Node::new(node::CODE).prop(prop::CONTENT, content);
            if let Some(class) = get_attr(attrs, "class") {
                code = code.prop(prop::CLASSES, class);
            }
            code
        }

        "a" => {
            let mut link = Node::new(node::LINK).children(children);
            if let Some(href) = get_attr(attrs, "href") {
                link = link.prop(prop::URL, href);
            }
            if let Some(title) = get_attr(attrs, "title") {
                link = link.prop(prop::TITLE, title);
            }
            link
        }

        "img" => {
            let mut img = Node::new(node::IMAGE);
            if let Some(src) = get_attr(attrs, "src") {
                if options.embed_resources {
                    if let Some((mime_type, data)) = parse_data_uri(&src) {
                        let resource = Resource::new(mime_type, data);
                        let id = ResourceId::new();
                        resources.insert(id.clone(), resource);
                        img = img.prop(prop::RESOURCE_ID, id.as_str().to_string());
                    } else {
                        img = img.prop(prop::URL, src);
                    }
                } else {
                    img = img.prop(prop::URL, src);
                }
            }
            if let Some(alt) = get_attr(attrs, "alt") {
                img = img.prop(prop::ALT, alt);
            }
            if let Some(title) = get_attr(attrs, "title") {
                img = img.prop(prop::TITLE, title);
            }
            img
        }

        "br" => Node::new(node::LINE_BREAK),

        "span" => apply_global_attrs(Node::new(node::SPAN).children(children), attrs),

        "q" => Node::new(node::QUOTED)
            .prop(prop::QUOTE_TYPE, "double")
            .children(children),

        "small" => Node::new(node::SMALL_CAPS).children(children),

        // Semantic annotation elements — preserved as span with html:tag.
        "abbr" => {
            let mut span = Node::new(node::SPAN)
                .prop("html:tag", "abbr")
                .children(children);
            if let Some(title) = get_attr(attrs, "title") {
                span = span.prop(prop::TITLE, title);
            }
            apply_global_attrs(span, attrs)
        }

        "mark" | "kbd" | "var" | "samp" | "cite" => apply_global_attrs(
            Node::new(node::SPAN)
                .prop("html:tag", tag.to_string())
                .children(children),
            attrs,
        ),

        _ => {
            warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::UnsupportedNode(format!("html:{}", tag)),
                format!("Unknown HTML element: {}", tag),
            ));

            if is_block_element(tag) {
                Node::new(node::DIV).children(children)
            } else {
                Node::new(node::SPAN).children(children)
            }
        }
    };

    vec![node]
}

/// Convert a `<math>` element to a math_inline/math_display node.
///
/// Full structural modeling into rescribe's `math:*` node kinds is a large
/// undertaking (MathML has its own presentation/content element vocabulary);
/// per CLAUDE.md's raw-preservation pattern, the element is captured verbatim
/// (including its own attributes, e.g. `xmlns`) so the writer can re-emit it
/// byte-for-byte. `display="block"` maps to math_display, anything else
/// (including absent) to math_inline, matching MathML/CSS semantics.
fn convert_mathml(tag: &str, attrs: &[(String, String)], children_nodes: &[HtmlNode]) -> Node {
    let kind = if get_attr(attrs, "display").as_deref() == Some("block") {
        "math_display"
    } else {
        "math_inline"
    };
    let math_el = HtmlNode::Element {
        tag: tag.to_string(),
        attrs: attrs.to_vec(),
        children: children_nodes.to_vec(),
        self_closing: false,
        span: Span::NONE,
    };
    let source = String::from_utf8(html_fmt::emit_fragment(std::slice::from_ref(&math_el)))
        .unwrap_or_default();
    Node::new(kind)
        .prop("math:format", "mathml")
        .prop("math:source", source)
}

/// Get an attribute value by name.
fn get_attr(attrs: &[(String, String)], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.to_string())
}

/// Check whether an element's `class` attribute contains a given class name.
fn has_class(attrs: &[(String, String)], class: &str) -> bool {
    get_attr(attrs, "class").is_some_and(|classes| classes.split_whitespace().any(|c| c == class))
}
