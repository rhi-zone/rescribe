//! Chunk-driven batch reader for PPTX presentations.
//!
//! [`BatchParser`] accepts arbitrary-sized chunks via [`BatchParser::feed`]
//! and parses the complete presentation on [`BatchParser::finish`].
//!
//! # Memory model
//!
//! All chunks are buffered until `finish()` is called.
//! True incremental parsing is future work.
//!
//! # Example
//!
//! ```ignore
//! use ooxml_pml::BatchParser;
//!
//! let mut parser = BatchParser::new();
//! for chunk in file_stream {
//!     parser.feed(&chunk);
//! }
//! let presentation = parser.finish()?;
//! ```

use std::io::Cursor;
use crate::Result;
use crate::presentation::Presentation;

/// Chunk-driven PPTX parser.
#[derive(Default)]
pub struct BatchParser {
    buf: Vec<u8>,
}

impl BatchParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    pub fn finish(self) -> Result<Presentation<Cursor<Vec<u8>>>> {
        Presentation::from_reader(Cursor::new(self.buf))
    }
}
