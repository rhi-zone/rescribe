//! CommonMark reader for rescribe.
//!
//! Parses CommonMark (with GFM strikethrough) into rescribe's document IR
//! using the `commonmark-fmt` crate.

use commonmark_fmt::{Block, CmDoc, FrontMatter, FrontMatterKind, Inline, ListItem, ListKind};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, ParseError, ParseOptions, Properties, Severity,
    Span, WarningKind,
};
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
        .map(|d| {
            FidelityWarning::new(
                Severity::Minor,
                WarningKind::FeatureLost(d.code.to_string()),
                d.message,
            )
        })
        .collect();

    let mut metadata = Properties::new();
    if let Some(fm) = &cm_doc.frontmatter {
        parse_frontmatter(fm, &mut metadata, &mut warnings);
    }

    let children = convert_doc(&cm_doc, &mut warnings);
    let root = Node::new(node::DOCUMENT).children(children);
    let doc = Document::new().with_content(root).with_metadata(metadata);
    ConversionResult::with_warnings(doc, warnings)
}

/// Parse front-matter content (raw YAML/TOML text captured by commonmark-fmt)
/// into document metadata. This is not CommonMark parsing — YAML/TOML are
/// different formats entirely, so this logic legitimately lives in the
/// adapter rather than in `commonmark-fmt`.
fn parse_frontmatter(
    fm: &FrontMatter,
    metadata: &mut Properties,
    warnings: &mut Vec<FidelityWarning>,
) {
    match fm.kind {
        FrontMatterKind::Yaml => parse_yaml_metadata(&fm.content, metadata, warnings),
        FrontMatterKind::Toml => parse_toml_metadata(&fm.content, metadata, warnings),
    }
}

fn parse_yaml_metadata(
    content: &str,
    metadata: &mut Properties,
    warnings: &mut Vec<FidelityWarning>,
) {
    match serde_yaml::from_str::<serde_yaml::Value>(content) {
        Ok(serde_yaml::Value::Mapping(map)) => {
            flatten_yaml_value("", &serde_yaml::Value::Mapping(map), metadata);
        }
        Ok(_) => warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::FeatureLost("yaml_frontmatter".to_string()),
            "YAML frontmatter must be a mapping/object",
        )),
        Err(e) => warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::FeatureLost("yaml_frontmatter".to_string()),
            format!("Failed to parse YAML frontmatter: {e}"),
        )),
    }
}

fn flatten_yaml_value(prefix: &str, value: &serde_yaml::Value, metadata: &mut Properties) {
    match value {
        serde_yaml::Value::Mapping(map) => {
            for (k, v) in map {
                if let serde_yaml::Value::String(key_str) = k {
                    let full_key = if prefix.is_empty() {
                        key_str.clone()
                    } else {
                        format!("{prefix}.{key_str}")
                    };
                    flatten_yaml_value(&full_key, v, metadata);
                }
            }
        }
        serde_yaml::Value::String(s) => metadata.set(prefix, s.clone()),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                metadata.set(prefix, i);
            } else if let Some(f) = n.as_f64() {
                metadata.set(prefix, f);
            }
        }
        serde_yaml::Value::Bool(b) => metadata.set(prefix, *b),
        serde_yaml::Value::Sequence(seq) => {
            let items: Vec<rescribe_core::PropValue> =
                seq.iter().filter_map(yaml_to_prop_value).collect();
            if !items.is_empty() {
                metadata.set(prefix, rescribe_core::PropValue::List(items));
            }
        }
        serde_yaml::Value::Null => {}
        serde_yaml::Value::Tagged(tagged) => flatten_yaml_value(prefix, &tagged.value, metadata),
    }
}

fn yaml_to_prop_value(value: &serde_yaml::Value) -> Option<rescribe_core::PropValue> {
    match value {
        serde_yaml::Value::String(s) => Some(rescribe_core::PropValue::String(s.clone())),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(rescribe_core::PropValue::Int(i))
            } else {
                n.as_f64().map(rescribe_core::PropValue::Float)
            }
        }
        serde_yaml::Value::Bool(b) => Some(rescribe_core::PropValue::Bool(*b)),
        serde_yaml::Value::Sequence(seq) => Some(rescribe_core::PropValue::List(
            seq.iter().filter_map(yaml_to_prop_value).collect(),
        )),
        serde_yaml::Value::Mapping(map) => {
            let items: std::collections::HashMap<String, rescribe_core::PropValue> = map
                .iter()
                .filter_map(|(k, v)| {
                    if let serde_yaml::Value::String(key) = k {
                        yaml_to_prop_value(v).map(|pv| (key.clone(), pv))
                    } else {
                        None
                    }
                })
                .collect();
            Some(rescribe_core::PropValue::Map(items))
        }
        serde_yaml::Value::Null => None,
        serde_yaml::Value::Tagged(tagged) => yaml_to_prop_value(&tagged.value),
    }
}

fn parse_toml_metadata(
    content: &str,
    metadata: &mut Properties,
    warnings: &mut Vec<FidelityWarning>,
) {
    match content.parse::<toml::Value>() {
        Ok(toml::Value::Table(table)) => {
            flatten_toml_value("", &toml::Value::Table(table), metadata);
        }
        Ok(_) => warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::FeatureLost("toml_frontmatter".to_string()),
            "TOML frontmatter must be a table/object",
        )),
        Err(e) => warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::FeatureLost("toml_frontmatter".to_string()),
            format!("Failed to parse TOML frontmatter: {e}"),
        )),
    }
}

fn flatten_toml_value(prefix: &str, value: &toml::Value, metadata: &mut Properties) {
    match value {
        toml::Value::Table(table) => {
            for (key, v) in table {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_toml_value(&full_key, v, metadata);
            }
        }
        toml::Value::String(s) => metadata.set(prefix, s.clone()),
        toml::Value::Integer(i) => metadata.set(prefix, *i),
        toml::Value::Float(f) => metadata.set(prefix, *f),
        toml::Value::Boolean(b) => metadata.set(prefix, *b),
        toml::Value::Array(arr) => {
            let items: Vec<rescribe_core::PropValue> =
                arr.iter().filter_map(toml_to_prop_value).collect();
            if !items.is_empty() {
                metadata.set(prefix, rescribe_core::PropValue::List(items));
            }
        }
        toml::Value::Datetime(dt) => metadata.set(prefix, dt.to_string()),
    }
}

fn toml_to_prop_value(value: &toml::Value) -> Option<rescribe_core::PropValue> {
    match value {
        toml::Value::String(s) => Some(rescribe_core::PropValue::String(s.clone())),
        toml::Value::Integer(i) => Some(rescribe_core::PropValue::Int(*i)),
        toml::Value::Float(f) => Some(rescribe_core::PropValue::Float(*f)),
        toml::Value::Boolean(b) => Some(rescribe_core::PropValue::Bool(*b)),
        toml::Value::Array(arr) => Some(rescribe_core::PropValue::List(
            arr.iter().filter_map(toml_to_prop_value).collect(),
        )),
        toml::Value::Table(table) => {
            let items: std::collections::HashMap<String, rescribe_core::PropValue> = table
                .iter()
                .filter_map(|(k, v)| toml_to_prop_value(v).map(|pv| (k.clone(), pv)))
                .collect();
            Some(rescribe_core::PropValue::Map(items))
        }
        toml::Value::Datetime(dt) => Some(rescribe_core::PropValue::String(dt.to_string())),
    }
}

fn convert_doc(doc: &CmDoc, warnings: &mut Vec<FidelityWarning>) -> Vec<Node> {
    doc.blocks
        .iter()
        .map(|b| convert_block(b, warnings))
        .collect()
}

fn convert_block(block: &Block, warnings: &mut Vec<FidelityWarning>) -> Node {
    match block {
        Block::Paragraph { inlines, span } => {
            let children = convert_inlines(inlines, warnings);
            span_node(Node::new(node::PARAGRAPH).children(children), span)
        }
        Block::Heading {
            level,
            inlines,
            span,
        } => {
            let children = convert_inlines(inlines, warnings);
            span_node(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, *level as i64)
                    .children(children),
                span,
            )
        }
        Block::CodeBlock {
            language,
            content,
            span,
        } => {
            let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
            if let Some(lang) = language {
                n = n.prop(prop::LANGUAGE, lang.clone());
            }
            span_node(n, span)
        }
        Block::HtmlBlock { content, span } => span_node(
            Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, "html")
                .prop(prop::CONTENT, content.clone()),
            span,
        ),
        Block::Blockquote { blocks, span } => {
            let children: Vec<Node> = blocks.iter().map(|b| convert_block(b, warnings)).collect();
            span_node(Node::new(node::BLOCKQUOTE).children(children), span)
        }
        Block::List {
            kind,
            items,
            tight,
            span,
        } => {
            let (ordered, start) = match kind {
                ListKind::Unordered { .. } => (false, None::<u64>),
                ListKind::Ordered { start, .. } => (true, Some(*start)),
            };
            let item_nodes: Vec<Node> = items
                .iter()
                .map(|item| convert_list_item(item, *tight, warnings))
                .collect();
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
        Block::Table {
            alignments,
            head,
            rows,
            span,
        } => {
            let align_strs: Vec<&str> = alignments
                .iter()
                .map(|a| match a {
                    commonmark_fmt::ColumnAlignment::None => "none",
                    commonmark_fmt::ColumnAlignment::Left => "left",
                    commonmark_fmt::ColumnAlignment::Center => "center",
                    commonmark_fmt::ColumnAlignment::Right => "right",
                })
                .collect();
            let head_row = Node::new(node::TABLE_ROW).children(head.cells.iter().map(|c| {
                Node::new(node::TABLE_CELL).children(convert_inlines(&c.inlines, warnings))
            }));
            let head_node = Node::new(node::TABLE_HEAD).child(head_row);
            let body_rows: Vec<Node> = rows
                .iter()
                .map(|r| {
                    Node::new(node::TABLE_ROW).children(r.cells.iter().map(|c| {
                        Node::new(node::TABLE_CELL).children(convert_inlines(&c.inlines, warnings))
                    }))
                })
                .collect();
            let mut children = vec![head_node];
            children.extend(body_rows);
            span_node(
                Node::new(node::TABLE)
                    .prop("column_alignments", align_strs.join(","))
                    .children(children),
                span,
            )
        }
    }
}

fn convert_list_item(item: &ListItem, tight: bool, warnings: &mut Vec<FidelityWarning>) -> Node {
    let children: Vec<Node> =
        if tight && item.blocks.len() == 1 && matches!(&item.blocks[0], Block::Paragraph { .. }) {
            if let Block::Paragraph { inlines, .. } = &item.blocks[0] {
                convert_inlines(inlines, warnings)
            } else {
                unreachable!()
            }
        } else {
            item.blocks
                .iter()
                .map(|b| convert_block(b, warnings))
                .collect()
        };
    let mut node = Node::new(node::LIST_ITEM).children(children);
    if let Some(checked) = item.checked {
        node = node.prop(prop::CHECKED, checked);
    }
    span_node(node, &item.span)
}

fn convert_inlines(inlines: &[Inline], warnings: &mut Vec<FidelityWarning>) -> Vec<Node> {
    inlines
        .iter()
        .map(|i| convert_inline(i, warnings))
        .collect()
}

fn convert_inline(inline: &Inline, warnings: &mut Vec<FidelityWarning>) -> Node {
    match inline {
        Inline::Text { content, span } => span_node(
            Node::new(node::TEXT).prop(prop::CONTENT, content.clone()),
            span,
        ),
        Inline::SoftBreak { span } => span_node(Node::new(node::SOFT_BREAK), span),
        Inline::HardBreak { span } => span_node(Node::new(node::LINE_BREAK), span),
        Inline::Emphasis { inlines, span } => span_node(
            Node::new(node::EMPHASIS).children(convert_inlines(inlines, warnings)),
            span,
        ),
        Inline::Strong { inlines, span } => span_node(
            Node::new(node::STRONG).children(convert_inlines(inlines, warnings)),
            span,
        ),
        Inline::Strikethrough { inlines, span } => span_node(
            Node::new(node::STRIKEOUT).children(convert_inlines(inlines, warnings)),
            span,
        ),
        Inline::Code { content, span } => span_node(
            Node::new(node::CODE).prop(prop::CONTENT, content.clone()),
            span,
        ),
        Inline::HtmlInline { content, span } => span_node(
            Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "html")
                .prop(prop::CONTENT, content.clone()),
            span,
        ),
        Inline::Link {
            inlines,
            url,
            title,
            span,
        } => {
            let mut n = Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(convert_inlines(inlines, warnings));
            if let Some(t) = title {
                n = n.prop(prop::TITLE, t.clone());
            }
            span_node(n, span)
        }
        Inline::Image {
            alt,
            url,
            title,
            span,
        } => {
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
        node.span = Some(Span {
            start: span.start,
            end: span.end,
        });
    }
    node
}
