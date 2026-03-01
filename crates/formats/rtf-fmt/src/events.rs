/// Low-level pull parser: tokenize RTF into a stream of [`Event`]s.
///
/// This is a zero-allocation iterator over the raw RTF token stream.
/// Most callers will prefer the higher-level [`parse`][crate::parse] API.
use crate::ast::Span;

/// A single RTF token.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// `{` — begin a group; state should be pushed
    GroupStart { span: Span },
    /// `}` — end a group; state should be popped
    GroupEnd { span: Span },
    /// `\word` or `\word-N` or `\wordN`
    ControlWord {
        name: String,
        param: Option<i32>,
        span: Span,
    },
    /// `\X` where X is a single non-alpha character (e.g. `\\`, `\{`, `\}`, `\'`)
    ControlSymbol {
        ch: char,
        hex_byte: Option<u8>,
        span: Span,
    },
    /// A run of literal text characters
    Text { text: String, span: Span },
}

/// Returns an iterator that tokenizes `input` into RTF [`Event`]s.
pub fn events(input: &str) -> impl Iterator<Item = Event> + '_ {
    EventIter { input, pos: 0 }
}

struct EventIter<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        loop {
            if self.pos >= self.input.len() {
                return None;
            }

            let ch = self.current_char()?;

            match ch {
                '{' => {
                    let start = self.pos;
                    self.advance();
                    return Some(Event::GroupStart {
                        span: Span::new(start, self.pos),
                    });
                }
                '}' => {
                    let start = self.pos;
                    self.advance();
                    return Some(Event::GroupEnd {
                        span: Span::new(start, self.pos),
                    });
                }
                '\\' => {
                    let start = self.pos;
                    self.advance(); // skip '\'
                    if self.pos >= self.input.len() {
                        return None;
                    }
                    let next = self.current_char()?;
                    if next.is_ascii_alphabetic() {
                        let (name, param) = self.read_control_word();
                        return Some(Event::ControlWord {
                            name,
                            param,
                            span: Span::new(start, self.pos),
                        });
                    } else if next == '\'' {
                        // \'XX hex-encoded byte
                        self.advance(); // skip '\''
                        let hex_byte = if self.pos + 2 <= self.input.len() {
                            let hex = &self.input[self.pos..self.pos + 2];
                            let b = u8::from_str_radix(hex, 16).ok();
                            self.pos += 2;
                            b
                        } else {
                            None
                        };
                        return Some(Event::ControlSymbol {
                            ch: '\'',
                            hex_byte,
                            span: Span::new(start, self.pos),
                        });
                    } else {
                        let sym = next;
                        self.advance();
                        return Some(Event::ControlSymbol {
                            ch: sym,
                            hex_byte: None,
                            span: Span::new(start, self.pos),
                        });
                    }
                }
                '\n' | '\r' => {
                    // Bare newlines in RTF are ignored (they're not paragraph breaks)
                    self.advance();
                    continue;
                }
                _ => {
                    let start = self.pos;
                    let mut text = String::new();
                    while self.pos < self.input.len() {
                        match self.current_char() {
                            Some(c @ ('{' | '}' | '\\' | '\n' | '\r')) => {
                                let _ = c;
                                break;
                            }
                            Some(c) => {
                                text.push(c);
                                self.advance();
                            }
                            None => break,
                        }
                    }
                    if text.is_empty() {
                        continue;
                    }
                    return Some(Event::Text {
                        text,
                        span: Span::new(start, self.pos),
                    });
                }
            }
        }
    }
}

impl<'a> EventIter<'a> {
    fn current_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(ch) = self.current_char() {
            self.pos += ch.len_utf8();
        }
    }

    /// Read a control word name and optional numeric parameter.
    ///
    /// Called after the `\` and after verifying the next char is alphabetic.
    fn read_control_word(&mut self) -> (String, Option<i32>) {
        let mut name = String::new();
        while self.pos < self.input.len() {
            match self.current_char() {
                Some(c) if c.is_ascii_alphabetic() => {
                    name.push(c);
                    self.advance();
                }
                _ => break,
            }
        }

        let mut negative = false;
        if self.pos < self.input.len() && self.current_char() == Some('-') {
            negative = true;
            self.advance();
        }

        let mut param_str = String::new();
        while self.pos < self.input.len() {
            match self.current_char() {
                Some(c) if c.is_ascii_digit() => {
                    param_str.push(c);
                    self.advance();
                }
                _ => break,
            }
        }

        let param = if param_str.is_empty() {
            None
        } else {
            param_str
                .parse::<i32>()
                .ok()
                .map(|n| if negative { -n } else { n })
        };

        // Optional trailing space is a delimiter; consume it
        if self.pos < self.input.len() && self.current_char() == Some(' ') {
            self.advance();
        }

        (name, param)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_simple() {
        let evts: Vec<_> = events(r"{\rtf1 Hello\par}").collect();
        assert!(evts.iter().any(|e| matches!(e, Event::GroupStart { .. })));
        assert!(evts.iter().any(|e| matches!(e, Event::GroupEnd { .. })));
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::ControlWord { name, .. } if name == "rtf"))
        );
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::Text { text, .. } if text == "Hello"))
        );
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::ControlWord { name, .. } if name == "par"))
        );
    }

    #[test]
    fn test_events_control_symbol() {
        let evts: Vec<_> = events(r"\{").collect();
        assert!(
            evts.iter()
                .any(|e| matches!(e, Event::ControlSymbol { ch: '{', .. }))
        );
    }

    #[test]
    fn test_events_hex() {
        let evts: Vec<_> = events(r"\'41").collect();
        assert!(evts.iter().any(|e| matches!(
            e,
            Event::ControlSymbol {
                ch: '\'',
                hex_byte: Some(0x41),
                ..
            }
        )));
    }

    #[test]
    fn test_events_spans() {
        let input = r"\b hello";
        let evts: Vec<_> = events(input).collect();
        // \b starts at byte 0
        let cw = evts
            .iter()
            .find(|e| matches!(e, Event::ControlWord { name, .. } if name == "b"));
        let cw = cw.unwrap();
        if let Event::ControlWord { span, .. } = cw {
            assert_eq!(span.start, 0);
        }
    }
}
