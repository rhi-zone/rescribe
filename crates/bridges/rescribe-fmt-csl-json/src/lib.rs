//! CSL JSON reader and writer for rescribe.
//!
//! Parses and emits CSL JSON (Citation Style Language JSON) against
//! rescribe's document IR, using the
//! `bibliography`/`bibliography_entry`/`bibliography_field` node kinds (see
//! `rescribe_std::node` and ADR 0005 in the rescribe repo). This wraps
//! `serde_json` directly against the CSL JSON schema — there is no separate
//! general-purpose CSL JSON parsing library to delegate to.
//!
//! # Example
//!
//! ```
//! use rescribe_fmt_csl_json::parse;
//!
//! let csl = r#"[{
//!   "id": "smith2020",
//!   "type": "article-journal",
//!   "title": "A Great Paper",
//!   "author": [{"family": "Smith", "given": "John"}],
//!   "issued": {"date-parts": [[2020]]}
//! }]"#;
//!
//! let result = parse(csl).unwrap();
//! let doc = result.value;
//! ```

mod read;
mod write;

pub use read::parse;
pub use write::emit;
