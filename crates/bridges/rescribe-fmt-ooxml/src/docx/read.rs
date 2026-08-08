use ooxml_wml::Document as OoxmlDocument;
use ooxml_wml::ext::{
    CellExt, DrawingExt, HyperlinkExt, ParagraphExt, RowExt, RunExt, RunPropertiesExt, TableExt,
};
use ooxml_wml::parse_numbering_levels;
use ooxml_wml::types::{
    BlockContent, BlockContentChoice, FootnoteEndnote, Hyperlink, Paragraph, ParagraphContent, Run,
    RunContent, RunContentChoice, STJc, Table,
};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, Properties, Resource,
    ResourceId, ResourceMap, Severity, SourceInfo, WarningKind,
};
use rescribe_std::{node, prop};
use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::Path;

/// Parse a DOCX file from a path.
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<ConversionResult<Document>, ParseError> {
    let doc = OoxmlDocument::open(path)
        .map_err(|e| ParseError::Invalid(format!("Failed to open DOCX: {}", e)))?;
    convert_document(doc)
}

/// Parse DOCX from a reader that implements Read + Seek.
pub fn parse<R: Read + Seek>(reader: R) -> Result<ConversionResult<Document>, ParseError> {
    let doc = OoxmlDocument::from_reader(reader)
        .map_err(|e| ParseError::Invalid(format!("Failed to parse DOCX: {}", e)))?;
    convert_document(doc)
}

/// Parse DOCX from bytes.
pub fn parse_bytes(bytes: &[u8]) -> Result<ConversionResult<Document>, ParseError> {
    let cursor = std::io::Cursor::new(bytes);
    parse(cursor)
}

/// Converter state for tracking resources and warnings during conversion.
struct Converter {
    warnings: Vec<FidelityWarning>,
    resources: ResourceMap,
    /// Footnote content keyed by footnote id, for inline lookup.
    footnotes: HashMap<i64, Vec<Node>>,
    /// Endnote content keyed by endnote id, for inline lookup.
    endnotes: HashMap<i64, Vec<Node>>,
    /// Maps num_id → (ilvl → is_ordered); true = decimal/numbered, false = bullet.
    /// Keyed per-level because sibling levels of one numbering instance can mix
    /// bulleted and numbered formatting.
    numbering_levels: HashMap<i64, HashMap<i64, bool>>,
    /// State of a currently-open complex field (`<w:fldChar>`/`<w:instrText>`
    /// sequence), threaded across consecutive runs within one paragraph
    /// (see `convert_run`'s field-code handling).
    field_state: FieldPhase,
}

/// Tracks progress through a DOCX complex-field run sequence:
/// `fldChar[begin]` (→ `Instr`) → one or more `instrText` runs (accumulated
/// into `Instr`'s string) → `fldChar[separate]` (→ `Display`) → the field's
/// visible result content (accumulated into `Display`'s node buffer) →
/// `fldChar[end]` (finalizes the field node and resets to `None`).
#[derive(Default)]
enum FieldPhase {
    #[default]
    None,
    Instr(String),
    Display(String, Vec<Node>),
}

impl Converter {
    fn new() -> Self {
        Self {
            warnings: Vec::new(),
            resources: ResourceMap::new(),
            footnotes: HashMap::new(),
            endnotes: HashMap::new(),
            numbering_levels: HashMap::new(),
            field_state: FieldPhase::None,
        }
    }

    fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::FeatureLost("docx".to_string()),
            message,
        ));
    }

    fn warn_lost(&mut self, message: impl Into<String>) {
        self.warnings.push(FidelityWarning::new(
            Severity::Major,
            WarningKind::FeatureLost("docx".to_string()),
            message,
        ));
    }

    fn add_resource(&mut self, data: Vec<u8>, content_type: &str) -> ResourceId {
        let id = ResourceId::new();
        let resource = Resource::new(content_type.to_string(), data);
        self.resources.insert(id.clone(), resource);
        id
    }
}

fn convert_document<R: Read + Seek>(
    mut doc: OoxmlDocument<R>,
) -> Result<ConversionResult<Document>, ParseError> {
    let mut converter = Converter::new();

    // Pre-load footnotes into converter state so convert_run can look them up.
    if let Ok(footnotes) = doc.get_footnotes() {
        let footnote_data: Vec<(i64, Vec<BlockContent>)> = footnotes
            .footnote
            .into_iter()
            .filter(|f| f.id > 0) // skip separator footnotes (id <= 0)
            .map(|f: FootnoteEndnote| (f.id, f.block_content))
            .collect();

        for (id, block_content) in footnote_data {
            let fn_node = convert_body_content(&mut converter, &mut doc, &block_content)?;
            converter.footnotes.insert(id, fn_node.children);
        }
    }

    // Pre-load endnotes into converter state.
    if let Ok(endnotes) = doc.get_endnotes() {
        let endnote_data: Vec<(i64, Vec<BlockContent>)> = endnotes
            .endnote
            .into_iter()
            .filter(|e| e.id > 0) // skip separator endnotes (id <= 0)
            .map(|e: FootnoteEndnote| (e.id, e.block_content))
            .collect();

        for (id, block_content) in endnote_data {
            let en_node = convert_body_content(&mut converter, &mut doc, &block_content)?;
            converter.endnotes.insert(id, en_node.children);
        }
    }

    // Pre-load numbering definitions to determine ordered vs unordered lists.
    if let Ok(xml) = doc.package_mut().read_part("word/numbering.xml") {
        converter.numbering_levels = parse_numbering_levels(&xml);
    } else {
        // Try via document relationships (numbering.xml may be at a non-default path).
        // Collect the path first to avoid holding an immutable borrow on `doc`.
        let numbering_path = doc.doc_relationships().iter().find_map(|rel| {
            if rel.relationship_type.contains("numbering") {
                let path = if rel.target.starts_with('/') {
                    rel.target.trim_start_matches('/').to_string()
                } else {
                    format!("word/{}", rel.target)
                };
                Some(path)
            } else {
                None
            }
        });
        if let Some(path) = numbering_path
            && let Ok(xml) = doc.package_mut().read_part(&path)
        {
            converter.numbering_levels = parse_numbering_levels(&xml);
        }
    }

    // Clone the body content to avoid borrow issues
    let body_content = doc.body().block_content.clone();

    // Convert body content
    let content = convert_body(&mut converter, &mut doc, &body_content)?;

    // Extract metadata
    let metadata = extract_metadata(&doc);

    // Build the final document
    let document = Document {
        content,
        resources: converter.resources,
        metadata,
        source: Some(SourceInfo {
            format: "docx".to_string(),
            metadata: Properties::new(),
        }),
    };

    Ok(ConversionResult {
        value: document,
        warnings: converter.warnings,
    })
}

/// Convert a slice of BlockContent into a document-level node.
fn convert_body_content<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    content: &[BlockContent],
) -> Result<Node, ParseError> {
    let mut children = Vec::new();
    convert_block_content_into(converter, doc, content, &mut children)?;
    Ok(Node::new(node::DOCUMENT).children(children))
}

/// One level of a currently-open (not yet closed) nested list, keyed by
/// `(num_id, ilvl)`. Frames are stacked outermost-first; a paragraph whose
/// `ilvl` increases relative to the top of the stack opens a new frame, one
/// whose `ilvl` decreases closes frames (folding each closed frame's `list`
/// node into the last item of its new parent frame, or into `out` if there
/// is no parent), and one at the same `(num_id, ilvl)` as the top frame
/// appends to it.
struct ListFrame {
    num_id: i64,
    ilvl: i64,
    ordered: bool,
    items: Vec<Node>,
}

/// Inner helper: push converted nodes into `out`, grouping list paragraphs
/// into properly nested `list`/`list_item` structures keyed on `(numId,
/// ilvl)` rather than `numId` alone -- a paragraph's `ilvl` (indent level)
/// determines nesting depth within one numbering instance; grouping on
/// `numId` alone (the previous behavior) flattened every level into one
/// list, silently dropping the nesting structure.
fn convert_block_content_into<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    content: &[BlockContent],
    out: &mut Vec<Node>,
) -> Result<(), ParseError> {
    let mut list_stack: Vec<ListFrame> = Vec::new();

    for block in content {
        match block {
            BlockContent::P(para) => {
                // Check for list membership
                if let Some((num_id, ilvl)) = para.numbering() {
                    let item_children = convert_paragraph_content(converter, doc, para)?;
                    // Raw-preserve the source `<w:numPr>` (numId + ilvl) on the
                    // item itself. The `list`/`list_item` nesting above already
                    // captures ilvl *structurally*; these mirror the exact
                    // source integers separately, since a numbering instance
                    // can jump levels (e.g. start directly at ilvl=2) in ways
                    // the writer's fresh depth-from-nesting count wouldn't
                    // otherwise reconstruct -- see write.rs's `write_list_items`,
                    // which prefers `docx:ilvl` when present.
                    let item = Node::new(node::LIST_ITEM)
                        .prop("docx:num-id", num_id)
                        .prop("docx:ilvl", ilvl)
                        .children(item_children);
                    let is_ordered = converter
                        .numbering_levels
                        .get(&num_id)
                        .and_then(|levels| levels.get(&ilvl))
                        .copied()
                        .unwrap_or(false);
                    push_list_item(&mut list_stack, out, num_id, ilvl, is_ordered, item);
                } else {
                    flush_list_stack(&mut list_stack, out);
                    if let Some(n) = convert_paragraph(converter, doc, para)? {
                        out.push(n);
                    }
                }
            }
            BlockContent::Tbl(table) => {
                flush_list_stack(&mut list_stack, out);
                out.push(convert_table(converter, doc, table)?);
            }
            BlockContent::Sdt(ctrl) => {
                flush_list_stack(&mut list_stack, out);
                if let Some(n) = convert_sdt_block(converter, doc, ctrl)? {
                    out.push(n);
                }
            }
            BlockContent::CustomXml(xml) => {
                flush_list_stack(&mut list_stack, out);
                for inner_block in &xml.block_content {
                    match inner_block {
                        BlockContentChoice::P(para) => {
                            if let Some(n) = convert_paragraph(converter, doc, para)? {
                                out.push(n);
                            }
                        }
                        BlockContentChoice::Tbl(table) => {
                            out.push(convert_table(converter, doc, table)?);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    flush_list_stack(&mut list_stack, out);
    Ok(())
}

/// Close a `ListFrame`, folding its finished `list` node into the last item
/// of the new stack top (nested case) or pushing it to `out` (top-level).
fn close_list_frame(stack: &mut [ListFrame], out: &mut Vec<Node>, frame: ListFrame) {
    let list_node = Node::new(node::LIST)
        .prop(prop::ORDERED, frame.ordered)
        .children(frame.items);
    match stack.last_mut() {
        Some(parent) => match parent.items.last_mut() {
            Some(last_item) => last_item.children.push(list_node),
            // Defensive: a parent frame with no items yet (shouldn't happen
            // for well-formed numbering, but avoids losing the nested list).
            None => parent
                .items
                .push(Node::new(node::LIST_ITEM).child(list_node)),
        },
        None => out.push(list_node),
    }
}

/// Push one list-item paragraph into the nesting stack, closing any frames
/// deeper than (or a different `numId` at the same depth as) the incoming
/// `ilvl`, then either appending to the matching frame or opening a new one.
fn push_list_item(
    stack: &mut Vec<ListFrame>,
    out: &mut Vec<Node>,
    num_id: i64,
    ilvl: i64,
    ordered: bool,
    item: Node,
) {
    while stack
        .last()
        .is_some_and(|f| f.ilvl > ilvl || (f.ilvl == ilvl && f.num_id != num_id))
    {
        let frame = stack.pop().unwrap();
        close_list_frame(stack, out, frame);
    }

    if let Some(top) = stack.last_mut()
        && top.ilvl == ilvl
        && top.num_id == num_id
    {
        top.items.push(item);
    } else {
        stack.push(ListFrame {
            num_id,
            ilvl,
            ordered,
            items: vec![item],
        });
    }
}

/// Close every remaining frame in the nesting stack, in depth order.
fn flush_list_stack(stack: &mut Vec<ListFrame>, out: &mut Vec<Node>) {
    while let Some(frame) = stack.pop() {
        close_list_frame(stack, out, frame);
    }
}

fn convert_body<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    content: &[BlockContent],
) -> Result<Node, ParseError> {
    let mut children = Vec::new();
    convert_block_content_into(converter, doc, content, &mut children)?;
    Ok(Node::new(node::DOCUMENT).children(children))
}

fn convert_paragraph<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    para: &Paragraph,
) -> Result<Option<Node>, ParseError> {
    // Horizontal rule: an otherwise-empty paragraph whose only content is a
    // bottom paragraph border (`<w:pBdr><w:bottom>`), Word's own convention
    // for "Insert Horizontal Line" (no visible text, so it must be detected
    // before the empty-paragraph short-circuit below drops it).
    if let Some(node) = detect_horizontal_rule(para) {
        return Ok(Some(node));
    }

    // Check if this is a heading
    let heading_level = detect_heading_level(para);

    // Convert paragraph content
    let inline_children = convert_paragraph_content(converter, doc, para)?;

    // Skip empty paragraphs (unless they have special meaning)
    if inline_children.is_empty() {
        return Ok(None);
    }

    if let Some(level) = heading_level {
        let node = Node::new(node::HEADING)
            .prop(prop::LEVEL, level as i64)
            .children(inline_children);
        Ok(Some(apply_para_layout_props(node, para)))
    } else if let Some(style) = code_block_style(para) {
        // Code block: paragraph style is one of a known set of DOCX
        // monospace/preformatted style IDs (see `code_block_style`'s doc
        // comment for why this is style-based, not font-based -- a
        // font-based heuristic would misclassify `inline_font_name`, which
        // sets a monospace run font with no paragraph style at all).
        if inline_children
            .iter()
            .any(|n| n.kind.as_str() != node::TEXT)
        {
            converter.warn(
                "code_block content had inline formatting (bold/italic/etc.); flattened to plain text",
            );
        }
        let mut text = String::new();
        flatten_inline_text(&inline_children, &mut text);
        let node = Node::new(node::CODE_BLOCK)
            .prop(prop::CONTENT, text)
            .prop("docx:pStyle", style);
        Ok(Some(apply_para_layout_props(node, para)))
    } else if is_blockquote_para(para) {
        // Blockquote: paragraph indented on both sides beyond the threshold
        // (see `is_blockquote_para`'s doc comment for why this is
        // indentation-based, not style-name-based -- a style-name heuristic
        // matching "Quote" would misclassify `para_style`, which uses
        // pStyle="Quote" with no indentation to test raw `docx:pStyle`
        // preservation on a plain paragraph).
        let mut inner = Node::new(node::PARAGRAPH).children(inline_children);
        if let Some(align) = para.alignment() {
            let align_str = match align {
                STJc::Left | STJc::Start => "left",
                STJc::Right | STJc::End => "right",
                STJc::Center => "center",
                STJc::Both => "justify",
                _ => "",
            };
            if !align_str.is_empty() {
                inner = inner.prop(prop::STYLE_ALIGN, align_str.to_string());
            }
        }
        let node = Node::new(node::BLOCKQUOTE).child(inner);
        Ok(Some(apply_para_layout_props(node, para)))
    } else {
        let mut node = Node::new(node::PARAGRAPH).children(inline_children);

        // Apply paragraph alignment
        if let Some(align) = para.alignment() {
            let align_str = match align {
                STJc::Left | STJc::Start => "left",
                STJc::Right | STJc::End => "right",
                STJc::Center => "center",
                STJc::Both => "justify",
                _ => "",
            };
            if !align_str.is_empty() {
                node = node.prop(prop::STYLE_ALIGN, align_str.to_string());
            }
        }

        Ok(Some(apply_para_layout_props(node, para)))
    }
}

/// Recursively concatenate all `text` node content, joining sibling `text`
/// nodes with no separator (mirrors write.rs's `flatten_text` used for
/// tracked-change round-tripping).
fn flatten_inline_text(nodes: &[Node], out: &mut String) {
    for node in nodes {
        if node.kind.as_str() == node::TEXT {
            if let Some(s) = node.props.get_str(prop::CONTENT) {
                out.push_str(s);
            }
        } else {
            flatten_inline_text(&node.children, out);
        }
    }
}

/// Detect a horizontal-rule paragraph: no run content, and a paragraph
/// border with a `bottom` side set (Word's "Insert Horizontal Line" writes
/// exactly this -- an empty paragraph with `<w:pBdr><w:bottom .../></w:pBdr>`
/// and no `<w:r>` children).
fn detect_horizontal_rule(para: &Paragraph) -> Option<Node> {
    if !para.paragraph_content.is_empty() {
        return None;
    }
    let props = para.properties()?;
    let bdr = props.paragraph_border.as_deref()?;
    let bottom = bdr.bottom.as_deref()?;
    let mut node = Node::new(node::HORIZONTAL_RULE);
    node = apply_border_prop(node, "hr", "bottom", Some(bottom));
    Some(node)
}

/// Known DOCX paragraph-style IDs used for preformatted/monospace content:
/// `HTMLPreformatted` is Word's own built-in style for `<pre>` content
/// pasted/imported from HTML; `Code`/`SourceCode`/`MacroText` are common
/// third-party-template conventions. Deliberately style-*name*-based (not
/// run-font-based -- see the caller's doc comment for why) and deliberately
/// a narrow allowlist (not a substring match on "code" alone, which would
/// over-match style names like "CodeText" used for inline code spans, or
/// unrelated custom styles).
fn code_block_style(para: &Paragraph) -> Option<String> {
    let style = &para.properties()?.paragraph_style.as_deref()?.value;
    let lower = style.to_lowercase();
    if matches!(
        lower.as_str(),
        "htmlpreformatted" | "code" | "sourcecode" | "macrotext" | "codeblock" | "preformatted"
    ) {
        Some(style.clone())
    } else {
        None
    }
}

/// A paragraph indented at least `BLOCKQUOTE_INDENT_THRESHOLD_TWIPS` on
/// *both* left and right sides is treated as a blockquote. Both-sides
/// symmetric indentation is specific enough to Word's "Quote"/"Intense
/// Quote" built-in styles (each indents 720 twips = 0.5in on both sides)
/// to avoid false-positiving on single-side-indented paragraphs used for
/// other reasons (nested outline levels, hanging indents, etc.).
const BLOCKQUOTE_INDENT_THRESHOLD_TWIPS: i64 = 720;

fn is_blockquote_para(para: &Paragraph) -> bool {
    let left = para.indent_left().unwrap_or(0);
    let right = para.indent_right().unwrap_or(0);
    left >= BLOCKQUOTE_INDENT_THRESHOLD_TWIPS && right >= BLOCKQUOTE_INDENT_THRESHOLD_TWIPS
}

/// Attach format-specific paragraph layout properties (`docx:*`) to a node.
///
/// These mirror the `rtf:para-props` pattern: format-specific constructs that have
/// no cross-format semantic go into namespaced properties so a DOCX writer can
/// re-emit them verbatim on roundtrip.
fn apply_para_layout_props(mut node: Node, para: &Paragraph) -> Node {
    if let Some(v) = para.space_before() {
        node = node.prop("docx:space-before", v);
    }
    if let Some(v) = para.space_after() {
        node = node.prop("docx:space-after", v);
    }
    if let Some(v) = para.line_spacing() {
        node = node.prop("docx:line-spacing", v);
    }
    if let Some(v) = para.line_spacing_rule() {
        node = node.prop("docx:line-spacing-rule", v.to_string());
    }
    if let Some(v) = para.indent_left() {
        node = node.prop("docx:indent-left", v);
    }
    if let Some(v) = para.indent_right() {
        node = node.prop("docx:indent-right", v);
    }
    if let Some(v) = para.indent_first_line() {
        node = node.prop("docx:indent-first-line", v);
    }
    if let Some(v) = para.indent_hanging() {
        node = node.prop("docx:indent-hanging", v);
    }

    if let Some(props) = para.properties() {
        // Paragraph style: format-specific (a docx:pStyle id is meaningless outside
        // DOCX without the accompanying styles.xml definition) — raw-preserve.
        if let Some(style) = &props.paragraph_style {
            node = node.prop("docx:pStyle", style.value.clone());
        }
        // Keep-together / keep-with-next: no cross-format equivalent construct.
        if props.keep_next.as_deref().is_some_and(is_on_off_true) {
            node = node.prop("docx:keep-next", true);
        }
        if props.keep_lines.as_deref().is_some_and(is_on_off_true) {
            node = node.prop("docx:keep-lines", true);
        }
        // Page break before: real cross-format concept (LaTeX \newpage, ODT
        // fo:break-before="page"), so it gets the semantic layout prop rather
        // than a docx:-namespaced raw one.
        if props
            .page_break_before
            .as_deref()
            .is_some_and(is_on_off_true)
        {
            node = node.prop(prop::LAYOUT_PAGE_BREAK, true);
        }
        // Paragraph border: STBorder has dozens of format-specific styles with no
        // cross-format equivalent (see the identical table-cell-border rationale).
        if let Some(bdr) = &props.paragraph_border {
            node = apply_border_prop(node, "para", "top", bdr.top.as_deref());
            node = apply_border_prop(node, "para", "bottom", bdr.bottom.as_deref());
            node = apply_border_prop(node, "para", "left", bdr.left.as_deref());
            node = apply_border_prop(node, "para", "right", bdr.right.as_deref());
        }
        // Paragraph shading: same semantic as a cell/run background color.
        if let Some(shd) = &props.shading
            && let Some(fill) = &shd.fill
            && fill != "auto"
        {
            node = node.prop(prop::STYLE_BG_COLOR, fill.clone());
        }
        // Paragraph frame (`<w:framePr>`, ECMA-376 17.3.1.11): the "old-style"
        // text-frame-around-a-paragraph mechanism (positioned/floated text,
        // predates DrawingML text boxes). No cross-format equivalent, so
        // raw-preserved like the rest of this function's `docx:*` props.
        if let Some(frame) = &props.frame_pr {
            node = node.prop("docx:frame", true);
            if let Some(v) = &frame.width {
                node = node.prop("docx:frame-width", v.clone());
            }
            if let Some(v) = &frame.height {
                node = node.prop("docx:frame-height", v.clone());
            }
            if let Some(wrap) = &frame.wrap {
                node = node.prop("docx:frame-wrap", wrap.to_string());
            }
            if let Some(anchor) = &frame.h_anchor {
                node = node.prop("docx:frame-h-anchor", anchor.to_string());
            }
            if let Some(anchor) = &frame.v_anchor {
                node = node.prop("docx:frame-v-anchor", anchor.to_string());
            }
            if let Some(v) = &frame.x {
                node = node.prop("docx:frame-x", v.clone());
            }
            if let Some(v) = &frame.y {
                node = node.prop("docx:frame-y", v.clone());
            }
        }
    }

    node
}

/// `true` iff an on/off element is present and not explicitly set to `false`/`0`/`off`.
fn is_on_off_true(elem: &ooxml_wml::types::OnOffElement) -> bool {
    match &elem.value {
        None => true, // element present with no val → on
        Some(v) => matches!(v.as_str(), "1" | "true" | "on"),
    }
}

fn detect_heading_level(para: &Paragraph) -> Option<u8> {
    if let Some(props) = para.properties() {
        if let Some(outline) = &props.outline_lvl {
            let level = outline.value as u8;
            return Some(level + 1);
        }

        if let Some(style) = &props.paragraph_style {
            let style_lower = style.value.to_lowercase();
            if style_lower.starts_with("heading") || style_lower.starts_with("titre") {
                for c in style_lower.chars() {
                    if let Some(digit) = c.to_digit(10)
                        && (1..=9).contains(&digit)
                    {
                        return Some(digit as u8);
                    }
                }
            }
        }
    }

    None
}

fn convert_paragraph_content<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    para: &Paragraph,
) -> Result<Vec<Node>, ParseError> {
    let mut children = Vec::new();

    for content in &para.paragraph_content {
        match content {
            ParagraphContent::R(run) => {
                for n in convert_run(converter, doc, run)? {
                    children.push(n);
                }
            }
            ParagraphContent::Hyperlink(link) => {
                if let Some(node) = convert_hyperlink(converter, doc, link)? {
                    children.push(node);
                }
            }
            ParagraphContent::Ins(ins) => {
                // Tracked insertion: keep the content (it's visible in the current
                // document state) and mark it as a tracked change so a writer can
                // re-emit the <w:ins> wrapper on round-trip.
                let mut inner = Vec::new();
                for item in &ins.run_content {
                    if let RunContentChoice::R(run) = item {
                        for n in convert_run(converter, doc, run)? {
                            inner.push(n);
                        }
                    }
                }
                if !inner.is_empty() {
                    children.push(wrap_tracked_change(
                        "ins",
                        ins.id,
                        &ins.author,
                        ins.date.as_deref(),
                        inner,
                    ));
                }
            }
            ParagraphContent::Del(del) => {
                // Tracked deletion: content is not visible in the current document
                // state, but dropping it entirely is a losslessness bug (a
                // DOCX->DOCX round-trip would lose the deleted text and the
                // tracked-change record). Keep it, wrapped so a writer can restore
                // the <w:del>/<w:delText> structure and callers that don't care
                // about revision history can filter on docx:tracked-change="del".
                let mut inner = Vec::new();
                for item in &del.run_content {
                    if let RunContentChoice::R(run) = item {
                        for n in convert_run(converter, doc, run)? {
                            inner.push(n);
                        }
                    }
                }
                if !inner.is_empty() {
                    children.push(wrap_tracked_change(
                        "del",
                        del.id,
                        &del.author,
                        del.date.as_deref(),
                        inner,
                    ));
                }
            }
            ParagraphContent::MoveFrom(move_from) => {
                // MoveFrom contains text being moved away — include it (it was visible).
                for item in &move_from.run_content {
                    if let RunContentChoice::R(run) = item {
                        for n in convert_run(converter, doc, run)? {
                            children.push(n);
                        }
                    }
                }
            }
            ParagraphContent::MoveTo(move_to) => {
                // MoveTo contains text at its new location — include it.
                for item in &move_to.run_content {
                    if let RunContentChoice::R(run) = item {
                        for n in convert_run(converter, doc, run)? {
                            children.push(n);
                        }
                    }
                }
            }
            ParagraphContent::Dir(dir) => {
                // Bidirectional content run — recurse into paragraph content.
                for inner in &dir.paragraph_content {
                    if let ParagraphContent::R(run) = inner {
                        for n in convert_run(converter, doc, run)? {
                            children.push(n);
                        }
                    }
                }
            }
            ParagraphContent::Bdo(bdo) => {
                // Bidirectional override — recurse into paragraph content.
                for inner in &bdo.paragraph_content {
                    if let ParagraphContent::R(run) = inner {
                        for n in convert_run(converter, doc, run)? {
                            children.push(n);
                        }
                    }
                }
            }
            ParagraphContent::Sdt(sdt) => {
                // Inline structured document tag — extract runs from content.
                if let Some(content) = &sdt.sdt_content {
                    for item in &content.paragraph_content {
                        if let ParagraphContent::R(run) = item {
                            for n in convert_run(converter, doc, run)? {
                                children.push(n);
                            }
                        }
                    }
                }
            }
            ParagraphContent::SmartTag(tag) => {
                // Smart tag wraps runs — just include the runs.
                for item in &tag.paragraph_content {
                    if let ParagraphContent::R(run) = item {
                        for n in convert_run(converter, doc, run)? {
                            children.push(n);
                        }
                    }
                }
            }
            ParagraphContent::CustomXml(cx) => {
                // Custom XML wraps runs — include the runs.
                for item in &cx.paragraph_content {
                    if let ParagraphContent::R(run) = item {
                        for n in convert_run(converter, doc, run)? {
                            children.push(n);
                        }
                    }
                }
            }
            ParagraphContent::SubDoc(_) => {
                converter.warn_lost("SubDoc reference not representable in IR");
            }
            ParagraphContent::FldSimple(field) => {
                // Extract displayed text from simple fields; instruction is in field.instruction
                converter.warn("Field instruction lost (display text preserved)");
                for item in &field.paragraph_content {
                    if let ParagraphContent::R(run) = item {
                        for n in convert_run(converter, doc, run)? {
                            children.push(n);
                        }
                    }
                }
            }
            // Bookmarks and comment ranges are zero-width position markers (no
            // visible text) with no cross-format equivalent node -- raw-preserve
            // as an empty `raw_inline` carrying the marker's id/name so a writer
            // can restore the exact bookmarkStart/End or commentRangeStart/End
            // pair, instead of dropping the marker (losing the anchor/range).
            ParagraphContent::BookmarkStart(bm) => {
                children.push(
                    Node::new(node::RAW_INLINE)
                        .prop(prop::FORMAT, "docx")
                        .prop("docx:bookmark-start-id", bm.id.to_string())
                        .prop("docx:bookmark-start-name", bm.name.clone()),
                );
            }
            ParagraphContent::BookmarkEnd(range) => {
                children.push(
                    Node::new(node::RAW_INLINE)
                        .prop(prop::FORMAT, "docx")
                        .prop("docx:bookmark-end-id", range.id.to_string()),
                );
            }
            ParagraphContent::CommentRangeStart(range) => {
                children.push(
                    Node::new(node::RAW_INLINE)
                        .prop(prop::FORMAT, "docx")
                        .prop("docx:comment-range-start-id", range.id.to_string()),
                );
            }
            ParagraphContent::CommentRangeEnd(range) => {
                children.push(
                    Node::new(node::RAW_INLINE)
                        .prop(prop::FORMAT, "docx")
                        .prop("docx:comment-range-end-id", range.id.to_string()),
                );
            }
            // Markers that carry no text content
            ParagraphContent::ProofErr(_)
            | ParagraphContent::PermStart(_)
            | ParagraphContent::PermEnd(_)
            | ParagraphContent::MoveFromRangeStart(_)
            | ParagraphContent::MoveFromRangeEnd(_)
            | ParagraphContent::MoveToRangeStart(_)
            | ParagraphContent::MoveToRangeEnd(_)
            | ParagraphContent::CustomXmlInsRangeStart(_)
            | ParagraphContent::CustomXmlInsRangeEnd(_)
            | ParagraphContent::CustomXmlDelRangeStart(_)
            | ParagraphContent::CustomXmlDelRangeEnd(_)
            | ParagraphContent::CustomXmlMoveFromRangeStart(_)
            | ParagraphContent::CustomXmlMoveFromRangeEnd(_)
            | ParagraphContent::CustomXmlMoveToRangeStart(_)
            | ParagraphContent::CustomXmlMoveToRangeEnd(_) => {
                // Structural markers with no text — silently skip.
            }
        }
    }

    Ok(children)
}

/// Convert a run, returning zero or more nodes (a run may produce a footnote_ref + text).
fn convert_run<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    run: &Run,
) -> Result<Vec<Node>, ParseError> {
    let mut result = Vec::new();

    // Handle footnote reference (takes precedence over text)
    if let Some(fn_ref) = run.footnote_ref() {
        let fn_id = fn_ref.id;
        let content = converter.footnotes.remove(&fn_id).unwrap_or_default();
        result.push(
            Node::new(node::FOOTNOTE_REF)
                .prop(prop::LABEL, fn_id.to_string())
                .children(content),
        );
        return Ok(result);
    }

    // Handle endnote reference
    if let Some(en_ref) = run.endnote_ref() {
        let en_id = en_ref.id;
        let content = converter.endnotes.remove(&en_id).unwrap_or_default();
        result.push(
            Node::new(node::FOOTNOTE_REF)
                .prop(prop::LABEL, format!("en{}", en_id))
                .children(content),
        );
        return Ok(result);
    }

    // Handle DrawingML images in the run
    for drawing in run.drawings() {
        for rel_id in drawing.all_image_rel_ids() {
            if let Some(image_node) = convert_image(converter, doc, rel_id)? {
                result.push(image_node);
            }
        }
    }

    // Handle DrawingML text boxes (`<w:drawing>` → ... → `<w:txbxContent>`).
    // Only the text content is extracted (see `txbx_content_texts`'s doc
    // comment) -- shape geometry/position/fill are DrawingML-namespaced XML
    // this bridge doesn't model, so this is deliberately a reasonable
    // subset: real text content is preserved (as a `div` carrying
    // `docx:frame-type = "textbox"`, one plain-text `paragraph` child per
    // `<w:p>` inside the box) rather than dropped, but shape/positioning
    // and any inline formatting *within* the box are not, and a
    // DOCX->IR->DOCX round trip re-flows that text into the surrounding
    // paragraph rather than reconstructing an actual text box (write.rs has
    // no `docx:frame-type` writer -- see COVERAGE.md).
    for drawing in run.drawings() {
        for text in drawing.txbx_content_texts() {
            if text.trim().is_empty() {
                continue;
            }
            converter.warn_lost(
                "text box content extracted as plain text; shape/position and DOCX round-trip of the box itself not preserved",
            );
            let paragraphs: Vec<Node> = text
                .split('\n')
                .filter(|line| !line.is_empty())
                .map(|line| Node::new(node::PARAGRAPH).child(create_text_node(line)))
                .collect();
            result.push(
                Node::new(node::DIV)
                    .prop("docx:frame-type", "textbox")
                    .children(paragraphs),
            );
        }
    }

    // Handle VML pictures (legacy format)
    if run
        .run_content
        .iter()
        .any(|c| matches!(c, RunContent::Pict(_)))
    {
        converter.warn_lost("VML picture content not fully supported");
    }

    // Walk run content directly (instead of the flattened `run.text()`) so that
    // `<w:br>` elements can be modeled as distinct `line_break` nodes rather than
    // collapsed into an opaque newline character. Tab (`<w:tab/>`) and carriage
    // return (`<w:cr/>`) content is preserved as literal `\t`/`\n` characters in the
    // surrounding text — lossless because the writer emits those characters back as
    // literal run text, and rescribe's own reader/writer pair treats them identically
    // on the next parse.
    let mut buf = String::new();
    for item in &run.run_content {
        match item {
            // Complex field code (`<w:fldChar>`/`<w:instrText>`): see
            // `FieldPhase`'s doc comment for the state machine. Runs whose
            // *only* content is field-machinery produce no `text` node of
            // their own; a field's visible result content (the runs between
            // `separate` and `end`) is captured into the field node's
            // children instead of emitted directly, so it isn't duplicated
            // at the top level.
            RunContent::FldChar(fc) => {
                flush_run_text_buf(&mut buf, run, converter, &mut result);
                match fc.fld_char_type {
                    ooxml_wml::types::STFldCharType::Begin => {
                        converter.field_state = FieldPhase::Instr(String::new());
                    }
                    ooxml_wml::types::STFldCharType::Separate => {
                        if let FieldPhase::Instr(instr) = std::mem::take(&mut converter.field_state)
                        {
                            converter.field_state = FieldPhase::Display(instr, Vec::new());
                        }
                    }
                    ooxml_wml::types::STFldCharType::End => {
                        match std::mem::take(&mut converter.field_state) {
                            FieldPhase::Display(instr, display) => {
                                result.push(
                                    Node::new(node::RAW_INLINE)
                                        .prop(prop::FORMAT, "docx")
                                        .prop("docx:field-instr", instr)
                                        .children(display),
                                );
                            }
                            FieldPhase::Instr(instr) => {
                                // A field with no `separate` (no visible result).
                                result.push(
                                    Node::new(node::RAW_INLINE)
                                        .prop(prop::FORMAT, "docx")
                                        .prop("docx:field-instr", instr),
                                );
                            }
                            FieldPhase::None => {}
                        }
                    }
                }
            }
            RunContent::InstrText(t) => {
                if let FieldPhase::Instr(ref mut instr) = converter.field_state
                    && let Some(text) = &t.text
                {
                    instr.push_str(text);
                }
            }
            RunContent::T(t) => {
                if let Some(text) = &t.text {
                    buf.push_str(text);
                }
            }
            // <w:delText> is the tracked-deletion equivalent of <w:t> (used inside
            // <w:del> runs); text content is otherwise identical.
            RunContent::DelText(t) => {
                if let Some(text) = &t.text {
                    buf.push_str(text);
                }
            }
            RunContent::Tab(_) => buf.push('\t'),
            RunContent::Cr(_) => buf.push('\n'),
            RunContent::Br(br) => {
                flush_run_text_buf(&mut buf, run, converter, &mut result);
                let mut node = Node::new(node::LINE_BREAK);
                match br.r#type {
                    Some(ooxml_wml::types::STBrType::Page) => {
                        node = node.prop(prop::LAYOUT_PAGE_BREAK, true);
                    }
                    Some(ooxml_wml::types::STBrType::Column) => {
                        node = node.prop(prop::LAYOUT_COLUMN, true);
                    }
                    _ => {}
                }
                push_or_capture(converter, &mut result, apply_formatting(run, node));
            }
            RunContent::CommentReference(cm) => {
                // `<w:commentReference>` anchors a comment (from comments.xml) at
                // this position. No cross-format "comment" node kind exists, and
                // resolving the referenced comment body would require pre-loading
                // comments.xml the way footnotes/endnotes are pre-loaded; raw-
                // preserve just the id so the marker isn't silently dropped and a
                // writer can restore the `<w:commentReference>` element.
                flush_run_text_buf(&mut buf, run, converter, &mut result);
                push_or_capture(
                    converter,
                    &mut result,
                    Node::new(node::RAW_INLINE)
                        .prop(prop::FORMAT, "docx")
                        .prop("docx:comment-ref-id", cm.id.to_string()),
                );
            }
            _ => {}
        }
    }
    flush_run_text_buf(&mut buf, run, converter, &mut result);

    Ok(result)
}

/// Push a node either directly onto the paragraph-level `result` list, or
/// (if a complex field's `separate`..`end` display phase is currently open)
/// into that field's captured-display buffer instead, so field-result
/// content ends up nested under the field's `raw_inline` node rather than
/// duplicated at the top level.
fn push_or_capture(converter: &mut Converter, result: &mut Vec<Node>, node: Node) {
    if let FieldPhase::Display(_, ref mut display) = converter.field_state {
        display.push(node);
    } else {
        result.push(node);
    }
}

/// Flush accumulated run text into a formatted text node, if non-empty.
fn flush_run_text_buf(
    buf: &mut String,
    run: &Run,
    converter: &mut Converter,
    result: &mut Vec<Node>,
) {
    if !buf.is_empty() {
        let text_node = create_text_node(buf);
        push_or_capture(converter, result, apply_formatting(run, text_node));
        buf.clear();
    }
}

fn convert_image<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    rel_id: &str,
) -> Result<Option<Node>, ParseError> {
    match doc.get_image_data(rel_id) {
        Ok(image_data) => {
            let resource_id = converter.add_resource(image_data.data, &image_data.content_type);
            let node = Node::new(node::IMAGE)
                .prop(prop::URL, format!("resource:{}", resource_id.as_str()));
            Ok(Some(node))
        }
        Err(_) => {
            converter.warn_lost(format!("Failed to load image: {}", rel_id));
            Ok(None)
        }
    }
}

fn convert_hyperlink<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    link: &Hyperlink,
) -> Result<Option<Node>, ParseError> {
    let mut children = Vec::new();

    for run in link.runs() {
        for n in convert_run(converter, doc, run)? {
            children.push(n);
        }
    }

    if children.is_empty() {
        return Ok(None);
    }

    let mut node = Node::new(node::LINK);

    if let Some(rel_id) = link.rel_id() {
        if let Some(url) = doc.get_hyperlink_url(rel_id) {
            node = node.prop(prop::URL, url.to_string());
        }
    } else if let Some(anchor) = link.anchor_str() {
        node = node.prop(prop::URL, format!("#{}", anchor));
    }

    Ok(Some(node.children(children)))
}

/// Convert a block-level structured document tag (`<w:sdt>`, e.g. a content
/// control) into a `div` node wrapping its inner block content, with
/// `docx:sdt-tag`/`docx:sdt-alias`/`docx:sdt-type` raw-preserved from
/// `sdtPr` so a writer can restore the wrapper. There is no cross-format
/// "content control" node kind in rescribe-std, so this follows the same
/// `docx:*`-namespaced raw-preservation pattern used for tracked changes,
/// mapping the SDT's *content* onto the existing generic `div` container
/// rather than dropping the wrapper information entirely.
fn convert_sdt_block<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    ctrl: &ooxml_wml::types::CTSdtBlock,
) -> Result<Option<Node>, ParseError> {
    let mut children = Vec::new();
    if let Some(content) = &ctrl.sdt_content {
        for inner_block in &content.block_content {
            match inner_block {
                BlockContentChoice::P(para) => {
                    if let Some(n) = convert_paragraph(converter, doc, para)? {
                        children.push(n);
                    }
                }
                BlockContentChoice::Tbl(table) => {
                    children.push(convert_table(converter, doc, table)?);
                }
                BlockContentChoice::Sdt(nested) => {
                    if let Some(n) = convert_sdt_block(converter, doc, nested)? {
                        children.push(n);
                    }
                }
                _ => {}
            }
        }
    }

    if children.is_empty() {
        return Ok(None);
    }

    let mut node = Node::new(node::DIV).children(children);
    if let Some(pr) = ctrl.sdt_pr.as_deref() {
        if let Some(tag) = pr.tag.as_deref() {
            node = node.prop("docx:sdt-tag", tag.value.clone());
        }
        if let Some(alias) = pr.alias.as_deref() {
            node = node.prop("docx:sdt-alias", alias.value.clone());
        }
        if let Some(sdt_type) = sdt_type_name(pr) {
            node = node.prop("docx:sdt-type", sdt_type);
        }
    }

    Ok(Some(node))
}

/// Identify which content-control kind a `CTSdtPr` declares, from the
/// mutually-exclusive "type" child elements defined by the schema
/// (`text`, `comboBox`, `dropDownList`, `date`, `richText`, `picture`,
/// `citation`, `group`, `bibliography`, `equation`, `docPartObj`,
/// `docPartList`). Returns `None` if no type child is present.
fn sdt_type_name(pr: &ooxml_wml::types::CTSdtPr) -> Option<&'static str> {
    if pr.text.is_some() {
        Some("text")
    } else if pr.combo_box.is_some() {
        Some("comboBox")
    } else if pr.drop_down_list.is_some() {
        Some("dropDownList")
    } else if pr.date.is_some() {
        Some("date")
    } else if pr.rich_text.is_some() {
        Some("richText")
    } else if pr.picture.is_some() {
        Some("picture")
    } else if pr.citation.is_some() {
        Some("citation")
    } else if pr.group.is_some() {
        Some("group")
    } else if pr.bibliography.is_some() {
        Some("bibliography")
    } else if pr.equation.is_some() {
        Some("equation")
    } else if pr.doc_part_obj.is_some() {
        Some("docPartObj")
    } else if pr.doc_part_list.is_some() {
        Some("docPartList")
    } else {
        None
    }
}

/// Tracks a currently-open vertical merge (`vMerge`) run for one grid column,
/// so continuation cells can be folded into the originating cell's `rowspan`
/// instead of emitted as their own (invisible) `table_cell` nodes.
struct OpenVMerge {
    row_idx: usize,
    cell_idx: usize,
    count: i64,
}

fn convert_table<R: Read + Seek>(
    converter: &mut Converter,
    doc: &mut OoxmlDocument<R>,
    table: &Table,
) -> Result<Node, ParseError> {
    let mut rows: Vec<Node> = Vec::new();
    // Indexed by grid column position.
    let mut open_merges: Vec<Option<OpenVMerge>> = Vec::new();

    for row in table.rows() {
        let mut cells = Vec::new();

        let is_header = row
            .properties()
            .and_then(|p| p.tbl_header.as_ref())
            .map(|h| match &h.value {
                None => true,
                Some(v) => matches!(v.as_str(), "1" | "true" | "on"),
            })
            .unwrap_or(false);

        let mut col: usize = 0;
        for cell in row.cells() {
            let props = cell.properties();
            let grid_span = props
                .and_then(|p| p.grid_span.as_deref())
                .map(|g| g.value.max(1) as usize)
                .unwrap_or(1);
            let vmerge = props.and_then(|p| p.vertical_merge.as_deref());
            let is_continuation = vmerge
                .is_some_and(|vm| !matches!(vm.value, Some(ooxml_wml::types::STMerge::Restart)));

            if is_continuation {
                // Fold into the merge opened by the origin (restart) cell above.
                if let Some(Some(open)) = open_merges.get_mut(col) {
                    open.count += 1;
                }
                col += grid_span;
                continue;
            }

            // Walk the cell's block content directly (not just cell.paragraphs())
            // so nested tables (a table inside a table cell -- legal DOCX, and a
            // real construct, not an edge case to drop) are recursed into instead
            // of silently disappearing.
            let mut cell_children = Vec::new();
            for block in &cell.block_content {
                match block {
                    BlockContent::P(para) => {
                        if let Some(node) = convert_paragraph(converter, doc, para)? {
                            cell_children.push(node);
                        }
                    }
                    BlockContent::Tbl(nested_table) => {
                        cell_children.push(convert_table(converter, doc, nested_table)?);
                    }
                    _ => {}
                }
            }

            let cell_kind = if is_header {
                node::TABLE_HEADER
            } else {
                node::TABLE_CELL
            };

            let mut node = Node::new(cell_kind).children(cell_children);
            if grid_span > 1 {
                node = node.prop(prop::COLSPAN, grid_span as i64);
            }
            if let Some(shd) = props.and_then(|p| p.shading.as_deref())
                && let Some(fill) = &shd.fill
                && fill != "auto"
            {
                node = node.prop(prop::STYLE_BG_COLOR, fill.clone());
            }
            if let Some(borders) = props.and_then(|p| p.tc_borders.as_deref()) {
                node = apply_border_prop(node, "cell", "top", borders.top.as_deref());
                node = apply_border_prop(node, "cell", "bottom", borders.bottom.as_deref());
                node = apply_border_prop(node, "cell", "left", borders.left.as_deref());
                node = apply_border_prop(node, "cell", "right", borders.right.as_deref());
            }

            cells.push(node);
            let cell_idx = cells.len() - 1;

            if open_merges.len() <= col {
                open_merges.resize_with(col + 1, || None);
            }
            let starts_merge = vmerge
                .is_some_and(|vm| matches!(vm.value, Some(ooxml_wml::types::STMerge::Restart)));
            open_merges[col] = if starts_merge {
                Some(OpenVMerge {
                    row_idx: rows.len(),
                    cell_idx,
                    count: 1,
                })
            } else {
                None
            };

            col += grid_span;
        }

        rows.push(Node::new(node::TABLE_ROW).children(cells));
    }

    for open in open_merges.into_iter().flatten() {
        if open.count > 1
            && let Some(cell) = rows[open.row_idx].children.get_mut(open.cell_idx)
        {
            cell.props.set(prop::ROWSPAN, open.count);
        }
    }

    Ok(Node::new(node::TABLE).children(rows))
}

/// Raw-preserve one side of a cell/paragraph border as `docx:{scope}-border-{side}`
/// (`"style;eighths-of-a-point;hex-color"`) — DOCX border styles (STBorder has
/// dozens of variants: wave, dashDotStroked, threeDEmboss, ...) have no
/// cross-format equivalent, so this follows the existing `docx:para-props`
/// raw-preservation pattern rather than lossily narrowing to a handful of
/// common styles.
fn apply_border_prop(
    mut node: Node,
    scope: &str,
    side: &str,
    border: Option<&ooxml_wml::types::CTBorder>,
) -> Node {
    if let Some(b) = border {
        let color = b.color.clone().unwrap_or_default();
        let size = b.size.map(|s| s.to_string()).unwrap_or_default();
        node = node.prop(
            format!("docx:{scope}-border-{side}"),
            format!("{};{};{}", b.value, size, color),
        );
    }
    node
}

/// Wrap tracked-insertion/-deletion content in a `span` carrying
/// `docx:tracked-change` ("ins"/"del") plus author/date, so a writer can
/// restore the `<w:ins>`/`<w:del>` wrapper and callers that don't care about
/// revision history can filter on the prop. There's no cross-format
/// "tracked change" node kind in rescribe-std, so this follows the existing
/// `docx:*`-namespaced raw-preservation pattern on a generic structural node
/// rather than dropping the content (which the reader previously did for
/// deletions -- a real losslessness bug).
fn wrap_tracked_change(
    kind: &str,
    id: i64,
    author: &str,
    date: Option<&str>,
    children: Vec<Node>,
) -> Node {
    let mut node = Node::new(node::SPAN)
        .prop("docx:tracked-change", kind.to_string())
        .prop("docx:tracked-change-id", id)
        .children(children);
    if !author.is_empty() {
        node = node.prop("docx:tracked-change-author", author.to_string());
    }
    if let Some(d) = date {
        node = node.prop("docx:tracked-change-date", d.to_string());
    }
    node
}

fn create_text_node(text: &str) -> Node {
    Node::new(node::TEXT).prop(prop::CONTENT, text.to_string())
}

fn apply_formatting(run: &Run, mut node: Node) -> Node {
    // --- Span-level styling (color, font, size, background) ---
    // Collect properties that go onto a span wrapper.
    let mut span_props = Properties::new();

    if let Some(props) = run.properties() {
        if let Some(color) = props.color_hex() {
            // "auto" is the default; skip it
            if color != "auto" && !color.is_empty() {
                span_props.set(prop::STYLE_COLOR, color.to_string());
            }
        }
        if let Some(font) = props.font_ascii()
            && !font.is_empty()
        {
            span_props.set(prop::STYLE_FONT, font.to_string());
        }
        if let Some(size_pts) = props.font_size_points() {
            span_props.set(prop::STYLE_SIZE, size_pts);
        }
        if let Some(highlight) = props.highlight_color() {
            let color_str = highlight.to_string();
            if color_str != "none" {
                span_props.set(prop::STYLE_BG_COLOR, color_str);
            }
        }
        if let Some(lang) = props.language()
            && let Some(v) = &lang.value
            && !v.is_empty()
        {
            span_props.set(prop::LANGUAGE, v.clone());
        }
    }

    if !span_props.is_empty() {
        let mut span_node = Node::new(node::SPAN);
        span_node.props = span_props;
        node = span_node.child(node);
    }

    // --- Semantic inline node wrappers ---

    if run.properties().is_some_and(|p| p.is_hidden()) {
        node = Node::new(node::HIDDEN).child(node);
    }

    if run.properties().is_some_and(|p| p.is_small_caps()) {
        node = Node::new(node::SMALL_CAPS).child(node);
    }

    if run.properties().is_some_and(|p| p.is_all_caps()) {
        node = Node::new(node::ALL_CAPS).child(node);
    }

    if run.properties().is_some_and(|p| p.is_subscript()) {
        node = Node::new(node::SUBSCRIPT).child(node);
    } else if run.properties().is_some_and(|p| p.is_superscript()) {
        node = Node::new(node::SUPERSCRIPT).child(node);
    }

    if run.is_strikethrough()
        || run
            .properties()
            .is_some_and(|p| p.is_double_strikethrough())
    {
        node = Node::new(node::STRIKEOUT).child(node);
    }

    if run.is_underline() {
        node = Node::new(node::UNDERLINE).child(node);
    }

    if run.is_italic() {
        node = Node::new(node::EMPHASIS).child(node);
    }

    if run.is_bold() {
        node = Node::new(node::STRONG).child(node);
    }

    // Inline code: run references one of a known set of monospace run
    // *styles* (`rStyle`, e.g. Word's built-in "HTMLTypewriter" or common
    // "CodeChar"/"SourceCodeChar" conventions) -- deliberately style-ID
    // based, not font-based, for the same reason as `code_block_style`:
    // a font-based heuristic would misclassify `inline_font_name`, which
    // sets a monospace run font directly with no `rStyle`.
    if let Some(style) = run
        .properties()
        .and_then(|p| p.run_style.as_deref())
        .filter(|s| is_code_run_style(&s.value))
    {
        node = Node::new(node::CODE)
            .prop("docx:rStyle", style.value.clone())
            .child(node);
    }

    node
}

/// Known DOCX run-style IDs used for inline code/monospace runs:
/// `HTMLTypewriter` is Word's own built-in style for `<tt>`/`<code>`
/// content imported from HTML; `CodeChar`/`SourceCodeChar` are common
/// third-party-template conventions (the run-level counterpart of
/// `code_block_style`'s paragraph-style allowlist).
fn is_code_run_style(style: &str) -> bool {
    let lower = style.to_lowercase();
    matches!(
        lower.as_str(),
        "htmltypewriter" | "codechar" | "sourcecodechar" | "code"
    )
}

fn extract_metadata<R: Read + Seek>(doc: &OoxmlDocument<R>) -> Properties {
    let mut metadata = Properties::new();

    if let Some(core) = doc.core_properties() {
        if let Some(title) = &core.title {
            metadata.set("title", title.clone());
        }
        if let Some(creator) = &core.creator {
            metadata.set("author", creator.clone());
        }
        if let Some(subject) = &core.subject {
            metadata.set("subject", subject.clone());
        }
        if let Some(description) = &core.description {
            metadata.set("description", description.clone());
        }
        if let Some(keywords) = &core.keywords {
            metadata.set("keywords", keywords.clone());
        }
        if let Some(category) = &core.category {
            metadata.set("category", category.clone());
        }
        if let Some(created) = &core.created {
            metadata.set("created", created.clone());
        }
        if let Some(modified) = &core.modified {
            metadata.set("modified", modified.clone());
        }
    }

    // Section properties: page size/margins/orientation. Real cross-format
    // concept (LaTeX \geometry, ODT fo:page-width, ...), but rescribe-std has no
    // dedicated page-layout node/prop yet, so this is raw-preserved on metadata
    // like the rest of docx-specific document properties.
    if let Some(sect_pr) = doc.body().sect_pr.as_deref() {
        use ooxml_wml::ext::SectionPropertiesExt;
        if let Some(w) = sect_pr.page_width_twips() {
            metadata.set("docx:page-width-twips", w as i64);
        }
        if let Some(h) = sect_pr.page_height_twips() {
            metadata.set("docx:page-height-twips", h as i64);
        }
        if let Some(orient) = sect_pr.page_orientation() {
            metadata.set("docx:page-orientation", orient.to_string());
        }
        if let Some(margins) = sect_pr.page_margins() {
            metadata.set("docx:margin-top-twips", margins.top.clone());
            metadata.set("docx:margin-bottom-twips", margins.bottom.clone());
            metadata.set("docx:margin-left-twips", margins.left.clone());
            metadata.set("docx:margin-right-twips", margins.right.clone());
        }
    }

    // Document-level default language, from styles.xml docDefaults/rPrDefault/rPr/lang.
    {
        let styles = doc.styles();
        if let Some(lang) = styles
            .doc_defaults
            .as_ref()
            .and_then(|dd| dd.r_pr_default.as_ref())
            .and_then(|rpd| rpd.r_pr.as_ref())
            .and_then(|rp| rp.lang.as_deref())
            .and_then(|l| l.value.as_ref())
            && !lang.is_empty()
        {
            metadata.set(prop::LANGUAGE, lang.clone());
        }
    }

    if let Some(app) = doc.app_properties() {
        if let Some(app_name) = &app.application {
            metadata.set("application", app_name.clone());
        }
        if let Some(pages) = app.pages {
            metadata.set("pages", pages as i64);
        }
        if let Some(words) = app.words {
            metadata.set("words", words as i64);
        }
        if let Some(paragraphs) = app.paragraphs {
            metadata.set("paragraphs", paragraphs as i64);
        }
    }

    metadata
}
