//! Streaming RTF writer — serializes RTF token events to bytes.
//!
//! [`Writer`] accepts [`TokenEvent`] items (the low-level RTF token stream) and
//! writes the corresponding RTF bytes to the underlying `Write` sink.
//!
//! This is the inverse of the `token_events()` tokenizer: feeding the output of
//! `token_events(input)` into a `Writer` reproduces the original RTF bytes
//! exactly, including each control word's optional trailing-space delimiter
//! (`TokenEvent::ControlWord::had_delimiter_space` carries that bit, which
//! would otherwise be unrecoverable once tokenized).
//!
//! # Example
//! ```no_run
//! use rtf_fmt::writer::Writer;
//! use rtf_fmt::TokenEvent;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! // Reproduce a minimal RTF document from tokens
//! w.write_event(TokenEvent::GroupStart { span: Default::default() });
//! w.write_event(TokenEvent::ControlWord { name: "rtf".into(), param: Some(1), had_delimiter_space: false, span: Default::default() });
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
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer { sink }
    }

    /// Write one RTF token event to the sink.
    pub fn write_event(&mut self, event: TokenEvent) {
        match event {
            TokenEvent::GroupStart { .. } => {
                let _ = self.sink.write_all(b"{");
            }
            TokenEvent::GroupEnd { .. } => {
                let _ = self.sink.write_all(b"}");
            }
            TokenEvent::ControlWord {
                name,
                param,
                had_delimiter_space,
                ..
            } => {
                let _ = self.sink.write_all(b"\\");
                let _ = self.sink.write_all(name.as_bytes());
                if let Some(n) = param {
                    let _ = write!(self.sink, "{}", n);
                }
                // Whether a trailing space delimiter is written is NOT
                // derivable from `name`/`param` alone — RTF only requires
                // one when needed to disambiguate what follows, and
                // `emit()`'s own canonical output includes stylistic spaces
                // that aren't structurally required at all (e.g. `\f0
                // Times` has one, `\u65?` does not, both are
                // param-carrying). `had_delimiter_space` is the exact bit
                // the tokenizer recorded from the source; reproducing it
                // verbatim is the only way to get byte-identical
                // re-serialization instead of a merely-valid reformatting.
                if had_delimiter_space {
                    let _ = self.sink.write_all(b" ");
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
            }
            TokenEvent::Text { text, .. } => {
                // RTF text: escape { } \ characters
                let escaped = escape_rtf_text(&text);
                let _ = self.sink.write_all(escaped.as_bytes());
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
        w.write_event(TokenEvent::ControlWord {
            name: "rtf".into(),
            param: Some(1),
            had_delimiter_space: false,
            span: span(),
        });
        let bytes = w.finish();
        assert_eq!(bytes, b"\\rtf1");
    }

    #[test]
    fn test_writer_control_word_no_param() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TokenEvent::ControlWord {
            name: "par".into(),
            param: None,
            had_delimiter_space: true,
            span: span(),
        });
        let bytes = w.finish();
        assert_eq!(bytes, b"\\par ");
    }

    /// The bit `TokenEvent::ControlWord::had_delimiter_space` exists
    /// specifically for: reproducing the exact source bytes, not just a
    /// valid reformatting. Both directions (space present / space absent)
    /// must round-trip for both param and no-param control words.
    #[test]
    fn test_writer_byte_identical_delimiter_space() {
        for input in [
            &br"\ansi \deff0"[..],
            &br"\ansi\deff0"[..],
            &br"\f0 Times"[..],
            &br"\u65?"[..],
        ] {
            let tokens: Vec<_> = crate::events::token_events(input).collect();
            let mut w = Writer::new(Vec::<u8>::new());
            for t in tokens {
                w.write_event(t);
            }
            let output = w.finish();
            assert_eq!(
                output,
                input,
                "delimiter-space round-trip diverged for {:?}",
                String::from_utf8_lossy(input)
            );
        }
    }

    #[test]
    fn test_writer_hex_symbol() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TokenEvent::ControlSymbol {
            ch: '\'',
            hex_byte: Some(0xe9),
            span: span(),
        });
        let bytes = w.finish();
        assert_eq!(bytes, b"\\'e9");
    }

    #[test]
    fn test_writer_text_escaping() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(TokenEvent::Text {
            text: "a{b}c\\d".into(),
            span: span(),
        });
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
                TokenEvent::GroupStart { .. } => TokenEvent::GroupStart {
                    span: Span::new(0, 0),
                },
                TokenEvent::GroupEnd { .. } => TokenEvent::GroupEnd {
                    span: Span::new(0, 0),
                },
                TokenEvent::ControlWord {
                    name,
                    param,
                    had_delimiter_space,
                    ..
                } => TokenEvent::ControlWord {
                    name,
                    param,
                    had_delimiter_space,
                    span: Span::new(0, 0),
                },
                TokenEvent::ControlSymbol { ch, hex_byte, .. } => TokenEvent::ControlSymbol {
                    ch,
                    hex_byte,
                    span: Span::new(0, 0),
                },
                TokenEvent::Text { text, .. } => TokenEvent::Text {
                    text,
                    span: Span::new(0, 0),
                },
            }
        }

        let t1: Vec<_> = tokens.into_iter().map(strip).collect();
        let t2: Vec<_> = tokens2.into_iter().map(strip).collect();
        assert_eq!(
            t1,
            t2,
            "token roundtrip mismatch\n  output: {:?}",
            String::from_utf8_lossy(&output)
        );
    }
}
