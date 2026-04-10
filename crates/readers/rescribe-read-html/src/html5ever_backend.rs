//! HTML parser using html5ever.

use html5ever::tendril::TendrilSink;
use html5ever::{Attribute, QualName, parse_document};
use markup5ever_rcdom::{Handle, NodeData, RcDom};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, ParseError, ParseOptions, Properties, Resource,
    ResourceId, ResourceMap, Severity, WarningKind,
};
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

    // Parse HTML using html5ever
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut input.as_bytes())
        .map_err(|e| ParseError::Invalid(format!("HTML parse error: {:?}", e)))?;

    // Extract metadata from <head> (and <html> lang attribute)
    extract_metadata(&dom.document, &mut metadata);

    // Convert DOM to rescribe nodes
    let children = convert_children(&dom.document, &mut warnings, &mut resources, options);

    let root = Node::new(node::DOCUMENT).children(children);
    let mut doc = Document::new().with_content(root).with_metadata(metadata);
    doc.resources = resources;

    Ok(ConversionResult::with_warnings(doc, warnings))
}

/// Extract metadata from HTML head element.
fn extract_metadata(handle: &Handle, metadata: &mut Properties) {
    if let NodeData::Element { name, attrs, .. } = &handle.data {
        let tag = name.local.as_ref();

        match tag {
            "html" => {
                let attrs = attrs.borrow();
                if let Some(lang) = get_attr(&attrs, "lang") {
                    metadata.set("lang", lang);
                }
            }
            "title" => {
                let title = extract_element_text(handle);
                if !title.is_empty() {
                    metadata.set("title", title);
                }
            }
            "meta" => {
                let attrs = attrs.borrow();
                // charset declaration: <meta charset="utf-8">
                if let Some(charset) = get_attr(&attrs, "charset") {
                    metadata.set("charset", charset);
                }
                // http-equiv content-type: <meta http-equiv="content-type" content="text/html; charset=utf-8">
                if get_attr(&attrs, "http-equiv")
                    .as_deref()
                    .map(|v| v.eq_ignore_ascii_case("content-type"))
                    == Some(true)
                    && let Some(content) = get_attr(&attrs, "content")
                {
                    metadata.set("content-type", content);
                }
                // Standard name/content pairs
                if let Some(name) = get_attr(&attrs, "name")
                    && let Some(content) = get_attr(&attrs, "content")
                {
                    metadata.set(&name, content);
                }
                // Open Graph properties (og: prefix stripped)
                if let Some(property) = get_attr(&attrs, "property")
                    && let Some(content) = get_attr(&attrs, "content")
                {
                    let key = property.strip_prefix("og:").unwrap_or(&property);
                    metadata.set(key, content);
                }
            }
            "link" => {
                let attrs = attrs.borrow();
                if get_attr(&attrs, "rel").as_deref() == Some("stylesheet")
                    && let Some(href) = get_attr(&attrs, "href")
                {
                    metadata.set("stylesheet", href);
                }
            }
            "base" => {
                let attrs = attrs.borrow();
                if let Some(href) = get_attr(&attrs, "href") {
                    metadata.set("base", href);
                }
            }
            _ => {}
        }
    }

    for child in handle.children.borrow().iter() {
        extract_metadata(child, metadata);
    }
}

/// Extract text content from an element.
fn extract_element_text(handle: &Handle) -> String {
    let mut text = String::new();
    for child in handle.children.borrow().iter() {
        if let NodeData::Text { contents } = &child.data {
            text.push_str(&contents.borrow());
        }
        text.push_str(&extract_element_text(child));
    }
    text
}

/// Apply global HTML attributes (id, class, lang, dir, style) to a node.
fn apply_global_attrs(mut node: Node, attrs: &[Attribute]) -> Node {
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

/// Convert child nodes of a DOM node.
fn convert_children(
    handle: &Handle,
    warnings: &mut Vec<FidelityWarning>,
    resources: &mut ResourceMap,
    options: &ParseOptions,
) -> Vec<Node> {
    let mut nodes = Vec::new();

    for child in handle.children.borrow().iter() {
        nodes.extend(convert_node(child, warnings, resources, options));
    }

    merge_text_nodes(&mut nodes);

    nodes
}

/// Convert a single DOM node to rescribe Node(s).
fn convert_node(
    handle: &Handle,
    warnings: &mut Vec<FidelityWarning>,
    resources: &mut ResourceMap,
    options: &ParseOptions,
) -> Vec<Node> {
    match &handle.data {
        NodeData::Document => {
            let children = convert_children(handle, warnings, resources, options);
            vec![Node::new(node::DOCUMENT).children(children)]
        }

        NodeData::Text { contents } => {
            let text = contents.borrow().to_string();
            if text.trim().is_empty() {
                return vec![];
            }
            vec![Node::new(node::TEXT).prop(prop::CONTENT, text)]
        }

        NodeData::Element { name, attrs, .. } => {
            let attrs_borrowed = attrs.borrow();
            convert_element(name, &attrs_borrowed, handle, warnings, resources, options)
        }

        NodeData::Comment { .. } => vec![],
        NodeData::Doctype { .. } => vec![],
        NodeData::ProcessingInstruction { .. } => vec![],
    }
}

/// Convert an HTML element to a rescribe Node.
fn convert_element(
    name: &QualName,
    attrs: &[Attribute],
    handle: &Handle,
    warnings: &mut Vec<FidelityWarning>,
    resources: &mut ResourceMap,
    options: &ParseOptions,
) -> Vec<Node> {
    let tag = name.local.as_ref();
    let children = convert_children(handle, warnings, resources, options);

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

        "blockquote" => {
            apply_global_attrs(Node::new(node::BLOCKQUOTE).children(children), attrs)
        }

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

/// Get an attribute value by name.
fn get_attr(attrs: &[Attribute], name: &str) -> Option<String> {
    attrs
        .iter()
        .find(|a| a.name.local.as_ref() == name)
        .map(|a| a.value.to_string())
}
