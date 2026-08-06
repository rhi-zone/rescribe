//! BibTeX reader and writer for rescribe.
//!
//! Parses BibTeX/BibLaTeX bibliography files into, and emits them from,
//! rescribe's document IR, using the
//! `bibliography`/`bibliography_entry`/`bibliography_field` node kinds (see
//! `rescribe_std::node` and ADR 0005 in the rescribe repo).
//!
//! Actual BibTeX syntax (entry headers, field escaping, brace wrapping) is
//! produced by the `biblatex` crate's own parser and
//! `Entry::to_bibtex_string()` / `Bibliography::to_bibtex_string()`. This
//! crate's job is translating between rescribe's IR shapes and
//! `biblatex::Entry` — it does not hand-roll BibTeX parsing, escaping, or
//! field/entry syntax itself.
//!
//! # Example
//!
//! ```
//! use rescribe_fmt_bibtex::parse;
//!
//! let bibtex = r#"
//! @article{smith2020,
//!   author = {John Smith},
//!   title = {A Great Paper},
//!   journal = {Nature},
//!   year = {2020},
//! }
//! "#;
//!
//! let result = parse(bibtex).unwrap();
//! let doc = result.value;
//! ```

mod read;
mod write;

pub use read::parse;
pub use write::{emit, emit_with_options};
