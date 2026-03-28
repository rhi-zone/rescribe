//! ODT (OpenDocument Text) reader for rescribe.
//!
//! Parses ODF/ODT documents into rescribe's document IR.

use quick_xml::Reader;
use quick_xml::events::Event;
use rescribe_core::{ConversionResult, Document, ParseError, ParseOptions, Properties};
use rescribe_std::{Node, node, prop};
use std::collections::HashMap;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Parse ODT input into a document.
pub fn parse(input: &[u8]) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse ODT input into a document with options.
pub fn parse_with_options(
    input: &[u8],
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let cursor = Cursor::new(input);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| ParseError::Invalid(format!("Invalid ODT: {}", e)))?;

    let mut metadata = Properties::new();

    // Read meta.xml for metadata
    if let Ok(mut meta_file) = archive.by_name("meta.xml") {
        let mut meta_content = String::new();
        meta_file.read_to_string(&mut meta_content).map_err(ParseError::Io)?;
        parse_metadata(&meta_content, &mut metadata);
    }

    // Read styles.xml for named styles (Bold, Italic, etc.)
    let named_styles = if let Ok(mut styles_file) = archive.by_name("styles.xml") {
        let mut styles_xml = String::new();
        styles_file.read_to_string(&mut styles_xml).map_err(ParseError::Io)?;
        parse_named_styles(&styles_xml)
    } else {
        HashMap::new()
    };

    // Read content.xml
    let mut content_file = archive
        .by_name("content.xml")
        .map_err(|e| ParseError::Invalid(format!("Missing content.xml: {}", e)))?;

    let mut content_xml = String::new();
    content_file.read_to_string(&mut content_xml).map_err(ParseError::Io)?;

    let content = parse_content(&content_xml, &named_styles)?;

    Ok(ConversionResult::ok(Document {
        content,
        resources: Default::default(),
        metadata,
        source: None,
    }))
}

// ── Style property types ───────────────────────────────────────────────────────

/// Resolved text formatting properties for a style.
#[derive(Debug, Clone, Default)]
struct TextProps {
    bold: bool,
    italic: bool,
    underline: bool,
    strikeout: bool,
    subscript: bool,
    superscript: bool,
    code: bool, // monospace font → inline code
}

impl TextProps {
    #[allow(dead_code)]
    fn is_any(&self) -> bool {
        self.bold || self.italic || self.underline || self.strikeout
            || self.subscript || self.superscript || self.code
    }
}

/// Whether a list level uses bullet (unordered) or number (ordered) markers.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ListLevelKind {
    Bullet,
    Number,
}

/// A named or automatic style entry.
#[derive(Debug, Clone, Default)]
struct StyleEntry {
    text: TextProps,
    /// For paragraph styles: the paragraph-level kind (code block, blockquote, etc.)
    para_kind: ParaKind,
    /// For list styles: the kind of the first level.
    list_level: Option<ListLevelKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum ParaKind {
    #[default]
    Normal,
    Heading(u8),
    Code,
    Blockquote,
}

// ── Style parsing ──────────────────────────────────────────────────────────────

/// Parse styles from styles.xml (named paragraph/character styles).
fn parse_named_styles(xml: &str) -> HashMap<String, StyleEntry> {
    parse_styles_from_xml(xml)
}

fn parse_styles_from_xml(xml: &str) -> HashMap<String, StyleEntry> {
    let mut map: HashMap<String, StyleEntry> = HashMap::new();
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_name = String::new();
    let mut current_family = String::new();
    let mut in_list_style = false;
    let mut current_list_name = String::new();
    let mut in_style = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "style:style" => {
                        current_name.clear();
                        current_family.clear();
                        in_style = true;
                        for attr in e.attributes().flatten() {
                            match attr.key.as_ref() {
                                b"style:name" => {
                                    current_name = String::from_utf8_lossy(&attr.value).to_string();
                                }
                                b"style:family" => {
                                    current_family = String::from_utf8_lossy(&attr.value).to_string();
                                }
                                _ => {}
                            }
                        }
                        // Initialize entry if new
                        if !current_name.is_empty() {
                            let entry = map.entry(current_name.clone()).or_default();
                            // Heuristic: style name contains Bold/Italic/etc
                            apply_name_heuristics(&current_name, entry);
                            // Heading levels by name convention
                            if (current_name.starts_with("Heading") || current_name.starts_with("heading"))
                                && let Some(level_str) = current_name
                                    .trim_start_matches("Heading")
                                    .trim_start_matches("heading")
                                    .trim()
                                    .split(|c: char| !c.is_ascii_digit())
                                    .next()
                                && let Ok(level) = level_str.parse::<u8>() {
                                    entry.para_kind = ParaKind::Heading(level.min(6));
                            }
                            if current_name.contains("Quotation") || current_name.contains("Blockquote") || current_name.contains("Quote") {
                                entry.para_kind = ParaKind::Blockquote;
                            }
                            if current_name.contains("Preformatted") || current_name.contains("Code") || current_name.contains("Monospace") {
                                if current_family == "paragraph" || current_family.is_empty() {
                                    entry.para_kind = ParaKind::Code;
                                } else {
                                    entry.text.code = true;
                                }
                            }
                        }
                    }
                    "style:text-properties" if in_style && !current_name.is_empty() => {
                        let entry = map.entry(current_name.clone()).or_default();
                        parse_text_properties_attrs(e.attributes(), &mut entry.text);
                    }
                    "style:paragraph-properties" if in_style && !current_name.is_empty() => {
                        // Could read margin-left for blockquote detection, but name heuristics above handle common cases
                    }
                    "text:list-style" => {
                        in_list_style = true;
                        current_list_name.clear();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"style:name" {
                                current_list_name = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                    }
                    "text:list-level-style-bullet" if in_list_style && !current_list_name.is_empty() => {
                        let entry = map.entry(current_list_name.clone()).or_default();
                        if entry.list_level.is_none() {
                            entry.list_level = Some(ListLevelKind::Bullet);
                        }
                    }
                    "text:list-level-style-number" if in_list_style && !current_list_name.is_empty() => {
                        let entry = map.entry(current_list_name.clone()).or_default();
                        if entry.list_level.is_none() {
                            entry.list_level = Some(ListLevelKind::Number);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "style:style" => { in_style = false; }
                    "text:list-style" => { in_list_style = false; }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    map
}

fn apply_name_heuristics(name: &str, entry: &mut StyleEntry) {
    let lower = name.to_lowercase();
    if lower.contains("bold") { entry.text.bold = true; }
    if lower.contains("italic") || lower.contains("oblique") { entry.text.italic = true; }
    if lower.contains("underline") || lower.contains("underl") { entry.text.underline = true; }
    if lower.contains("strike") || lower.contains("through") { entry.text.strikeout = true; }
    if lower.contains("subscript") || lower == "sub" { entry.text.subscript = true; }
    if lower.contains("superscript") || lower == "sup" { entry.text.superscript = true; }
    if lower.contains("code") || lower.contains("preformat") || lower.contains("monospace") || lower.contains("verbatim") {
        entry.text.code = true;
    }
}

fn parse_text_properties_attrs(
    attrs: quick_xml::events::attributes::Attributes<'_>,
    props: &mut TextProps,
) {
    for attr in attrs.flatten() {
        match attr.key.as_ref() {
            b"fo:font-weight" => {
                if attr.value.as_ref() == b"bold" { props.bold = true; }
            }
            b"fo:font-style" => {
                if attr.value.as_ref() == b"italic" || attr.value.as_ref() == b"oblique" {
                    props.italic = true;
                }
            }
            b"style:text-underline-style" => {
                let v = attr.value.as_ref();
                if v != b"none" && !v.is_empty() { props.underline = true; }
            }
            b"style:text-line-through-style" => {
                let v = attr.value.as_ref();
                if v != b"none" && !v.is_empty() { props.strikeout = true; }
            }
            b"style:text-position" => {
                // "sub N%" or "super N%" or "sub" or "super"
                let val = String::from_utf8_lossy(attr.value.as_ref()).to_lowercase();
                if val.starts_with("sub") { props.subscript = true; }
                if val.starts_with("super") { props.superscript = true; }
            }
            b"style:font-name" | b"fo:font-family" => {
                // Monospace/Code detection by common font names
                let val = String::from_utf8_lossy(attr.value.as_ref()).to_lowercase();
                if val.contains("courier") || val.contains("mono") || val.contains("consol")
                    || val.contains("fixed") || val.contains("inconsolata") || val.contains("menlo")
                    || val == "code2000" || val == "source code pro"
                {
                    props.code = true;
                }
            }
            _ => {}
        }
    }
}

// ── Metadata parsing ───────────────────────────────────────────────────────────

fn parse_metadata(xml: &str, metadata: &mut Properties) {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut current_element = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_element = String::from_utf8_lossy(e.name().as_ref()).to_string();
            }
            Ok(Event::Text(e)) => {
                let text = String::from_utf8_lossy(e.as_ref()).to_string();
                match current_element.as_str() {
                    "dc:title" => { metadata.set("title", text); }
                    "dc:creator" => { metadata.set("author", text); }
                    "dc:date" | "dc:date-modified" => { metadata.set("date", text); }
                    "dc:description" => { metadata.set("description", text); }
                    "dc:language" => { metadata.set("language", text); }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
}

// ── Content parsing ────────────────────────────────────────────────────────────

/// An inline formatting context layer.
struct InlineCtx {
    /// Node kind for the wrapping node (empty = plain text context).
    kind: InlineCtxKind,
    /// Accumulated child nodes.
    children: Vec<Node>,
    /// Accumulated plain text since last child push.
    text: String,
}

enum InlineCtxKind {
    Plain,
    Strong,
    Emphasis,
    Underline,
    Strikeout,
    Code,
    Subscript,
    Superscript,
    Link { url: String },
}

impl InlineCtx {
    fn plain() -> Self {
        InlineCtx { kind: InlineCtxKind::Plain, children: Vec::new(), text: String::new() }
    }

    /// Flush accumulated text as a TEXT node (if non-empty).
    fn flush_text(&mut self) {
        if !self.text.is_empty() {
            let t = std::mem::take(&mut self.text);
            self.children.push(Node::new(node::TEXT).prop(prop::CONTENT, t));
        }
    }

    /// Finalise this context and return the wrapper node + its children,
    /// or the children directly (for Plain context).
    fn into_node(mut self) -> Vec<Node> {
        self.flush_text();
        match self.kind {
            InlineCtxKind::Plain => self.children,
            InlineCtxKind::Strong => {
                if self.children.is_empty() { return vec![]; }
                let mut n = Node::new(node::STRONG);
                for c in self.children { n = n.child(c); }
                vec![n]
            }
            InlineCtxKind::Emphasis => {
                if self.children.is_empty() { return vec![]; }
                let mut n = Node::new(node::EMPHASIS);
                for c in self.children { n = n.child(c); }
                vec![n]
            }
            InlineCtxKind::Underline => {
                if self.children.is_empty() { return vec![]; }
                let mut n = Node::new(node::UNDERLINE);
                for c in self.children { n = n.child(c); }
                vec![n]
            }
            InlineCtxKind::Strikeout => {
                if self.children.is_empty() { return vec![]; }
                let mut n = Node::new(node::STRIKEOUT);
                for c in self.children { n = n.child(c); }
                vec![n]
            }
            InlineCtxKind::Code => {
                if self.children.is_empty() { return vec![]; }
                // Flatten text content for code spans
                let content: String = self.children.iter()
                    .filter_map(|c| c.props.get_str(prop::CONTENT).map(str::to_owned))
                    .collect();
                vec![Node::new(node::CODE).prop(prop::CONTENT, content)]
            }
            InlineCtxKind::Subscript => {
                if self.children.is_empty() { return vec![]; }
                let mut n = Node::new(node::SUBSCRIPT);
                for c in self.children { n = n.child(c); }
                vec![n]
            }
            InlineCtxKind::Superscript => {
                if self.children.is_empty() { return vec![]; }
                let mut n = Node::new(node::SUPERSCRIPT);
                for c in self.children { n = n.child(c); }
                vec![n]
            }
            InlineCtxKind::Link { url } => {
                let mut n = Node::new(node::LINK).prop(prop::URL, url);
                for c in self.children { n = n.child(c); }
                vec![n]
            }
        }
    }
}

/// Append `nodes` into the top InlineCtx's children.
fn push_nodes_to_top(stack: &mut [InlineCtx], nodes: Vec<Node>) {
    if let Some(top) = stack.last_mut() {
        top.children.extend(nodes);
    }
}

/// Resolve a `text:style-name` to inline formatting kind(s), given automatic
/// and named style maps.
fn style_to_ctx_kind(
    style_name: &str,
    auto_styles: &HashMap<String, StyleEntry>,
    named_styles: &HashMap<String, StyleEntry>,
) -> Option<InlineCtxKind> {
    // Look up in auto_styles first, then named_styles
    let props = auto_styles.get(style_name)
        .or_else(|| named_styles.get(style_name))
        .map(|e| &e.text);

    if let Some(p) = props {
        // Return the most specific / first applicable kind.
        // Nested formatting (bold+italic) requires multiple push/pop cycles;
        // we handle only the "most prominent" here.  Real-world ODT rarely
        // nests multiple character properties inside a single span.
        if p.code { return Some(InlineCtxKind::Code); }
        if p.subscript { return Some(InlineCtxKind::Subscript); }
        if p.superscript { return Some(InlineCtxKind::Superscript); }
        if p.bold && p.italic {
            // Return strong; italic content inside will be a separate span in
            // most real-world documents.
            return Some(InlineCtxKind::Strong);
        }
        if p.bold { return Some(InlineCtxKind::Strong); }
        if p.italic { return Some(InlineCtxKind::Emphasis); }
        if p.underline { return Some(InlineCtxKind::Underline); }
        if p.strikeout { return Some(InlineCtxKind::Strikeout); }
    } else {
        // No style definition found; fall back to name heuristics
        let lower = style_name.to_lowercase();
        if lower.contains("code") || lower.contains("preformat") || lower.contains("verbatim") {
            return Some(InlineCtxKind::Code);
        }
        if lower.contains("subscript") || lower == "sub" { return Some(InlineCtxKind::Subscript); }
        if lower.contains("superscript") || lower == "sup" { return Some(InlineCtxKind::Superscript); }
        if lower.contains("bold") { return Some(InlineCtxKind::Strong); }
        if lower.contains("italic") || lower.contains("oblique") { return Some(InlineCtxKind::Emphasis); }
        if lower.contains("underline") { return Some(InlineCtxKind::Underline); }
        if lower.contains("strike") { return Some(InlineCtxKind::Strikeout); }
    }
    None
}

/// State for tracking where we are in the document.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ParsePhase {
    Init,
    InAutoStyles,
    InBody,
}

fn parse_content(
    xml: &str,
    named_styles: &HashMap<String, StyleEntry>,
) -> Result<Node, ParseError> {
    let mut reader = Reader::from_str(xml);
    // Do NOT trim_text: spaces between inline elements matter.
    reader.config_mut().trim_text(false);

    // First pass: collect automatic styles from content.xml
    let auto_styles = collect_auto_styles(xml);

    // Now do the real parse
    let mut doc = Node::new(node::DOCUMENT);
    let mut buf = Vec::new();
    let mut phase = ParsePhase::Init;

    // Block-level state
    let mut in_paragraph = false;
    let mut current_para_style = String::new();
    let mut para_depth: usize = 0;

    // Inline context stack: bottom = paragraph content, top = innermost formatting
    let mut inline_stack: Vec<InlineCtx> = Vec::new();

    // Note citation skipping
    let mut in_note_citation: usize = 0;  // depth counter

    // Table state
    let mut table_stack: Vec<Node> = Vec::new();  // outer tables
    let mut row_stack: Vec<Node> = Vec::new();
    let mut cell_stack: Vec<Node> = Vec::new();
    let mut in_table_cell = false;

    // List state
    // Each entry is (ordered: bool, list_node)
    let mut list_stack: Vec<(bool, Node)> = Vec::new();
    let mut in_list_item = false;

    // Blockquote accumulation (consecutive quotation paragraphs → single blockquote)
    let mut pending_blockquote: Option<Vec<Node>> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) => {
                if phase != ParsePhase::InBody {
                    buf.clear();
                    continue;
                }
                let name_bytes = e.name().as_ref().to_vec();
                let name = String::from_utf8_lossy(&name_bytes).to_string();
                match name.as_str() {
                    "text:line-break" if in_paragraph && in_note_citation == 0 => {
                        if let Some(top) = inline_stack.last_mut() {
                            top.flush_text();
                            top.children.push(Node::new(node::LINE_BREAK));
                        }
                    }
                    "text:tab" if in_paragraph && in_note_citation == 0 => {
                        if let Some(top) = inline_stack.last_mut() {
                            top.text.push('\t');
                        }
                    }
                    // Self-closing paragraph — treat as empty paragraph
                    "text:p" | "text:h" if !in_table_cell => {
                        let mut style = String::new();
                        for attr in e.attributes().flatten() {
                            if matches!(attr.key.as_ref(), b"text:style-name" | b"text:outline-level") {
                                style = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        let node = build_para_node(&style, vec![], name.as_str() == "text:h", &auto_styles, named_styles);
                        let kind = resolve_para_kind(&style, name.as_str() == "text:h", &auto_styles, named_styles);
                        if kind == ParaKind::Blockquote {
                            pending_blockquote.get_or_insert_with(Vec::new).push(node);
                        } else {
                            flush_pending_blockquote(&mut pending_blockquote, &mut doc);
                            doc = doc.child(node);
                        }
                    }
                    _ => {}
                }
                buf.clear();
                continue;
            }

            Ok(Event::Start(ref e)) => {
                let name_bytes = e.name().as_ref().to_vec();
                let name = String::from_utf8_lossy(&name_bytes).to_string();

                match name.as_str() {
                    "office:automatic-styles" => {
                        phase = ParsePhase::InAutoStyles;
                    }
                    "office:body" => {
                        phase = ParsePhase::InBody;
                    }
                    _ if phase != ParsePhase::InBody => {}

                    "text:p" | "text:h" if !in_table_cell || para_depth == 0 => {
                        // We allow text:p inside table cells (para_depth check handles nesting)
                        if para_depth == 0 {
                            in_paragraph = true;
                            current_para_style.clear();
                            inline_stack.clear();
                            inline_stack.push(InlineCtx::plain());
                            in_note_citation = 0;

                            for attr in e.attributes().flatten() {
                                match attr.key.as_ref() {
                                    b"text:style-name" | b"text:outline-level" => {
                                        current_para_style =
                                            String::from_utf8_lossy(&attr.value).to_string();
                                    }
                                    _ => {}
                                }
                            }
                            // Also read outline-level for text:h
                            if name == "text:h" {
                                for attr in e.attributes().flatten() {
                                    if attr.key.as_ref() == b"text:outline-level" {
                                        current_para_style =
                                            String::from_utf8_lossy(&attr.value).to_string();
                                    }
                                }
                            }
                        } else if in_paragraph
                            && let Some(top) = inline_stack.last_mut()
                            && !top.text.is_empty() && !top.text.ends_with(' ') {
                            // Inner paragraph (e.g. footnote body): add a space
                            top.text.push(' ');
                        }
                        para_depth += 1;
                    }

                    "text:p" | "text:h" if in_table_cell => {
                        // Paragraph inside a table cell
                        if para_depth == 0 {
                            in_paragraph = true;
                            current_para_style.clear();
                            inline_stack.clear();
                            inline_stack.push(InlineCtx::plain());
                            in_note_citation = 0;
                        }
                        para_depth += 1;
                    }

                    "text:span" if in_paragraph && in_note_citation == 0 => {
                        let mut style_name = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"text:style-name" {
                                style_name = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        // Flush current text before opening new span
                        if let Some(top) = inline_stack.last_mut() {
                            top.flush_text();
                        }
                        let kind = style_to_ctx_kind(&style_name, &auto_styles, named_styles);
                        inline_stack.push(InlineCtx {
                            kind: kind.unwrap_or(InlineCtxKind::Plain),
                            children: Vec::new(),
                            text: String::new(),
                        });
                    }

                    "text:a" if in_paragraph && in_note_citation == 0 => {
                        let mut href = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"xlink:href" {
                                href = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        if let Some(top) = inline_stack.last_mut() {
                            top.flush_text();
                        }
                        inline_stack.push(InlineCtx {
                            kind: InlineCtxKind::Link { url: href },
                            children: Vec::new(),
                            text: String::new(),
                        });
                    }

                    "text:note-citation" => {
                        in_note_citation += 1;
                    }

                    "text:list" => {
                        let mut list_style_name = String::new();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"text:style-name" {
                                list_style_name = String::from_utf8_lossy(&attr.value).to_string();
                            }
                        }
                        let ordered = is_ordered_list(&list_style_name, &auto_styles, named_styles);
                        list_stack.push((ordered, Node::new(node::LIST)));
                    }

                    "text:list-item" => {
                        in_list_item = true;
                    }

                    "table:table" => {
                        table_stack.push(Node::new(node::TABLE));
                    }

                    "table:table-row" | "table:table-header-rows" => {
                        row_stack.push(Node::new(node::TABLE_ROW));
                    }

                    "table:table-cell" | "table:covered-table-cell" => {
                        in_table_cell = true;
                        cell_stack.push(Node::new(node::TABLE_CELL));
                        para_depth = 0;
                        in_paragraph = false;
                        inline_stack.clear();
                    }

                    _ => {}
                }
            }



            Ok(Event::End(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                match name.as_str() {
                    "office:automatic-styles" if phase == ParsePhase::InAutoStyles => {
                        phase = ParsePhase::Init; // wait for office:body
                    }

                    "text:p" | "text:h" => {
                        para_depth = para_depth.saturating_sub(1);
                        if para_depth == 0 && in_paragraph {
                            // Close the paragraph: flatten the inline stack
                            let children = flatten_inline_stack(&mut inline_stack);

                            let node = build_para_node(
                                &current_para_style,
                                children,
                                name.as_str() == "text:h",
                                &auto_styles,
                                named_styles,
                            );

                            // Dispatch: table cell, list item, or document
                            if in_table_cell {
                                if let Some(cell) = cell_stack.last_mut() {
                                    *cell = cell.clone().child(node);
                                }
                            } else if in_list_item {
                                // We'll attach to list item at text:list-item close
                                // Push to a side buffer via list_item_content
                                // For now: attach to the list item directly
                                attach_to_list_item_buf(&mut list_stack, node);
                            } else {
                                let kind = resolve_para_kind(
                                    &current_para_style,
                                    name.as_str() == "text:h",
                                    &auto_styles,
                                    named_styles,
                                );
                                if kind == ParaKind::Blockquote {
                                    pending_blockquote.get_or_insert_with(Vec::new).push(node);
                                } else {
                                    flush_pending_blockquote(&mut pending_blockquote, &mut doc);
                                    doc = doc.child(node);
                                }
                            }

                            in_paragraph = false;
                            inline_stack.clear();
                            current_para_style.clear();
                        }
                    }

                    "text:span" if !inline_stack.is_empty() && in_paragraph && in_note_citation == 0 => {
                        // Pop the span context and push its result into the parent
                        let ctx = inline_stack.pop().unwrap();
                        let nodes = ctx.into_node();
                        push_nodes_to_top(&mut inline_stack, nodes);
                    }

                    "text:a" if !inline_stack.is_empty() && in_paragraph && in_note_citation == 0 => {
                        let ctx = inline_stack.pop().unwrap();
                        let nodes = ctx.into_node();
                        push_nodes_to_top(&mut inline_stack, nodes);
                    }

                    "text:note-citation" if in_note_citation > 0 => {
                        in_note_citation -= 1;
                    }

                    "text:list-item" => {
                        in_list_item = false;
                        // The items were already attached via attach_to_list_item_buf
                    }

                    "text:list" => {
                        if let Some((ordered, mut list_node)) = list_stack.pop() {
                            if ordered {
                                list_node = list_node.prop("ordered", true);
                            }
                            if list_stack.is_empty() {
                                flush_pending_blockquote(&mut pending_blockquote, &mut doc);
                                doc = doc.child(list_node);
                            } else if let Some((_, parent_list)) = list_stack.last_mut() {
                                // Nested list: append inside the last list_item of the parent
                                if let Some(last_item) = parent_list.children.last_mut()
                                    && last_item.kind.as_str() == node::LIST_ITEM
                                {
                                    *last_item = last_item.clone().child(list_node);
                                } else {
                                    // No list_item yet (unusual): append directly
                                    *parent_list = parent_list.clone().child(list_node);
                                }
                            }
                        }
                    }

                    "table:table-cell" | "table:covered-table-cell" => {
                        in_table_cell = !cell_stack.is_empty() && cell_stack.len() > 1;
                        if let Some(cell) = cell_stack.pop()
                            && let Some(row) = row_stack.last_mut() {
                            *row = row.clone().child(cell);
                        }
                        if cell_stack.is_empty() { in_table_cell = false; }
                    }

                    "table:table-row" => {
                        if let Some(row) = row_stack.pop()
                            && let Some(table) = table_stack.last_mut() {
                            *table = table.clone().child(row);
                        }
                    }

                    "table:table" => {
                        if let Some(table) = table_stack.pop() {
                            flush_pending_blockquote(&mut pending_blockquote, &mut doc);
                            doc = doc.child(table);
                        }
                    }

                    _ => {}
                }
            }

            Ok(Event::Text(ref e)) => {
                if in_paragraph && in_note_citation == 0 && para_depth > 0 {
                    let text = String::from_utf8_lossy(e.as_ref()).to_string();
                    if let Some(top) = inline_stack.last_mut() {
                        top.text.push_str(&text);
                    }
                }
            }

            Ok(Event::GeneralRef(ref e)) => {
                if in_paragraph && in_note_citation == 0 && para_depth > 0
                    && let Some(ch) = decode_general_ref(e.as_ref())
                    && let Some(top) = inline_stack.last_mut() {
                    top.text.push(ch);
                }
            }

            Ok(Event::Eof) => break,
            Err(e) => return Err(ParseError::Invalid(format!("XML error: {}", e))),
            _ => {}
        }
        buf.clear();
    }

    flush_pending_blockquote(&mut pending_blockquote, &mut doc);

    Ok(doc)
}

/// Flatten the inline stack into a list of children nodes.
/// All open contexts are closed in order (handles cases where spans weren't explicitly closed).
fn flatten_inline_stack(stack: &mut Vec<InlineCtx>) -> Vec<Node> {
    if stack.is_empty() {
        return Vec::new();
    }
    // Close from innermost outward
    while stack.len() > 1 {
        let ctx = stack.pop().unwrap();
        let nodes = ctx.into_node();
        push_nodes_to_top(stack, nodes);
    }
    let mut top = stack.pop().unwrap();
    top.flush_text();
    top.children
}

/// Build the paragraph/heading/code_block/etc. node from its children.
fn build_para_node(
    style: &str,
    children: Vec<Node>,
    is_heading_tag: bool,
    auto_styles: &HashMap<String, StyleEntry>,
    named_styles: &HashMap<String, StyleEntry>,
) -> Node {
    let kind = resolve_para_kind(style, is_heading_tag, auto_styles, named_styles);

    match kind {
        ParaKind::Heading(level) => {
            let mut n = Node::new(node::HEADING).prop(prop::LEVEL, level as i64);
            for c in children { n = n.child(c); }
            n
        }
        ParaKind::Code => {
            // Flatten all text content for code blocks
            let content: String = children.iter()
                .flat_map(extract_text_content)
                .collect::<Vec<_>>()
                .join("");
            Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content)
        }
        ParaKind::Blockquote => {
            // Wrap in a paragraph inside the blockquote
            let mut para = Node::new(node::PARAGRAPH);
            for c in children { para = para.child(c); }
            para // caller will wrap in blockquote
        }
        ParaKind::Normal => {
            let mut n = Node::new(node::PARAGRAPH);
            for c in children { n = n.child(c); }
            n
        }
    }
}

fn extract_text_content(node: &Node) -> Vec<String> {
    let mut result = Vec::new();
    if node.kind.as_str() == node::TEXT {
        if let Some(c) = node.props.get_str(prop::CONTENT) {
            result.push(c.to_owned());
        }
    } else if node.kind.as_str() == node::LINE_BREAK {
        result.push("\n".to_owned());
    }
    for child in &node.children {
        result.extend(extract_text_content(child));
    }
    result
}

fn resolve_para_kind(
    style: &str,
    is_heading_tag: bool,
    auto_styles: &HashMap<String, StyleEntry>,
    named_styles: &HashMap<String, StyleEntry>,
) -> ParaKind {
    if style.is_empty() {
        if is_heading_tag { return ParaKind::Heading(1); }
        return ParaKind::Normal;
    }

    // text:h always → Heading
    if is_heading_tag {
        // style is the outline-level number for text:h
        if let Ok(level) = style.parse::<u8>() {
            return ParaKind::Heading(level.min(6));
        }
        return ParaKind::Heading(1);
    }

    // Look up in auto/named styles
    let entry = auto_styles.get(style).or_else(|| named_styles.get(style));
    if let Some(e) = entry
        && e.para_kind != ParaKind::Normal {
        return e.para_kind;
    }

    // Heuristic fallback on style name
    let lower = style.to_lowercase();
    if lower.starts_with("heading") {
        let suffix = lower.trim_start_matches("heading").trim();
        if let Some(num) = suffix.chars().next().filter(|c| c.is_ascii_digit()) {
            let level = (num as u8 - b'0').min(6);
            return ParaKind::Heading(level);
        }
        return ParaKind::Heading(1);
    }
    if lower.contains("preformat") || lower.contains("code") || lower.contains("monospace") || lower.contains("verbatim") || lower == "source text" {
        return ParaKind::Code;
    }
    if lower.contains("quotation") || lower.contains("blockquote") || lower.contains("quote") {
        return ParaKind::Blockquote;
    }

    ParaKind::Normal
}

fn is_ordered_list(
    style_name: &str,
    auto_styles: &HashMap<String, StyleEntry>,
    named_styles: &HashMap<String, StyleEntry>,
) -> bool {
    if style_name.is_empty() { return false; }

    let entry = auto_styles.get(style_name).or_else(|| named_styles.get(style_name));
    if let Some(e) = entry
        && let Some(kind) = e.list_level {
        return kind == ListLevelKind::Number;
    }

    // Heuristic fallback
    let lower = style_name.to_lowercase();
    lower.contains("numb") || lower.contains("order") || lower.contains("decimal")
        || lower == "list number" || lower == "list_number"
}

/// Collect automatic styles from content.xml (character + list styles).
fn collect_auto_styles(xml: &str) -> HashMap<String, StyleEntry> {
    // Reuse the same parser as styles.xml
    parse_styles_from_xml(xml)
}

/// Attach a paragraph node to the current list item.
fn attach_to_list_item_buf(list_stack: &mut [(bool, Node)], para: Node) {
    if let Some((_, list_node)) = list_stack.last_mut() {
        // Find or create last list_item in the list
        let item = Node::new(node::LIST_ITEM).child(para);
        *list_node = list_node.clone().child(item);
    }
}

fn flush_pending_blockquote(pending: &mut Option<Vec<Node>>, doc: &mut Node) {
    if let Some(paras) = pending.take()
        && !paras.is_empty() {
        let mut bq = Node::new(node::BLOCKQUOTE);
        for p in paras { bq = bq.child(p); }
        *doc = doc.clone().child(bq);
    }
}

// ── XML entity / character reference decoding ─────────────────────────────────

/// Decode a `GeneralRef` event (the content between `&` and `;`) to a char.
///
/// Handles:
/// - Decimal numeric refs: `#160` → U+00A0
/// - Hex numeric refs: `#xa0` or `#xA0` → U+00A0
/// - Named XML entities: `amp`, `lt`, `gt`, `apos`, `quot`
fn decode_general_ref(content: &[u8]) -> Option<char> {
    let s = std::str::from_utf8(content).ok()?;
    if let Some(dec) = s.strip_prefix('#').and_then(|r| {
        if r.starts_with(['x', 'X']) {
            u32::from_str_radix(&r[1..], 16).ok()
        } else {
            r.parse::<u32>().ok()
        }
    }) {
        char::from_u32(dec)
    } else {
        match s {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "apos" => Some('\''),
            "quot" => Some('"'),
            _ => None,
        }
    }
}

// ── XML escape ─────────────────────────────────────────────────────────────────

#[allow(dead_code)]
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    fn create_test_odt(content_xml: &str) -> Vec<u8> {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buffer);
            let options = SimpleFileOptions::default();

            zip.start_file("mimetype", options).unwrap();
            zip.write_all(b"application/vnd.oasis.opendocument.text").unwrap();

            zip.start_file("content.xml", options).unwrap();
            zip.write_all(content_xml.as_bytes()).unwrap();

            zip.finish().unwrap();
        }
        buffer.into_inner()
    }

    fn ns() -> &'static str {
        r#"xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
           xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0"
           xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0"
           xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0"
           xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
           xmlns:xlink="http://www.w3.org/1999/xlink""#
    }

    fn body(content: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content {ns}>
  <office:body>
    <office:text>
      {content}
    </office:text>
  </office:body>
</office:document-content>"#,
            ns = ns(),
            content = content
        )
    }

    fn body_with_styles(auto_styles: &str, content: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content {ns}>
  <office:automatic-styles>
    {auto_styles}
  </office:automatic-styles>
  <office:body>
    <office:text>
      {content}
    </office:text>
  </office:body>
</office:document-content>"#,
            ns = ns(),
            auto_styles = auto_styles,
            content = content
        )
    }

    #[test]
    fn test_parse_basic() {
        let odt = create_test_odt(&body("<text:p>Hello world</text:p>"));
        let result = parse(&odt).unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_heading() {
        let odt = create_test_odt(&body(r#"<text:h text:outline-level="1">Title</text:h>"#));
        let result = parse(&odt).unwrap();
        let heading = &result.value.content.children[0];
        assert_eq!(heading.kind.as_str(), node::HEADING);
        assert_eq!(heading.props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_bold_named_style() {
        let xml = body(r#"<text:p>Some <text:span text:style-name="Bold">bold</text:span> text.</text:p>"#);
        let odt = create_test_odt(&xml);
        let result = parse(&odt).unwrap();
        let para = &result.value.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
        // Children: TEXT("Some "), STRONG, TEXT(" text.")
        let strong = para.children.iter().find(|c| c.kind.as_str() == node::STRONG);
        assert!(strong.is_some(), "should have a strong node");
    }

    #[test]
    fn test_parse_bold_auto_style() {
        let auto = r#"<style:style style:name="T1" style:family="text">
            <style:text-properties fo:font-weight="bold"/>
        </style:style>"#;
        let xml = body_with_styles(
            auto,
            r#"<text:p>Some <text:span text:style-name="T1">bold</text:span> text.</text:p>"#,
        );
        let odt = create_test_odt(&xml);
        let result = parse(&odt).unwrap();
        let para = &result.value.content.children[0];
        let strong = para.children.iter().find(|c| c.kind.as_str() == node::STRONG);
        assert!(strong.is_some(), "auto-style T1 with fo:font-weight=bold should produce strong node");
    }

    #[test]
    fn test_parse_italic() {
        let xml = body(r#"<text:p><text:span text:style-name="Italic">italic</text:span></text:p>"#);
        let odt = create_test_odt(&xml);
        let result = parse(&odt).unwrap();
        let para = &result.value.content.children[0];
        let em = para.children.iter().find(|c| c.kind.as_str() == node::EMPHASIS);
        assert!(em.is_some(), "should have an emphasis node");
    }

    #[test]
    fn test_parse_hyperlink() {
        let xml = body(r#"<text:p><text:a xlink:type="simple" xlink:href="https://example.com">link text</text:a></text:p>"#);
        let odt = create_test_odt(&xml);
        let result = parse(&odt).unwrap();
        let para = &result.value.content.children[0];
        let link = para.children.iter().find(|c| c.kind.as_str() == node::LINK);
        assert!(link.is_some(), "should have a link node");
        assert_eq!(link.unwrap().props.get_str(prop::URL), Some("https://example.com"));
    }

    #[test]
    fn test_parse_table() {
        let xml = body(r#"
        <table:table>
          <table:table-row>
            <table:table-cell><text:p>Cell 1</text:p></table:table-cell>
            <table:table-cell><text:p>Cell 2</text:p></table:table-cell>
          </table:table-row>
        </table:table>"#);
        let odt = create_test_odt(&xml);
        let result = parse(&odt).unwrap();
        let table = &result.value.content.children[0];
        assert_eq!(table.kind.as_str(), node::TABLE);
        assert_eq!(table.children.len(), 1); // one row
        let row = &table.children[0];
        assert_eq!(row.kind.as_str(), node::TABLE_ROW);
        assert_eq!(row.children.len(), 2); // two cells
    }

    #[test]
    fn test_parse_ordered_list() {
        let auto = r#"<text:list-style style:name="L1">
            <text:list-level-style-number text:level="1" style:num-format="1"/>
        </text:list-style>"#;
        let xml = body_with_styles(auto, r#"
        <text:list text:style-name="L1">
          <text:list-item><text:p>one</text:p></text:list-item>
          <text:list-item><text:p>two</text:p></text:list-item>
        </text:list>"#);
        let odt = create_test_odt(&xml);
        let result = parse(&odt).unwrap();
        let list = &result.value.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool("ordered"), Some(true));
    }
}

