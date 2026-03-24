//! Markdown parser using pulldown-cmark.

use std::ops::Range;

use pulldown_cmark::{
    CodeBlockKind, Event, HeadingLevel, MetadataBlockKind, Options, Parser, Tag, TagEnd,
};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, ParseError, ParseOptions, Properties, Severity,
    Span, WarningKind,
};
use rescribe_std::{Node, node, prop};

/// Parse markdown text into a rescribe Document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse markdown with custom options.
pub fn parse_with_options(
    input: &str,
    options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let mut warnings = Vec::new();
    let mut metadata = Properties::new();

    // CommonMark spec normalization:
    // - U+0000 → U+FFFD (security requirement)
    // - VT (\x0b) / FF (\x0c) → space (CommonMark whitespace characters)
    // - \r\n → \n, bare \r → \n (CommonMark line ending normalization)
    let normalized_input;
    let input = if input.contains('\x00')
        || input.contains('\x0b')
        || input.contains('\x0c')
        || input.contains('\r')
    {
        normalized_input = input
            .replace('\x00', "\u{FFFD}")
            .replace(['\x0b', '\x0c'], " ")
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        normalized_input.as_str()
    } else {
        input
    };

    // Enable common extensions including full GFM support
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    opts.insert(Options::ENABLE_GFM); // GitHub-style blockquotes like [!NOTE]

    let parser = Parser::new_ext(input, opts);
    // Collect events with source ranges for span tracking
    let events: Vec<_> = parser.into_offset_iter().collect();

    let children = parse_events(
        &events,
        input,
        &mut warnings,
        &mut metadata,
        options.preserve_source_info,
    );

    // Wrap children in a document root node
    let root = Node::new(node::DOCUMENT).children(children);
    let doc = Document::new().with_content(root).with_metadata(metadata);
    Ok(ConversionResult::with_warnings(doc, warnings))
}

/// Parse a slice of events into nodes.
fn parse_events(
    events: &[(Event<'_>, Range<usize>)],
    input: &str,
    warnings: &mut Vec<FidelityWarning>,
    metadata: &mut Properties,
    preserve_spans: bool,
) -> Vec<Node> {
    let mut nodes = Vec::new();
    let mut idx = 0;

    while idx < events.len() {
        let (node, consumed) =
            parse_event(&events[idx..], input, warnings, metadata, preserve_spans);
        if let Some(n) = node {
            nodes.push(n);
        }
        idx += consumed.max(1);
    }

    merge_text_nodes(nodes)
}

/// Merge consecutive text nodes into one.
///
/// pulldown-cmark can emit multiple adjacent Text events for what is logically
/// a single run of text (e.g. `])` → Text("]") + Text(")")). Merging here
/// keeps the IR consistent with the tree-sitter backend.
fn merge_text_nodes(nodes: Vec<Node>) -> Vec<Node> {
    let mut out: Vec<Node> = Vec::with_capacity(nodes.len());
    for node in nodes {
        if node.kind.as_str() == node::TEXT
            && let Some(last) = out.last_mut()
            && last.kind.as_str() == node::TEXT
            && matches!((last.span, node.span), (None, None) | (Some(_), Some(_)))
        {
            let prev = last.props.get_str(prop::CONTENT).unwrap_or("").to_string();
            let next = node.props.get_str(prop::CONTENT).unwrap_or("");
            last.props.set(prop::CONTENT, format!("{prev}{next}"));
            if let (Some(a), Some(b)) = (last.span.as_mut(), node.span) {
                a.end = b.end;
            }
            continue;
        }
        out.push(node);
    }
    out
}

/// Helper to optionally add span to a node.
fn with_span(mut node: Node, range: &Range<usize>, preserve_spans: bool) -> Node {
    if preserve_spans {
        node.span = Some(Span {
            start: range.start,
            end: range.end,
        });
    }
    node
}

/// Parse a single event or matched tag pair, returning the node and events consumed.
fn parse_event(
    events: &[(Event<'_>, Range<usize>)],
    input: &str,
    warnings: &mut Vec<FidelityWarning>,
    metadata: &mut Properties,
    preserve_spans: bool,
) -> (Option<Node>, usize) {
    let (event, range) = &events[0];
    match event {
        Event::Start(tag) => {
            parse_tag(tag.clone(), events, input, warnings, metadata, preserve_spans)
        }
        Event::Text(text) => (
            Some(with_span(
                Node::new(node::TEXT).prop(prop::CONTENT, text.to_string()),
                range,
                preserve_spans,
            )),
            1,
        ),
        Event::Code(code) => (
            Some(with_span(
                Node::new(node::CODE).prop(prop::CONTENT, code.to_string()),
                range,
                preserve_spans,
            )),
            1,
        ),
        Event::SoftBreak => (
            Some(with_span(
                Node::new(node::SOFT_BREAK),
                range,
                preserve_spans,
            )),
            1,
        ),
        Event::HardBreak => {
            let mut n = Node::new(node::LINE_BREAK);
            if preserve_spans {
                // Detect backslash vs two-space hard break
                if range.start < input.len() && input.as_bytes()[range.start] == b'\\' {
                    n = n.prop(prop::MD_BREAK_CHAR, "\\");
                }
                // Two-space break: no property needed (default); only set for backslash
            }
            (Some(with_span(n, range, preserve_spans)), 1)
        }
        Event::Rule => {
            let mut n = Node::new(node::HORIZONTAL_RULE);
            if preserve_spans && let Some(&first_byte) = input.as_bytes().get(range.start) {
                let marker = match first_byte {
                    b'-' => "-",
                    b'*' => "*",
                    b'_' => "_",
                    _ => "",
                };
                if !marker.is_empty() {
                    n = n.prop(prop::MD_BREAK_CHAR, marker);
                }
            }
            (Some(with_span(n, range, preserve_spans)), 1)
        }
        Event::End(_) => (None, 1), // Handled by parent
        Event::Html(html) => {
            // Raw HTML block
            let node = Node::new(node::RAW_BLOCK)
                .prop(prop::FORMAT, "html")
                .prop(prop::CONTENT, html.to_string());
            (Some(with_span(node, range, preserve_spans)), 1)
        }
        Event::InlineHtml(html) => {
            // Raw HTML inline
            let node = Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "html")
                .prop(prop::CONTENT, html.to_string());
            (Some(with_span(node, range, preserve_spans)), 1)
        }
        Event::FootnoteReference(label) => {
            let node = Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.to_string());
            (Some(with_span(node, range, preserve_spans)), 1)
        }
        Event::TaskListMarker(_checked) => {
            // Task list markers are handled in list item parsing (parse_tag for Tag::Item)
            // This branch should rarely be reached since we process them there
            (None, 1)
        }
        Event::InlineMath(math) => {
            let node = Node::new("math_inline")
                .prop("math:format", "latex")
                .prop("math:source", math.to_string());
            (Some(with_span(node, range, preserve_spans)), 1)
        }
        Event::DisplayMath(math) => {
            let node = Node::new("math_display")
                .prop("math:format", "latex")
                .prop("math:source", math.to_string());
            (Some(with_span(node, range, preserve_spans)), 1)
        }
    }
}

/// Normalize list item children so inline content is always wrapped in a paragraph.
///
/// pulldown-cmark omits paragraph wrappers for tight list items and emits bare
/// inline nodes for mixed items like `- text\n  - sublist`. Wrapping keeps the
/// IR uniform regardless of tight/loose status.
fn normalize_item_children(mut children: Vec<Node>) -> Vec<Node> {
    if children.is_empty() {
        return children;
    }
    // Find the first block-level child
    let first_block = children.iter().position(|n| {
        matches!(
            n.kind.as_str(),
            node::PARAGRAPH
                | node::CODE_BLOCK
                | node::BLOCKQUOTE
                | node::LIST
                | node::TABLE
                | node::HORIZONTAL_RULE
                | node::DEFINITION_LIST
                | node::FOOTNOTE_DEF
        )
    });
    match first_block {
        None => {
            // All inline — wrap everything in a paragraph
            vec![Node::new(node::PARAGRAPH).children(children)]
        }
        Some(0) => children, // Already starts with a block
        Some(idx) => {
            // Leading inline nodes before first block — wrap them in a paragraph
            let rest: Vec<Node> = children.drain(idx..).collect();
            let para = Node::new(node::PARAGRAPH).children(children);
            let mut result = vec![para];
            result.extend(rest);
            result
        }
    }
}

/// Parse a tag and its contents.
fn parse_tag(
    tag: Tag<'_>,
    events: &[(Event<'_>, Range<usize>)],
    input: &str,
    warnings: &mut Vec<FidelityWarning>,
    metadata: &mut Properties,
    preserve_spans: bool,
) -> (Option<Node>, usize) {
    // Find the matching end tag
    let end_idx = find_matching_end(&events[1..], &tag);
    let inner_events = &events[1..=end_idx];
    let children = parse_events(inner_events, input, warnings, metadata, preserve_spans);
    let consumed = end_idx + 2; // +1 for start, +1 for end

    // Calculate span from start of first event to end of last event
    let tag_range = {
        let start = events[0].1.start;
        let end = events[end_idx + 1].1.end;
        start..end
    };

    let node = match tag {
        Tag::Paragraph => Some(with_span(
            Node::new(node::PARAGRAPH).children(children),
            &tag_range,
            preserve_spans,
        )),

        Tag::Heading { level, id, .. } => {
            let level_num = match level {
                HeadingLevel::H1 => 1,
                HeadingLevel::H2 => 2,
                HeadingLevel::H3 => 3,
                HeadingLevel::H4 => 4,
                HeadingLevel::H5 => 5,
                HeadingLevel::H6 => 6,
            };
            let mut h = Node::new(node::HEADING)
                .prop(prop::LEVEL, level_num as i64)
                .children(children);
            if let Some(id) = id {
                h = h.prop(prop::ID, id.to_string());
            }
            if preserve_spans {
                // Setext headings use `=` (H1) or `-` (H2) underlines; ATX uses `#`.
                // pulldown's heading range covers the full block; detect setext by
                // finding the first `\n` in the range and checking the char after it.
                let end = tag_range.end.min(input.len());
                let range_text = &input[tag_range.start..end];
                let style = if let Some(nl_pos) = range_text.find('\n') {
                    let after = range_text.as_bytes().get(nl_pos + 1).copied();
                    if after == Some(b'=') || after == Some(b'-')
                    {
                        "setext"
                    } else {
                        "atx"
                    }
                } else {
                    "atx"
                };
                h = h.prop(prop::MD_HEADING_STYLE, style);
            }
            Some(with_span(h, &tag_range, preserve_spans))
        }

        Tag::BlockQuote(_) => Some(with_span(
            Node::new(node::BLOCKQUOTE).children(children),
            &tag_range,
            preserve_spans,
        )),

        Tag::CodeBlock(kind) => {
            // For code blocks, children should be text content
            let content = children
                .into_iter()
                .filter_map(|n| {
                    if n.kind.as_str() == node::TEXT {
                        n.props.get_str(prop::CONTENT).map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("");

            // Strip exactly one trailing newline: the newline before the closing
            // fence is a formatting artifact, not part of the code content.
            let content = content.strip_suffix('\n').map(str::to_string).unwrap_or(content);
            let mut node = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content);
            if let CodeBlockKind::Fenced(lang) = &kind {
                let lang_str = lang.to_string();
                if !lang_str.is_empty() {
                    node = node.prop(prop::LANGUAGE, lang_str);
                }
                if preserve_spans && let Some(&fence_byte) = input.as_bytes().get(tag_range.start) {
                    let fence_char = if fence_byte == b'~' { "~" } else { "`" };
                    node = node.prop(prop::MD_FENCE_CHAR, fence_char);
                    // Count fence length
                    let fence_len = input[tag_range.start..]
                        .bytes()
                        .take_while(|&b| b == fence_byte)
                        .count();
                    node = node.prop(prop::MD_FENCE_LENGTH, fence_len as i64);
                }
            }
            Some(with_span(node, &tag_range, preserve_spans))
        }

        Tag::List(start) => {
            let ordered = start.is_some();
            // Detect tight vs loose: tight items don't wrap content in paragraphs.
            let tight = !inner_events
                .iter()
                .any(|(e, _)| matches!(e, Event::Start(Tag::Paragraph)));
            let mut list = Node::new(node::LIST)
                .prop(prop::ORDERED, ordered)
                .prop(prop::TIGHT, tight)
                .children(children);
            if let Some(start_num) = start {
                list = list.prop(prop::START, start_num as i64);
            }
            if preserve_spans && start.is_none() {
                // Unordered list: detect marker character
                if let Some(&marker_byte) = input.as_bytes().get(tag_range.start) {
                    let marker = match marker_byte {
                        b'-' => "-",
                        b'*' => "*",
                        b'+' => "+",
                        _ => "",
                    };
                    if !marker.is_empty() {
                        list = list.prop(prop::MD_LIST_MARKER, marker);
                    }
                }
            }
            Some(with_span(list, &tag_range, preserve_spans))
        }

        Tag::Item => {
            // Check for task list marker in inner events
            let task_checked = inner_events.iter().find_map(|(event, _)| {
                if let Event::TaskListMarker(checked) = event {
                    Some(*checked)
                } else {
                    None
                }
            });

            // pulldown-cmark omits paragraph wrappers in tight list items, and
            // for mixed items (e.g. "- text\n  - sublist") it emits the text
            // as a bare inline node followed by the sublist block. Normalize:
            // any leading inline run (before the first block child) gets wrapped
            // in a paragraph so both tight and loose items have uniform structure.
            let normalized_children = normalize_item_children(children);

            let mut item = Node::new(node::LIST_ITEM).children(normalized_children);
            if let Some(checked) = task_checked {
                item = item.prop(prop::CHECKED, checked);
            }
            Some(with_span(item, &tag_range, preserve_spans))
        }

        Tag::FootnoteDefinition(label) => Some(with_span(
            Node::new(node::FOOTNOTE_DEF)
                .prop(prop::LABEL, label.to_string())
                .children(children),
            &tag_range,
            preserve_spans,
        )),

        Tag::Table(alignments) => {
            // Store alignment info
            let align_strs: Vec<_> = alignments
                .iter()
                .map(|a| match a {
                    pulldown_cmark::Alignment::None => "none",
                    pulldown_cmark::Alignment::Left => "left",
                    pulldown_cmark::Alignment::Center => "center",
                    pulldown_cmark::Alignment::Right => "right",
                })
                .collect();
            Some(with_span(
                Node::new(node::TABLE)
                    .prop("column_alignments", align_strs.join(","))
                    .children(children),
                &tag_range,
                preserve_spans,
            ))
        }

        Tag::TableHead => {
            // pulldown-cmark emits TableCell events directly inside TableHead (no
            // TableRow wrapper). Wrap them in a TABLE_ROW so the structure matches
            // tree-sitter: TABLE_HEAD → TABLE_ROW → TABLE_CELL.
            let row = Node::new(node::TABLE_ROW).children(children);
            Some(with_span(
                Node::new(node::TABLE_HEAD).child(row),
                &tag_range,
                preserve_spans,
            ))
        }

        Tag::TableRow => Some(with_span(
            Node::new(node::TABLE_ROW).children(children),
            &tag_range,
            preserve_spans,
        )),

        Tag::TableCell => Some(with_span(
            Node::new(node::TABLE_CELL).children(children),
            &tag_range,
            preserve_spans,
        )),

        Tag::Emphasis => {
            let mut n = Node::new(node::EMPHASIS).children(children);
            if preserve_spans && let Some(&marker_byte) = input.as_bytes().get(tag_range.start) {
                let marker = if marker_byte == b'_' { "_" } else { "*" };
                n = n.prop(prop::MD_EMPHASIS_MARKER, marker);
            }
            Some(with_span(n, &tag_range, preserve_spans))
        }

        Tag::Strong => {
            let mut n = Node::new(node::STRONG).children(children);
            if preserve_spans && let Some(&marker_byte) = input.as_bytes().get(tag_range.start) {
                let marker = if marker_byte == b'_' { "__" } else { "**" };
                n = n.prop(prop::MD_STRONG_MARKER, marker);
            }
            Some(with_span(n, &tag_range, preserve_spans))
        }

        Tag::Strikethrough => Some(with_span(
            Node::new(node::STRIKEOUT).children(children),
            &tag_range,
            preserve_spans,
        )),

        Tag::Link {
            dest_url, title, ..
        } => {
            let mut link = Node::new(node::LINK)
                .prop(prop::URL, dest_url.to_string())
                .children(children);
            if !title.is_empty() {
                link = link.prop(prop::TITLE, title.to_string());
            }
            Some(with_span(link, &tag_range, preserve_spans))
        }

        Tag::Image {
            dest_url, title, ..
        } => {
            // For images, children are alt text
            let alt = children
                .into_iter()
                .filter_map(|n| {
                    if n.kind.as_str() == node::TEXT {
                        n.props.get_str(prop::CONTENT).map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("");

            let mut img = Node::new(node::IMAGE)
                .prop(prop::URL, dest_url.to_string())
                .prop(prop::ALT, alt);
            if !title.is_empty() {
                img = img.prop(prop::TITLE, title.to_string());
            }
            Some(with_span(img, &tag_range, preserve_spans))
        }

        Tag::HtmlBlock => {
            // Raw HTML block - content is in children
            let content = children
                .into_iter()
                .filter_map(|n| n.props.get_str(prop::CONTENT).map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join("");
            Some(with_span(
                Node::new(node::RAW_BLOCK)
                    .prop(prop::FORMAT, "html")
                    .prop(prop::CONTENT, content),
                &tag_range,
                preserve_spans,
            ))
        }

        Tag::MetadataBlock(kind) => {
            // Extract content from children (text nodes)
            let content = children
                .iter()
                .filter_map(|n| {
                    if n.kind.as_str() == node::TEXT {
                        n.props.get_str(prop::CONTENT).map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("");

            match kind {
                MetadataBlockKind::YamlStyle => {
                    parse_yaml_metadata(&content, metadata, warnings);
                }
                MetadataBlockKind::PlusesStyle => {
                    parse_toml_metadata(&content, metadata, warnings);
                }
            }
            None
        }

        Tag::DefinitionList => Some(with_span(
            Node::new(node::DEFINITION_LIST).children(children),
            &tag_range,
            preserve_spans,
        )),

        Tag::DefinitionListTitle => Some(with_span(
            Node::new(node::DEFINITION_TERM).children(children),
            &tag_range,
            preserve_spans,
        )),

        Tag::DefinitionListDefinition => Some(with_span(
            Node::new(node::DEFINITION_DESC).children(children),
            &tag_range,
            preserve_spans,
        )),
    };

    (node, consumed)
}

/// Find the index of the matching end tag.
fn find_matching_end(events: &[(Event<'_>, Range<usize>)], start_tag: &Tag<'_>) -> usize {
    let mut depth = 1;
    for (i, (event, _)) in events.iter().enumerate() {
        match event {
            Event::Start(t) if tags_match(t, start_tag) => depth += 1,
            Event::End(t) if tag_end_matches(t, start_tag) => {
                depth -= 1;
                if depth == 0 {
                    return i;
                }
            }
            _ => {}
        }
    }
    events.len().saturating_sub(1)
}

/// Check if two start tags are the same type.
fn tags_match(a: &Tag<'_>, b: &Tag<'_>) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
}

/// Check if an end tag matches a start tag.
fn tag_end_matches(end: &TagEnd, start: &Tag<'_>) -> bool {
    matches!(
        (end, start),
        (TagEnd::Paragraph, Tag::Paragraph)
            | (TagEnd::Heading(_), Tag::Heading { .. })
            | (TagEnd::BlockQuote(_), Tag::BlockQuote(_))
            | (TagEnd::CodeBlock, Tag::CodeBlock(_))
            | (TagEnd::List(_), Tag::List(_))
            | (TagEnd::Item, Tag::Item)
            | (TagEnd::FootnoteDefinition, Tag::FootnoteDefinition(_))
            | (TagEnd::Table, Tag::Table(_))
            | (TagEnd::TableHead, Tag::TableHead)
            | (TagEnd::TableRow, Tag::TableRow)
            | (TagEnd::TableCell, Tag::TableCell)
            | (TagEnd::Emphasis, Tag::Emphasis)
            | (TagEnd::Strong, Tag::Strong)
            | (TagEnd::Strikethrough, Tag::Strikethrough)
            | (TagEnd::Link, Tag::Link { .. })
            | (TagEnd::Image, Tag::Image { .. })
            | (TagEnd::HtmlBlock, Tag::HtmlBlock)
            | (TagEnd::MetadataBlock(_), Tag::MetadataBlock(_))
            | (TagEnd::DefinitionList, Tag::DefinitionList)
            | (TagEnd::DefinitionListTitle, Tag::DefinitionListTitle)
            | (
                TagEnd::DefinitionListDefinition,
                Tag::DefinitionListDefinition
            )
    )
}

/// Parse YAML frontmatter and populate document metadata.
fn parse_yaml_metadata(
    content: &str,
    metadata: &mut Properties,
    warnings: &mut Vec<FidelityWarning>,
) {
    let yaml: Result<serde_yaml::Value, _> = serde_yaml::from_str(content);

    match yaml {
        Ok(serde_yaml::Value::Mapping(map)) => {
            flatten_yaml_value("", &serde_yaml::Value::Mapping(map), metadata);
        }
        Ok(_) => {
            warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::FeatureLost("yaml_frontmatter".to_string()),
                "YAML frontmatter must be a mapping/object",
            ));
        }
        Err(e) => {
            warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::FeatureLost("yaml_frontmatter".to_string()),
                format!("Failed to parse YAML frontmatter: {}", e),
            ));
        }
    }
}

/// Recursively flatten YAML values into metadata with dot-notation keys.
fn flatten_yaml_value(prefix: &str, value: &serde_yaml::Value, metadata: &mut Properties) {
    match value {
        serde_yaml::Value::Mapping(map) => {
            for (k, v) in map {
                if let serde_yaml::Value::String(key_str) = k {
                    let full_key = if prefix.is_empty() {
                        key_str.clone()
                    } else {
                        format!("{}.{}", prefix, key_str)
                    };
                    flatten_yaml_value(&full_key, v, metadata);
                }
            }
        }
        serde_yaml::Value::String(s) => {
            metadata.set(prefix, s.clone());
        }
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                metadata.set(prefix, i);
            } else if let Some(f) = n.as_f64() {
                metadata.set(prefix, f);
            }
        }
        serde_yaml::Value::Bool(b) => {
            metadata.set(prefix, *b);
        }
        serde_yaml::Value::Sequence(seq) => {
            // Store arrays as PropValue::List
            let items: Vec<rescribe_core::PropValue> =
                seq.iter().filter_map(yaml_to_prop_value).collect();
            if !items.is_empty() {
                metadata.set(prefix, rescribe_core::PropValue::List(items));
            }
        }
        serde_yaml::Value::Null => {}
        serde_yaml::Value::Tagged(tagged) => {
            // Ignore the tag and process the inner value
            flatten_yaml_value(prefix, &tagged.value, metadata);
        }
    }
}

/// Convert a YAML value to a PropValue.
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
        serde_yaml::Value::Sequence(seq) => {
            let items: Vec<rescribe_core::PropValue> =
                seq.iter().filter_map(yaml_to_prop_value).collect();
            Some(rescribe_core::PropValue::List(items))
        }
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

/// Parse TOML frontmatter and populate document metadata.
fn parse_toml_metadata(
    content: &str,
    metadata: &mut Properties,
    warnings: &mut Vec<FidelityWarning>,
) {
    let toml_result: Result<toml::Value, _> = content.parse();

    match toml_result {
        Ok(toml::Value::Table(table)) => {
            flatten_toml_value("", &toml::Value::Table(table), metadata);
        }
        Ok(_) => {
            warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::FeatureLost("toml_frontmatter".to_string()),
                "TOML frontmatter must be a table/object",
            ));
        }
        Err(e) => {
            warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::FeatureLost("toml_frontmatter".to_string()),
                format!("Failed to parse TOML frontmatter: {}", e),
            ));
        }
    }
}

/// Recursively flatten TOML values into metadata with dot-notation keys.
fn flatten_toml_value(prefix: &str, value: &toml::Value, metadata: &mut Properties) {
    match value {
        toml::Value::Table(table) => {
            for (key, v) in table {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_toml_value(&full_key, v, metadata);
            }
        }
        toml::Value::String(s) => {
            metadata.set(prefix, s.clone());
        }
        toml::Value::Integer(i) => {
            metadata.set(prefix, *i);
        }
        toml::Value::Float(f) => {
            metadata.set(prefix, *f);
        }
        toml::Value::Boolean(b) => {
            metadata.set(prefix, *b);
        }
        toml::Value::Array(arr) => {
            let items: Vec<rescribe_core::PropValue> =
                arr.iter().filter_map(toml_to_prop_value).collect();
            if !items.is_empty() {
                metadata.set(prefix, rescribe_core::PropValue::List(items));
            }
        }
        toml::Value::Datetime(dt) => {
            metadata.set(prefix, dt.to_string());
        }
    }
}

/// Convert a TOML value to a PropValue.
fn toml_to_prop_value(value: &toml::Value) -> Option<rescribe_core::PropValue> {
    match value {
        toml::Value::String(s) => Some(rescribe_core::PropValue::String(s.clone())),
        toml::Value::Integer(i) => Some(rescribe_core::PropValue::Int(*i)),
        toml::Value::Float(f) => Some(rescribe_core::PropValue::Float(*f)),
        toml::Value::Boolean(b) => Some(rescribe_core::PropValue::Bool(*b)),
        toml::Value::Array(arr) => {
            let items: Vec<rescribe_core::PropValue> =
                arr.iter().filter_map(toml_to_prop_value).collect();
            Some(rescribe_core::PropValue::List(items))
        }
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
