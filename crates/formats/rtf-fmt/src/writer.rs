//! Streaming RTF writer — serializes RTF token events to bytes.
//!
//! [`Writer`] accepts [`TokenEvent`] items (the low-level RTF token stream) and
//! writes the corresponding RTF bytes to the underlying `Write` sink.
//!
//! This is the inverse of the `token_events()` tokenizer: feeding the output of
//! `token_events(input)` into a `Writer` should reproduce the original RTF bytes
//! (modulo whitespace normalization in control word delimiters).
//!
//! # Example
//! ```no_run
//! use rtf_fmt::writer::Writer;
//! use rtf_fmt::TokenEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! // Reproduce a minimal RTF document from tokens
//! w.write_event(TokenEvent::GroupStart { span: Default::default() });
//! w.write_event(TokenEvent::ControlWord { name: "rtf".into(), param: Some(1), span: Default::default() });
//! w.write_event(TokenEvent::Text { text: "Hello".into(), span: Default::default() });
//! w.write_event(TokenEvent::GroupEnd { span: Default::default() });
//! let bytes = w.finish();
//! ```

use crate::events::TokenEvent;
use std::io::Write;

/// Streaming RTF writer.
///
/// Feed token events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to recover the sink.
pub struct Writer<W: Write> {
    sink: W,
    /// Track whether the previous output was a control word (to know if we
    /// need a delimiter space before the next one or before literal text).
    last_was_control: bool,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer { sink, last_was_control: false }
    }

    /// Write one RTF token event to the sink.
    pub fn write_event(&mut self, event: TokenEvent) {
        match event {
            TokenEvent::GroupStart { .. } => {
                let _ = self.sink.write_all(b"{");
                self.last_was_control = false;
            }
            TokenEvent::GroupEnd { .. } => {
                let _ = self.sink.write_all(b"}");
                self.last_was_control = false;
            }
            TokenEvent::ControlWord { name, param, .. } => {
                let _ = self.sink.write_all(b"\\");
                let _ = self.sink.write_all(name.as_bytes());
                if let Some(n) = param {
                    let _ = write!(self.sink, "{}", n);
                    // Numeric parameter: no trailing space needed (delimiter is end of digits)
                    self.last_was_control = false;
                } else {
                    // Word-only control word: needs a trailing space as delimiter
                    let _ = self.sink.write_all(b" ");
                    self.last_was_control = true;
                }
            }
            TokenEvent::ControlSymbol { ch, hex_byte, .. } => {
                if ch == '\'' {
                    if let Some(b) = hex_byte {
                        let _ = write!(self.sink, "\\'{:02x}", b);
                    } else {
                        let _ = self.sink.write_all(b"\\'");
                    }
                } else {
                    let _ = self.sink.write_all(b"\\");
                    let mut buf = [0u8; 4];
                    let s = ch.encode_utf8(&mut buf);
                    let _ = self.sink.write_all(s.as_bytes());
                }
                self.last_was_control = false;
            }
            TokenEvent::Text { text, .. } => {
                // RTF text: escape { } \ characters
                let escaped = escape_rtf_text(&text);
                let _ = self.sink.write_all(escaped.as_bytes());
                self.last_was_control = false;
            }
        }
    }

    /// Flush and return the underlying sink.
    pub fn finish(self) -> W {
        self.sink
    }
}

fn escape_rtf_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Span;

    fn span() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn test_writer_group() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TokenEvent::GroupStart { span: span() });
        w.write_event(TokenEvent::GroupEnd { span: span() });
        assert_eq!(w.finish(), b"{}");
    }

    #[test]
    fn test_writer_control_word_with_param() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TokenEvent::ControlWord { name: "rtf".into(), param: Some(1), span: span() });
        let bytes = w.finish();
        assert_eq!(bytes, b"\\rtf1");
    }

    #[test]
    fn test_writer_control_word_no_param() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TokenEvent::ControlWord { name: "par".into(), param: None, span: span() });
        let bytes = w.finish();
        assert_eq!(bytes, b"\\par ");
    }

    #[test]
    fn test_writer_hex_symbol() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TokenEvent::ControlSymbol { ch: '\'', hex_byte: Some(0xe9), span: span() });
        let bytes = w.finish();
        assert_eq!(bytes, b"\\'e9");
    }

    #[test]
    fn test_writer_text_escaping() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TokenEvent::Text { text: "a{b}c\\d".into(), span: span() });
        let bytes = w.finish();
        assert_eq!(bytes, b"a\\{b\\}c\\\\d");
    }

    #[test]
    fn test_writer_roundtrip_tokens() {
        // Tokenize an RTF snippet, write it back, re-tokenize — token streams should match.
        let input = b"{\\rtf1\\ansi Hello World}";
        let tokens: Vec<_> = crate::events::token_events(input).collect();

        let mut w = Writer::new(Vec::<u8>::new());
        for ev in tokens.clone() {
            w.write_event(ev);
        }
        let output = w.finish();

        let tokens2: Vec<_> = crate::events::token_events(&output).collect();

        // Strip spans for comparison (positions differ after re-serialization)
        fn strip(ev: TokenEvent) -> TokenEvent {
            match ev {
                TokenEvent::GroupStart { .. } => TokenEvent::GroupStart { span: Span::new(0, 0) },
                TokenEvent::GroupEnd { .. } => TokenEvent::GroupEnd { span: Span::new(0, 0) },
                TokenEvent::ControlWord { name, param, .. } => {
                    TokenEvent::ControlWord { name, param, span: Span::new(0, 0) }
                }
                TokenEvent::ControlSymbol { ch, hex_byte, .. } => {
                    TokenEvent::ControlSymbol { ch, hex_byte, span: Span::new(0, 0) }
                }
                TokenEvent::Text { text, .. } => TokenEvent::Text { text, span: Span::new(0, 0) },
            }
        }

        let t1: Vec<_> = tokens.into_iter().map(strip).collect();
        let t2: Vec<_> = tokens2.into_iter().map(strip).collect();
        assert_eq!(t1, t2, "token roundtrip mismatch\n  output: {:?}", String::from_utf8_lossy(&output));
    }
}
