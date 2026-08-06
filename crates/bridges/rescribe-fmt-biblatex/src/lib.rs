//! BibLaTeX reader and writer for rescribe.
//!
//! Parses BibLaTeX bibliography files into, and emits them from, rescribe's
//! document IR. Handles BibLaTeX-specific entry types and fields (date,
//! journaltitle, etc.).
//!
//! Actual BibLaTeX syntax (entry headers, field escaping, brace wrapping) is
//! produced by the `biblatex` crate's own parser and
//! `Entry::to_biblatex_string()` / `Bibliography::to_biblatex_string()` (the
//! same crate `rescribe-fmt-bibtex` uses). This crate's job is translating
//! between rescribe's IR shapes and `biblatex::Entry` — it does not
//! hand-roll BibLaTeX parsing, escaping, or field/entry syntax itself.
//!
//! # Example
//!
//! ```
//! use rescribe_fmt_biblatex::parse;
//!
//! let biblatex = r#"
//! @article{smith2020,
//!   author = {John Smith},
//!   title = {A Great Paper},
//!   journaltitle = {Nature},
//!   date = {2020-05-15},
//! }
//! "#;
//!
//! let result = parse(biblatex).unwrap();
//! let doc = result.value;
//! ```

mod read;
mod write;

pub use read::{parse, parse_with_options};
pub use write::{emit, emit_with_options};
