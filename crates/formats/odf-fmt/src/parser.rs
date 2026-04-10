//! ODF document parser.
//!
//! Parses ODF ZIP archives (`.odt`, `.ods`, `.odp`) into [`OdfDocument`].
//!
//! The parser reads:
//! - `mimetype` — document type identification
//! - `content.xml` — body content and automatic styles
//! - `styles.xml` — named styles and page layouts
//! - `meta.xml` — document metadata
//! - `Pictures/` and `media/` — embedded images
//!
//! # Buffer ownership
//!
//! Each public or recursive parse function creates its own `Vec` event buffer
//! rather than sharing a mutable borrow from the caller. This sidesteps the
//! quick-xml lifetime constraint where `BytesStart<'_>` borrows from the
//! caller's buffer while the caller also needs to pass that buffer into the
//! recursive call.

use crate::ast::*;
use crate::error::{Diagnostic, Error, ParseResult};
use quick_xml::Reader;
use quick_xml::events::BytesStart;
use quick_xml::events::Event;
use std::collections::HashMap;
use std::io::{Cursor, Read};
use zip::ZipArchive;

/// Parse an ODF ZIP archive from bytes.
pub fn parse(input: &[u8]) -> Result<ParseResult<OdfDocument>, Error> {
    let cursor = Cursor::new(input);
    let mut archive = ZipArchive::new(cursor)?;
    let mut diags = Vec::new();

    // Read mimetype
    let mimetype = read_zip_text(&mut archive, "mimetype")
        .unwrap_or_else(|| "application/vnd.oasis.opendocument.text".to_string());
    let mimetype = mimetype.trim().to_string();

    // Embedded images
    let file_names: Vec<String> = archive.file_names().map(str::to_owned).collect();
    let mut images = HashMap::new();
    for name in &file_names {
        if (name.starts_with("Pictures/") || name.starts_with("media/"))
            && let Ok(mut f) = archive.by_name(name)
        {
            let mut data = Vec::new();
            if f.read_to_end(&mut data).is_ok() && !data.is_empty() {
                images.insert(name.clone(), data);
            }
        }
    }

    // styles.xml
    let (named_styles, page_layouts) =
        if let Some(xml) = read_zip_text(&mut archive, "styles.xml") {
            parse_styles_xml(&xml, &mut diags)
        } else {
            (Vec::new(), Vec::new())
        };

    // meta.xml
    let meta = if let Some(xml) = read_zip_text(&mut archive, "meta.xml") {
        parse_meta_xml(&xml)
    } else {
        OdfMeta::default()
    };

    // content.xml
    let (body, automatic_styles) = if let Some(xml) = read_zip_text(&mut archive, "content.xml") {
        parse_content_xml(&xml, &mut diags)
    } else {
        (OdfBody::Empty, Vec::new())
    };

    let doc = OdfDocument {
        mimetype,
        body,
        automatic_styles,
        named_styles,
        page_layouts,
        meta,
        images,
    };

    Ok(ParseResult::with_diagnostics(doc, diags))
}

// ── ZIP helpers ───────────────────────────────────────────────────────────────

fn read_zip_text(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Option<String> {
    let mut file = archive.by_name(name).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;
    Some(content)
}

// ── content.xml ──────────────────────────────────────────────────────────────

fn parse_content_xml(
    xml: &str,
    diags: &mut Vec<Diagnostic>,
) -> (OdfBody, Vec<StyleEntry>) {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut automatic_styles = Vec::new();
    let mut body = OdfBody::Empty;
    let mut in_body = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = element_name(e.name().as_ref());
                match name.as_str() {
                    "office:automatic-styles" => {
                        automatic_styles =
                            parse_styles_block(&mut reader, "office:automatic-styles");
                    }
                    "office:body" => {
                        in_body = true;
                    }
                    "office:text" if in_body => {
                        let blocks = parse_text_body(&mut reader, diags);
                        body = OdfBody::Text(blocks);
                        in_body = false;
                    }
                    "office:spreadsheet" if in_body => {
                        let raw = capture_raw_until(&mut reader, "office:spreadsheet");
                        body = OdfBody::Spreadsheet(raw);
                        in_body = false;
                    }
                    "office:presentation" if in_body => {
                        let raw = capture_raw_until(&mut reader, "office:presentation");
                        body = OdfBody::Presentation(raw);
                        in_body = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(_)) | Ok(Event::Empty(_)) => {}
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    (body, automatic_styles)
}

// ── Text body ─────────────────────────────────────────────────────────────────

fn parse_text_body(reader: &mut Reader<&[u8]>, diags: &mut Vec<Diagnostic>) -> Vec<TextBlock> {
    parse_text_blocks(reader, "office:text", diags)
}

fn parse_text_blocks(
    reader: &mut Reader<&[u8]>,
    end_tag: &str,
    diags: &mut Vec<Diagnostic>,
) -> Vec<TextBlock> {
    let mut blocks = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = element_name(e.name().as_ref());
                // Extract attributes before clearing buf in recursive calls
                let attrs = collect_attrs(e);
                buf.clear();
                let block = match name.as_str() {
                    "text:p" => Some(TextBlock::Paragraph(parse_paragraph_attrs(&attrs, reader, diags))),
                    "text:h" => Some(TextBlock::Heading(parse_heading_attrs(&attrs, reader, diags))),
                    "text:list" => Some(TextBlock::List(parse_list_attrs(&attrs, reader, diags))),
                    "table:table" => Some(TextBlock::Table(parse_table_attrs(&attrs, reader, diags))),
                    "text:section" => Some(TextBlock::Section(parse_section_attrs(&attrs, reader, diags))),
                    "draw:frame" => Some(TextBlock::Frame(parse_frame_attrs(&attrs, reader))),
                    "text:soft-page-break" => {
                        skip_element(reader);
                        None
                    }
                    _ => {
                        let raw = capture_raw_from_attrs(&name, &attrs, reader);
                        Some(TextBlock::Unknown { name, raw })
                    }
                };
                if let Some(b) = block {
                    blocks.push(b);
                }
                continue; // buf was already cleared
            }
            Ok(Event::Empty(ref e)) => {
                let name = element_name(e.name().as_ref());
                match name.as_str() {
                    "text:p" => {
                        let style_name = attr_from_list(
                            &collect_attrs(e), "text:style-name");
                        blocks.push(TextBlock::Paragraph(Paragraph {
                            style_name,
                            ..Default::default()
                        }));
                    }
                    "text:soft-page-break" => {}
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                if element_name(e.name().as_ref()) == end_tag {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    blocks
}

// ── Paragraph / Heading ───────────────────────────────────────────────────────

fn parse_paragraph_attrs(
    attrs: &[(String, String)],
    reader: &mut Reader<&[u8]>,
    diags: &mut Vec<Diagnostic>,
) -> Paragraph {
    Paragraph {
        style_name: attr_from_list(attrs, "text:style-name"),
        cond_style_name: attr_from_list(attrs, "text:cond-style-name"),
        content: parse_inlines(reader, "text:p", diags),
    }
}

fn parse_heading_attrs(
    attrs: &[(String, String)],
    reader: &mut Reader<&[u8]>,
    diags: &mut Vec<Diagnostic>,
) -> Heading {
    let outline_level = attr_from_list(attrs, "text:outline-level")
        .and_then(|s| s.parse::<u32>().ok());
    Heading {
        style_name: attr_from_list(attrs, "text:style-name"),
        outline_level,
        is_list_header: attr_from_list(attrs, "text:is-list-header")
            .map(|s| s == "true")
            .unwrap_or(false),
        content: parse_inlines(reader, "text:h", diags),
    }
}

// ── List ──────────────────────────────────────────────────────────────────────

fn parse_list_attrs(
    attrs: &[(String, String)],
    reader: &mut Reader<&[u8]>,
    diags: &mut Vec<Diagnostic>,
) -> List {
    let style_name = attr_from_list(attrs, "text:style-name");
    let continue_numbering = attr_from_list(attrs, "text:continue-numbering")
        .map(|s| s == "true")
        .unwrap_or(false);
    let mut items = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = element_name(e.name().as_ref());
                let item_attrs = collect_attrs(e);
                buf.clear();
                match tag.as_str() {
                    "text:list-item" | "text:list-header" => {
                        let start_value = attr_from_list(&item_attrs, "text:start-value")
                            .and_then(|s| s.parse::<u32>().ok());
                        let end = if tag == "text:list-item" { "text:list-item" } else { "text:list-header" };
                        let content = parse_text_blocks(reader, end, diags);
                        items.push(ListItem { start_value, content });
                    }
                    _ => { skip_element(reader); }
                }
                continue;
            }
            Ok(Event::Empty(_)) => {}
            Ok(Event::End(ref e)) => {
                if element_name(e.name().as_ref()) == "text:list" { break; }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    List { style_name, continue_numbering, items }
}

// ── Table ─────────────────────────────────────────────────────────────────────

fn parse_table_attrs(
    attrs: &[(String, String)],
    reader: &mut Reader<&[u8]>,
    diags: &mut Vec<Diagnostic>,
) -> Table {
    let style_name = attr_from_list(attrs, "table:style-name");
    let name = attr_from_list(attrs, "table:name");
    let mut rows = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = element_name(e.name().as_ref());
                let row_attrs = collect_attrs(e);
                buf.clear();
                match tag.as_str() {
                    "table:table-row" => {
                        rows.push(parse_table_row_attrs(&row_attrs, reader, diags));
                    }
                    "table:table-header-rows" => {
                        let header_rows = parse_header_rows(reader, diags);
                        rows.extend(header_rows);
                    }
                    _ => { skip_element(reader); }
                }
                continue;
            }
            Ok(Event::Empty(_)) => {}
            Ok(Event::End(ref e)) => {
                if element_name(e.name().as_ref()) == "table:table" { break; }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Table { style_name, name, rows }
}

fn parse_header_rows(
    reader: &mut Reader<&[u8]>,
    diags: &mut Vec<Diagnostic>,
) -> Vec<TableRow> {
    let mut rows = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = element_name(e.name().as_ref());
                let row_attrs = collect_attrs(e);
                buf.clear();
                if tag == "table:table-row" {
                    rows.push(parse_table_row_attrs(&row_attrs, reader, diags));
                } else {
                    skip_element(reader);
                }
                continue;
            }
            Ok(Event::End(ref e)) => {
                if element_name(e.name().as_ref()) == "table:table-header-rows" { break; }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    rows
}

fn parse_table_row_attrs(
    attrs: &[(String, String)],
    reader: &mut Reader<&[u8]>,
    diags: &mut Vec<Diagnostic>,
) -> TableRow {
    let style_name = attr_from_list(attrs, "table:style-name");
    let mut cells = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = element_name(e.name().as_ref());
                let covered = tag == "table:covered-table-cell";
                let cell_attrs = collect_attrs(e);
                buf.clear();
                if tag == "table:table-cell" || covered {
                    let end = if covered { "table:covered-table-cell" } else { "table:table-cell" };
                    cells.push(parse_table_cell_attrs(&cell_attrs, covered, end, reader, diags));
                } else {
                    skip_element(reader);
                }
                continue;
            }
            Ok(Event::Empty(ref e)) => {
                let tag = element_name(e.name().as_ref());
                let covered = tag == "table:covered-table-cell";
                if tag == "table:table-cell" || covered {
                    let cell_attrs = collect_attrs(e);
                    cells.push(TableCell {
                        style_name: attr_from_list(&cell_attrs, "table:style-name"),
                        value_type: attr_from_list(&cell_attrs, "office:value-type"),
                        raw_value: cell_raw_value_attrs(&cell_attrs),
                        col_span: attr_from_list(&cell_attrs, "table:number-columns-spanned")
                            .and_then(|s| s.parse().ok()),
                        row_span: attr_from_list(&cell_attrs, "table:number-rows-spanned")
                            .and_then(|s| s.parse().ok()),
                        covered,
                        content: Vec::new(),
                    });
                }
            }
            Ok(Event::End(ref e)) => {
                if element_name(e.name().as_ref()) == "table:table-row" { break; }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    TableRow { style_name, cells }
}

fn parse_table_cell_attrs(
    attrs: &[(String, String)],
    covered: bool,
    end_tag: &str,
    reader: &mut Reader<&[u8]>,
    diags: &mut Vec<Diagnostic>,
) -> TableCell {
    TableCell {
        style_name: attr_from_list(attrs, "table:style-name"),
        value_type: attr_from_list(attrs, "office:value-type"),
        raw_value: cell_raw_value_attrs(attrs),
        col_span: attr_from_list(attrs, "table:number-columns-spanned").and_then(|s| s.parse().ok()),
        row_span: attr_from_list(attrs, "table:number-rows-spanned").and_then(|s| s.parse().ok()),
        covered,
        content: parse_text_blocks(reader, end_tag, diags),
    }
}

fn cell_raw_value_attrs(attrs: &[(String, String)]) -> Option<String> {
    for key in ["office:value", "office:date-value", "office:time-value",
                "office:boolean-value", "office:string-value"] {
        if let Some(v) = attr_from_list(attrs, key) {
            return Some(v);
        }
    }
    None
}

// ── Section ───────────────────────────────────────────────────────────────────

fn parse_section_attrs(
    attrs: &[(String, String)],
    reader: &mut Reader<&[u8]>,
    diags: &mut Vec<Diagnostic>,
) -> Section {
    Section {
        style_name: attr_from_list(attrs, "text:style-name"),
        name: attr_from_list(attrs, "text:name"),
        protected: attr_from_list(attrs, "text:protected")
            .map(|s| s == "true")
            .unwrap_or(false),
        content: parse_text_blocks(reader, "text:section", diags),
    }
}

// ── Frame ─────────────────────────────────────────────────────────────────────

fn parse_frame_attrs(
    attrs: &[(String, String)],
    reader: &mut Reader<&[u8]>,
) -> Frame {
    let style_name = attr_from_list(attrs, "draw:style-name");
    let name = attr_from_list(attrs, "draw:name");
    let anchor_type = attr_from_list(attrs, "text:anchor-type");
    let width = attr_from_list(attrs, "svg:width");
    let height = attr_from_list(attrs, "svg:height");
    let content = parse_frame_content(reader);
    Frame { style_name, name, anchor_type, width, height, content }
}

fn parse_frame_content(reader: &mut Reader<&[u8]>) -> FrameContent {
    let mut content = FrameContent::Empty;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = element_name(e.name().as_ref());
                let attrs = collect_attrs(e);
                buf.clear();
                match tag.as_str() {
                    "draw:image" => {
                        let href = attr_from_list(&attrs, "xlink:href").unwrap_or_default();
                        let mime_type = attr_from_list(&attrs, "draw:mime-type");
                        skip_element(reader);
                        content = FrameContent::Image { href, mime_type };
                    }
                    "draw:text-box" => {
                        let text = parse_text_blocks(reader, "draw:text-box", &mut Vec::new());
                        content = FrameContent::TextBox(text);
                    }
                    _ => {
                        let raw = capture_raw_from_name_attrs(&tag, &attrs, reader);
                        content = FrameContent::Other(raw);
                    }
                }
                continue;
            }
            Ok(Event::Empty(ref e)) => {
                let tag = element_name(e.name().as_ref());
                if tag == "draw:image" {
                    let attrs = collect_attrs(e);
                    let href = attr_from_list(&attrs, "xlink:href").unwrap_or_default();
                    let mime_type = attr_from_list(&attrs, "draw:mime-type");
                    content = FrameContent::Image { href, mime_type };
                }
            }
            Ok(Event::End(ref e)) => {
                if element_name(e.name().as_ref()) == "draw:frame" { break; }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    content
}

// ── Inlines ───────────────────────────────────────────────────────────────────

fn parse_inlines(
    reader: &mut Reader<&[u8]>,
    end_tag: &str,
    diags: &mut Vec<Diagnostic>,
) -> Vec<Inline> {
    let mut inlines = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(ref e)) => {
                let s = e.decode().unwrap_or_default().into_owned();
                if !s.is_empty() {
                    inlines.push(Inline::Text(s));
                }
            }
            Ok(Event::Start(ref e)) => {
                let name = element_name(e.name().as_ref());
                let attrs = collect_attrs(e);
                buf.clear();
                match name.as_str() {
                    "text:span" => {
                        let span = Span {
                            style_name: attr_from_list(&attrs, "text:style-name"),
                            content: parse_inlines(reader, "text:span", diags),
                        };
                        inlines.push(Inline::Span(span));
                    }
                    "text:a" => {
                        let link = Hyperlink {
                            href: attr_from_list(&attrs, "xlink:href"),
                            title: attr_from_list(&attrs, "xlink:title"),
                            style_name: attr_from_list(&attrs, "text:style-name"),
                            content: parse_inlines(reader, "text:a", diags),
                        };
                        inlines.push(Inline::Hyperlink(link));
                    }
                    "text:note" => {
                        let note = parse_note_attrs(&attrs, reader, diags);
                        inlines.push(Inline::Note(note));
                    }
                    "draw:frame" => {
                        let frame = parse_frame_attrs(&attrs, reader);
                        inlines.push(Inline::Frame(frame));
                    }
                    // Common field elements
                    "text:page-number" | "text:date" | "text:time" | "text:author-name"
                    | "text:author-initials" | "text:chapter" | "text:file-name"
                    | "text:sequence" | "text:reference-ref" | "text:bookmark-ref" => {
                        let field_name = name.clone();
                        let value = read_text_until(reader, &name);
                        inlines.push(Inline::Field { name: field_name, value });
                    }
                    _ => {
                        let raw = capture_raw_from_name_attrs(&name, &attrs, reader);
                        inlines.push(Inline::Unknown { name, raw });
                    }
                }
                continue;
            }
            Ok(Event::Empty(ref e)) => {
                let name = element_name(e.name().as_ref());
                match name.as_str() {
                    "text:line-break" => inlines.push(Inline::LineBreak),
                    "text:tab" => inlines.push(Inline::Tab),
                    "text:s" => {
                        let count = collect_attrs(e)
                            .into_iter()
                            .find(|(k, _)| k == "text:c")
                            .and_then(|(_, v)| v.parse::<u32>().ok())
                            .unwrap_or(1);
                        inlines.push(Inline::Space { count });
                    }
                    "text:soft-page-break" => inlines.push(Inline::SoftPageBreak),
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                if element_name(e.name().as_ref()) == end_tag { break; }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    inlines
}

fn parse_note_attrs(
    attrs: &[(String, String)],
    reader: &mut Reader<&[u8]>,
    diags: &mut Vec<Diagnostic>,
) -> Note {
    let note_class = match attr_from_list(attrs, "text:note-class").as_deref() {
        Some("endnote") => NoteClass::Endnote,
        _ => NoteClass::Footnote,
    };
    let id = attr_from_list(attrs, "text:id");
    let mut citation = None;
    let mut body = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = element_name(e.name().as_ref());
                buf.clear();
                match tag.as_str() {
                    "text:note-citation" => {
                        citation = Some(read_text_until(reader, "text:note-citation"));
                    }
                    "text:note-body" => {
                        body = parse_text_blocks(reader, "text:note-body", diags);
                    }
                    _ => { skip_element(reader); }
                }
                continue;
            }
            Ok(Event::End(ref e)) => {
                if element_name(e.name().as_ref()) == "text:note" { break; }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    Note { note_class, id, citation, body }
}

// ── styles.xml ────────────────────────────────────────────────────────────────

fn parse_styles_xml(
    xml: &str,
    _diags: &mut Vec<Diagnostic>,
) -> (Vec<StyleEntry>, Vec<PageLayout>) {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut named_styles = Vec::new();
    let mut page_layouts = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = element_name(e.name().as_ref());
                match name.as_str() {
                    "office:styles" | "office:automatic-styles" => {
                        let styles = parse_styles_block(&mut reader, &name);
                        named_styles.extend(styles);
                    }
                    "style:page-layout" => {
                        let attrs = collect_attrs(e);
                        buf.clear();
                        let layout = parse_page_layout_attrs(&attrs, &mut reader);
                        page_layouts.push(layout);
                        continue;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    (named_styles, page_layouts)
}

fn parse_styles_block(
    reader: &mut Reader<&[u8]>,
    end_tag: &str,
) -> Vec<StyleEntry> {
    let mut styles = Vec::new();
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = element_name(e.name().as_ref());
                let attrs = collect_attrs(e);
                buf.clear();
                match name.as_str() {
                    "style:style" => {
                        let entry = parse_style_element_attrs(&attrs, reader);
                        styles.push(entry);
                    }
                    _ => { skip_element(reader); }
                }
                continue;
            }
            Ok(Event::Empty(ref e)) => {
                if element_name(e.name().as_ref()) == "style:style" {
                    let attrs = collect_attrs(e);
                    styles.push(StyleEntry {
                        name: attr_from_list(&attrs, "style:name").unwrap_or_default(),
                        family: attr_from_list(&attrs, "style:family"),
                        display_name: attr_from_list(&attrs, "style:display-name"),
                        parent_style_name: attr_from_list(&attrs, "style:parent-style-name"),
                        list_style_name: attr_from_list(&attrs, "style:list-style-name"),
                        ..Default::default()
                    });
                }
            }
            Ok(Event::End(ref e)) => {
                if element_name(e.name().as_ref()) == end_tag { break; }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    styles
}

fn parse_style_element_attrs(
    attrs: &[(String, String)],
    reader: &mut Reader<&[u8]>,
) -> StyleEntry {
    let mut entry = StyleEntry {
        name: attr_from_list(attrs, "style:name").unwrap_or_default(),
        family: attr_from_list(attrs, "style:family"),
        display_name: attr_from_list(attrs, "style:display-name"),
        parent_style_name: attr_from_list(attrs, "style:parent-style-name"),
        list_style_name: attr_from_list(attrs, "style:list-style-name"),
        ..Default::default()
    };
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = element_name(e.name().as_ref());
                let prop_attrs = collect_attrs(e);
                buf.clear();
                match name.as_str() {
                    "style:text-properties" => {
                        parse_text_props_into(&prop_attrs, &mut entry.text_props);
                        skip_element_children(reader, "style:text-properties");
                    }
                    "style:paragraph-properties" => {
                        parse_para_props_into(&prop_attrs, &mut entry.para_props);
                        skip_element_children(reader, "style:paragraph-properties");
                    }
                    _ => { skip_element(reader); }
                }
                continue;
            }
            Ok(Event::End(ref e)) => {
                if element_name(e.name().as_ref()) == "style:style" { break; }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    entry
}

fn parse_text_props_into(attrs: &[(String, String)], props: &mut TextProperties) {
    for (key, val) in attrs {
        match key.as_str() {
            "fo:font-weight" => props.bold = val == "bold",
            "fo:font-style" => props.italic = val == "italic",
            "style:text-underline-style" => props.underline = val != "none" && !val.is_empty(),
            "style:text-line-through-style" => props.strikethrough = val != "none" && !val.is_empty(),
            "style:text-position" => {
                if val.starts_with("sub") { props.subscript = true; }
                else if val.starts_with("super") { props.superscript = true; }
            }
            "fo:color" => props.color = Some(val.clone()),
            "fo:background-color" => props.background_color = Some(val.clone()),
            "fo:font-size" if props.font_size.is_none() => props.font_size = Some(val.clone()),
            "style:font-size-asian" if props.font_size.is_none() => props.font_size = Some(val.clone()),
            "style:font-name" if props.font_name.is_none() => props.font_name = Some(val.clone()),
            "fo:font-family" if props.font_name.is_none() => props.font_name = Some(val.clone()),
            _ => {}
        }
    }
}

fn parse_para_props_into(attrs: &[(String, String)], props: &mut ParagraphProperties) {
    for (key, val) in attrs {
        match key.as_str() {
            "fo:text-align" => {
                if val != "start" { props.align = Some(val.clone()); }
            }
            "fo:margin-left" => props.margin_left = non_zero_measure(val),
            "fo:margin-right" => props.margin_right = non_zero_measure(val),
            "fo:margin-top" => props.margin_top = non_zero_measure(val),
            "fo:margin-bottom" => props.margin_bottom = non_zero_measure(val),
            "fo:text-indent" => props.text_indent = non_zero_measure(val),
            "fo:line-height" => {
                if val != "100%" && val != "normal" { props.line_height = Some(val.clone()); }
            }
            "fo:border" => props.border = Some(val.clone()),
            "fo:background-color" => props.background_color = Some(val.clone()),
            "fo:keep-together" => props.keep_together = val == "always",
            "fo:keep-with-next" => props.keep_with_next = val == "always",
            "fo:break-before" => props.page_break_before = matches!(val.as_str(), "page" | "even-page" | "odd-page"),
            "fo:break-after" => props.page_break_after = matches!(val.as_str(), "page" | "even-page" | "odd-page"),
            _ => {}
        }
    }
}

fn parse_page_layout_attrs(
    attrs: &[(String, String)],
    reader: &mut Reader<&[u8]>,
) -> PageLayout {
    let mut layout = PageLayout {
        name: attr_from_list(attrs, "style:name").unwrap_or_default(),
        ..Default::default()
    };
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let name = element_name(e.name().as_ref());
                if name == "style:page-layout-properties" {
                    for (key, val) in collect_attrs(e) {
                        match key.as_str() {
                            "fo:page-width" => layout.page_width = Some(val),
                            "fo:page-height" => layout.page_height = Some(val),
                            "fo:margin-top" => layout.margin_top = Some(val),
                            "fo:margin-bottom" => layout.margin_bottom = Some(val),
                            "fo:margin-left" => layout.margin_left = Some(val),
                            "fo:margin-right" => layout.margin_right = Some(val),
                            "style:print-orientation" => layout.print_orientation = Some(val),
                            _ => {}
                        }
                    }
                }
                buf.clear();
                skip_element_children(reader, &name);
                continue;
            }
            Ok(Event::End(ref e)) => {
                if element_name(e.name().as_ref()) == "style:page-layout" { break; }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    layout
}

// ── meta.xml ─────────────────────────────────────────────────────────────────

fn parse_meta_xml(xml: &str) -> OdfMeta {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut meta = OdfMeta::default();
    let mut in_meta = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = element_name(e.name().as_ref());
                if name == "office:meta" {
                    in_meta = true;
                } else if in_meta {
                    buf.clear();
                    let text = read_text_until(&mut reader, &name);
                    match name.as_str() {
                        "dc:title" => meta.title = Some(text),
                        "dc:creator" => meta.creator = Some(text),
                        "dc:description" => meta.description = Some(text),
                        "dc:subject" => meta.subject = Some(text),
                        "dc:language" => meta.language = Some(text),
                        "meta:creation-date" => meta.creation_date = Some(text),
                        "dc:date" => meta.modification_date = Some(text),
                        "meta:editing-duration" => meta.editing_duration = Some(text),
                        "meta:generator" => meta.generator = Some(text),
                        "meta:keyword" => meta.keywords.push(text),
                        _ => {}
                    }
                    continue;
                }
            }
            Ok(Event::Empty(ref e)) if in_meta => {
                if element_name(e.name().as_ref()) == "meta:document-statistic" {
                    let mut stats = DocumentStatistics::default();
                    for (key, val) in collect_attrs(e) {
                        match key.as_str() {
                            "meta:page-count" => stats.page_count = val.parse().ok(),
                            "meta:paragraph-count" => stats.paragraph_count = val.parse().ok(),
                            "meta:word-count" => stats.word_count = val.parse().ok(),
                            "meta:character-count" => stats.character_count = val.parse().ok(),
                            "meta:table-count" => stats.table_count = val.parse().ok(),
                            "meta:image-count" => stats.image_count = val.parse().ok(),
                            "meta:object-count" => stats.object_count = val.parse().ok(),
                            _ => {}
                        }
                    }
                    meta.document_statistics = Some(stats);
                }
            }
            Ok(Event::End(ref e)) => {
                if element_name(e.name().as_ref()) == "office:meta" { in_meta = false; }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }

    meta
}

// ── Low-level helpers ─────────────────────────────────────────────────────────

/// Extract the local element name including namespace prefix as a `String`.
fn element_name(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw).to_string()
}

/// Collect all attributes from a `BytesStart` into an owned `(key, value)` list.
fn collect_attrs(e: &BytesStart<'_>) -> Vec<(String, String)> {
    e.attributes()
        .flatten()
        .map(|a| {
            let k = String::from_utf8_lossy(a.key.as_ref()).to_string();
            let v = String::from_utf8_lossy(&a.value).to_string();
            (k, v)
        })
        .collect()
}

/// Look up an attribute value by key in a collected attrs list.
fn attr_from_list(attrs: &[(String, String)], key: &str) -> Option<String> {
    attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

/// Skip an element and all its children.
fn skip_element(reader: &mut Reader<&[u8]>) {
    let mut depth = 1u32;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(_)) => {
                depth -= 1;
                if depth == 0 { break; }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
}

/// Skip children of an element until its closing tag (we are already inside the element).
///
/// When `depth == 0` and we see the closing `end_tag`, we stop.
fn skip_element_children(reader: &mut Reader<&[u8]>, end_tag: &str) {
    let mut depth = 0u32;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(ref e)) => {
                if depth == 0 && element_name(e.name().as_ref()) == end_tag { break; }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
}

/// Collect all text content until the closing tag `end_tag`.
fn read_text_until(reader: &mut Reader<&[u8]>, end_tag: &str) -> String {
    let mut text = String::new();
    let mut depth = 0u32;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(ref e)) => {
                text.push_str(&e.decode().unwrap_or_default());
            }
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(ref e)) => {
                if depth == 0 && element_name(e.name().as_ref()) == end_tag { break; }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    text
}

/// Capture inner content of an element as a raw XML string.
fn capture_raw_until(reader: &mut Reader<&[u8]>, end_tag: &str) -> String {
    let mut raw = String::new();
    let mut depth = 0u32;
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(ref e)) => {
                raw.push_str(&e.decode().unwrap_or_default());
            }
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let name = element_name(e.name().as_ref());
                raw.push_str(&format!("<{name}"));
                for (k, v) in collect_attrs(e) {
                    raw.push_str(&format!(" {k}=\"{v}\""));
                }
                raw.push('>');
            }
            Ok(Event::End(ref e)) => {
                let name = element_name(e.name().as_ref());
                if depth == 0 && name == end_tag { break; }
                raw.push_str(&format!("</{name}>"));
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Empty(ref e)) => {
                let name = element_name(e.name().as_ref());
                raw.push_str(&format!("<{name}"));
                for (k, v) in collect_attrs(e) {
                    raw.push_str(&format!(" {k}=\"{v}\""));
                }
                raw.push_str("/>");
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    raw
}

/// Capture an element's full XML: opening tag (with given name and pre-collected attrs) + children + closing tag.
fn capture_raw_from_name_attrs(
    name: &str,
    attrs: &[(String, String)],
    reader: &mut Reader<&[u8]>,
) -> String {
    let mut raw = format!("<{name}");
    for (k, v) in attrs {
        raw.push_str(&format!(" {k}=\"{v}\""));
    }
    raw.push('>');
    let inner = capture_raw_until(reader, name);
    raw.push_str(&inner);
    raw.push_str(&format!("</{name}>"));
    raw
}

/// Alias used in parse_text_blocks for unknown elements.
fn capture_raw_from_attrs(
    name: &str,
    attrs: &[(String, String)],
    reader: &mut Reader<&[u8]>,
) -> String {
    capture_raw_from_name_attrs(name, attrs, reader)
}

/// Return `Some(s)` if `s` is not an "effectively zero" measurement.
fn non_zero_measure(val: &str) -> Option<String> {
    let trimmed = val.trim();
    if trimmed.is_empty()
        || trimmed == "0"
        || trimmed == "0cm"
        || trimmed == "0in"
        || trimmed == "0pt"
        || trimmed == "0mm"
    {
        None
    } else {
        Some(val.to_string())
    }
}
