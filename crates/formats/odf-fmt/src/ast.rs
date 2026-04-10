//! AST types for ODF documents.
//!
//! These types represent parsed ODF document content independent of any
//! downstream IR or format. The primary target is `.odt` (text documents),
//! with partial support for `.ods` (spreadsheets) and `.odp` (presentations)
//! via the raw-content fallback.

use std::collections::HashMap;

// ── Document ──────────────────────────────────────────────────────────────────

/// A parsed ODF document (`.odt`, `.ods`, `.odp`, or flat `.fodt`).
#[derive(Debug, Clone, Default)]
pub struct OdfDocument {
    /// The MIME type, e.g. `application/vnd.oasis.opendocument.text`.
    pub mimetype: String,
    /// The document body.
    pub body: OdfBody,
    /// Automatic styles declared in `content.xml` `<office:automatic-styles>`.
    pub automatic_styles: Vec<StyleEntry>,
    /// Named styles declared in `styles.xml` `<office:styles>`.
    pub named_styles: Vec<StyleEntry>,
    /// Page layout and master page definitions from `styles.xml`.
    pub page_layouts: Vec<PageLayout>,
    /// Document metadata from `meta.xml`.
    pub meta: OdfMeta,
    /// Embedded images keyed by path within the ZIP (e.g. `"Pictures/img1.png"`).
    pub images: HashMap<String, Vec<u8>>,
}

/// The body content of an ODF document.
#[derive(Debug, Clone, Default)]
pub enum OdfBody {
    /// Text document body (`office:text`).
    Text(Vec<TextBlock>),
    /// Spreadsheet body (`office:spreadsheet`) — raw XML for now.
    Spreadsheet(String),
    /// Presentation body (`office:presentation`) — raw XML for now.
    Presentation(String),
    /// Unknown or empty body.
    #[default]
    Empty,
}

// ── Text blocks ───────────────────────────────────────────────────────────────

/// A block-level element in the text body.
#[derive(Debug, Clone)]
pub enum TextBlock {
    Paragraph(Paragraph),
    Heading(Heading),
    List(List),
    Table(Table),
    Section(Section),
    Frame(Frame),
    /// An element that has no specific representation in this AST.
    /// Preserved as raw XML bytes for roundtrip fidelity.
    Unknown { name: String, raw: String },
}

/// `<text:p>` — a paragraph.
#[derive(Debug, Clone, Default)]
pub struct Paragraph {
    /// Value of `text:style-name`.
    pub style_name: Option<String>,
    /// Value of `text:cond-style-name`.
    pub cond_style_name: Option<String>,
    /// Inline content.
    pub content: Vec<Inline>,
}

/// `<text:h>` — a heading.
#[derive(Debug, Clone, Default)]
pub struct Heading {
    /// Value of `text:style-name`.
    pub style_name: Option<String>,
    /// Value of `text:outline-level` (1–10).
    pub outline_level: Option<u32>,
    /// Whether the heading is numbered.
    pub is_list_header: bool,
    /// Inline content.
    pub content: Vec<Inline>,
}

/// `<text:list>` — a list (ordered or unordered).
#[derive(Debug, Clone, Default)]
pub struct List {
    /// Value of `text:style-name` (list style name).
    pub style_name: Option<String>,
    /// Whether this list continues a previous list.
    pub continue_numbering: bool,
    /// List items.
    pub items: Vec<ListItem>,
}

/// `<text:list-item>` — a list item.
#[derive(Debug, Clone, Default)]
pub struct ListItem {
    /// Value of `text:start-value` (overrides numbering).
    pub start_value: Option<u32>,
    /// Block content inside the list item (paragraphs, sub-lists, etc.).
    pub content: Vec<TextBlock>,
}

/// `<text:section>` — a named section.
#[derive(Debug, Clone, Default)]
pub struct Section {
    pub style_name: Option<String>,
    pub name: Option<String>,
    pub protected: bool,
    pub content: Vec<TextBlock>,
}

// ── Tables ────────────────────────────────────────────────────────────────────

/// `<table:table>` — a table.
#[derive(Debug, Clone, Default)]
pub struct Table {
    pub style_name: Option<String>,
    pub name: Option<String>,
    pub rows: Vec<TableRow>,
}

/// `<table:table-row>` — a table row.
#[derive(Debug, Clone, Default)]
pub struct TableRow {
    pub style_name: Option<String>,
    pub cells: Vec<TableCell>,
}

/// `<table:table-cell>` or `<table:covered-table-cell>`.
#[derive(Debug, Clone, Default)]
pub struct TableCell {
    pub style_name: Option<String>,
    /// `office:value-type`: `"string"`, `"float"`, `"date"`, `"boolean"`, etc.
    pub value_type: Option<String>,
    /// `office:value` (numeric), `office:date-value`, `office:boolean-value`, etc.
    pub raw_value: Option<String>,
    /// Number of columns this cell spans.
    pub col_span: Option<u32>,
    /// Number of rows this cell spans.
    pub row_span: Option<u32>,
    /// Whether this cell is covered by a spanning cell.
    pub covered: bool,
    /// Block content inside the cell (usually paragraphs).
    pub content: Vec<TextBlock>,
}

// ── Frames / images ───────────────────────────────────────────────────────────

/// `<draw:frame>` — a positioned frame (may contain an image or text-box).
#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub style_name: Option<String>,
    pub name: Option<String>,
    pub anchor_type: Option<String>,
    pub width: Option<String>,
    pub height: Option<String>,
    pub content: FrameContent,
}

/// What lives inside a `<draw:frame>`.
#[derive(Debug, Clone, Default)]
pub enum FrameContent {
    /// `<draw:image>` with `xlink:href` pointing to an image in the ZIP.
    Image { href: String, mime_type: Option<String> },
    /// `<draw:text-box>` with block content.
    TextBox(Vec<TextBlock>),
    /// Anything else (preserved as raw XML).
    Other(String),
    #[default]
    Empty,
}

// ── Inline elements ───────────────────────────────────────────────────────────

/// An inline element inside a paragraph or heading.
#[derive(Debug, Clone)]
pub enum Inline {
    /// A run of plain text.
    Text(String),
    /// `<text:span>` — a styled inline run.
    Span(Span),
    /// `<text:a>` — a hyperlink.
    Hyperlink(Hyperlink),
    /// `<text:line-break>` — a forced line break.
    LineBreak,
    /// `<text:tab>` — a tab character.
    Tab,
    /// `<text:s>` — one or more consecutive spaces.
    Space { count: u32 },
    /// `<text:soft-page-break>` — a soft page break.
    SoftPageBreak,
    /// `<text:note>` — a footnote or endnote.
    Note(Note),
    /// `<draw:frame>` inline (anchor-type="as-char").
    Frame(Frame),
    /// Inline field (page number, date, etc.) — captured as raw element name + value.
    Field { name: String, value: String },
    /// An inline element not otherwise handled; preserved as raw XML string.
    Unknown { name: String, raw: String },
}

/// `<text:span>` — a styled run of inline content.
#[derive(Debug, Clone, Default)]
pub struct Span {
    /// Value of `text:style-name`.
    pub style_name: Option<String>,
    pub content: Vec<Inline>,
}

/// `<text:a>` — a hyperlink.
#[derive(Debug, Clone, Default)]
pub struct Hyperlink {
    /// `xlink:href` value.
    pub href: Option<String>,
    /// `xlink:title` value.
    pub title: Option<String>,
    pub style_name: Option<String>,
    pub content: Vec<Inline>,
}

/// `<text:note>` — a footnote or endnote.
#[derive(Debug, Clone, Default)]
pub struct Note {
    /// `text:note-class`: `"footnote"` or `"endnote"`.
    pub note_class: NoteClass,
    /// `text:id` attribute.
    pub id: Option<String>,
    /// Content of `<text:note-citation>` (the in-text marker).
    pub citation: Option<String>,
    /// Block content of `<text:note-body>`.
    pub body: Vec<TextBlock>,
}

/// Whether a note is a footnote or endnote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NoteClass {
    #[default]
    Footnote,
    Endnote,
}

// ── Styles ────────────────────────────────────────────────────────────────────

/// A style entry from `<style:style>`.
#[derive(Debug, Clone, Default)]
pub struct StyleEntry {
    /// `style:name`
    pub name: String,
    /// `style:family`: `"paragraph"`, `"text"`, `"table"`, `"table-row"`, etc.
    pub family: Option<String>,
    /// `style:display-name`
    pub display_name: Option<String>,
    /// `style:parent-style-name`
    pub parent_style_name: Option<String>,
    /// `style:list-style-name`
    pub list_style_name: Option<String>,
    /// Parsed text formatting properties.
    pub text_props: TextProperties,
    /// Parsed paragraph layout properties.
    pub para_props: ParagraphProperties,
}

/// Resolved text properties from `<style:text-properties>`.
#[derive(Debug, Clone, Default)]
pub struct TextProperties {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub subscript: bool,
    pub superscript: bool,
    /// Font name (if set).
    pub font_name: Option<String>,
    /// Font size (e.g. `"12pt"`).
    pub font_size: Option<String>,
    /// Foreground color (e.g. `"#ff0000"`).
    pub color: Option<String>,
    /// Background color.
    pub background_color: Option<String>,
}

/// Resolved paragraph properties from `<style:paragraph-properties>`.
#[derive(Debug, Clone, Default)]
pub struct ParagraphProperties {
    pub align: Option<String>,
    pub margin_left: Option<String>,
    pub margin_right: Option<String>,
    pub margin_top: Option<String>,
    pub margin_bottom: Option<String>,
    pub text_indent: Option<String>,
    pub line_height: Option<String>,
    pub border: Option<String>,
    pub background_color: Option<String>,
    pub keep_together: bool,
    pub keep_with_next: bool,
    pub page_break_before: bool,
    pub page_break_after: bool,
}

/// A page layout definition from `<style:page-layout>`.
#[derive(Debug, Clone, Default)]
pub struct PageLayout {
    pub name: String,
    pub page_width: Option<String>,
    pub page_height: Option<String>,
    pub margin_top: Option<String>,
    pub margin_bottom: Option<String>,
    pub margin_left: Option<String>,
    pub margin_right: Option<String>,
    pub print_orientation: Option<String>, // "portrait" | "landscape"
}

// ── Metadata ──────────────────────────────────────────────────────────────────

/// Document metadata from `meta.xml` `<office:meta>`.
#[derive(Debug, Clone, Default)]
pub struct OdfMeta {
    pub title: Option<String>,
    pub creator: Option<String>,
    pub description: Option<String>,
    pub subject: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
    pub keywords: Vec<String>,
    pub language: Option<String>,
    pub generator: Option<String>,
    pub editing_duration: Option<String>,
    pub document_statistics: Option<DocumentStatistics>,
}

/// Word/page counts from `<meta:document-statistic>`.
#[derive(Debug, Clone, Default)]
pub struct DocumentStatistics {
    pub page_count: Option<u32>,
    pub paragraph_count: Option<u32>,
    pub word_count: Option<u32>,
    pub character_count: Option<u32>,
    pub table_count: Option<u32>,
    pub image_count: Option<u32>,
    pub object_count: Option<u32>,
}
