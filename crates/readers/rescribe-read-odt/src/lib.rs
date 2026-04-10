//! ODT (OpenDocument Text) reader for rescribe.
//!
//! Parses ODF/ODT documents into rescribe's document IR by delegating to
//! `odf-fmt` for all ZIP unpacking and XML parsing.

use odf_fmt::ast::{
    FrameContent, Inline, ListItem, NoteClass, OdfBody, OdfDocument, StyleEntry, TextBlock,
};
use rescribe_core::{ConversionResult, Document, ParseError, ParseOptions, Properties, Resource,
    ResourceId, ResourceMap};
use rescribe_std::{Node, node, prop};

/// Parse ODT input into a document.
pub fn parse(input: &[u8]) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse ODT input into a document with options.
pub fn parse_with_options(
    input: &[u8],
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let result = odf_fmt::parse(input).map_err(|e| ParseError::Invalid(e.to_string()))?;
    let odf_doc = result.value;
    convert_document(odf_doc)
}

// ── Document conversion ───────────────────────────────────────────────────────

fn convert_document(odf: OdfDocument) -> Result<ConversionResult<Document>, ParseError> {
    // Metadata
    let mut metadata = Properties::new();
    if let Some(v) = &odf.meta.title { metadata.set("title", v.as_str()); }
    if let Some(v) = &odf.meta.creator { metadata.set("author", v.as_str()); }
    if let Some(v) = &odf.meta.modification_date.as_ref().or(odf.meta.creation_date.as_ref()) {
        metadata.set("date", v.as_str());
    }
    if let Some(v) = &odf.meta.description { metadata.set("description", v.as_str()); }
    if let Some(v) = &odf.meta.language { metadata.set("language", v.as_str()); }
    for (name, value) in &odf.meta.user_defined {
        metadata.set(format!("meta:{name}"), value.as_str());
    }

    // Page layout from first page layout entry
    if let Some(pl) = odf.page_layouts.first() {
        if let Some(v) = &pl.page_width { metadata.set("page-width", v.as_str()); }
        if let Some(v) = &pl.page_height { metadata.set("page-height", v.as_str()); }
        if let Some(v) = &pl.margin_top { metadata.set("margin-top", v.as_str()); }
        if let Some(v) = &pl.margin_bottom { metadata.set("margin-bottom", v.as_str()); }
        if let Some(v) = &pl.margin_left { metadata.set("margin-left", v.as_str()); }
        if let Some(v) = &pl.margin_right { metadata.set("margin-right", v.as_str()); }
    }

    // Embedded images
    let mut resources = ResourceMap::new();
    let mut image_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (path, data) in &odf.images {
        if !data.is_empty() {
            let mime = mime_from_name(path);
            let res_id = ResourceId::new();
            let id_str = res_id.as_str().to_owned();
            resources.insert(
                res_id,
                Resource::new(mime, data.clone()).with_name(path.clone()),
            );
            image_map.insert(path.clone(), id_str);
        }
    }

    // Style maps: merge named + automatic
    let ctx = StyleCtx {
        named: &odf.named_styles,
        auto: &odf.automatic_styles,
        image_map: &image_map,
        list_styles: &odf.list_styles,
    };

    // Convert body
    let empty_blocks: Vec<odf_fmt::ast::TextBlock> = Vec::new();
    let body_blocks = match &odf.body {
        OdfBody::Text(blocks) => blocks,
        OdfBody::Empty => &empty_blocks,
        _ => {
            return Err(ParseError::Invalid(
                "Not an ODT text document (body is not office:text)".to_owned(),
            ));
        }
    };

    let mut doc = Node::new(node::DOCUMENT);
    let mut pending_footnotes: Vec<Node> = Vec::new();
    let mut pending_blockquote: Option<Vec<Node>> = None;

    for block in body_blocks {
        let (nodes, footnotes) = convert_block(block, &ctx);
        pending_footnotes.extend(footnotes);
        for n in nodes {
            let is_bq = n.kind.as_str() == node::PARAGRAPH
                && n.props.get_str("odt:is-blockquote").is_some();
            if is_bq {
                pending_blockquote.get_or_insert_with(Vec::new).push({
                    let mut stripped = n.clone();
                    stripped.props.remove("odt:is-blockquote");
                    stripped
                });
            } else {
                flush_pending_blockquote(&mut pending_blockquote, &mut doc);
                doc = doc.child(n);
                for fn_def in pending_footnotes.drain(..) {
                    doc = doc.child(fn_def);
                }
            }
        }
    }
    flush_pending_blockquote(&mut pending_blockquote, &mut doc);

    Ok(ConversionResult::ok(Document {
        content: doc,
        resources,
        metadata,
        source: None,
    }))
}

// ── Style context ─────────────────────────────────────────────────────────────

struct StyleCtx<'a> {
    named: &'a [StyleEntry],
    auto: &'a [StyleEntry],
    image_map: &'a std::collections::HashMap<String, String>,
    list_styles: &'a [(String, bool)],
}

impl<'a> StyleCtx<'a> {
    fn find_style(&self, name: &str) -> Option<&StyleEntry> {
        self.auto.iter().find(|s| s.name == name)
            .or_else(|| self.named.iter().find(|s| s.name == name))
    }

    fn is_ordered_list_style(&self, name: &str) -> Option<bool> {
        self.list_styles.iter().find(|(n, _)| n == name).map(|(_, o)| *o)
    }
}

// ── Para-kind resolution ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParaKind {
    Normal,
    Heading(u8),
    Code,
    Blockquote,
    HorizontalRule,
}

fn resolve_para_kind(style_name: Option<&str>, is_heading_tag: bool, outline_level: Option<u32>, ctx: &StyleCtx<'_>) -> ParaKind {
    if is_heading_tag {
        let level = outline_level.unwrap_or(1).min(6) as u8;
        return ParaKind::Heading(level.max(1));
    }

    let name = match style_name {
        Some(n) if !n.is_empty() => n,
        _ => return ParaKind::Normal,
    };

    // Check style entry first
    if let Some(entry) = ctx.find_style(name) {
        // Use display_name or name for heuristics
        let check = entry.display_name.as_deref().unwrap_or(&entry.name);
        if let k @ (ParaKind::Code | ParaKind::Blockquote | ParaKind::HorizontalRule | ParaKind::Heading(_)) = para_kind_from_name(check) {
            return k;
        }
        // Also check parent style
        if let Some(parent) = &entry.parent_style_name
            && let Some(parent_entry) = ctx.find_style(parent) {
            let pcheck = parent_entry.display_name.as_deref().unwrap_or(&parent_entry.name);
            if let k @ (ParaKind::Code | ParaKind::Blockquote | ParaKind::HorizontalRule | ParaKind::Heading(_)) = para_kind_from_name(pcheck) {
                return k;
            }
        }
    }

    // Heuristic on raw style name
    para_kind_from_name(name)
}

fn para_kind_from_name(name: &str) -> ParaKind {
    let lower = name.to_lowercase();
    if lower.starts_with("heading") {
        let suffix = lower.trim_start_matches("heading").trim();
        if let Some(c) = suffix.chars().next().filter(|c| c.is_ascii_digit()) {
            let level = ((c as u8) - b'0').min(6);
            return ParaKind::Heading(level.max(1));
        }
        return ParaKind::Heading(1);
    }
    if lower.contains("preformat") || lower.contains("code") || lower.contains("monospace")
        || lower.contains("verbatim") || lower == "source text" {
        return ParaKind::Code;
    }
    if lower.contains("quotation") || lower.contains("blockquote") || lower.contains("quote") {
        return ParaKind::Blockquote;
    }
    if lower.contains("horizontal") || lower.contains("hrule") || lower.contains("h-rule") {
        return ParaKind::HorizontalRule;
    }
    ParaKind::Normal
}

// ── Block conversion ──────────────────────────────────────────────────────────

/// Returns (block nodes, footnote_def nodes collected during conversion).
fn convert_block(block: &TextBlock, ctx: &StyleCtx<'_>) -> (Vec<Node>, Vec<Node>) {
    match block {
        TextBlock::Paragraph(p) => {
            let kind = resolve_para_kind(p.style_name.as_deref(), false, None, ctx);
            let (children, footnotes) = convert_inlines(&p.content, ctx);

            let node = match kind {
                ParaKind::Code => {
                    let content = extract_text_from_children(&children);
                    Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content)
                }
                ParaKind::HorizontalRule => Node::new(node::HORIZONTAL_RULE),
                ParaKind::Blockquote => {
                    // Mark for blockquote accumulation
                    let mut n = Node::new(node::PARAGRAPH);
                    for c in children { n = n.child(c); }
                    n = n.prop("odt:is-blockquote", "1");
                    if let Some(sn) = &p.style_name
                        && !sn.is_empty() {
                        n = n.prop("odt:style-name", sn.as_str());
                    }
                    return (vec![n], footnotes);
                }
                _ => {
                    let mut n = Node::new(node::PARAGRAPH);
                    for c in children { n = n.child(c); }
                    if let Some(sn) = &p.style_name
                        && !sn.is_empty() {
                        n = n.prop("odt:style-name", sn.as_str());
                    }
                    n
                }
            };

            // Apply para layout props from style
            let node = if let Some(sn) = &p.style_name {
                apply_para_props_from_style(node, sn, ctx)
            } else {
                node
            };

            (vec![node], footnotes)
        }

        TextBlock::Heading(h) => {
            let level = h.outline_level.unwrap_or(1).min(6) as u8;
            let (children, footnotes) = convert_inlines(&h.content, ctx);
            let mut n = Node::new(node::HEADING).prop(prop::LEVEL, level as i64);
            for c in children { n = n.child(c); }
            (vec![n], footnotes)
        }

        TextBlock::List(list) => {
            let ordered = is_ordered_list(list.style_name.as_deref(), ctx);
            let mut list_node = Node::new(node::LIST);
            if ordered { list_node = list_node.prop("ordered", true); }
            let mut all_footnotes = Vec::new();

            for item in &list.items {
                let (item_node, fn_defs) = convert_list_item(item, ctx);
                list_node = list_node.child(item_node);
                all_footnotes.extend(fn_defs);
            }

            (vec![list_node], all_footnotes)
        }

        TextBlock::Table(t) => {
            let mut table_node = Node::new(node::TABLE);
            let mut all_footnotes = Vec::new();

            for row in &t.rows {
                let mut row_node = Node::new(node::TABLE_ROW);
                for cell in &row.cells {
                    let mut cell_node = Node::new(node::TABLE_CELL);
                    if let Some(cs) = cell.col_span.filter(|&v| v > 1) {
                        cell_node = cell_node.prop(prop::COLSPAN, cs as i64);
                    }
                    if let Some(rs) = cell.row_span.filter(|&v| v > 1) {
                        cell_node = cell_node.prop(prop::ROWSPAN, rs as i64);
                    }
                    for block in &cell.content {
                        let (nodes, fn_defs) = convert_block(block, ctx);
                        for n in nodes { cell_node = cell_node.child(n); }
                        all_footnotes.extend(fn_defs);
                    }
                    row_node = row_node.child(cell_node);
                }
                table_node = table_node.child(row_node);
            }

            (vec![table_node], all_footnotes)
        }

        TextBlock::Section(s) => {
            let mut all_nodes = Vec::new();
            let mut all_footnotes = Vec::new();
            for block in &s.content {
                let (nodes, fn_defs) = convert_block(block, ctx);
                all_nodes.extend(nodes);
                all_footnotes.extend(fn_defs);
            }
            (all_nodes, all_footnotes)
        }

        TextBlock::Frame(frame) => {
            match &frame.content {
                FrameContent::Image { href, .. } => {
                    let mut img = Node::new(node::IMAGE);
                    let src = ctx.image_map.get(href).map(String::as_str).unwrap_or(href.as_str());
                    img = img.prop("src", src);
                    if let Some(n) = &frame.name { img = img.prop("odt:name", n.as_str()); }
                    (vec![img], Vec::new())
                }
                FrameContent::TextBox(blocks) => {
                    let mut div = Node::new(node::DIV);
                    let mut all_footnotes = Vec::new();
                    for block in blocks {
                        let (nodes, fn_defs) = convert_block(block, ctx);
                        for n in nodes { div = div.child(n); }
                        all_footnotes.extend(fn_defs);
                    }
                    (vec![div], all_footnotes)
                }
                _ => (Vec::new(), Vec::new()),
            }
        }

        TextBlock::Unknown { .. } => (Vec::new(), Vec::new()),
    }
}

fn convert_list_item(item: &ListItem, ctx: &StyleCtx<'_>) -> (Node, Vec<Node>) {
    let mut item_node = Node::new(node::LIST_ITEM);
    let mut all_footnotes = Vec::new();

    for block in &item.content {
        let (nodes, fn_defs) = convert_block(block, ctx);
        for n in nodes { item_node = item_node.child(n); }
        all_footnotes.extend(fn_defs);
    }

    (item_node, all_footnotes)
}

// ── Inline conversion ─────────────────────────────────────────────────────────

fn convert_inlines(inlines: &[Inline], ctx: &StyleCtx<'_>) -> (Vec<Node>, Vec<Node>) {
    let mut nodes: Vec<Node> = Vec::new();
    let mut footnotes = Vec::new();

    for inline in inlines {
        let (mut ns, mut fns) = convert_inline(inline, ctx);
        footnotes.append(&mut fns);
        for n in ns.drain(..) {
            // Coalesce adjacent text nodes into a single node.
            if n.kind.as_str() == node::TEXT {
                let new_content = n.props.get_str(prop::CONTENT).unwrap_or("").to_owned();
                if let Some(last) = nodes.last_mut()
                    && last.kind.as_str() == node::TEXT
                    && last.children.is_empty()
                {
                    let prev = last.props.get_str(prop::CONTENT).unwrap_or("").to_owned();
                    let merged = prev + &new_content;
                    last.props.set(prop::CONTENT, merged.as_str());
                    continue;
                }
            }
            nodes.push(n);
        }
    }

    (nodes, footnotes)
}

fn convert_inline(inline: &Inline, ctx: &StyleCtx<'_>) -> (Vec<Node>, Vec<Node>) {
    match inline {
        Inline::Text(s) => {
            if s.is_empty() {
                (Vec::new(), Vec::new())
            } else {
                (vec![Node::new(node::TEXT).prop(prop::CONTENT, s.as_str())], Vec::new())
            }
        }

        Inline::Tab => (
            vec![Node::new(node::TEXT).prop(prop::CONTENT, "\t")],
            Vec::new(),
        ),

        Inline::SoftHyphen => (
            vec![Node::new(node::TEXT).prop(prop::CONTENT, "\u{00AD}")],
            Vec::new(),
        ),

        Inline::Space { count } => {
            let spaces = " ".repeat(*count as usize);
            (vec![Node::new(node::TEXT).prop(prop::CONTENT, spaces)], Vec::new())
        }

        Inline::LineBreak => (vec![Node::new(node::LINE_BREAK)], Vec::new()),

        Inline::SoftPageBreak => (Vec::new(), Vec::new()),

        Inline::Span(span) => {
            let (children, footnotes) = convert_inlines(&span.content, ctx);
            if children.is_empty() {
                return (Vec::new(), footnotes);
            }

            let style_name = span.style_name.as_deref().unwrap_or("");
            let wrapper = inline_kind_from_style(style_name, ctx);

            let result = wrap_inline_nodes(children, wrapper, style_name, ctx);
            (result, footnotes)
        }

        Inline::Hyperlink(link) => {
            let (children, footnotes) = convert_inlines(&link.content, ctx);
            let href = link.href.as_deref().unwrap_or("");
            let mut n = Node::new(node::LINK).prop(prop::URL, href);
            if let Some(title) = &link.title
                && !title.is_empty() {
                n = n.prop(prop::TITLE, title.as_str());
            }
            for c in children { n = n.child(c); }
            (vec![n], footnotes)
        }

        Inline::Note(note) => {
            let id = note.id.clone().unwrap_or_default();

            // Footnote ref inline
            let ref_node = Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, id.as_str());

            // Footnote def node (collected and emitted after the paragraph)
            let mut def = Node::new(node::FOOTNOTE_DEF).prop(prop::LABEL, id.as_str());
            if note.note_class == NoteClass::Endnote {
                def = def.prop("odt:note-class", "endnote");
            }
            for block in &note.body {
                let (nodes, _) = convert_block(block, ctx);
                for n in nodes { def = def.child(n); }
            }

            (vec![ref_node], vec![def])
        }

        Inline::Frame(frame) => {
            let (nodes, footnotes) = convert_block(&TextBlock::Frame(frame.clone()), ctx);
            (nodes, footnotes)
        }

        Inline::Field { value, .. } => {
            if value.is_empty() {
                (Vec::new(), Vec::new())
            } else {
                (vec![Node::new(node::TEXT).prop(prop::CONTENT, value.as_str())], Vec::new())
            }
        }

        Inline::Bookmark { name } => {
            if name.is_empty() {
                (Vec::new(), Vec::new())
            } else {
                let n = Node::new(node::SPAN).prop(prop::ID, name.as_str());
                (vec![n], Vec::new())
            }
        }

        Inline::Annotation { content } => {
            let n = Node::new(node::SPAN).prop("odt:annotation", content.as_str());
            (vec![n], Vec::new())
        }

        Inline::Unknown { .. } => (Vec::new(), Vec::new()),
    }
}

// ── Inline kind resolution from style ────────────────────────────────────────

#[derive(Clone)]
enum InlineKind {
    Plain,
    Strong,
    Emphasis,
    Underline,
    Strikeout,
    Code,
    Subscript,
    Superscript,
    Span {
        color: Option<String>,
        font_size: Option<String>,
        font_name: Option<String>,
        small_caps: bool,
    },
}

fn inline_kind_from_style(style_name: &str, ctx: &StyleCtx<'_>) -> InlineKind {
    if style_name.is_empty() {
        return InlineKind::Plain;
    }

    if let Some(entry) = ctx.find_style(style_name) {
        let p = &entry.text_props;
        // Check monospace font → code
        let is_mono = p.font_name.as_ref().map(|f| {
            let lf = f.to_lowercase();
            lf.contains("courier") || lf.contains("mono") || lf.contains("consol")
                || lf.contains("fixed") || lf.contains("inconsolata") || lf.contains("menlo")
                || lf == "code2000" || lf == "source code pro"
        }).unwrap_or(false);

        if is_mono { return InlineKind::Code; }
        if p.subscript { return InlineKind::Subscript; }
        if p.superscript { return InlineKind::Superscript; }
        if p.bold { return InlineKind::Strong; }
        if p.italic { return InlineKind::Emphasis; }
        if p.underline { return InlineKind::Underline; }
        if p.strikethrough { return InlineKind::Strikeout; }
        let is_small_caps = p.font_variant.as_deref() == Some("small-caps");
        if is_small_caps || p.color.is_some() || p.font_size.is_some() || p.font_name.is_some() {
            let is_non_mono_font = p.font_name.as_ref().map(|_f| !is_mono).unwrap_or(false);
            let font_name_for_span = if is_non_mono_font { p.font_name.clone() } else { None };
            return InlineKind::Span {
                color: p.color.clone(),
                font_size: p.font_size.clone(),
                font_name: font_name_for_span,
                small_caps: is_small_caps,
            };
        }
    }

    // Heuristic on style name
    let lower = style_name.to_lowercase();
    if lower.contains("code") || lower.contains("preformat") || lower.contains("verbatim")
        || lower.contains("monospace") {
        return InlineKind::Code;
    }
    if lower.contains("subscript") || lower == "sub" { return InlineKind::Subscript; }
    if lower.contains("superscript") || lower == "sup" { return InlineKind::Superscript; }
    if lower.contains("bold") { return InlineKind::Strong; }
    if lower.contains("italic") || lower.contains("oblique") { return InlineKind::Emphasis; }
    if lower.contains("underline") { return InlineKind::Underline; }
    if lower.contains("strike") { return InlineKind::Strikeout; }

    InlineKind::Plain
}

fn wrap_inline_nodes(children: Vec<Node>, kind: InlineKind, style_name: &str, _ctx: &StyleCtx<'_>) -> Vec<Node> {
    match kind {
        InlineKind::Plain => {
            // Pass-through: plain spans contribute no wrapper node
            let _ = style_name;
            children
        }
        InlineKind::Strong => {
            let mut n = Node::new(node::STRONG);
            for c in children { n = n.child(c); }
            vec![n]
        }
        InlineKind::Emphasis => {
            let mut n = Node::new(node::EMPHASIS);
            for c in children { n = n.child(c); }
            vec![n]
        }
        InlineKind::Underline => {
            let mut n = Node::new(node::UNDERLINE);
            for c in children { n = n.child(c); }
            vec![n]
        }
        InlineKind::Strikeout => {
            let mut n = Node::new(node::STRIKEOUT);
            for c in children { n = n.child(c); }
            vec![n]
        }
        InlineKind::Code => {
            let content = extract_text_from_children(&children);
            vec![Node::new(node::CODE).prop(prop::CONTENT, content)]
        }
        InlineKind::Subscript => {
            let mut n = Node::new(node::SUBSCRIPT);
            for c in children { n = n.child(c); }
            vec![n]
        }
        InlineKind::Superscript => {
            let mut n = Node::new(node::SUPERSCRIPT);
            for c in children { n = n.child(c); }
            vec![n]
        }
        InlineKind::Span { color, font_size, font_name, small_caps } => {
            let mut n = Node::new(node::SPAN);
            if let Some(c) = color { n = n.prop("style:color", c); }
            if let Some(s) = font_size { n = n.prop("style:size", s); }
            if let Some(f) = font_name { n = n.prop("style:font", f); }
            if small_caps { n = n.prop("style:variant", "small-caps"); }
            for c in children { n = n.child(c); }
            vec![n]
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn extract_text_from_children(nodes: &[Node]) -> String {
    nodes.iter().map(extract_text_node).collect::<Vec<_>>().join("")
}

fn extract_text_node(n: &Node) -> String {
    if n.kind.as_str() == node::TEXT {
        n.props.get_str(prop::CONTENT).unwrap_or("").to_owned()
    } else if n.kind.as_str() == node::LINE_BREAK {
        "\n".to_owned()
    } else {
        extract_text_from_children(&n.children)
    }
}

fn is_ordered_list(style_name: Option<&str>, ctx: &StyleCtx<'_>) -> bool {
    let name = match style_name {
        Some(n) if !n.is_empty() => n,
        _ => return false,
    };
    // Check parsed list style info first
    if let Some(ordered) = ctx.is_ordered_list_style(name) {
        return ordered;
    }
    // Fall through to heuristic on style name
    let lower = name.to_lowercase();
    lower.contains("numb") || lower.contains("order") || lower.contains("decimal")
        || lower == "list number" || lower == "list_number"
}

fn apply_para_props_from_style(mut n: Node, style_name: &str, ctx: &StyleCtx<'_>) -> Node {
    if let Some(entry) = ctx.find_style(style_name) {
        let p = &entry.para_props;
        if let Some(v) = &p.align { n = n.prop("style:align", v.as_str()); }
        if let Some(v) = &p.margin_left { n = n.prop("style:margin-left", v.as_str()); }
        if let Some(v) = &p.margin_right { n = n.prop("style:margin-right", v.as_str()); }
        if let Some(v) = &p.margin_top { n = n.prop("style:margin-top", v.as_str()); }
        if let Some(v) = &p.margin_bottom { n = n.prop("style:margin-bottom", v.as_str()); }
        if let Some(v) = &p.text_indent { n = n.prop("style:text-indent", v.as_str()); }
        if let Some(v) = &p.line_height { n = n.prop("style:line-height", v.as_str()); }
        if let Some(v) = &p.border { n = n.prop("style:border", v.as_str()); }
        if let Some(v) = &p.background_color { n = n.prop("style:background", v.as_str()); }
        if p.keep_together { n = n.prop("style:keep-together", "always"); }
        if p.keep_with_next { n = n.prop("style:keep-with-next", "always"); }
    }
    n
}

fn flush_pending_blockquote(pending: &mut Option<Vec<Node>>, doc: &mut Node) {
    if let Some(paras) = pending.take()
        && !paras.is_empty() {
        let mut bq = Node::new(node::BLOCKQUOTE);
        for p in paras { bq = bq.child(p); }
        *doc = doc.clone().child(bq);
    }
}

fn mime_from_name(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower.ends_with(".png") { "image/png" }
    else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") { "image/jpeg" }
    else if lower.ends_with(".gif") { "image/gif" }
    else if lower.ends_with(".svg") { "image/svg+xml" }
    else if lower.ends_with(".webp") { "image/webp" }
    else if lower.ends_with(".tiff") || lower.ends_with(".tif") { "image/tiff" }
    else if lower.ends_with(".bmp") { "image/bmp" }
    else { "application/octet-stream" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_odt_bytes(content_xml: &str) -> Vec<u8> {
        use std::io::{Cursor, Write};
        use zip::ZipWriter;
        use zip::write::SimpleFileOptions;

        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buf);
            let options = SimpleFileOptions::default();
            zip.start_file("mimetype", options).unwrap();
            zip.write_all(b"application/vnd.oasis.opendocument.text").unwrap();
            zip.start_file("content.xml", options).unwrap();
            zip.write_all(content_xml.as_bytes()).unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
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
        let odt = make_odt_bytes(&body("<text:p>Hello world</text:p>"));
        let result = parse(&odt).unwrap();
        assert!(!result.value.content.children.is_empty());
    }

    #[test]
    fn test_parse_heading() {
        let odt = make_odt_bytes(&body(r#"<text:h text:outline-level="1">Title</text:h>"#));
        let result = parse(&odt).unwrap();
        let heading = &result.value.content.children[0];
        assert_eq!(heading.kind.as_str(), node::HEADING);
        assert_eq!(heading.props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_bold_named_style() {
        let xml = body(r#"<text:p>Some <text:span text:style-name="Bold">bold</text:span> text.</text:p>"#);
        let odt = make_odt_bytes(&xml);
        let result = parse(&odt).unwrap();
        let para = &result.value.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
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
        let odt = make_odt_bytes(&xml);
        let result = parse(&odt).unwrap();
        let para = &result.value.content.children[0];
        let strong = para.children.iter().find(|c| c.kind.as_str() == node::STRONG);
        assert!(strong.is_some(), "auto-style T1 with fo:font-weight=bold should produce strong node");
    }

    #[test]
    fn test_parse_italic() {
        let xml = body(r#"<text:p><text:span text:style-name="Italic">italic</text:span></text:p>"#);
        let odt = make_odt_bytes(&xml);
        let result = parse(&odt).unwrap();
        let para = &result.value.content.children[0];
        let em = para.children.iter().find(|c| c.kind.as_str() == node::EMPHASIS);
        assert!(em.is_some(), "should have an emphasis node");
    }

    #[test]
    fn test_parse_hyperlink() {
        let xml = body(r#"<text:p><text:a xlink:type="simple" xlink:href="https://example.com">link text</text:a></text:p>"#);
        let odt = make_odt_bytes(&xml);
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
        let odt = make_odt_bytes(&xml);
        let result = parse(&odt).unwrap();
        let table = &result.value.content.children[0];
        assert_eq!(table.kind.as_str(), node::TABLE);
        assert_eq!(table.children.len(), 1);
        let row = &table.children[0];
        assert_eq!(row.kind.as_str(), node::TABLE_ROW);
        assert_eq!(row.children.len(), 2);
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
        let odt = make_odt_bytes(&xml);
        let result = parse(&odt).unwrap();
        let list = &result.value.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        // Ordered detection via style name: "L1" is not a recognized ordered pattern,
        // so this tests the list structure at minimum.
        assert_eq!(list.children.len(), 2);
    }
}
