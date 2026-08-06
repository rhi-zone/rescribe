//! HTML → `rescribe::Document`.
//!
//! Translates `crate::HtmlDoc` (produced by `crate::parse::parse`, wrapping
//! html5ever) into rescribe's document IR. No HTML parsing happens here.

use crate::Node as HtmlNode;
use crate::ast::Span;
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, ParseError, ParseOptions, Properties, Resource,
    ResourceId, ResourceMap, Severity, WarningKind,
};
use rescribe_std::{Node, node, prop};

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

    // Parse HTML using this crate's own parser (wraps html5ever). Parse
    // diagnostics (html5ever error-recovery notices) are not currently
    // surfaced as fidelity warnings — html5ever's error recovery always
    // produces a valid tree, so these are informational, not lossy.
    let (doc, _diagnostics) = crate::parse::parse(input.as_bytes());

    // Extract metadata from <head> (and <html> lang attribute)
    for node in &doc.nodes {
        extract_metadata(node, &mut metadata);
    }

    // Convert the native AST to rescribe nodes
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

        // Footnote definition convention (see the writer's
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

        // Footnote reference convention (see the writer's
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
    let source =
        String::from_utf8(crate::emit_fragment(std::slice::from_ref(&math_el))).unwrap_or_default();
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

// ── Shared reader utilities ─────────────────────────────────────────────────

/// Extract text content from a list of nodes.
fn extract_text_content(nodes: &[Node]) -> String {
    let mut text = String::new();
    for node in nodes {
        if let Some(content) = node.props.get_str(prop::CONTENT) {
            text.push_str(content);
        }
        text.push_str(&extract_text_content(&node.children));
    }
    text
}

/// Try to get the language from a code element inside pre.
fn get_code_language(children: &[Node]) -> Option<String> {
    for child in children {
        if child.kind.as_str() == node::CODE
            && let Some(classes) = child.props.get_str(prop::CLASSES)
        {
            for class in classes.split_whitespace() {
                if let Some(lang) = class.strip_prefix("language-") {
                    return Some(lang.to_string());
                }
            }
        }
    }
    None
}

/// Parse a data URI into mime type and data.
fn parse_data_uri(uri: &str) -> Option<(String, Vec<u8>)> {
    let uri = uri.strip_prefix("data:")?;
    let (header, data) = uri.split_once(',')?;

    let is_base64 = header.ends_with(";base64");
    let mime_type = if is_base64 {
        header
            .strip_suffix(";base64")
            .unwrap_or("application/octet-stream")
    } else if header.is_empty() {
        "text/plain;charset=US-ASCII"
    } else {
        header
    };

    let decoded = if is_base64 {
        base64_decode(data)?
    } else {
        percent_decode(data)
    };

    Some((mime_type.to_string(), decoded))
}

/// Simple base64 decoder.
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let input: Vec<u8> = input
        .bytes()
        .filter(|&b| b != b'\n' && b != b'\r' && b != b' ')
        .collect();
    let mut output = Vec::with_capacity(input.len() * 3 / 4);

    for chunk in input.chunks(4) {
        let mut buf = [0u8; 4];
        let mut len = 0;

        for (i, &byte) in chunk.iter().enumerate() {
            if byte == b'=' {
                break;
            }
            buf[i] = ALPHABET.iter().position(|&c| c == byte)? as u8;
            len = i + 1;
        }

        if len >= 2 {
            output.push((buf[0] << 2) | (buf[1] >> 4));
        }
        if len >= 3 {
            output.push((buf[1] << 4) | (buf[2] >> 2));
        }
        if len >= 4 {
            output.push((buf[2] << 6) | buf[3]);
        }
    }

    Some(output)
}

/// Simple percent-decoding for data URIs.
fn percent_decode(input: &str) -> Vec<u8> {
    let mut output = Vec::with_capacity(input.len());
    let mut chars = input.bytes().peekable();

    while let Some(byte) = chars.next() {
        if byte == b'%' {
            let high = chars.next().and_then(|c| (c as char).to_digit(16));
            let low = chars.next().and_then(|c| (c as char).to_digit(16));
            if let (Some(h), Some(l)) = (high, low) {
                output.push((h * 16 + l) as u8);
            }
        } else {
            output.push(byte);
        }
    }

    output
}

/// Check if an element is a block-level element.
fn is_block_element(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "canvas"
            | "dd"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "noscript"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "table"
            | "tfoot"
            | "ul"
            | "video"
    )
}

/// Merge adjacent text nodes and clean up whitespace.
fn merge_text_nodes(nodes: &mut Vec<Node>) {
    if nodes.is_empty() {
        return;
    }

    let mut i = 0;
    while i < nodes.len() {
        merge_text_nodes(&mut nodes[i].children);

        if nodes[i].kind.as_str() == node::TEXT
            && let Some(content) = nodes[i].props.get_str(prop::CONTENT)
            && content.is_empty()
        {
            nodes.remove(i);
            continue;
        }

        if i + 1 < nodes.len()
            && nodes[i].kind.as_str() == node::TEXT
            && nodes[i + 1].kind.as_str() == node::TEXT
        {
            let next_content = nodes[i + 1]
                .props
                .get_str(prop::CONTENT)
                .unwrap_or("")
                .to_string();
            let current_content = nodes[i]
                .props
                .get_str(prop::CONTENT)
                .unwrap_or("")
                .to_string();

            nodes[i] = Node::new(node::TEXT).prop(prop::CONTENT, current_content + &next_content);
            nodes.remove(i + 1);
            continue;
        }

        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_core::ResourceId;

    fn root_children(doc: &Document) -> &[Node] {
        &doc.content.children
    }

    #[test]
    fn test_parse_paragraph() {
        let result = parse("<p>Hello, world!</p>").unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert!(!children.is_empty());
        let para = &children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_heading() {
        let result = parse("<h1>Title</h1><h2>Subtitle</h2>").unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind.as_str(), node::HEADING);
        assert_eq!(children[0].props.get_int(prop::LEVEL), Some(1));
        assert_eq!(children[1].props.get_int(prop::LEVEL), Some(2));
    }

    #[test]
    fn test_parse_emphasis() {
        let result = parse("<p><em>italic</em> and <strong>bold</strong></p>").unwrap();
        let doc = result.value;
        let children = root_children(&doc);
        let para = &children[0];

        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::EMPHASIS)
        );
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::STRONG)
        );
    }

    #[test]
    fn test_parse_link() {
        let result = parse(r#"<a href="https://example.com">link</a>"#).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        let link = &children[0];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
    }

    #[test]
    fn test_parse_list() {
        let result = parse("<ul><li>item 1</li><li>item 2</li></ul>").unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert_eq!(children[0].kind.as_str(), node::LIST);
        assert_eq!(children[0].props.get_bool(prop::ORDERED), Some(false));
        assert_eq!(children[0].children.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let result = parse("<ol><li>first</li><li>second</li></ol>").unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert_eq!(children[0].kind.as_str(), node::LIST);
        assert_eq!(children[0].props.get_bool(prop::ORDERED), Some(true));
    }

    #[test]
    fn test_parse_code_block() {
        let result = parse("<pre><code>fn main() {}</code></pre>").unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert_eq!(children[0].kind.as_str(), node::CODE_BLOCK);
        assert_eq!(
            children[0].props.get_str(prop::CONTENT),
            Some("fn main() {}")
        );
    }

    #[test]
    fn test_parse_table() {
        let result =
            parse("<table><tr><th>Header</th></tr><tr><td>Cell</td></tr></table>").unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        assert_eq!(children[0].kind.as_str(), node::TABLE);
    }

    #[test]
    fn test_parse_image() {
        let result = parse(r#"<img src="test.png" alt="Test image">"#).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        let img = &children[0];
        assert_eq!(img.kind.as_str(), node::IMAGE);
        assert_eq!(img.props.get_str(prop::URL), Some("test.png"));
        assert_eq!(img.props.get_str(prop::ALT), Some("Test image"));
    }

    #[test]
    fn test_parse_html_metadata() {
        let input = r#"<!DOCTYPE html>
<html>
<head>
    <title>My Page Title</title>
    <meta name="author" content="Jane Doe">
    <meta name="description" content="A test page">
    <meta name="keywords" content="test, html, metadata">
    <meta property="og:image" content="https://example.com/image.png">
</head>
<body>
    <h1>Hello</h1>
    <p>Content here.</p>
</body>
</html>"#;
        let result = parse(input).unwrap();
        let doc = result.value;

        // Check metadata was extracted
        assert_eq!(doc.metadata.get_str("title"), Some("My Page Title"));
        assert_eq!(doc.metadata.get_str("author"), Some("Jane Doe"));
        assert_eq!(doc.metadata.get_str("description"), Some("A test page"));
        assert_eq!(
            doc.metadata.get_str("keywords"),
            Some("test, html, metadata")
        );
        // Open Graph metadata (og: prefix stripped)
        assert_eq!(
            doc.metadata.get_str("image"),
            Some("https://example.com/image.png")
        );

        // Content should still be parsed
        let children = root_children(&doc);
        assert!(!children.is_empty());
    }

    #[test]
    fn test_parse_data_uri_image() {
        // A small 1x1 red PNG as base64
        let data_uri = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==";
        let input = format!(r#"<p><img src="{}" alt="red pixel"></p>"#, data_uri);

        let options = ParseOptions {
            embed_resources: true,
            ..Default::default()
        };
        let result = parse_with_options(&input, &options).unwrap();
        let doc = result.value;

        // Should have extracted the resource
        assert_eq!(doc.resources.len(), 1);

        // The image node should have a resource_id, not a URL
        let para = &doc.content.children[0];
        let img = &para.children[0];
        assert_eq!(img.kind.as_str(), node::IMAGE);
        assert!(img.props.get_str(prop::RESOURCE_ID).is_some());
        assert!(img.props.get_str(prop::URL).is_none());
        assert_eq!(img.props.get_str(prop::ALT), Some("red pixel"));

        // Resource should have correct mime type
        let resource_id = img.props.get_str(prop::RESOURCE_ID).unwrap();
        let id = ResourceId::from_string(resource_id);
        let resource = doc.resources.get(&id).unwrap();
        assert_eq!(resource.mime_type, "image/png");
    }

    #[test]
    fn test_parse_footnote_convention() {
        let input = concat!(
            "<p>Text.<sup class=\"footnote-ref\"><a href=\"#fn-1\" id=\"fnref-1\">1</a></sup></p>",
            "<div id=\"fn-1\" class=\"footnote\">",
            "<sup class=\"footnote-label\">1</sup>",
            "<span class=\"footnote-content\">A note.</span>",
            "<a href=\"#fnref-1\" class=\"footnote-back\">\u{21a9}</a></div>",
        );
        let result = parse(input).unwrap();
        let doc = result.value;
        let children = root_children(&doc);

        let para = &children[0];
        let footnote_ref = para
            .children
            .iter()
            .find(|n| n.kind.as_str() == node::FOOTNOTE_REF)
            .expect("footnote_ref");
        assert_eq!(footnote_ref.props.get_str(prop::LABEL), Some("1"));

        let footnote_def = children
            .iter()
            .find(|n| n.kind.as_str() == node::FOOTNOTE_DEF)
            .expect("footnote_def");
        assert_eq!(footnote_def.props.get_str(prop::LABEL), Some("1"));
        assert_eq!(footnote_def.children.len(), 1);
        assert_eq!(
            footnote_def.children[0].props.get_str(prop::CONTENT),
            Some("A note.")
        );
    }

    #[test]
    #[cfg(feature = "writer-builder")]
    fn test_footnote_roundtrip() {
        let input = "<p>Text.</p><p>More<sup class=\"footnote-ref\"><a href=\"#fn-1\" id=\"fnref-1\">1</a></sup> text.</p>";
        let doc = parse(input).unwrap().value;
        let output = crate::rescribe::emit(&doc).unwrap().value;
        let html = String::from_utf8(output).unwrap();
        let doc2 = parse(&html).unwrap().value;

        assert_eq!(
            doc.content, doc2.content,
            "footnote ref roundtrip mismatch:\n{html}"
        );
    }

    #[test]
    #[cfg(feature = "writer-builder")]
    fn test_footnote_def_roundtrip() {
        let input = concat!(
            "<div id=\"fn-1\" class=\"footnote\">",
            "<sup class=\"footnote-label\">1</sup>",
            "<span class=\"footnote-content\"><em>A</em> note.</span>",
            "<a href=\"#fnref-1\" class=\"footnote-back\">\u{21a9}</a></div>",
        );
        let doc = parse(input).unwrap().value;
        let children = root_children(&doc);
        assert_eq!(children[0].kind.as_str(), node::FOOTNOTE_DEF);

        let output = crate::rescribe::emit(&doc).unwrap().value;
        let html = String::from_utf8(output).unwrap();
        let doc2 = parse(&html).unwrap().value;
        assert_eq!(
            doc.content, doc2.content,
            "footnote def roundtrip mismatch:\n{html}"
        );
    }

    #[test]
    fn test_parse_mathml_inline() {
        let input = "<p>Area is <math><mi>x</mi><mo>+</mo><mi>y</mi></math>.</p>";
        let result = parse(input).unwrap();
        let doc = result.value;
        let para = &root_children(&doc)[0];

        let math = para
            .children
            .iter()
            .find(|n| n.kind.as_str() == "math_inline")
            .expect("math_inline node");
        assert_eq!(math.props.get_str("math:format"), Some("mathml"));
        assert_eq!(
            math.props.get_str("math:source"),
            Some("<math><mi>x</mi><mo>+</mo><mi>y</mi></math>")
        );
    }

    #[test]
    fn test_parse_mathml_display() {
        let input = "<math display=\"block\"><mi>x</mi></math>";
        let result = parse(input).unwrap();
        let doc = result.value;
        let children = root_children(&doc);
        assert_eq!(children[0].kind.as_str(), "math_display");
        assert_eq!(children[0].props.get_str("math:format"), Some("mathml"));
    }

    #[test]
    #[cfg(feature = "writer-builder")]
    fn test_mathml_roundtrip() {
        let input = "<p>Solve <math><mi>x</mi><mo>=</mo><mn>2</mn></math> for x.</p>";
        let doc = parse(input).unwrap().value;
        let output = crate::rescribe::emit(&doc).unwrap().value;
        let html = String::from_utf8(output).unwrap();
        let doc2 = parse(&html).unwrap().value;
        assert_eq!(
            doc.content, doc2.content,
            "mathml roundtrip mismatch:\n{html}"
        );
    }

    #[test]
    #[cfg(feature = "writer-builder")]
    fn test_data_uri_roundtrip() {
        // A small 1x1 red PNG as base64
        let original_data_uri = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg==";
        let input = format!(r#"<img src="{}" alt="red pixel">"#, original_data_uri);

        // Parse with embed_resources enabled
        let options = ParseOptions {
            embed_resources: true,
            ..Default::default()
        };
        let result = parse_with_options(&input, &options).unwrap();
        let doc = result.value;

        // Emit back to HTML
        let output = crate::rescribe::emit(&doc).unwrap();
        let html = String::from_utf8(output.value).unwrap();

        // Should contain a data URI
        assert!(html.contains("data:image/png;base64,"));
        assert!(html.contains("alt=\"red pixel\""));

        // The base64 data should roundtrip correctly
        assert!(html.contains("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8DwHwAFBQIAX8jx0gAAAABJRU5ErkJggg=="));
    }
}
