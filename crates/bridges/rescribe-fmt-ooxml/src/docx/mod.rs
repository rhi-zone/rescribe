//! DOCX (Word) reader + writer for rescribe.
//!
//! Translates between Word documents (.docx) and rescribe's document IR
//! using the `ooxml-wml` crate.
//!
//! # Example
//!
//! ```ignore
//! use rescribe_fmt_ooxml::docx::parse_file;
//!
//! let result = parse_file("document.docx")?;
//! let doc = result.value;
//! // Process the document...
//! ```

mod read;
mod write;

pub use read::{parse, parse_bytes, parse_file};
pub use write::emit;

#[cfg(test)]
mod tests {
    // Tests would go here, but require actual DOCX files.
    // Integration tests can be added with test fixtures.
}
