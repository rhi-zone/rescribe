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

/// ANSI terminal format support.
#[cfg(any(feature = "read-ansi", feature = "write-ansi"))]
pub mod ansi {
    #[cfg(feature = "read-ansi")]
    pub use rescribe_read_ansi::parse;
    #[cfg(feature = "read-ansi")]
    pub use rescribe_read_ansi::parse_with_options;
    #[cfg(feature = "write-ansi")]
    pub use rescribe_write_ansi::emit;
    #[cfg(feature = "write-ansi")]
    pub use rescribe_write_ansi::emit_with_options;
}

/// AsciiDoc format support.
#[cfg(any(feature = "read-asciidoc", feature = "write-asciidoc"))]
pub mod asciidoc {
    #[cfg(feature = "read-asciidoc")]
    pub use rescribe_read_asciidoc::parse;
    #[cfg(feature = "read-asciidoc")]
    pub use rescribe_read_asciidoc::parse_with_options;
    #[cfg(feature = "write-asciidoc")]
    pub use rescribe_write_asciidoc::emit;
    #[cfg(feature = "write-asciidoc")]
    pub use rescribe_write_asciidoc::emit_with_options;
}

/// BBCode forum markup format support.
#[cfg(any(feature = "read-bbcode", feature = "write-bbcode"))]
pub mod bbcode {
    #[cfg(feature = "read-bbcode")]
    pub use rescribe_read_bbcode::parse;
    #[cfg(feature = "read-bbcode")]
    pub use rescribe_read_bbcode::parse_with_options;
    #[cfg(feature = "write-bbcode")]
    pub use rescribe_write_bbcode::emit;
    #[cfg(feature = "write-bbcode")]
    pub use rescribe_write_bbcode::emit_with_options;
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
    #[cfg(feature = "read-biblatex")]
    pub use rescribe_read_biblatex::parse;
    #[cfg(feature = "read-biblatex")]
    pub use rescribe_read_biblatex::parse_with_options;
    #[cfg(feature = "write-biblatex")]
    pub use rescribe_write_biblatex::emit;
    #[cfg(feature = "write-biblatex")]
    pub use rescribe_write_biblatex::emit_with_options;
}

/// BibTeX format support.
#[cfg(any(feature = "read-bibtex", feature = "write-bibtex"))]
pub mod bibtex {
    #[cfg(feature = "read-bibtex")]
    pub use rescribe_read_bibtex::parse;
    #[cfg(feature = "write-bibtex")]
    pub use rescribe_write_bibtex::emit;
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
    #[cfg(feature = "read-creole")]
    pub use rescribe_read_creole::parse;
    #[cfg(feature = "read-creole")]
    pub use rescribe_read_creole::parse_with_options;
    #[cfg(feature = "write-creole")]
    pub use rescribe_write_creole::emit;
    #[cfg(feature = "write-creole")]
    pub use rescribe_write_creole::emit_with_options;
}

/// CSL JSON format support.
#[cfg(any(feature = "read-csl-json", feature = "write-csl-json"))]
pub mod csl_json {
    #[cfg(feature = "read-csl-json")]
    pub use rescribe_read_csl_json::parse;
    #[cfg(feature = "write-csl-json")]
    pub use rescribe_write_csl_json::emit;
}

/// CSV (Comma-Separated Values) format support.
#[cfg(any(feature = "read-csv", feature = "write-csv"))]
pub mod csv {
    #[cfg(feature = "read-csv")]
    pub use rescribe_read_csv::parse;
    #[cfg(feature = "read-csv")]
    pub use rescribe_read_csv::parse_with_options;
    #[cfg(feature = "write-csv")]
    pub use rescribe_write_csv::emit;
    #[cfg(feature = "write-csv")]
    pub use rescribe_write_csv::emit_with_options;
}

/// Djot format support.
#[cfg(any(feature = "read-djot", feature = "write-djot"))]
pub mod djot {
    #[cfg(feature = "read-djot")]
    pub use rescribe_read_djot::parse;
    #[cfg(feature = "write-djot")]
    pub use rescribe_write_djot::emit;
}

/// DocBook format support.
#[cfg(any(feature = "read-docbook", feature = "write-docbook"))]
pub mod docbook {
    #[cfg(feature = "read-docbook")]
    pub use rescribe_read_docbook::parse;
    #[cfg(feature = "write-docbook")]
    pub use rescribe_write_docbook::emit;
}

/// DOCX (Word) format support.
#[cfg(any(feature = "read-docx", feature = "write-docx"))]
pub mod docx {
    #[cfg(feature = "read-docx")]
    pub use rescribe_read_docx::parse;
    #[cfg(feature = "read-docx")]
    pub use rescribe_read_docx::parse_bytes;
    #[cfg(feature = "read-docx")]
    pub use rescribe_read_docx::parse_file;
    #[cfg(feature = "write-docx")]
    pub use rescribe_write_docx::emit;
}

/// DokuWiki format support.
#[cfg(any(feature = "read-dokuwiki", feature = "write-dokuwiki"))]
pub mod dokuwiki {
    #[cfg(feature = "read-dokuwiki")]
    pub use rescribe_read_dokuwiki::parse;
    #[cfg(feature = "read-dokuwiki")]
    pub use rescribe_read_dokuwiki::parse_with_options;
    #[cfg(feature = "write-dokuwiki")]
    pub use rescribe_write_dokuwiki::emit;
    #[cfg(feature = "write-dokuwiki")]
    pub use rescribe_write_dokuwiki::emit_with_options;
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
    #[cfg(feature = "read-endnotexml")]
    pub use rescribe_read_endnotexml::parse;
    #[cfg(feature = "read-endnotexml")]
    pub use rescribe_read_endnotexml::parse_with_options;
    #[cfg(feature = "write-endnotexml")]
    pub use rescribe_write_endnotexml::emit;
    #[cfg(feature = "write-endnotexml")]
    pub use rescribe_write_endnotexml::emit_with_options;
}

/// EPUB format support.
#[cfg(any(feature = "read-epub", feature = "write-epub"))]
pub mod epub {
    #[cfg(feature = "read-epub")]
    pub use rescribe_read_epub::parse;
    #[cfg(feature = "read-epub")]
    pub use rescribe_read_epub::parse_bytes;
    #[cfg(feature = "read-epub")]
    pub use rescribe_read_epub::parse_file;
    #[cfg(feature = "write-epub")]
    pub use rescribe_write_epub::emit;
}

/// FictionBook 2 (FB2) format support.
#[cfg(any(feature = "read-fb2", feature = "write-fb2"))]
pub mod fb2 {
    #[cfg(feature = "read-fb2")]
    pub use rescribe_read_fb2::parse;
    #[cfg(feature = "read-fb2")]
    pub use rescribe_read_fb2::parse_with_options;
    #[cfg(feature = "write-fb2")]
    pub use rescribe_write_fb2::emit;
    #[cfg(feature = "write-fb2")]
    pub use rescribe_write_fb2::emit_with_options;
}

/// Fountain screenplay format support.
#[cfg(any(feature = "read-fountain", feature = "write-fountain"))]
pub mod fountain {
    #[cfg(feature = "read-fountain")]
    pub use rescribe_read_fountain::parse;
    #[cfg(feature = "read-fountain")]
    pub use rescribe_read_fountain::parse_with_options;
    #[cfg(feature = "write-fountain")]
    pub use rescribe_write_fountain::emit;
    #[cfg(feature = "write-fountain")]
    pub use rescribe_write_fountain::emit_with_options;
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
    #[cfg(feature = "read-haddock")]
    pub use rescribe_read_haddock::parse;
    #[cfg(feature = "read-haddock")]
    pub use rescribe_read_haddock::parse_with_options;
    #[cfg(feature = "write-haddock")]
    pub use rescribe_write_haddock::emit;
    #[cfg(feature = "write-haddock")]
    pub use rescribe_write_haddock::emit_with_options;
}

/// HTML format support.
#[cfg(any(feature = "read-html", feature = "write-html"))]
pub mod html {
    #[cfg(feature = "read-html")]
    pub use rescribe_read_html::parse;
    #[cfg(feature = "read-html")]
    pub use rescribe_read_html::parse_with_options;
    #[cfg(feature = "write-html")]
    pub use rescribe_write_html::emit;
    #[cfg(feature = "write-html")]
    pub use rescribe_write_html::emit_full_document;
    #[cfg(feature = "write-html")]
    pub use rescribe_write_html::emit_with_options;
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
    #[cfg(feature = "read-ipynb")]
    pub use rescribe_read_ipynb::parse;
    #[cfg(feature = "read-ipynb")]
    pub use rescribe_read_ipynb::parse_bytes;
    #[cfg(feature = "write-ipynb")]
    pub use rescribe_write_ipynb::emit;
}

/// JATS (Journal Article Tag Suite) format support.
#[cfg(any(feature = "read-jats", feature = "write-jats"))]
pub mod jats {
    #[cfg(feature = "read-jats")]
    pub use rescribe_read_jats::parse;
    #[cfg(feature = "write-jats")]
    pub use rescribe_write_jats::emit;
}

/// Jira/Confluence markup format support.
#[cfg(any(feature = "read-jira", feature = "write-jira"))]
pub mod jira {
    #[cfg(feature = "read-jira")]
    pub use rescribe_read_jira::parse;
    #[cfg(feature = "read-jira")]
    pub use rescribe_read_jira::parse_with_options;
    #[cfg(feature = "write-jira")]
    pub use rescribe_write_jira::emit;
    #[cfg(feature = "write-jira")]
    pub use rescribe_write_jira::emit_with_options;
}

/// LaTeX format support.
#[cfg(any(feature = "read-latex", feature = "write-latex"))]
pub mod latex {
    #[cfg(feature = "read-latex")]
    pub use rescribe_read_latex::parse;
    #[cfg(feature = "read-latex")]
    pub use rescribe_read_latex::parse_with_options;
    #[cfg(feature = "write-latex")]
    pub use rescribe_write_latex::emit;
    #[cfg(feature = "write-latex")]
    pub use rescribe_write_latex::emit_full_document;
    #[cfg(feature = "write-latex")]
    pub use rescribe_write_latex::emit_with_options;
}

/// Man page (roff/troff) format support.
#[cfg(any(feature = "read-man", feature = "write-man"))]
pub mod man {
    #[cfg(feature = "read-man")]
    pub use rescribe_read_man::parse;
    #[cfg(feature = "write-man")]
    pub use rescribe_write_man::emit;
    #[cfg(feature = "write-man")]
    pub use rescribe_write_man::emit_with_options;
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
    #[cfg(feature = "read-markua")]
    pub use rescribe_read_markua::parse;
    #[cfg(feature = "read-markua")]
    pub use rescribe_read_markua::parse_with_options;
    #[cfg(feature = "write-markua")]
    pub use rescribe_write_markua::emit;
    #[cfg(feature = "write-markua")]
    pub use rescribe_write_markua::emit_with_options;
}

/// MediaWiki format support.
#[cfg(any(feature = "read-mediawiki", feature = "write-mediawiki"))]
pub mod mediawiki {
    #[cfg(feature = "read-mediawiki")]
    pub use rescribe_read_mediawiki::parse;
    #[cfg(feature = "write-mediawiki")]
    pub use rescribe_write_mediawiki::emit;
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
    #[cfg(feature = "read-multimarkdown")]
    pub use rescribe_read_multimarkdown::parse;
    #[cfg(feature = "read-multimarkdown")]
    pub use rescribe_read_multimarkdown::parse_with_options;
    #[cfg(feature = "write-multimarkdown")]
    pub use rescribe_write_multimarkdown::emit;
    #[cfg(feature = "write-multimarkdown")]
    pub use rescribe_write_multimarkdown::emit_with_options;
}

/// Muse (Emacs Muse) format support.
#[cfg(any(feature = "read-muse", feature = "write-muse"))]
pub mod muse {
    #[cfg(feature = "read-muse")]
    pub use rescribe_read_muse::parse;
    #[cfg(feature = "read-muse")]
    pub use rescribe_read_muse::parse_with_options;
    #[cfg(feature = "write-muse")]
    pub use rescribe_write_muse::emit;
    #[cfg(feature = "write-muse")]
    pub use rescribe_write_muse::emit_with_options;
}

/// Native debug format support.
#[cfg(any(feature = "read-native", feature = "write-native"))]
pub mod native {
    #[cfg(feature = "read-native")]
    pub use rescribe_read_native::parse;
    #[cfg(feature = "read-native")]
    pub use rescribe_read_native::parse_with_options;
    #[cfg(feature = "write-native")]
    pub use rescribe_write_native::emit;
    #[cfg(feature = "write-native")]
    pub use rescribe_write_native::emit_with_options;
}

/// ODT (OpenDocument Text) format support.
#[cfg(any(feature = "read-odt", feature = "write-odt"))]
pub mod odt {
    #[cfg(feature = "read-odt")]
    pub use rescribe_read_odt::parse;
    #[cfg(feature = "read-odt")]
    pub use rescribe_read_odt::parse_with_options;
    #[cfg(feature = "write-odt")]
    pub use rescribe_write_odt::emit;
    #[cfg(feature = "write-odt")]
    pub use rescribe_write_odt::emit_with_options;
}

/// OPML format support.
#[cfg(any(feature = "read-opml", feature = "write-opml"))]
pub mod opml {
    #[cfg(feature = "read-opml")]
    pub use rescribe_read_opml::parse;
    #[cfg(feature = "write-opml")]
    pub use rescribe_write_opml::emit;
}

/// Org-mode format support.
#[cfg(any(feature = "read-org", feature = "write-org"))]
pub mod org {
    #[cfg(feature = "read-org")]
    pub use rescribe_read_org::parse;
    #[cfg(feature = "read-org")]
    pub use rescribe_read_org::parse_with_options;
    #[cfg(feature = "write-org")]
    pub use rescribe_write_org::emit;
    #[cfg(feature = "write-org")]
    pub use rescribe_write_org::emit_with_options;
}

/// Pandoc JSON format support.
#[cfg(any(feature = "read-pandoc-json", feature = "write-pandoc-json"))]
pub mod pandoc_json {
    #[cfg(feature = "read-pandoc-json")]
    pub use rescribe_read_pandoc_json::parse;
    #[cfg(feature = "read-pandoc-json")]
    pub use rescribe_read_pandoc_json::parse_with_options;
    #[cfg(feature = "write-pandoc-json")]
    pub use rescribe_write_pandoc_json::emit;
    #[cfg(feature = "write-pandoc-json")]
    pub use rescribe_write_pandoc_json::emit_with_options;
}

/// PDF format support (reader only).
#[cfg(feature = "read-pdf")]
pub mod pdf {
    pub use rescribe_read_pdf::parse;
    pub use rescribe_read_pdf::parse_with_options;
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
    #[cfg(feature = "read-pod")]
    pub use rescribe_read_pod::parse;
    #[cfg(feature = "read-pod")]
    pub use rescribe_read_pod::parse_with_options;
    #[cfg(feature = "write-pod")]
    pub use rescribe_write_pod::emit;
    #[cfg(feature = "write-pod")]
    pub use rescribe_write_pod::emit_with_options;
}

/// PPTX (PowerPoint) format support.
#[cfg(any(feature = "read-pptx", feature = "write-pptx"))]
pub mod pptx {
    #[cfg(feature = "read-pptx")]
    pub use rescribe_read_pptx::parse;
    #[cfg(feature = "read-pptx")]
    pub use rescribe_read_pptx::parse_with_options;
    #[cfg(feature = "write-pptx")]
    pub use rescribe_write_pptx::emit;
    #[cfg(feature = "write-pptx")]
    pub use rescribe_write_pptx::emit_with_options;
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
    #[cfg(feature = "read-ris")]
    pub use rescribe_read_ris::parse;
    #[cfg(feature = "read-ris")]
    pub use rescribe_read_ris::parse_with_options;
    #[cfg(feature = "write-ris")]
    pub use rescribe_write_ris::emit;
    #[cfg(feature = "write-ris")]
    pub use rescribe_write_ris::emit_with_options;
}

/// reStructuredText format support.
#[cfg(any(feature = "read-rst", feature = "write-rst"))]
pub mod rst {
    #[cfg(feature = "read-rst")]
    pub use rescribe_read_rst::parse;
    #[cfg(feature = "read-rst")]
    pub use rescribe_read_rst::parse_with_options;
    #[cfg(feature = "write-rst")]
    pub use rescribe_write_rst::emit;
    #[cfg(feature = "write-rst")]
    pub use rescribe_write_rst::emit_with_options;
}

/// RTF (Rich Text Format) support.
#[cfg(any(feature = "read-rtf", feature = "write-rtf"))]
pub mod rtf {
    #[cfg(feature = "read-rtf")]
    pub use rescribe_read_rtf::parse;
    #[cfg(feature = "read-rtf")]
    pub use rescribe_read_rtf::parse_with_options;
    #[cfg(feature = "write-rtf")]
    pub use rescribe_write_rtf::emit;
    #[cfg(feature = "write-rtf")]
    pub use rescribe_write_rtf::emit_with_options;
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
    #[cfg(feature = "read-t2t")]
    pub use rescribe_read_t2t::parse;
    #[cfg(feature = "read-t2t")]
    pub use rescribe_read_t2t::parse_with_options;
    #[cfg(feature = "write-t2t")]
    pub use rescribe_write_t2t::emit;
    #[cfg(feature = "write-t2t")]
    pub use rescribe_write_t2t::emit_with_options;
}

/// TEI (Text Encoding Initiative) format support.
#[cfg(any(feature = "read-tei", feature = "write-tei"))]
pub mod tei {
    #[cfg(feature = "read-tei")]
    pub use rescribe_read_tei::parse;
    #[cfg(feature = "write-tei")]
    pub use rescribe_write_tei::emit;
}

/// Texinfo (GNU documentation) format support.
#[cfg(any(feature = "read-texinfo", feature = "write-texinfo"))]
pub mod texinfo {
    #[cfg(feature = "read-texinfo")]
    pub use rescribe_read_texinfo::parse;
    #[cfg(feature = "read-texinfo")]
    pub use rescribe_read_texinfo::parse_with_options;
    #[cfg(feature = "write-texinfo")]
    pub use rescribe_write_texinfo::emit;
    #[cfg(feature = "write-texinfo")]
    pub use rescribe_write_texinfo::emit_with_options;
}

/// Textile markup format support.
#[cfg(any(feature = "read-textile", feature = "write-textile"))]
pub mod textile {
    #[cfg(feature = "read-textile")]
    pub use rescribe_read_textile::parse;
    #[cfg(feature = "read-textile")]
    pub use rescribe_read_textile::parse_with_options;
    #[cfg(feature = "write-textile")]
    pub use rescribe_write_textile::emit;
    #[cfg(feature = "write-textile")]
    pub use rescribe_write_textile::emit_with_options;
}

/// TikiWiki format support.
#[cfg(any(feature = "read-tikiwiki", feature = "write-tikiwiki"))]
pub mod tikiwiki {
    #[cfg(feature = "read-tikiwiki")]
    pub use rescribe_read_tikiwiki::parse;
    #[cfg(feature = "read-tikiwiki")]
    pub use rescribe_read_tikiwiki::parse_with_options;
    #[cfg(feature = "write-tikiwiki")]
    pub use rescribe_write_tikiwiki::emit;
    #[cfg(feature = "write-tikiwiki")]
    pub use rescribe_write_tikiwiki::emit_with_options;
}

/// TSV (Tab-Separated Values) format support.
#[cfg(any(feature = "read-tsv", feature = "write-tsv"))]
pub mod tsv {
    #[cfg(feature = "read-tsv")]
    pub use rescribe_read_tsv::parse;
    #[cfg(feature = "read-tsv")]
    pub use rescribe_read_tsv::parse_with_options;
    #[cfg(feature = "write-tsv")]
    pub use rescribe_write_tsv::emit;
    #[cfg(feature = "write-tsv")]
    pub use rescribe_write_tsv::emit_with_options;
}

/// TWiki format support.
#[cfg(any(feature = "read-twiki", feature = "write-twiki"))]
pub mod twiki {
    #[cfg(feature = "read-twiki")]
    pub use rescribe_read_twiki::parse;
    #[cfg(feature = "read-twiki")]
    pub use rescribe_read_twiki::parse_with_options;
    #[cfg(feature = "write-twiki")]
    pub use rescribe_write_twiki::emit;
    #[cfg(feature = "write-twiki")]
    pub use rescribe_write_twiki::emit_with_options;
}

/// Typst format support.
#[cfg(any(feature = "read-typst", feature = "write-typst"))]
pub mod typst {
    #[cfg(feature = "read-typst")]
    pub use rescribe_read_typst::parse;
    #[cfg(feature = "read-typst")]
    pub use rescribe_read_typst::parse_with_options;
    #[cfg(feature = "write-typst")]
    pub use rescribe_write_typst::emit;
    #[cfg(feature = "write-typst")]
    pub use rescribe_write_typst::emit_with_options;
}

/// VimWiki format support.
#[cfg(any(feature = "read-vimwiki", feature = "write-vimwiki"))]
pub mod vimwiki {
    #[cfg(feature = "read-vimwiki")]
    pub use rescribe_read_vimwiki::parse;
    #[cfg(feature = "read-vimwiki")]
    pub use rescribe_read_vimwiki::parse_with_options;
    #[cfg(feature = "write-vimwiki")]
    pub use rescribe_write_vimwiki::emit;
    #[cfg(feature = "write-vimwiki")]
    pub use rescribe_write_vimwiki::emit_with_options;
}

/// XLSX (Excel) format support.
#[cfg(any(feature = "read-xlsx", feature = "write-xlsx"))]
pub mod xlsx {
    #[cfg(feature = "read-xlsx")]
    pub use rescribe_read_xlsx::parse;
    #[cfg(feature = "read-xlsx")]
    pub use rescribe_read_xlsx::parse_bytes;
    #[cfg(feature = "read-xlsx")]
    pub use rescribe_read_xlsx::parse_file;
    #[cfg(feature = "write-xlsx")]
    pub use rescribe_write_xlsx::emit;
    #[cfg(feature = "write-xlsx")]
    pub use rescribe_write_xlsx::emit_with_options;
}

/// XWiki format support.
#[cfg(any(feature = "read-xwiki", feature = "write-xwiki"))]
pub mod xwiki {
    #[cfg(feature = "read-xwiki")]
    pub use rescribe_read_xwiki::parse;
    #[cfg(feature = "read-xwiki")]
    pub use rescribe_read_xwiki::parse_with_options;
    #[cfg(feature = "write-xwiki")]
    pub use rescribe_write_xwiki::emit;
    #[cfg(feature = "write-xwiki")]
    pub use rescribe_write_xwiki::emit_with_options;
}

/// ZimWiki (Zim Desktop Wiki) format support.
#[cfg(any(feature = "read-zimwiki", feature = "write-zimwiki"))]
pub mod zimwiki {
    #[cfg(feature = "read-zimwiki")]
    pub use rescribe_read_zimwiki::parse;
    #[cfg(feature = "read-zimwiki")]
    pub use rescribe_read_zimwiki::parse_with_options;
    #[cfg(feature = "write-zimwiki")]
    pub use rescribe_write_zimwiki::emit;
    #[cfg(feature = "write-zimwiki")]
    pub use rescribe_write_zimwiki::emit_with_options;
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
