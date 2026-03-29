//! Chunk-driven batch reader for DOCX documents.
//!
//! [`BatchParser`] accepts arbitrary-sized chunks via [`BatchParser::feed`]
//! and parses the complete document on [`BatchParser::finish`].
//!
//! # Memory model
//!
//! All chunks are buffered until `finish()` is called, so peak memory equals
//! the full DOCX size.  This satisfies the batch-reader API contract.
//! True incremental parsing (O(chunk) memory) is future work.
//!
//! # Example
//!
//! ```ignore
//! use ooxml_wml::BatchParser;
//!
//! let mut parser = BatchParser::new();
//! for chunk in file_stream {
//!     parser.feed(&chunk);
//! }
//! let doc = parser.finish()?;
//! ```

use std::io::Cursor;
use crate::Result;
use crate::document::Document;

/// Chunk-driven DOCX parser.
///
/// See the [module documentation](self) for details.
#[derive(Default)]
pub struct BatchParser {
    buf: Vec<u8>,
}

impl BatchParser {
    /// Create a new parser.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a chunk of bytes to the internal buffer.
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Parse the buffered bytes as a DOCX document.
    pub fn finish(self) -> Result<Document<Cursor<Vec<u8>>>> {
        Document::from_reader(Cursor::new(self.buf))
    }
}
