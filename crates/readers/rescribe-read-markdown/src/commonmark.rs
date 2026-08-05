//! Markdown → rescribe IR adapter using the `commonmark-fmt` crate.
//!
//! Translates [`commonmark_fmt::CmDoc`] into a rescribe [`Document`].
//! All IR construction happens here; `commonmark-fmt` has no rescribe dependency.

use commonmark_fmt::{Block, CmDoc, FrontMatter, FrontMatterKind, Inline, ListItem, ListKind};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, ParseOptions, Properties, Severity, Span,
    WarningKind,
};
use rescribe_format_api::Parse as _;
use rescribe_std::{Node, node, prop};

/// Parse markdown bytes into a rescribe Document.
pub fn parse_with_options(input: &[u8], _opts: &ParseOptions) -> ConversionResult<Document> {
    let (cm_doc, diags) = CmDoc::parse(input);

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
    detect_unsupported_extensions(input, &mut warnings);

    let children = convert_doc(&cm_doc, &mut warnings);
    let root = Node::new(node::DOCUMENT).children(children);
    let doc = Document::new().with_content(root).with_metadata(metadata);
    ConversionResult::with_warnings(doc, warnings)
}

/// Parse front-matter content (raw YAML/TOML text captured by commonmark-fmt)
/// into document metadata. Not CommonMark parsing — a different format
/// entirely, so this belongs in the adapter, matching the precedent already
/// established in `pulldown.rs`.
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

/// Best-effort, heuristic detection of Markdown extension syntax the default
/// backend does not yet model (footnotes, definition lists, math). These
/// constructs degrade gracefully to plain prose (CommonMark has no syntax
/// conflict with them), so nothing is corrupted — but the semantic construct
/// is lost, so CLAUDE.md requires a fidelity warning rather than silence.
///
/// This is intentionally conservative source-text pattern matching, not a
/// parser: it can both under- and over-detect (e.g. `$` used as a literal
/// currency sign would false-positive as math). Callers who need these
/// constructs modeled should use `backend_pulldown::parse` (the `pulldown`
/// feature), which supports them natively. Tracked in TODO.md.
/// Historically scanned raw source for footnote/definition-list/math syntax
/// the default backend couldn't yet parse, emitting a heuristic fidelity
/// warning instead of silently dropping the construct. Now a no-op: as of
/// this pass `commonmark-fmt`'s `footnotes`/`definition-lists`/`math`
/// features are real and requested by this crate's `Cargo.toml`, so
/// `convert_block`/`convert_inline` (via `Block::FootnoteDefinition`/
/// `DefinitionList`, `Inline::FootnoteReference`/`InlineMath`/`DisplayMath`)
/// genuinely parse and convert all three — a heuristic "possibly unsupported"
/// warning here would now be a false positive on correctly-converted input.
/// Kept as a named function (rather than deleting the call site in
/// `parse_with_options`) in case a future, still-unsupported extension needs
/// the same kind of heuristic detection.
fn detect_unsupported_extensions(_input: &[u8], _warnings: &mut Vec<FidelityWarning>) {}

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
            maybe_span(Node::new(node::PARAGRAPH).children(children), span)
        }
        Block::Heading {
            level,
            inlines,
            span,
        } => {
            let children = convert_inlines(inlines, warnings);
            let n = Node::new(node::HEADING)
                .prop(prop::LEVEL, *level as i64)
                .children(children);
            maybe_span(n, span)
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
            maybe_span(list, span)
        }
        Block::ThematicBreak { span } => maybe_span(Node::new(node::HORIZONTAL_RULE), span),
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
            maybe_span(
                Node::new(node::TABLE)
                    .prop("column_alignments", align_strs.join(","))
                    .children(children),
                span,
            )
        }
        Block::FootnoteDefinition {
            label,
            blocks,
            span,
        } => {
            let children: Vec<Node> = blocks.iter().map(|b| convert_block(b, warnings)).collect();
            maybe_span(
                Node::new(node::FOOTNOTE_DEF)
                    .prop(prop::LABEL, label.clone())
                    .children(children),
                span,
            )
        }
        Block::DefinitionList { items, span, .. } => {
            let mut children: Vec<Node> = Vec::new();
            for item in items {
                children.push(
                    Node::new(node::DEFINITION_TERM)
                        .children(convert_inlines(&item.term, warnings)),
                );
                for def_blocks in &item.definitions {
                    let def_children: Vec<Node> = def_blocks
                        .iter()
                        .map(|b| convert_block(b, warnings))
                        .collect();
                    children.push(Node::new(node::DEFINITION_DESC).children(def_children));
                }
            }
            maybe_span(Node::new(node::DEFINITION_LIST).children(children), span)
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
fn convert_list_item(item: &ListItem, tight: bool, warnings: &mut Vec<FidelityWarning>) -> Node {
    let children: Vec<Node> = if tight
        && item.blocks.len() == 1
        && let Block::Paragraph { inlines, .. } = &item.blocks[0]
    {
        // Tight item: unwrap the implicit paragraph — emit inlines directly.
        convert_inlines(inlines, warnings)
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
    maybe_span(node, &item.span)
}

fn convert_inlines(inlines: &[Inline], warnings: &mut Vec<FidelityWarning>) -> Vec<Node> {
    inlines
        .iter()
        .map(|i| convert_inline(i, warnings))
        .collect()
}

fn convert_inline(inline: &Inline, warnings: &mut Vec<FidelityWarning>) -> Node {
    match inline {
        Inline::Text { content, span } => maybe_span(
            Node::new(node::TEXT).prop(prop::CONTENT, content.clone()),
            span,
        ),
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
        Inline::Code { content, span } => maybe_span(
            Node::new(node::CODE).prop(prop::CONTENT, content.clone()),
            span,
        ),
        Inline::HtmlInline { content, span } => {
            let n = Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "html")
                .prop(prop::CONTENT, content.clone());
            maybe_span(n, span)
        }
        Inline::Link {
            inlines,
            url,
            title,
            span,
        } => {
            let children = convert_inlines(inlines, warnings);
            let mut n = Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .children(children);
            if let Some(t) = title {
                n = n.prop(prop::TITLE, t.clone());
            }
            maybe_span(n, span)
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
            maybe_span(n, span)
        }
        Inline::FootnoteReference { label, span } => maybe_span(
            Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.clone()),
            span,
        ),
        Inline::InlineMath { source, span } => maybe_span(
            Node::new("math_inline")
                .prop("math:format", "latex")
                .prop("math:source", source.clone()),
            span,
        ),
        Inline::DisplayMath { source, span } => maybe_span(
            Node::new("math_display")
                .prop("math:format", "latex")
                .prop("math:source", source.clone()),
            span,
        ),
    }
}

/// Attach a [`Span`] to a node if the span is non-zero (i.e. not `Span::NONE`).
///
/// `commonmark-fmt` always records spans; we store them unconditionally since
/// rescribe-read-markdown's public API doesn't yet thread `preserve_source_info`
/// through to this adapter. Callers that want no spans can strip them afterward.
fn maybe_span(mut node: Node, span: &commonmark_fmt::Span) -> Node {
    if span.start != 0 || span.end != 0 {
        node.span = Some(Span {
            start: span.start,
            end: span.end,
        });
    }
    node
}
