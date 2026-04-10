//! ODF document writer.
//!
//! Serialises an [`OdfDocument`] back to an ODF ZIP archive.

use crate::ast::*;
use crate::error::Error;
use std::io::{Cursor, Write};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

/// Serialise an [`OdfDocument`] to ODF ZIP bytes.
pub fn emit(doc: &OdfDocument) -> Result<Vec<u8>, Error> {
    let buf = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(buf);

    let stored = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // mimetype must be the first entry, stored (uncompressed), no extra fields
    zip.start_file("mimetype", stored)?;
    zip.write_all(doc.mimetype.as_bytes())?;

    // META-INF/manifest.xml
    zip.start_file("META-INF/manifest.xml", deflated)?;
    write!(zip, "{}", build_manifest(doc))?;

    // meta.xml
    zip.start_file("meta.xml", deflated)?;
    write!(zip, "{}", build_meta_xml(&doc.meta))?;

    // styles.xml
    zip.start_file("styles.xml", deflated)?;
    write!(zip, "{}", build_styles_xml(doc))?;

    // content.xml
    zip.start_file("content.xml", deflated)?;
    write!(zip, "{}", build_content_xml(doc))?;

    // Embedded images
    for (path, data) in &doc.images {
        zip.start_file(path, deflated)?;
        zip.write_all(data)?;
    }

    let cursor = zip.finish()?;
    Ok(cursor.into_inner())
}

// ── Manifest ──────────────────────────────────────────────────────────────────

fn build_manifest(doc: &OdfDocument) -> String {
    let mut s = String::new();
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    s.push_str("<manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\" manifest:version=\"1.3\">\n");
    s.push_str(&format!(
        " <manifest:file-entry manifest:full-path=\"/\" manifest:version=\"1.3\" manifest:media-type=\"{}\"/>\n",
        xml_escape(&doc.mimetype)
    ));
    s.push_str(" <manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/>\n");
    s.push_str(" <manifest:file-entry manifest:full-path=\"styles.xml\" manifest:media-type=\"text/xml\"/>\n");
    s.push_str(" <manifest:file-entry manifest:full-path=\"meta.xml\" manifest:media-type=\"text/xml\"/>\n");
    for path in doc.images.keys() {
        let mime = mime_from_path(path);
        s.push_str(&format!(
            " <manifest:file-entry manifest:full-path=\"{path}\" manifest:media-type=\"{mime}\"/>\n"
        ));
    }
    s.push_str("</manifest:manifest>\n");
    s
}

// ── meta.xml ─────────────────────────────────────────────────────────────────

fn build_meta_xml(meta: &OdfMeta) -> String {
    let mut s = String::new();
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    s.push_str("<office:document-meta");
    s.push_str(" xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\"");
    s.push_str(" xmlns:meta=\"urn:oasis:names:tc:opendocument:xmlns:meta:1.0\"");
    s.push_str(" xmlns:dc=\"http://purl.org/dc/elements/1.1/\"");
    s.push_str(" office:version=\"1.3\"");
    s.push_str(">\n");
    s.push_str("<office:meta>\n");

    if let Some(v) = &meta.generator {
        s.push_str(&format!("<meta:generator>{}</meta:generator>\n", xml_escape(v)));
    }
    if let Some(v) = &meta.title {
        s.push_str(&format!("<dc:title>{}</dc:title>\n", xml_escape(v)));
    }
    if let Some(v) = &meta.creator {
        s.push_str(&format!("<dc:creator>{}</dc:creator>\n", xml_escape(v)));
    }
    if let Some(v) = &meta.description {
        s.push_str(&format!("<dc:description>{}</dc:description>\n", xml_escape(v)));
    }
    if let Some(v) = &meta.subject {
        s.push_str(&format!("<dc:subject>{}</dc:subject>\n", xml_escape(v)));
    }
    if let Some(v) = &meta.language {
        s.push_str(&format!("<dc:language>{}</dc:language>\n", xml_escape(v)));
    }
    if let Some(v) = &meta.creation_date {
        s.push_str(&format!("<meta:creation-date>{}</meta:creation-date>\n", xml_escape(v)));
    }
    if let Some(v) = &meta.modification_date {
        s.push_str(&format!("<dc:date>{}</dc:date>\n", xml_escape(v)));
    }
    if let Some(v) = &meta.editing_duration {
        s.push_str(&format!("<meta:editing-duration>{}</meta:editing-duration>\n", xml_escape(v)));
    }
    for kw in &meta.keywords {
        s.push_str(&format!("<meta:keyword>{}</meta:keyword>\n", xml_escape(kw)));
    }
    if let Some(stats) = &meta.document_statistics {
        s.push_str("<meta:document-statistic");
        if let Some(v) = stats.page_count { s.push_str(&format!(" meta:page-count=\"{v}\"")); }
        if let Some(v) = stats.paragraph_count { s.push_str(&format!(" meta:paragraph-count=\"{v}\"")); }
        if let Some(v) = stats.word_count { s.push_str(&format!(" meta:word-count=\"{v}\"")); }
        if let Some(v) = stats.character_count { s.push_str(&format!(" meta:character-count=\"{v}\"")); }
        if let Some(v) = stats.table_count { s.push_str(&format!(" meta:table-count=\"{v}\"")); }
        if let Some(v) = stats.image_count { s.push_str(&format!(" meta:image-count=\"{v}\"")); }
        if let Some(v) = stats.object_count { s.push_str(&format!(" meta:object-count=\"{v}\"")); }
        s.push_str("/>\n");
    }
    s.push_str("</office:meta>\n");
    s.push_str("</office:document-meta>\n");
    s
}

// ── styles.xml ────────────────────────────────────────────────────────────────

fn build_styles_xml(doc: &OdfDocument) -> String {
    let mut s = String::new();
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    s.push_str(CONTENT_NS_DECLS);
    s.push_str(" office:version=\"1.3\">\n");

    // Named styles
    s.push_str("<office:styles>\n");
    for style in &doc.named_styles {
        write_style_entry(&mut s, style);
    }
    s.push_str("</office:styles>\n");

    // Page layouts
    for layout in &doc.page_layouts {
        write_page_layout(&mut s, layout);
    }

    s.push_str("</office:document-styles>\n");
    s
}

fn write_style_entry(s: &mut String, style: &StyleEntry) {
    s.push_str("<style:style");
    s.push_str(&format!(" style:name=\"{}\"", xml_escape(&style.name)));
    if let Some(f) = &style.family {
        s.push_str(&format!(" style:family=\"{}\"", xml_escape(f)));
    }
    if let Some(d) = &style.display_name {
        s.push_str(&format!(" style:display-name=\"{}\"", xml_escape(d)));
    }
    if let Some(p) = &style.parent_style_name {
        s.push_str(&format!(" style:parent-style-name=\"{}\"", xml_escape(p)));
    }
    if let Some(l) = &style.list_style_name {
        s.push_str(&format!(" style:list-style-name=\"{}\"", xml_escape(l)));
    }
    s.push_str(">\n");

    // text-properties
    let tp = &style.text_props;
    let has_text = tp.bold || tp.italic || tp.underline || tp.strikethrough
        || tp.subscript || tp.superscript
        || tp.color.is_some() || tp.background_color.is_some()
        || tp.font_size.is_some() || tp.font_name.is_some();
    if has_text {
        s.push_str("<style:text-properties");
        if tp.bold { s.push_str(" fo:font-weight=\"bold\""); }
        if tp.italic { s.push_str(" fo:font-style=\"italic\""); }
        if tp.underline { s.push_str(" style:text-underline-style=\"solid\" style:text-underline-width=\"auto\" style:text-underline-color=\"font-color\""); }
        if tp.strikethrough { s.push_str(" style:text-line-through-style=\"solid\""); }
        if tp.subscript { s.push_str(" style:text-position=\"sub 58%\""); }
        else if tp.superscript { s.push_str(" style:text-position=\"super 58%\""); }
        if let Some(c) = &tp.color { s.push_str(&format!(" fo:color=\"{}\"", xml_escape(c))); }
        if let Some(c) = &tp.background_color { s.push_str(&format!(" fo:background-color=\"{}\"", xml_escape(c))); }
        if let Some(sz) = &tp.font_size { s.push_str(&format!(" fo:font-size=\"{}\"", xml_escape(sz))); }
        if let Some(f) = &tp.font_name { s.push_str(&format!(" style:font-name=\"{}\"", xml_escape(f))); }
        s.push_str("/>\n");
    }

    // paragraph-properties
    let pp = &style.para_props;
    let has_para = pp.align.is_some() || pp.margin_left.is_some() || pp.margin_right.is_some()
        || pp.margin_top.is_some() || pp.margin_bottom.is_some() || pp.text_indent.is_some()
        || pp.line_height.is_some() || pp.border.is_some() || pp.background_color.is_some()
        || pp.keep_together || pp.keep_with_next || pp.page_break_before || pp.page_break_after;
    if has_para {
        s.push_str("<style:paragraph-properties");
        if let Some(a) = &pp.align { s.push_str(&format!(" fo:text-align=\"{}\"", xml_escape(a))); }
        if let Some(v) = &pp.margin_left { s.push_str(&format!(" fo:margin-left=\"{}\"", xml_escape(v))); }
        if let Some(v) = &pp.margin_right { s.push_str(&format!(" fo:margin-right=\"{}\"", xml_escape(v))); }
        if let Some(v) = &pp.margin_top { s.push_str(&format!(" fo:margin-top=\"{}\"", xml_escape(v))); }
        if let Some(v) = &pp.margin_bottom { s.push_str(&format!(" fo:margin-bottom=\"{}\"", xml_escape(v))); }
        if let Some(v) = &pp.text_indent { s.push_str(&format!(" fo:text-indent=\"{}\"", xml_escape(v))); }
        if let Some(v) = &pp.line_height { s.push_str(&format!(" fo:line-height=\"{}\"", xml_escape(v))); }
        if let Some(v) = &pp.border { s.push_str(&format!(" fo:border=\"{}\"", xml_escape(v))); }
        if let Some(v) = &pp.background_color { s.push_str(&format!(" fo:background-color=\"{}\"", xml_escape(v))); }
        if pp.keep_together { s.push_str(" fo:keep-together=\"always\""); }
        if pp.keep_with_next { s.push_str(" fo:keep-with-next=\"always\""); }
        if pp.page_break_before { s.push_str(" fo:break-before=\"page\""); }
        if pp.page_break_after { s.push_str(" fo:break-after=\"page\""); }
        s.push_str("/>\n");
    }

    s.push_str("</style:style>\n");
}

fn write_page_layout(s: &mut String, layout: &PageLayout) {
    s.push_str(&format!("<style:page-layout style:name=\"{}\">\n", xml_escape(&layout.name)));
    let has_props = layout.page_width.is_some() || layout.page_height.is_some()
        || layout.margin_top.is_some() || layout.print_orientation.is_some();
    if has_props {
        s.push_str("<style:page-layout-properties");
        if let Some(v) = &layout.page_width { s.push_str(&format!(" fo:page-width=\"{}\"", xml_escape(v))); }
        if let Some(v) = &layout.page_height { s.push_str(&format!(" fo:page-height=\"{}\"", xml_escape(v))); }
        if let Some(v) = &layout.margin_top { s.push_str(&format!(" fo:margin-top=\"{}\"", xml_escape(v))); }
        if let Some(v) = &layout.margin_bottom { s.push_str(&format!(" fo:margin-bottom=\"{}\"", xml_escape(v))); }
        if let Some(v) = &layout.margin_left { s.push_str(&format!(" fo:margin-left=\"{}\"", xml_escape(v))); }
        if let Some(v) = &layout.margin_right { s.push_str(&format!(" fo:margin-right=\"{}\"", xml_escape(v))); }
        if let Some(v) = &layout.print_orientation { s.push_str(&format!(" style:print-orientation=\"{}\"", xml_escape(v))); }
        s.push_str("/>\n");
    }
    s.push_str("</style:page-layout>\n");
}

// ── content.xml ──────────────────────────────────────────────────────────────

const CONTENT_NS_DECLS: &str = "<office:document-styles\
 xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\"\
 xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\"\
 xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\"\
 xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\"\
 xmlns:draw=\"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0\"\
 xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\"\
 xmlns:xlink=\"http://www.w3.org/1999/xlink\"\
 xmlns:meta=\"urn:oasis:names:tc:opendocument:xmlns:meta:1.0\"\
 xmlns:dc=\"http://purl.org/dc/elements/1.1/\"\
 xmlns:svg=\"urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0\"";

fn build_content_xml(doc: &OdfDocument) -> String {
    let mut s = String::new();
    s.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    s.push_str("<office:document-content");
    s.push_str(" xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\"");
    s.push_str(" xmlns:style=\"urn:oasis:names:tc:opendocument:xmlns:style:1.0\"");
    s.push_str(" xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\"");
    s.push_str(" xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\"");
    s.push_str(" xmlns:draw=\"urn:oasis:names:tc:opendocument:xmlns:drawing:1.0\"");
    s.push_str(" xmlns:fo=\"urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0\"");
    s.push_str(" xmlns:xlink=\"http://www.w3.org/1999/xlink\"");
    s.push_str(" xmlns:meta=\"urn:oasis:names:tc:opendocument:xmlns:meta:1.0\"");
    s.push_str(" xmlns:svg=\"urn:oasis:names:tc:opendocument:xmlns:svg-compatible:1.0\"");
    s.push_str(" office:version=\"1.3\"");
    s.push_str(">\n");

    // Automatic styles
    s.push_str("<office:automatic-styles>\n");
    for style in &doc.automatic_styles {
        write_style_entry(&mut s, style);
    }
    s.push_str("</office:automatic-styles>\n");

    // Body
    s.push_str("<office:body>\n");
    match &doc.body {
        OdfBody::Text(blocks) => {
            s.push_str("<office:text>\n");
            for block in blocks {
                write_block(&mut s, block);
            }
            s.push_str("</office:text>\n");
        }
        OdfBody::Spreadsheet(body) => {
            s.push_str("<office:spreadsheet>\n");
            write_spreadsheet_body(&mut s, body);
            s.push_str("</office:spreadsheet>\n");
        }
        OdfBody::Presentation(body) => {
            s.push_str("<office:presentation>\n");
            write_presentation_body(&mut s, body);
            s.push_str("</office:presentation>\n");
        }
        OdfBody::Empty => {}
    }
    s.push_str("</office:body>\n");
    s.push_str("</office:document-content>\n");
    s
}

fn write_block(s: &mut String, block: &TextBlock) {
    match block {
        TextBlock::Paragraph(p) => write_paragraph(s, p),
        TextBlock::Heading(h) => write_heading(s, h),
        TextBlock::List(l) => write_list(s, l),
        TextBlock::Table(t) => write_table(s, t),
        TextBlock::Section(sec) => write_section(s, sec),
        TextBlock::Frame(f) => write_frame(s, f),
        TextBlock::Unknown { raw, .. } => s.push_str(raw),
    }
}

fn write_paragraph(s: &mut String, p: &Paragraph) {
    s.push_str("<text:p");
    if let Some(sn) = &p.style_name {
        s.push_str(&format!(" text:style-name=\"{}\"", xml_escape(sn)));
    }
    if let Some(csn) = &p.cond_style_name {
        s.push_str(&format!(" text:cond-style-name=\"{}\"", xml_escape(csn)));
    }
    s.push('>');
    for inline in &p.content {
        write_inline(s, inline);
    }
    s.push_str("</text:p>\n");
}

fn write_heading(s: &mut String, h: &Heading) {
    s.push_str("<text:h");
    if let Some(sn) = &h.style_name {
        s.push_str(&format!(" text:style-name=\"{}\"", xml_escape(sn)));
    }
    if let Some(level) = h.outline_level {
        s.push_str(&format!(" text:outline-level=\"{level}\""));
    }
    if h.is_list_header {
        s.push_str(" text:is-list-header=\"true\"");
    }
    s.push('>');
    for inline in &h.content {
        write_inline(s, inline);
    }
    s.push_str("</text:h>\n");
}

fn write_list(s: &mut String, l: &List) {
    s.push_str("<text:list");
    if let Some(sn) = &l.style_name {
        s.push_str(&format!(" text:style-name=\"{}\"", xml_escape(sn)));
    }
    if l.continue_numbering {
        s.push_str(" text:continue-numbering=\"true\"");
    }
    s.push_str(">\n");
    for item in &l.items {
        s.push_str("<text:list-item");
        if let Some(sv) = item.start_value {
            s.push_str(&format!(" text:start-value=\"{sv}\""));
        }
        s.push_str(">\n");
        for block in &item.content {
            write_block(s, block);
        }
        s.push_str("</text:list-item>\n");
    }
    s.push_str("</text:list>\n");
}

fn write_table(s: &mut String, t: &Table) {
    s.push_str("<table:table");
    if let Some(sn) = &t.style_name {
        s.push_str(&format!(" table:style-name=\"{}\"", xml_escape(sn)));
    }
    if let Some(n) = &t.name {
        s.push_str(&format!(" table:name=\"{}\"", xml_escape(n)));
    }
    s.push_str(">\n");
    for row in &t.rows {
        s.push_str("<table:table-row");
        if let Some(sn) = &row.style_name {
            s.push_str(&format!(" table:style-name=\"{}\"", xml_escape(sn)));
        }
        s.push_str(">\n");
        for cell in &row.cells {
            let tag = if cell.covered { "table:covered-table-cell" } else { "table:table-cell" };
            s.push_str(&format!("<{tag}"));
            if let Some(sn) = &cell.style_name {
                s.push_str(&format!(" table:style-name=\"{}\"", xml_escape(sn)));
            }
            if let Some(vt) = &cell.value_type {
                s.push_str(&format!(" office:value-type=\"{}\"", xml_escape(vt)));
                if let Some(rv) = &cell.raw_value {
                    let val_attr = match vt.as_str() {
                        "date" => "office:date-value",
                        "time" => "office:time-value",
                        "boolean" => "office:boolean-value",
                        "string" => "office:string-value",
                        _ => "office:value",
                    };
                    s.push_str(&format!(" {val_attr}=\"{}\"", xml_escape(rv)));
                }
            }
            if let Some(cs) = cell.col_span {
                s.push_str(&format!(" table:number-columns-spanned=\"{cs}\""));
            }
            if let Some(rs) = cell.row_span {
                s.push_str(&format!(" table:number-rows-spanned=\"{rs}\""));
            }
            if cell.content.is_empty() {
                s.push_str("/>\n");
            } else {
                s.push_str(">\n");
                for block in &cell.content {
                    write_block(s, block);
                }
                s.push_str(&format!("</{tag}>\n"));
            }
        }
        s.push_str("</table:table-row>\n");
    }
    s.push_str("</table:table>\n");
}

fn write_section(s: &mut String, sec: &Section) {
    s.push_str("<text:section");
    if let Some(sn) = &sec.style_name {
        s.push_str(&format!(" text:style-name=\"{}\"", xml_escape(sn)));
    }
    if let Some(n) = &sec.name {
        s.push_str(&format!(" text:name=\"{}\"", xml_escape(n)));
    }
    if sec.protected {
        s.push_str(" text:protected=\"true\"");
    }
    s.push_str(">\n");
    for block in &sec.content {
        write_block(s, block);
    }
    s.push_str("</text:section>\n");
}

fn write_frame(s: &mut String, f: &Frame) {
    s.push_str("<draw:frame");
    if let Some(sn) = &f.style_name {
        s.push_str(&format!(" draw:style-name=\"{}\"", xml_escape(sn)));
    }
    if let Some(n) = &f.name {
        s.push_str(&format!(" draw:name=\"{}\"", xml_escape(n)));
    }
    if let Some(at) = &f.anchor_type {
        s.push_str(&format!(" text:anchor-type=\"{}\"", xml_escape(at)));
    }
    if let Some(w) = &f.width {
        s.push_str(&format!(" svg:width=\"{}\"", xml_escape(w)));
    }
    if let Some(h) = &f.height {
        s.push_str(&format!(" svg:height=\"{}\"", xml_escape(h)));
    }
    s.push_str(">\n");
    match &f.content {
        FrameContent::Image { href, mime_type } => {
            s.push_str(&format!("<draw:image xlink:href=\"{}\" xlink:type=\"simple\" xlink:show=\"embed\" xlink:actuate=\"onLoad\"",
                xml_escape(href)));
            if let Some(mt) = mime_type {
                s.push_str(&format!(" draw:mime-type=\"{}\"", xml_escape(mt)));
            }
            s.push_str("/>\n");
        }
        FrameContent::TextBox(blocks) => {
            s.push_str("<draw:text-box>\n");
            for block in blocks {
                write_block(s, block);
            }
            s.push_str("</draw:text-box>\n");
        }
        FrameContent::Other(raw) => { s.push_str(raw); }
        FrameContent::Empty => {}
    }
    s.push_str("</draw:frame>\n");
}

fn write_inline(s: &mut String, inline: &Inline) {
    match inline {
        Inline::Text(t) => s.push_str(&xml_escape(t)),
        Inline::Span(span) => {
            s.push_str("<text:span");
            if let Some(sn) = &span.style_name {
                s.push_str(&format!(" text:style-name=\"{}\"", xml_escape(sn)));
            }
            s.push('>');
            for i in &span.content {
                write_inline(s, i);
            }
            s.push_str("</text:span>");
        }
        Inline::Hyperlink(link) => {
            s.push_str("<text:a");
            if let Some(href) = &link.href {
                s.push_str(&format!(" xlink:href=\"{}\" xlink:type=\"simple\"", xml_escape(href)));
            }
            if let Some(title) = &link.title {
                s.push_str(&format!(" xlink:title=\"{}\"", xml_escape(title)));
            }
            if let Some(sn) = &link.style_name {
                s.push_str(&format!(" text:style-name=\"{}\"", xml_escape(sn)));
            }
            s.push('>');
            for i in &link.content {
                write_inline(s, i);
            }
            s.push_str("</text:a>");
        }
        Inline::LineBreak => s.push_str("<text:line-break/>"),
        Inline::Tab => s.push_str("<text:tab/>"),
        Inline::Space { count } => {
            if *count == 1 {
                s.push_str("<text:s/>");
            } else {
                s.push_str(&format!("<text:s text:c=\"{count}\"/>"));
            }
        }
        Inline::SoftPageBreak => s.push_str("<text:soft-page-break/>"),
        Inline::Note(note) => {
            s.push_str("<text:note");
            let class = match note.note_class {
                NoteClass::Footnote => "footnote",
                NoteClass::Endnote => "endnote",
            };
            s.push_str(&format!(" text:note-class=\"{class}\""));
            if let Some(id) = &note.id {
                s.push_str(&format!(" text:id=\"{}\"", xml_escape(id)));
            }
            s.push_str(">\n");
            if let Some(cit) = &note.citation {
                s.push_str(&format!("<text:note-citation>{}</text:note-citation>\n", xml_escape(cit)));
            }
            s.push_str("<text:note-body>\n");
            for block in &note.body {
                write_block(s, block);
            }
            s.push_str("</text:note-body>\n");
            s.push_str("</text:note>");
        }
        Inline::Frame(f) => write_frame(s, f),
        Inline::Field { name, value } => {
            s.push_str(&format!("<{name}>{}</{name}>", xml_escape(value)));
        }
        Inline::Unknown { raw, .. } => s.push_str(raw),
    }
}

// ── Spreadsheet writer ────────────────────────────────────────────────────────

fn write_spreadsheet_body(s: &mut String, body: &SpreadsheetBody) {
    for sheet in &body.sheets {
        write_sheet(s, sheet);
    }
    if !body.named_ranges.is_empty() {
        s.push_str("<table:named-expressions>\n");
        for nr in &body.named_ranges {
            s.push_str(&format!("<table:named-range table:name=\"{}\"", xml_escape(&nr.name)));
            if let Some(v) = &nr.cell_range_address {
                s.push_str(&format!(" table:cell-range-address=\"{}\"", xml_escape(v)));
            }
            if let Some(v) = &nr.base_cell_address {
                s.push_str(&format!(" table:base-cell-address=\"{}\"", xml_escape(v)));
            }
            s.push_str("/>\n");
        }
        s.push_str("</table:named-expressions>\n");
    }
}

fn write_sheet(s: &mut String, sheet: &Sheet) {
    s.push_str("<table:table");
    if let Some(n) = &sheet.name { s.push_str(&format!(" table:name=\"{}\"", xml_escape(n))); }
    if let Some(sn) = &sheet.style_name { s.push_str(&format!(" table:style-name=\"{}\"", xml_escape(sn))); }
    if !sheet.print { s.push_str(" table:print=\"false\""); }
    s.push_str(">\n");

    for col in &sheet.columns {
        s.push_str("<table:table-column");
        if let Some(sn) = &col.style_name { s.push_str(&format!(" table:style-name=\"{}\"", xml_escape(sn))); }
        if let Some(ds) = &col.default_cell_style_name { s.push_str(&format!(" table:default-cell-style-name=\"{}\"", xml_escape(ds))); }
        if let Some(r) = col.repeated { s.push_str(&format!(" table:number-columns-repeated=\"{r}\"")); }
        if let Some(v) = &col.visibility { s.push_str(&format!(" table:visibility=\"{}\"", xml_escape(v))); }
        s.push_str("/>\n");
    }

    for row in &sheet.rows {
        s.push_str("<table:table-row");
        if let Some(sn) = &row.style_name { s.push_str(&format!(" table:style-name=\"{}\"", xml_escape(sn))); }
        if let Some(ds) = &row.default_cell_style_name { s.push_str(&format!(" table:default-cell-style-name=\"{}\"", xml_escape(ds))); }
        if let Some(r) = row.repeated { s.push_str(&format!(" table:number-rows-repeated=\"{r}\"")); }
        s.push_str(">\n");

        for cell in &row.cells {
            let tag = if cell.covered { "table:covered-table-cell" } else { "table:table-cell" };
            s.push_str(&format!("<{tag}"));
            if let Some(sn) = &cell.style_name { s.push_str(&format!(" table:style-name=\"{}\"", xml_escape(sn))); }
            if let Some(vt) = &cell.value_type {
                s.push_str(&format!(" office:value-type=\"{}\"", xml_escape(vt)));
                if let Some(v) = &cell.value {
                    let attr = match vt.as_str() {
                        "date" => "office:date-value",
                        "time" => "office:time-value",
                        "boolean" => "office:boolean-value",
                        "string" => "office:string-value",
                        _ => "office:value",
                    };
                    s.push_str(&format!(" {attr}=\"{}\"", xml_escape(v)));
                }
            }
            if let Some(f) = &cell.formula { s.push_str(&format!(" table:formula=\"{}\"", xml_escape(f))); }
            if let Some(cs) = cell.col_span { s.push_str(&format!(" table:number-columns-spanned=\"{cs}\"")); }
            if let Some(rs) = cell.row_span { s.push_str(&format!(" table:number-rows-spanned=\"{rs}\"")); }
            if let Some(r) = cell.repeated { s.push_str(&format!(" table:number-columns-repeated=\"{r}\"")); }
            if cell.content.is_empty() {
                s.push_str("/>\n");
            } else {
                s.push_str(">\n");
                for block in &cell.content {
                    write_block(s, block);
                }
                s.push_str(&format!("</{tag}>\n"));
            }
        }
        s.push_str("</table:table-row>\n");
    }
    s.push_str("</table:table>\n");
}

// ── Presentation writer ───────────────────────────────────────────────────────

fn write_presentation_body(s: &mut String, body: &PresentationBody) {
    for page in &body.pages {
        write_draw_page(s, page);
    }
}

fn write_draw_page(s: &mut String, page: &DrawPage) {
    s.push_str("<draw:page");
    if let Some(n) = &page.name { s.push_str(&format!(" draw:name=\"{}\"", xml_escape(n))); }
    if let Some(sn) = &page.style_name { s.push_str(&format!(" draw:style-name=\"{}\"", xml_escape(sn))); }
    if let Some(mp) = &page.master_page_name { s.push_str(&format!(" draw:master-page-name=\"{}\"", xml_escape(mp))); }
    if let Some(l) = &page.layout_name { s.push_str(&format!(" presentation:presentation-page-layout-name=\"{}\"", xml_escape(l))); }
    s.push_str(">\n");
    for shape in &page.shapes { write_draw_shape(s, shape); }
    if let Some(notes) = &page.notes {
        s.push_str("<presentation:notes");
        if let Some(sn) = &notes.style_name { s.push_str(&format!(" draw:style-name=\"{}\"", xml_escape(sn))); }
        s.push_str(">\n");
        for shape in &notes.shapes { write_draw_shape(s, shape); }
        s.push_str("</presentation:notes>\n");
    }
    s.push_str("</draw:page>\n");
}

fn write_draw_shape(s: &mut String, shape: &DrawShape) {
    s.push_str("<draw:frame");
    if let Some(sn) = &shape.style_name { s.push_str(&format!(" draw:style-name=\"{}\"", xml_escape(sn))); }
    if let Some(ts) = &shape.text_style_name { s.push_str(&format!(" draw:text-style-name=\"{}\"", xml_escape(ts))); }
    if let Some(n) = &shape.name { s.push_str(&format!(" draw:name=\"{}\"", xml_escape(n))); }
    if let Some(pc) = &shape.presentation_class { s.push_str(&format!(" presentation:class=\"{}\"", xml_escape(pc))); }
    if let Some(x) = &shape.x { s.push_str(&format!(" svg:x=\"{}\"", xml_escape(x))); }
    if let Some(y) = &shape.y { s.push_str(&format!(" svg:y=\"{}\"", xml_escape(y))); }
    if let Some(w) = &shape.width { s.push_str(&format!(" svg:width=\"{}\"", xml_escape(w))); }
    if let Some(h) = &shape.height { s.push_str(&format!(" svg:height=\"{}\"", xml_escape(h))); }
    s.push_str(">\n");
    match &shape.content {
        DrawShapeContent::TextBox(blocks) => {
            s.push_str("<draw:text-box>\n");
            for block in blocks { write_block(s, block); }
            s.push_str("</draw:text-box>\n");
        }
        DrawShapeContent::Image { href, mime_type } => {
            s.push_str(&format!("<draw:image xlink:href=\"{}\" xlink:type=\"simple\" xlink:show=\"embed\" xlink:actuate=\"onLoad\"",
                xml_escape(href)));
            if let Some(mt) = mime_type { s.push_str(&format!(" draw:mime-type=\"{}\"", xml_escape(mt))); }
            s.push_str("/>\n");
        }
        DrawShapeContent::Other(raw) => { s.push_str(raw); s.push('\n'); }
        DrawShapeContent::Empty => {}
    }
    s.push_str("</draw:frame>\n");
}

// ── Utilities ─────────────────────────────────────────────────────────────────

/// Escape a string for use in XML text or attribute values.
pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// Guess a MIME type from a file path extension.
fn mime_from_path(path: &str) -> &'static str {
    if path.ends_with(".png") { "image/png" }
    else if path.ends_with(".jpg") || path.ends_with(".jpeg") { "image/jpeg" }
    else if path.ends_with(".gif") { "image/gif" }
    else if path.ends_with(".svg") { "image/svg+xml" }
    else if path.ends_with(".tiff") || path.ends_with(".tif") { "image/tiff" }
    else if path.ends_with(".bmp") { "image/bmp" }
    else if path.ends_with(".webp") { "image/webp" }
    else { "application/octet-stream" }
}
