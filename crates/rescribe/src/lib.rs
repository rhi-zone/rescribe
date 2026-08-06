//! Rescribe - Universal document conversion library
//!
//! Rescribe is a document conversion library inspired by Pandoc, with:
//! - Open node kinds (not fixed enum)
//! - Property bags for extensibility
//! - Fidelity tracking (know what was lost in conversion)
//! - Embedded resource handling
//! - Roundtrip-friendly design
//!
//! # Quick Start
//!
//! ```rust
//! use rescribe::prelude::*;
//!
//! // Parse markdown
//! let doc = rescribe::markdown::parse("# Hello\n\nWorld!").unwrap();
//!
//! // Convert to HTML
//! let html = rescribe::html::emit(&doc.value).unwrap();
//! let html_str = String::from_utf8(html.value).unwrap();
//!
//! assert!(html_str.contains("<h1>Hello</h1>"));
//! ```
//!
//! # Features
//!
//! Each format has three feature flags:
//!
//! - `read-{fmt}` — reader only
//! - `write-{fmt}` — writer only
//! - `lang-{fmt}` — both (convenience alias)
//!
//! `std` and `math` enable standard/math node kind helpers.
//! `all` enables everything.
//!
//! # Architecture
//!
//! Documents are represented as trees of `Node`s with:
//! - `kind`: A string identifying the node type (e.g., "paragraph", "heading")
//! - `props`: A property bag with typed values
//! - `children`: Child nodes
//!
//! Format-specific crates implement parsers (readers) and emitters (writers)
//! that convert between bytes and the document IR.

// Re-export core types
pub use rescribe_core::*;

/// Standard node kinds and helpers.
#[cfg(feature = "std")]
pub mod std {
    pub use rescribe_std::*;
}

/// Math node kinds.
#[cfg(feature = "math")]
pub mod math {
    pub use rescribe_math::*;
}

/// jq-style querying of the document IR (embeds the `jaq` engine).
#[cfg(feature = "query")]
pub mod query;

/// ANSI terminal format support.
#[cfg(any(feature = "read-ansi", feature = "write-ansi"))]
pub mod ansi {
    #[cfg(feature = "write-ansi")]
    pub use ansi_fmt::rescribe::emit;
    #[cfg(feature = "write-ansi")]
    pub use ansi_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-ansi")]
    pub use ansi_fmt::rescribe::parse;
    #[cfg(feature = "read-ansi")]
    pub use ansi_fmt::rescribe::parse_with_options;
}

/// AsciiDoc format support.
#[cfg(any(feature = "read-asciidoc", feature = "write-asciidoc"))]
pub mod asciidoc {
    #[cfg(feature = "write-asciidoc")]
    pub use asciidoc::rescribe::emit;
    #[cfg(feature = "write-asciidoc")]
    pub use asciidoc::rescribe::emit_with_options;
    #[cfg(feature = "read-asciidoc")]
    pub use asciidoc::rescribe::parse;
    #[cfg(feature = "read-asciidoc")]
    pub use asciidoc::rescribe::parse_with_options;
}

/// BBCode forum markup format support.
#[cfg(any(feature = "read-bbcode", feature = "write-bbcode"))]
pub mod bbcode {
    #[cfg(feature = "write-bbcode")]
    pub use bbcode_fmt::rescribe::emit;
    #[cfg(feature = "write-bbcode")]
    pub use bbcode_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-bbcode")]
    pub use bbcode_fmt::rescribe::parse;
    #[cfg(feature = "read-bbcode")]
    pub use bbcode_fmt::rescribe::parse_with_options;
}

/// Beamer (LaTeX presentation) format support (writer only).
#[cfg(feature = "write-beamer")]
pub mod beamer {
    pub use rescribe_write_beamer::emit;
    pub use rescribe_write_beamer::emit_with_options;
}

/// BibLaTeX bibliographic format support.
#[cfg(any(feature = "read-biblatex", feature = "write-biblatex"))]
pub mod biblatex {
    #[cfg(feature = "write-biblatex")]
    pub use rescribe_fmt_biblatex::emit;
    #[cfg(feature = "write-biblatex")]
    pub use rescribe_fmt_biblatex::emit_with_options;
    #[cfg(feature = "read-biblatex")]
    pub use rescribe_fmt_biblatex::parse;
    #[cfg(feature = "read-biblatex")]
    pub use rescribe_fmt_biblatex::parse_with_options;
}

/// BibTeX format support.
#[cfg(any(feature = "read-bibtex", feature = "write-bibtex"))]
pub mod bibtex {
    #[cfg(feature = "write-bibtex")]
    pub use rescribe_fmt_bibtex::emit;
    #[cfg(feature = "write-bibtex")]
    pub use rescribe_fmt_bibtex::emit_with_options;
    #[cfg(feature = "read-bibtex")]
    pub use rescribe_fmt_bibtex::parse;
}

/// Chunked HTML format support (writer only).
#[cfg(feature = "write-chunkedhtml")]
pub mod chunkedhtml {
    pub use rescribe_write_chunkedhtml::HtmlChunk;
    pub use rescribe_write_chunkedhtml::emit;
    pub use rescribe_write_chunkedhtml::emit_with_options;
}

/// CommonMark format support.
#[cfg(any(feature = "read-commonmark", feature = "write-commonmark"))]
pub mod commonmark {
    #[cfg(feature = "read-commonmark")]
    pub use rescribe_read_commonmark::parse;
    #[cfg(feature = "read-commonmark")]
    pub use rescribe_read_commonmark::parse_with_options;
    #[cfg(feature = "write-commonmark")]
    pub use rescribe_write_commonmark::emit;
    #[cfg(feature = "write-commonmark")]
    pub use rescribe_write_commonmark::emit_with_options;
}

/// ConTeXt format support (writer only).
#[cfg(feature = "write-context")]
pub mod context {
    pub use rescribe_write_context::emit;
    pub use rescribe_write_context::emit_with_options;
}

/// Creole wiki markup format support.
#[cfg(any(feature = "read-creole", feature = "write-creole"))]
pub mod creole {
    #[cfg(feature = "write-creole")]
    pub use creole::rescribe::emit;
    #[cfg(feature = "write-creole")]
    pub use creole::rescribe::emit_with_options;
    #[cfg(feature = "read-creole")]
    pub use creole::rescribe::parse;
    #[cfg(feature = "read-creole")]
    pub use creole::rescribe::parse_with_options;
}

/// CSL JSON format support.
#[cfg(any(feature = "read-csl-json", feature = "write-csl-json"))]
pub mod csl_json {
    #[cfg(feature = "write-csl-json")]
    pub use rescribe_fmt_csl_json::emit;
    #[cfg(feature = "read-csl-json")]
    pub use rescribe_fmt_csl_json::parse;
}

/// CSV (Comma-Separated Values) format support.
#[cfg(any(feature = "read-csv", feature = "write-csv"))]
pub mod csv {
    #[cfg(feature = "write-csv")]
    pub use csv_fmt::rescribe::emit;
    #[cfg(feature = "write-csv")]
    pub use csv_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-csv")]
    pub use csv_fmt::rescribe::parse;
    #[cfg(feature = "read-csv")]
    pub use csv_fmt::rescribe::parse_with_options;
}

/// Djot format support.
#[cfg(any(feature = "read-djot", feature = "write-djot"))]
pub mod djot {
    #[cfg(feature = "write-djot")]
    pub use djot_fmt::rescribe::emit;
    #[cfg(feature = "read-djot")]
    pub use djot_fmt::rescribe::parse;
}

/// DocBook format support.
#[cfg(any(feature = "read-docbook", feature = "write-docbook"))]
pub mod docbook {
    #[cfg(feature = "write-docbook")]
    pub use docbook_fmt::rescribe::emit;
    #[cfg(feature = "read-docbook")]
    pub use docbook_fmt::rescribe::parse;
}

/// DOCX (Word) format support.
#[cfg(any(feature = "read-docx", feature = "write-docx"))]
pub mod docx {
    #[cfg(feature = "write-docx")]
    pub use rescribe_fmt_ooxml::docx::emit;
    #[cfg(feature = "read-docx")]
    pub use rescribe_fmt_ooxml::docx::parse;
    #[cfg(feature = "read-docx")]
    pub use rescribe_fmt_ooxml::docx::parse_bytes;
    #[cfg(feature = "read-docx")]
    pub use rescribe_fmt_ooxml::docx::parse_file;
}

/// DokuWiki format support.
#[cfg(any(feature = "read-dokuwiki", feature = "write-dokuwiki"))]
pub mod dokuwiki {
    #[cfg(feature = "write-dokuwiki")]
    pub use dokuwiki::rescribe::emit;
    #[cfg(feature = "write-dokuwiki")]
    pub use dokuwiki::rescribe::emit_with_options;
    #[cfg(feature = "read-dokuwiki")]
    pub use dokuwiki::rescribe::parse;
    #[cfg(feature = "read-dokuwiki")]
    pub use dokuwiki::rescribe::parse_with_options;
}

/// DZSlides HTML presentation format support (writer only).
#[cfg(feature = "write-dzslides")]
pub mod dzslides {
    pub use rescribe_write_dzslides::emit;
    pub use rescribe_write_dzslides::emit_with_options;
}

/// EndNote XML bibliographic format support.
#[cfg(any(feature = "read-endnotexml", feature = "write-endnotexml"))]
pub mod endnotexml {
    #[cfg(feature = "write-endnotexml")]
    pub use endnotexml_fmt::rescribe::emit;
    #[cfg(feature = "write-endnotexml")]
    pub use endnotexml_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-endnotexml")]
    pub use endnotexml_fmt::rescribe::parse;
    #[cfg(feature = "read-endnotexml")]
    pub use endnotexml_fmt::rescribe::parse_with_options;
}

/// EPUB format support.
#[cfg(any(feature = "read-epub", feature = "write-epub"))]
pub mod epub {
    #[cfg(feature = "write-epub")]
    pub use epub_fmt::rescribe::emit;
    #[cfg(feature = "read-epub")]
    pub use epub_fmt::rescribe::parse;
    #[cfg(feature = "read-epub")]
    pub use epub_fmt::rescribe::parse as parse_bytes;
}

/// FictionBook 2 (FB2) format support.
#[cfg(any(feature = "read-fb2", feature = "write-fb2"))]
pub mod fb2 {
    #[cfg(feature = "write-fb2")]
    pub use fb2_fmt::rescribe::emit;
    #[cfg(feature = "write-fb2")]
    pub use fb2_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-fb2")]
    pub use fb2_fmt::rescribe::parse;
    #[cfg(feature = "read-fb2")]
    pub use fb2_fmt::rescribe::parse_with_options;
}

/// Fountain screenplay format support.
#[cfg(any(feature = "read-fountain", feature = "write-fountain"))]
pub mod fountain {
    #[cfg(feature = "write-fountain")]
    pub use fountain_fmt::rescribe::emit;
    #[cfg(feature = "write-fountain")]
    pub use fountain_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-fountain")]
    pub use fountain_fmt::rescribe::parse;
    #[cfg(feature = "read-fountain")]
    pub use fountain_fmt::rescribe::parse_with_options;
}

/// GitHub Flavored Markdown (GFM) format support.
#[cfg(any(feature = "read-gfm", feature = "write-gfm"))]
pub mod gfm {
    #[cfg(feature = "read-gfm")]
    pub use rescribe_read_gfm::parse;
    #[cfg(feature = "read-gfm")]
    pub use rescribe_read_gfm::parse_with_options;
    #[cfg(feature = "write-gfm")]
    pub use rescribe_write_gfm::emit;
    #[cfg(feature = "write-gfm")]
    pub use rescribe_write_gfm::emit_with_options;
}

/// Haddock (Haskell documentation) format support.
#[cfg(any(feature = "read-haddock", feature = "write-haddock"))]
pub mod haddock {
    #[cfg(feature = "write-haddock")]
    pub use haddock_fmt::rescribe::emit;
    #[cfg(feature = "write-haddock")]
    pub use haddock_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-haddock")]
    pub use haddock_fmt::rescribe::parse;
    #[cfg(feature = "read-haddock")]
    pub use haddock_fmt::rescribe::parse_with_options;
}

/// HTML format support.
#[cfg(any(feature = "read-html", feature = "write-html"))]
pub mod html {
    #[cfg(feature = "write-html")]
    pub use html_fmt::rescribe::emit;
    #[cfg(feature = "write-html")]
    pub use html_fmt::rescribe::emit_full_document;
    #[cfg(feature = "write-html")]
    pub use html_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-html")]
    pub use html_fmt::rescribe::parse;
    #[cfg(feature = "read-html")]
    pub use html_fmt::rescribe::parse_with_options;
}

/// ICML (InCopy Markup Language) format support (writer only).
#[cfg(feature = "write-icml")]
pub mod icml {
    pub use rescribe_write_icml::emit;
    pub use rescribe_write_icml::emit_with_options;
}

/// Jupyter notebook (ipynb) format support.
#[cfg(any(feature = "read-ipynb", feature = "write-ipynb"))]
pub mod ipynb {
    #[cfg(feature = "write-ipynb")]
    pub use rescribe_fmt_ipynb::emit;
    #[cfg(feature = "read-ipynb")]
    pub use rescribe_fmt_ipynb::parse;
    #[cfg(feature = "read-ipynb")]
    pub use rescribe_fmt_ipynb::parse_bytes;
}

/// JATS (Journal Article Tag Suite) format support.
#[cfg(any(feature = "read-jats", feature = "write-jats"))]
pub mod jats {
    #[cfg(feature = "write-jats")]
    pub use jats_fmt::rescribe::emit;
    #[cfg(feature = "read-jats")]
    pub use jats_fmt::rescribe::parse;
}

/// Jira/Confluence markup format support.
#[cfg(any(feature = "read-jira", feature = "write-jira"))]
pub mod jira {
    #[cfg(feature = "write-jira")]
    pub use jira_fmt::rescribe::emit;
    #[cfg(feature = "write-jira")]
    pub use jira_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-jira")]
    pub use jira_fmt::rescribe::parse;
    #[cfg(feature = "read-jira")]
    pub use jira_fmt::rescribe::parse_with_options;
}

/// LaTeX format support.
#[cfg(any(feature = "read-latex", feature = "write-latex"))]
pub mod latex {
    #[cfg(feature = "write-latex")]
    pub use latex_fmt::rescribe::emit;
    #[cfg(feature = "write-latex")]
    pub use latex_fmt::rescribe::emit_full_document;
    #[cfg(feature = "write-latex")]
    pub use latex_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-latex")]
    pub use latex_fmt::rescribe::parse;
    #[cfg(feature = "read-latex")]
    pub use latex_fmt::rescribe::parse_with_options;
}

/// Man page (roff/troff) format support.
#[cfg(any(feature = "read-man", feature = "write-man"))]
pub mod man {
    #[cfg(feature = "write-man")]
    pub use man_fmt::rescribe::emit;
    #[cfg(feature = "write-man")]
    pub use man_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-man")]
    pub use man_fmt::rescribe::parse;
}

/// Markdown format support.
#[cfg(any(feature = "read-markdown", feature = "write-markdown"))]
pub mod markdown {
    #[cfg(feature = "read-markdown")]
    pub use rescribe_read_markdown::parse;
    #[cfg(feature = "read-markdown")]
    pub use rescribe_read_markdown::parse_with_options;
    #[cfg(feature = "write-markdown")]
    pub use rescribe_write_markdown::emit;
    #[cfg(feature = "write-markdown")]
    pub use rescribe_write_markdown::emit_with_options;
}

/// Markdown strict (original Markdown.pl) format support.
#[cfg(any(feature = "read-markdown-strict", feature = "write-markdown-strict"))]
pub mod markdown_strict {
    #[cfg(feature = "read-markdown-strict")]
    pub use rescribe_read_markdown_strict::parse;
    #[cfg(feature = "read-markdown-strict")]
    pub use rescribe_read_markdown_strict::parse_with_options;
    #[cfg(feature = "write-markdown-strict")]
    pub use rescribe_write_markdown_strict::emit;
    #[cfg(feature = "write-markdown-strict")]
    pub use rescribe_write_markdown_strict::emit_with_options;
}

/// Markua (Leanpub) format support.
#[cfg(any(feature = "read-markua", feature = "write-markua"))]
pub mod markua {
    #[cfg(feature = "write-markua")]
    pub use markua::rescribe::emit;
    #[cfg(feature = "write-markua")]
    pub use markua::rescribe::emit_with_options;
    #[cfg(feature = "read-markua")]
    pub use markua::rescribe::parse;
    #[cfg(feature = "read-markua")]
    pub use markua::rescribe::parse_with_options;
}

/// MediaWiki format support.
#[cfg(any(feature = "read-mediawiki", feature = "write-mediawiki"))]
pub mod mediawiki {
    #[cfg(feature = "write-mediawiki")]
    pub use mediawiki_fmt::rescribe::emit;
    #[cfg(feature = "read-mediawiki")]
    pub use mediawiki_fmt::rescribe::parse;
}

/// Groff ms macro format support (writer only).
#[cfg(feature = "write-ms")]
pub mod ms {
    pub use rescribe_write_ms::emit;
    pub use rescribe_write_ms::emit_with_options;
}

/// MultiMarkdown format support.
#[cfg(any(feature = "read-multimarkdown", feature = "write-multimarkdown"))]
pub mod multimarkdown {
    #[cfg(feature = "write-multimarkdown")]
    pub use multimarkdown_fmt::rescribe::emit;
    #[cfg(feature = "write-multimarkdown")]
    pub use multimarkdown_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-multimarkdown")]
    pub use multimarkdown_fmt::rescribe::parse;
    #[cfg(feature = "read-multimarkdown")]
    pub use multimarkdown_fmt::rescribe::parse_with_options;
}

/// Muse (Emacs Muse) format support.
#[cfg(any(feature = "read-muse", feature = "write-muse"))]
pub mod muse {
    #[cfg(feature = "write-muse")]
    pub use muse_fmt::rescribe::emit;
    #[cfg(feature = "write-muse")]
    pub use muse_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-muse")]
    pub use muse_fmt::rescribe::parse;
    #[cfg(feature = "read-muse")]
    pub use muse_fmt::rescribe::parse_with_options;
}

/// Native debug format support.
#[cfg(any(feature = "read-native", feature = "write-native"))]
pub mod native {
    #[cfg(feature = "write-native")]
    pub use native::rescribe::emit;
    #[cfg(feature = "write-native")]
    pub use native::rescribe::emit_with_options;
    #[cfg(feature = "read-native")]
    pub use native::rescribe::parse;
    #[cfg(feature = "read-native")]
    pub use native::rescribe::parse_with_options;
}

/// ODT (OpenDocument Text) format support.
#[cfg(any(feature = "read-odt", feature = "write-odt"))]
pub mod odt {
    #[cfg(feature = "write-odt")]
    pub use odf_fmt::rescribe::emit;
    #[cfg(feature = "write-odt")]
    pub use odf_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-odt")]
    pub use odf_fmt::rescribe::parse;
    #[cfg(feature = "read-odt")]
    pub use odf_fmt::rescribe::parse_with_options;
}

/// OPML format support.
#[cfg(any(feature = "read-opml", feature = "write-opml"))]
pub mod opml {
    #[cfg(feature = "write-opml")]
    pub use opml_fmt::rescribe::emit;
    #[cfg(feature = "read-opml")]
    pub use opml_fmt::rescribe::parse;
}

/// Org-mode format support.
#[cfg(any(feature = "read-org", feature = "write-org"))]
pub mod org {
    #[cfg(feature = "write-org")]
    pub use org_fmt::rescribe::emit;
    #[cfg(feature = "write-org")]
    pub use org_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-org")]
    pub use org_fmt::rescribe::parse;
    #[cfg(feature = "read-org")]
    pub use org_fmt::rescribe::parse_with_options;
}

/// Pandoc JSON format support.
#[cfg(any(feature = "read-pandoc-json", feature = "write-pandoc-json"))]
pub mod pandoc_json {
    #[cfg(feature = "write-pandoc-json")]
    pub use rescribe_fmt_pandoc_json::emit;
    #[cfg(feature = "write-pandoc-json")]
    pub use rescribe_fmt_pandoc_json::emit_with_options;
    #[cfg(feature = "read-pandoc-json")]
    pub use rescribe_fmt_pandoc_json::parse;
    #[cfg(feature = "read-pandoc-json")]
    pub use rescribe_fmt_pandoc_json::parse_with_options;
}

/// PDF format support (reader only).
#[cfg(feature = "read-pdf")]
pub mod pdf {
    pub use rescribe_fmt_pdf::parse;
    pub use rescribe_fmt_pdf::parse_with_options;
}

/// Plain text format support (writer only).
#[cfg(feature = "write-plaintext")]
pub mod plaintext {
    pub use rescribe_write_plaintext::emit;
    pub use rescribe_write_plaintext::emit_with_options;
}

/// POD (Plain Old Documentation) format support.
#[cfg(any(feature = "read-pod", feature = "write-pod"))]
pub mod pod {
    #[cfg(feature = "write-pod")]
    pub use pod_fmt::rescribe::emit;
    #[cfg(feature = "write-pod")]
    pub use pod_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-pod")]
    pub use pod_fmt::rescribe::parse;
    #[cfg(feature = "read-pod")]
    pub use pod_fmt::rescribe::parse_with_options;
}

/// PPTX (PowerPoint) format support.
#[cfg(any(feature = "read-pptx", feature = "write-pptx"))]
pub mod pptx {
    #[cfg(feature = "write-pptx")]
    pub use rescribe_fmt_ooxml::pptx::emit;
    #[cfg(feature = "write-pptx")]
    pub use rescribe_fmt_ooxml::pptx::emit_with_options;
    #[cfg(feature = "read-pptx")]
    pub use rescribe_fmt_ooxml::pptx::parse;
    #[cfg(feature = "read-pptx")]
    pub use rescribe_fmt_ooxml::pptx::parse_with_options;
}

/// reveal.js HTML presentation format support (writer only).
#[cfg(feature = "write-revealjs")]
pub mod revealjs {
    pub use rescribe_write_revealjs::emit;
    pub use rescribe_write_revealjs::emit_with_options;
}

/// RIS (Research Information Systems) bibliographic format support.
#[cfg(any(feature = "read-ris", feature = "write-ris"))]
pub mod ris {
    #[cfg(feature = "write-ris")]
    pub use ris::rescribe::emit;
    #[cfg(feature = "write-ris")]
    pub use ris::rescribe::emit_with_options;
    #[cfg(feature = "read-ris")]
    pub use ris::rescribe::parse;
    #[cfg(feature = "read-ris")]
    pub use ris::rescribe::parse_with_options;
}

/// reStructuredText format support.
#[cfg(any(feature = "read-rst", feature = "write-rst"))]
pub mod rst {
    #[cfg(feature = "write-rst")]
    pub use rst_fmt::rescribe::emit;
    #[cfg(feature = "write-rst")]
    pub use rst_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-rst")]
    pub use rst_fmt::rescribe::parse;
    #[cfg(feature = "read-rst")]
    pub use rst_fmt::rescribe::parse_with_options;
}

/// RTF (Rich Text Format) support.
#[cfg(any(feature = "read-rtf", feature = "write-rtf"))]
pub mod rtf {
    #[cfg(feature = "write-rtf")]
    pub use rtf_fmt::rescribe::emit;
    #[cfg(feature = "write-rtf")]
    pub use rtf_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-rtf")]
    pub use rtf_fmt::rescribe::parse;
    #[cfg(feature = "read-rtf")]
    pub use rtf_fmt::rescribe::parse_with_options;
}

/// S5 HTML presentation format support (writer only).
#[cfg(feature = "write-s5")]
pub mod s5 {
    pub use rescribe_write_s5::emit;
    pub use rescribe_write_s5::emit_with_options;
}

/// Slideous HTML slideshow format support (writer only).
#[cfg(feature = "write-slideous")]
pub mod slideous {
    pub use rescribe_write_slideous::emit;
    pub use rescribe_write_slideous::emit_with_options;
}

/// W3C Slidy HTML presentation format support (writer only).
#[cfg(feature = "write-slidy")]
pub mod slidy {
    pub use rescribe_write_slidy::emit;
    pub use rescribe_write_slidy::emit_with_options;
}

/// txt2tags (t2t) format support.
#[cfg(any(feature = "read-t2t", feature = "write-t2t"))]
pub mod t2t {
    #[cfg(feature = "write-t2t")]
    pub use t2t::rescribe::emit;
    #[cfg(feature = "write-t2t")]
    pub use t2t::rescribe::emit_with_options;
    #[cfg(feature = "read-t2t")]
    pub use t2t::rescribe::parse;
    #[cfg(feature = "read-t2t")]
    pub use t2t::rescribe::parse_with_options;
}

/// TEI (Text Encoding Initiative) format support.
#[cfg(any(feature = "read-tei", feature = "write-tei"))]
pub mod tei {
    #[cfg(feature = "write-tei")]
    pub use tei_fmt::rescribe::emit;
    #[cfg(feature = "read-tei")]
    pub use tei_fmt::rescribe::parse;
}

/// Texinfo (GNU documentation) format support.
#[cfg(any(feature = "read-texinfo", feature = "write-texinfo"))]
pub mod texinfo {
    #[cfg(feature = "write-texinfo")]
    pub use texinfo::rescribe::emit;
    #[cfg(feature = "write-texinfo")]
    pub use texinfo::rescribe::emit_with_options;
    #[cfg(feature = "read-texinfo")]
    pub use texinfo::rescribe::parse;
    #[cfg(feature = "read-texinfo")]
    pub use texinfo::rescribe::parse_with_options;
}

/// Textile markup format support.
#[cfg(any(feature = "read-textile", feature = "write-textile"))]
pub mod textile {
    #[cfg(feature = "write-textile")]
    pub use textile_fmt::rescribe::emit;
    #[cfg(feature = "write-textile")]
    pub use textile_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-textile")]
    pub use textile_fmt::rescribe::parse;
    #[cfg(feature = "read-textile")]
    pub use textile_fmt::rescribe::parse_with_options;
}

/// TikiWiki format support.
#[cfg(any(feature = "read-tikiwiki", feature = "write-tikiwiki"))]
pub mod tikiwiki {
    #[cfg(feature = "write-tikiwiki")]
    pub use tikiwiki::rescribe::emit;
    #[cfg(feature = "write-tikiwiki")]
    pub use tikiwiki::rescribe::emit_with_options;
    #[cfg(feature = "read-tikiwiki")]
    pub use tikiwiki::rescribe::parse;
    #[cfg(feature = "read-tikiwiki")]
    pub use tikiwiki::rescribe::parse_with_options;
}

/// TSV (Tab-Separated Values) format support.
#[cfg(any(feature = "read-tsv", feature = "write-tsv"))]
pub mod tsv {
    #[cfg(feature = "write-tsv")]
    pub use tsv_fmt::rescribe::emit;
    #[cfg(feature = "write-tsv")]
    pub use tsv_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-tsv")]
    pub use tsv_fmt::rescribe::parse;
    #[cfg(feature = "read-tsv")]
    pub use tsv_fmt::rescribe::parse_with_options;
}

/// TWiki format support.
#[cfg(any(feature = "read-twiki", feature = "write-twiki"))]
pub mod twiki {
    #[cfg(feature = "write-twiki")]
    pub use twiki::rescribe::emit;
    #[cfg(feature = "write-twiki")]
    pub use twiki::rescribe::emit_with_options;
    #[cfg(feature = "read-twiki")]
    pub use twiki::rescribe::parse;
    #[cfg(feature = "read-twiki")]
    pub use twiki::rescribe::parse_with_options;
}

/// Typst format support.
#[cfg(any(feature = "read-typst", feature = "write-typst"))]
pub mod typst {
    #[cfg(feature = "write-typst")]
    pub use typst_fmt::rescribe::emit;
    #[cfg(feature = "write-typst")]
    pub use typst_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-typst")]
    pub use typst_fmt::rescribe::parse;
    #[cfg(feature = "read-typst")]
    pub use typst_fmt::rescribe::parse_with_options;
}

/// VimWiki format support.
#[cfg(any(feature = "read-vimwiki", feature = "write-vimwiki"))]
pub mod vimwiki {
    #[cfg(feature = "write-vimwiki")]
    pub use vimwiki_fmt::rescribe::emit;
    #[cfg(feature = "write-vimwiki")]
    pub use vimwiki_fmt::rescribe::emit_with_options;
    #[cfg(feature = "read-vimwiki")]
    pub use vimwiki_fmt::rescribe::parse;
    #[cfg(feature = "read-vimwiki")]
    pub use vimwiki_fmt::rescribe::parse_with_options;
}

/// XLSX (Excel) format support.
#[cfg(any(feature = "read-xlsx", feature = "write-xlsx"))]
pub mod xlsx {
    #[cfg(feature = "write-xlsx")]
    pub use rescribe_fmt_ooxml::xlsx::emit;
    #[cfg(feature = "write-xlsx")]
    pub use rescribe_fmt_ooxml::xlsx::emit_with_options;
    #[cfg(feature = "read-xlsx")]
    pub use rescribe_fmt_ooxml::xlsx::parse;
    #[cfg(feature = "read-xlsx")]
    pub use rescribe_fmt_ooxml::xlsx::parse_bytes;
    #[cfg(feature = "read-xlsx")]
    pub use rescribe_fmt_ooxml::xlsx::parse_file;
}

/// XWiki format support.
#[cfg(any(feature = "read-xwiki", feature = "write-xwiki"))]
pub mod xwiki {
    #[cfg(feature = "write-xwiki")]
    pub use xwiki::rescribe::emit;
    #[cfg(feature = "write-xwiki")]
    pub use xwiki::rescribe::emit_with_options;
    #[cfg(feature = "read-xwiki")]
    pub use xwiki::rescribe::parse;
    #[cfg(feature = "read-xwiki")]
    pub use xwiki::rescribe::parse_with_options;
}

/// ZimWiki (Zim Desktop Wiki) format support.
#[cfg(any(feature = "read-zimwiki", feature = "write-zimwiki"))]
pub mod zimwiki {
    #[cfg(feature = "write-zimwiki")]
    pub use zimwiki::rescribe::emit;
    #[cfg(feature = "write-zimwiki")]
    pub use zimwiki::rescribe::emit_with_options;
    #[cfg(feature = "read-zimwiki")]
    pub use zimwiki::rescribe::parse;
    #[cfg(feature = "read-zimwiki")]
    pub use zimwiki::rescribe::parse_with_options;
}

/// Common imports for typical usage.
pub mod prelude {
    pub use crate::{ConversionResult, Document, Node, PropValue, Properties};

    #[cfg(feature = "std")]
    pub use crate::std::{builder, node, prop};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(feature = "read-markdown", feature = "write-html", feature = "std"))]
    fn test_markdown_to_html() {
        let result = markdown::parse("# Hello\n\nWorld!").unwrap();
        let doc = result.value;

        let html_result = html::emit(&doc).unwrap();
        let html = String::from_utf8(html_result.value).unwrap();

        assert!(html.contains("<h1>"));
        assert!(html.contains("Hello"));
        assert!(html.contains("<p>"));
        assert!(html.contains("World!"));
    }

    #[test]
    #[cfg(all(feature = "read-markdown", feature = "write-latex"))]
    fn test_markdown_to_latex() {
        let result = markdown::parse("# Title\n\n**Bold** text").unwrap();
        let doc = result.value;

        let latex_result = latex::emit(&doc).unwrap();
        let latex = String::from_utf8(latex_result.value).unwrap();

        assert!(latex.contains("\\section{Title}"));
        assert!(latex.contains("\\textbf{Bold}"));
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_build_document_manually() {
        use crate::std::builder::doc;

        let document = doc(|d| {
            d.heading(1, |i| i.text("Manual Document"))
                .para(|i| i.text("This is ").strong(|i| i.text("bold")).text(" text."))
        });

        assert_eq!(document.content.children.len(), 2);
    }
}
