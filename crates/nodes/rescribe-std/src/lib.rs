//! Standard node kinds and property keys for rescribe.
//!
//! This crate provides the standard vocabulary for document representation.
//! It re-exports `rescribe-core` so users only need one import.

pub use rescribe_core::*;

/// Type-safe document builder API.
pub mod builder;

/// Standard node kind constants.
pub mod node {
    // Block-level nodes
    /// Root document container.
    pub const DOCUMENT: &str = "document";
    /// A paragraph of text.
    pub const PARAGRAPH: &str = "paragraph";
    /// A heading (use `level` property for h1-h6).
    pub const HEADING: &str = "heading";
    /// A fenced or indented code block.
    pub const CODE_BLOCK: &str = "code_block";
    /// A block quotation.
    pub const BLOCKQUOTE: &str = "blockquote";
    /// A list (use `ordered` property to distinguish).
    pub const LIST: &str = "list";
    /// An item in a list.
    pub const LIST_ITEM: &str = "list_item";
    /// A table.
    pub const TABLE: &str = "table";
    /// A row in a table.
    pub const TABLE_ROW: &str = "table_row";
    /// A cell in a table row.
    pub const TABLE_CELL: &str = "table_cell";
    /// A header cell in a table.
    pub const TABLE_HEADER: &str = "table_header";
    /// A figure with caption.
    pub const FIGURE: &str = "figure";
    /// A thematic break / horizontal rule.
    pub const HORIZONTAL_RULE: &str = "horizontal_rule";
    /// A generic block container (like HTML div).
    pub const DIV: &str = "div";
    /// Raw format-specific block content.
    pub const RAW_BLOCK: &str = "raw_block";
    /// A definition list.
    pub const DEFINITION_LIST: &str = "definition_list";
    /// A term in a definition list.
    pub const DEFINITION_TERM: &str = "definition_term";
    /// A description in a definition list.
    pub const DEFINITION_DESC: &str = "definition_desc";
    /// Caption for figures/tables.
    pub const CAPTION: &str = "caption";
    /// Table head section.
    pub const TABLE_HEAD: &str = "table_head";
    /// Table body section.
    pub const TABLE_BODY: &str = "table_body";
    /// Table foot section.
    pub const TABLE_FOOT: &str = "table_foot";
    /// A bibliography / reference list container. Children are `bibliography_entry` nodes.
    pub const BIBLIOGRAPHY: &str = "bibliography";
    /// One citation/reference entry within a `bibliography`. Children are
    /// `bibliography_field` nodes, and (for structural nesting cases such as
    /// DocBook's `biblioset` or TEI's `analytic`/`monogr`/`series` levels)
    /// nested `bibliography_entry` nodes.
    pub const BIBLIOGRAPHY_ENTRY: &str = "bibliography_entry";
    /// A single tagged field within a `bibliography_entry` (author, title,
    /// publisher, etc. — see `prop::FIELD_ROLE`). Children are ordinary
    /// inline nodes, so markup nested inside a field (e.g. an italicized
    /// journal title) is preserved rather than flattened to a string.
    pub const BIBLIOGRAPHY_FIELD: &str = "bibliography_field";
    /// One worksheet in a spreadsheet document (ADR 0015). Children are
    /// `sheet_row` nodes. A multi-sheet workbook is currently represented as
    /// multiple sibling `sheet` nodes under `document` — a dedicated
    /// `workbook` container is left for a future decision (see ADR 0015).
    pub const SHEET: &str = "sheet";
    /// One row within a `sheet`. Children are `sheet_cell` nodes.
    pub const SHEET_ROW: &str = "sheet_row";
    /// One cell within a `sheet_row`. The cell's value is a typed scalar or
    /// formula carried directly as properties on this node (see
    /// `prop::VALUE_TYPE`, `prop::VALUE`, `prop::VALUE_FORMULA`) — not
    /// nested block/inline content like `table_cell`. See ADR 0015 for why
    /// this is a distinct kind rather than a reuse of `table_cell`.
    pub const SHEET_CELL: &str = "sheet_cell";
    /// A container positioned by absolute coordinates rather than document
    /// flow (ADR 0015) — presentation shapes/slides, DOCX/PPTX text-boxes,
    /// RTF `\shp`/`\do` shape groups. Position/size/rotation/stacking order
    /// are carried as `prop::POSITION_*` properties (EMU, see ADR 0015 for
    /// the precision analysis). Children are the shape's actual content
    /// (text, image, or nested blocks) — unconstrained by this decision.
    pub const POSITIONED_CONTAINER: &str = "positioned_container";
    /// A chart (ADR 0016). Block-level, siblings with other block content.
    /// Carries `TITLE`, `prop::CHART_TYPE`, and legend/axis presence
    /// properties (`prop::CHART_LEGEND`, `prop::CHART_LEGEND_POSITION`,
    /// `prop::CHART_HAS_CATEGORY_AXIS`, `prop::CHART_HAS_VALUE_AXIS`).
    /// Children are `chart_series` nodes, one per data series. Also carries
    /// an unconditional format-namespaced raw-XML fallback (`ooxml:chart-xml`
    /// / `odf:chart-xml`) so v1's semantic subset stays lossless — see ADR
    /// 0016 Decision 4.
    pub const CHART: &str = "chart";
    /// One data series within a `chart` (ADR 0016). Carries `TITLE` and the
    /// values/categories properties (`prop::CHART_VALUES`,
    /// `prop::CHART_VALUES_REF`, `prop::CHART_CATEGORIES`,
    /// `prop::CHART_CATEGORIES_REF`).
    pub const CHART_SERIES: &str = "chart_series";

    // Inline-level nodes
    /// Plain text content (use `content` property).
    pub const TEXT: &str = "text";
    /// Emphasized text (typically italic).
    pub const EMPHASIS: &str = "emphasis";
    /// Strong text (typically bold).
    pub const STRONG: &str = "strong";
    /// Strikethrough text.
    pub const STRIKEOUT: &str = "strikeout";
    /// Underlined text.
    pub const UNDERLINE: &str = "underline";
    /// Subscript text.
    pub const SUBSCRIPT: &str = "subscript";
    /// Superscript text.
    pub const SUPERSCRIPT: &str = "superscript";
    /// Inline code.
    pub const CODE: &str = "code";
    /// A hyperlink (use `url` and optional `title` properties).
    pub const LINK: &str = "link";
    /// An image (use `url`, `alt`, optional `title` properties).
    pub const IMAGE: &str = "image";
    /// A hard line break.
    pub const LINE_BREAK: &str = "line_break";
    /// A soft line break (may render as space).
    pub const SOFT_BREAK: &str = "soft_break";
    /// A generic inline container (like HTML span).
    pub const SPAN: &str = "span";
    /// Raw format-specific inline content.
    pub const RAW_INLINE: &str = "raw_inline";
    /// A footnote reference.
    pub const FOOTNOTE_REF: &str = "footnote_ref";
    /// A footnote definition.
    pub const FOOTNOTE_DEF: &str = "footnote_def";
    /// Small caps text.
    pub const SMALL_CAPS: &str = "small_caps";
    /// All-caps text (rendered uppercase; original-case content preserved).
    pub const ALL_CAPS: &str = "all_caps";
    /// Hidden text (present in document but not displayed).
    pub const HIDDEN: &str = "hidden";
    /// Quoted text (use `quote_type` property: single/double).
    pub const QUOTED: &str = "quoted";
    /// A citation.
    pub const CITE: &str = "cite";
}

/// Standard property key constants.
pub mod prop {
    // Semantic properties (format-agnostic)
    /// Heading level (1-6).
    pub const LEVEL: &str = "level";
    /// Whether a list is ordered.
    pub const ORDERED: &str = "ordered";
    /// Programming language for code blocks.
    pub const LANGUAGE: &str = "language";
    /// URL for links and images.
    pub const URL: &str = "url";
    /// Title attribute for links and images.
    pub const TITLE: &str = "title";
    /// Alt text for images.
    pub const ALT: &str = "alt";
    /// Text content for text nodes.
    pub const CONTENT: &str = "content";
    /// Reference to an embedded resource.
    pub const RESOURCE_ID: &str = "resource";
    /// Identifier/anchor name.
    pub const ID: &str = "id";
    /// CSS classes (as list).
    pub const CLASSES: &str = "classes";
    /// Start number for ordered lists.
    pub const START: &str = "start";
    /// List style type (decimal, lower-alpha, etc.).
    pub const LIST_STYLE: &str = "list_style";
    /// Tight list (no paragraph wrapping).
    pub const TIGHT: &str = "tight";
    /// Task list item checked state.
    pub const CHECKED: &str = "checked";
    /// Format for raw blocks/inlines.
    pub const FORMAT: &str = "format";
    /// Quote type (single, double).
    pub const QUOTE_TYPE: &str = "quote_type";
    /// Footnote/reference label.
    pub const LABEL: &str = "label";
    /// Column alignment (left, center, right).
    pub const ALIGN: &str = "align";
    /// Column span for table cells.
    pub const COLSPAN: &str = "colspan";
    /// Row span for table cells.
    pub const ROWSPAN: &str = "rowspan";
    /// Role of a `bibliography_field` node: one of `author`, `editor`,
    /// `title`, `container_title`, `publisher`, `publisher_location`,
    /// `edition`, `volume`, `issue`, `page_first`, `page_last`,
    /// `identifier`, `misc`. Repeated fields (e.g. multiple authors) are
    /// represented as multiple sibling `bibliography_field` nodes sharing
    /// the same `FIELD_ROLE`, in document order.
    pub const FIELD_ROLE: &str = "field:role";
    /// Identifier scheme for a `bibliography_field` with `FIELD_ROLE ==
    /// "identifier"` (e.g. `doi`, `isbn`, `issn`, `url`).
    pub const FIELD_SCHEME: &str = "field:scheme";
    /// Structured date on a `bibliography_entry`, as a `PropValue::Map`
    /// with keys `year`, and optionally `month` / `day` (partial dates omit
    /// the missing keys). Kept as a property rather than a child node
    /// because date sub-parts are atomic, non-markup-bearing data in every
    /// schema surveyed (DocBook/JATS/TEI/OOXML) — a structured Map lets
    /// writers reformat per regional convention without re-parsing an
    /// ambiguous flat string.
    pub const DATE: &str = "date";
    /// Type of a `sheet_cell`'s value (ADR 0015): one of `string`, `number`,
    /// `currency`, `percentage`, `date`, `time`, `boolean`, or
    /// `formula-result` (the type of a formula's computed result, as
    /// distinct from the formula source text itself — see
    /// `VALUE_FORMULA`). Union of ODF's `office:value-type` (which
    /// distinguishes all of these) and OOXML SpreadsheetML's narrower
    /// `CellValue` (`Empty`/`String`/`Number`/`Boolean`/`Error`, with
    /// `Date`/`Currency` resolved indirectly via number-format strings) —
    /// see ADR 0015 Decision 2.
    pub const VALUE_TYPE: &str = "value:type";
    /// A `sheet_cell`'s value, as its string representation (ADR 0015).
    /// Kept as a string rather than a typed `PropValue::Float`/`Int` so
    /// arbitrary-precision source values (e.g. ODF's decimal attributes)
    /// round-trip exactly; readers/writers that need a numeric value parse
    /// this string using `VALUE_TYPE` to know how.
    pub const VALUE: &str = "value:data";
    /// A `sheet_cell`'s formula source text (e.g. an OpenFormula or A1-style
    /// expression), kept separate from `VALUE`/`VALUE_TYPE` (the computed
    /// result) so both survive round-trip (ADR 0015).
    pub const VALUE_FORMULA: &str = "value:formula";

    // Position properties (absolute positioning, ADR 0015)
    /// `positioned_container` horizontal offset, in EMU (914,400 per inch).
    pub const POSITION_X: &str = "position:x";
    /// `positioned_container` vertical offset, in EMU.
    pub const POSITION_Y: &str = "position:y";
    /// `positioned_container` width, in EMU.
    pub const POSITION_WIDTH: &str = "position:width";
    /// `positioned_container` height, in EMU.
    pub const POSITION_HEIGHT: &str = "position:height";
    /// `positioned_container` rotation, in degrees.
    pub const POSITION_ROTATION: &str = "position:rotation";
    /// `positioned_container` stacking order (higher paints on top).
    pub const POSITION_Z_ORDER: &str = "position:z_order";

    // Chart properties (ADR 0016)
    /// A `chart`'s type, as an open string (e.g. `bar`, `line`, `pie`,
    /// `scatter`, `radar`, `stock`, `bubble`, `surface`, `doughnut`,
    /// `of-pie`, plus format-specific 3D/combo variants) — the union of
    /// OOXML's and ODF's chart-type vocabularies, per ADR 0016 Decision 3.
    /// No closed enum, matching `NodeKind`'s existing open-string convention.
    pub const CHART_TYPE: &str = "chart:type";
    /// Whether a `chart` shows a legend (ADR 0016).
    pub const CHART_LEGEND: &str = "chart:legend";
    /// A `chart`'s legend position, as an open string (e.g. `right`,
    /// `bottom`, `top`, `left`), present only when `CHART_LEGEND` is true
    /// (ADR 0016).
    pub const CHART_LEGEND_POSITION: &str = "chart:legend-position";
    /// Whether a `chart` has a category axis (ADR 0016).
    pub const CHART_HAS_CATEGORY_AXIS: &str = "chart:has-category-axis";
    /// Whether a `chart` has a value axis (ADR 0016).
    pub const CHART_HAS_VALUE_AXIS: &str = "chart:has-value-axis";
    /// A `chart_series`'s literal values, or the cached snapshot of a
    /// reference-backed series (`PropValue::List`), per ADR 0016 Decisions 1-2.
    /// Paired with `CHART_VALUES_REF` when reference-backed; present alone
    /// when the series carries literal (non-referenced) data.
    pub const CHART_VALUES: &str = "chart:values";
    /// A `chart_series`'s cell-range reference for its values
    /// (`PropValue::String`, the verbatim range-reference formula string —
    /// OOXML's `Sheet1!$B$2:$B$5` syntax or ODF's `table:cell-range-address`
    /// syntax, stored as-is), per ADR 0016 Decision 1. Absent for
    /// literal-data series.
    pub const CHART_VALUES_REF: &str = "chart:values-ref";
    /// A `chart_series`'s literal category labels, or the cached snapshot of
    /// a reference-backed series (`PropValue::List`), analogous to
    /// `CHART_VALUES` (ADR 0016).
    pub const CHART_CATEGORIES: &str = "chart:categories";
    /// A `chart_series`'s cell-range reference for its category labels,
    /// analogous to `CHART_VALUES_REF` (ADR 0016).
    pub const CHART_CATEGORIES_REF: &str = "chart:categories-ref";
    /// Verbatim OOXML chart-part XML (e.g. `chart1.xml`), captured
    /// unconditionally on every `chart` node sourced from OOXML so v1's
    /// semantic-subset model stays lossless (ADR 0016 Decision 4).
    pub const OOXML_CHART_XML: &str = "ooxml:chart-xml";
    /// Verbatim ODF `office:chart` subtree XML, captured unconditionally on
    /// every `chart` node sourced from ODF so v1's semantic-subset model
    /// stays lossless (ADR 0016 Decision 4).
    pub const ODF_CHART_XML: &str = "odf:chart-xml";

    // Style properties (presentational)
    /// Font family.
    pub const STYLE_FONT: &str = "style:font";
    /// Font size.
    pub const STYLE_SIZE: &str = "style:size";
    /// Text color.
    pub const STYLE_COLOR: &str = "style:color";
    /// Text alignment.
    pub const STYLE_ALIGN: &str = "style:align";
    /// Background color.
    pub const STYLE_BG_COLOR: &str = "style:bg_color";
    /// Font weight.
    pub const STYLE_WEIGHT: &str = "style:weight";

    // Layout properties (positioning)
    /// Page break before.
    pub const LAYOUT_PAGE_BREAK: &str = "layout:page_break";
    /// Column specification.
    pub const LAYOUT_COLUMN: &str = "layout:column";
    /// Float positioning.
    pub const LAYOUT_FLOAT: &str = "layout:float";

    // Format-specific prefixes (for dynamic property names)
    /// HTML-specific properties prefix.
    pub const HTML_PREFIX: &str = "html:";
    /// LaTeX-specific properties prefix.
    pub const LATEX_PREFIX: &str = "latex:";
    /// DOCX-specific properties prefix.
    pub const DOCX_PREFIX: &str = "docx:";
    /// Markdown-specific properties prefix.
    pub const MD_PREFIX: &str = "md:";

    // Source info properties (for preserve_source_info / use_source_info)
    // These capture original formatting style from source documents.

    /// Markdown heading style: "atx" (# Heading) or "setext" (underlined).
    pub const MD_HEADING_STYLE: &str = "md:heading_style";
    /// Markdown emphasis marker: "*" or "_".
    pub const MD_EMPHASIS_MARKER: &str = "md:emphasis_marker";
    /// Markdown strong marker: "**" or "__".
    pub const MD_STRONG_MARKER: &str = "md:strong_marker";
    /// Markdown unordered list marker: "-", "*", or "+".
    pub const MD_LIST_MARKER: &str = "md:list_marker";
    /// Markdown code fence character: "`" or "~".
    pub const MD_FENCE_CHAR: &str = "md:fence_char";
    /// Markdown code fence length (3 or more).
    pub const MD_FENCE_LENGTH: &str = "md:fence_length";
    /// Markdown thematic break character: "-", "*", or "_".
    pub const MD_BREAK_CHAR: &str = "md:break_char";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::doc;

    #[test]
    fn test_create_text_node() {
        let document = doc(|d| d.para(|i| i.text("Hello, world!")));
        let para = &document.content.children[0];
        let text_node = &para.children[0];
        assert_eq!(text_node.kind.as_str(), node::TEXT);
        assert_eq!(
            text_node.props.get_str(prop::CONTENT),
            Some("Hello, world!")
        );
    }

    #[test]
    fn test_create_heading() {
        let document = doc(|d| d.heading(1, |i| i.text("Title")));
        let h1 = &document.content.children[0];
        assert_eq!(h1.kind.as_str(), node::HEADING);
        assert_eq!(h1.props.get_int(prop::LEVEL), Some(1));
        assert_eq!(h1.children.len(), 1);
    }

    #[test]
    fn test_create_link() {
        let document = doc(|d| d.para(|i| i.link("https://example.com", |i| i.text("Example"))));
        let para = &document.content.children[0];
        let link = &para.children[0];
        assert_eq!(link.kind.as_str(), node::LINK);
        assert_eq!(link.props.get_str(prop::URL), Some("https://example.com"));
    }

    #[test]
    fn test_create_list() {
        let document =
            doc(|d| d.bullet_list(|l| l.item(|i| i.text("Item 1")).item(|i| i.text("Item 2"))));
        let list = &document.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(false));
        assert_eq!(list.children.len(), 2);
    }
}
