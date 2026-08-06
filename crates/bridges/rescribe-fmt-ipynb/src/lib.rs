//! Jupyter notebook (ipynb) reader and writer for rescribe.
//!
//! Parses and emits Jupyter notebooks against rescribe's document IR.
//! Markdown-cell content is delegated to `rescribe-read-markdown` /
//! `rescribe-write-markdown`; the ipynb container format itself is handled
//! here directly via `serde_json` against Jupyter's notebook JSON schema —
//! there is no separate general-purpose ipynb parsing library to delegate
//! to.
//!
//! # Example
//!
//! ```ignore
//! use rescribe_fmt_ipynb::parse;
//!
//! let ipynb_content = r#"{"nbformat": 4, "cells": []}"#;
//! let result = parse(ipynb_content)?;
//! let doc = result.value;
//! ```

mod read;
mod write;

pub use read::{parse, parse_bytes};
pub use write::emit;
