//! Streaming event iterator over ANSI input.
//!
//! The event stream is the parser itself as an iterator — `EventIter` holds
//! the parse state and `next()` advances it, returning one event per call.

use std::borrow::Cow;

use crate::ast::{Color, CursorDirection, EraseMode, Style};

/// A streaming event from an ANSI byte stream.
#[derive(Debug, Clone, PartialEq)]
pub enum Event<'a> {
    /// Plain or styled text.
    Text {
        text: Cow<'a, str>,
        style: Style,
    },
    /// A newline.
    Newline,
    /// Style change (SGR sequence applied — no text emitted).
    SetStyle(Style),
    /// Reset all styling to default (SGR 0).
    ResetStyle,
    /// Cursor movement.
    CursorMove {
        direction: CursorDirection,
        count: u32,
    },
    /// Cursor absolute position.
    CursorPosition {
        row: u32,
        col: u32,
    },
    /// Erase in display.
    EraseDisplay(EraseMode),
    /// Erase in line.
    EraseLine(EraseMode),
    /// Cursor show/hide.
    CursorVisibility(bool),
    /// Save cursor position.
    SaveCursor,
    /// Restore cursor position.
    RestoreCursor,
    /// Scroll region.
    ScrollRegion {
        top: u32,
        bottom: u32,
    },
    /// Hyperlink with URL and display text.
    Hyperlink {
        url: String,
        text: String,
        style: Style,
    },
    /// Unrecognised escape sequence preserved verbatim.
    RawEscape(Cow<'a, str>),
}

/// Owned version of [`Event`] (no lifetime parameter).
pub type OwnedEvent = Event<'static>;

impl<'a> Event<'a> {
    /// Convert to an owned event.
    pub fn into_owned(self) -> OwnedEvent {
        match self {
            Event::Text { text, style } => Event::Text {
                text: Cow::Owned(text.into_owned()),
                style,
            },
            Event::Newline => Event::Newline,
            Event::SetStyle(s) => Event::SetStyle(s),
            Event::ResetStyle => Event::ResetStyle,
            Event::CursorMove { direction, count } => Event::CursorMove { direction, count },
            Event::CursorPosition { row, col } => Event::CursorPosition { row, col },
            Event::EraseDisplay(m) => Event::EraseDisplay(m),
            Event::EraseLine(m) => Event::EraseLine(m),
            Event::CursorVisibility(v) => Event::CursorVisibility(v),
            Event::SaveCursor => Event::SaveCursor,
            Event::RestoreCursor => Event::RestoreCursor,
            Event::ScrollRegion { top, bottom } => Event::ScrollRegion { top, bottom },
            Event::Hyperlink { url, text, style } => Event::Hyperlink { url, text, style },
            Event::RawEscape(s) => Event::RawEscape(Cow::Owned(s.into_owned())),
        }
    }
}

/// Pull-based event iterator over ANSI input.
///
/// Created by [`events()`].  Each call to `next()` advances the parse state
/// and returns one event.
pub struct EventIter<'a> {
    input: &'a [u8],
    pos: usize,
    style: Style,
    /// Buffered events from complex sequences (e.g., SGR that doesn't produce
    /// a text event on its own).
    pending: Vec<Event<'a>>,
}

impl<'a> EventIter<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        EventIter {
            input,
            pos: 0,
            style: Style::default(),
            pending: Vec::new(),
        }
    }
}

impl<'a> Iterator for EventIter<'a> {
    type Item = Event<'a>;

    fn next(&mut self) -> Option<Event<'a>> {
        // Drain any pending events first.
        if !self.pending.is_empty() {
            return Some(self.pending.remove(0));
        }

        if self.pos >= self.input.len() {
            return None;
        }

        let b = self.input[self.pos];

        if b == b'\n' {
            self.pos += 1;
            return Some(Event::Newline);
        }

        if b == 0x1b {
            return self.parse_escape();
        }

        // Accumulate plain text.
        let start = self.pos;
        while self.pos < self.input.len()
            && self.input[self.pos] != 0x1b
            && self.input[self.pos] != b'\n'
        {
            self.pos += 1;
        }
        let text = String::from_utf8_lossy(&self.input[start..self.pos]);
        Some(Event::Text {
            text: Cow::Owned(text.into_owned()),
            style: self.style.clone(),
        })
    }
}

impl<'a> EventIter<'a> {
    fn parse_escape(&mut self) -> Option<Event<'a>> {
        self.pos += 1; // skip ESC
        if self.pos >= self.input.len() {
            return Some(Event::RawEscape(Cow::Borrowed("\x1b")));
        }

        match self.input[self.pos] {
            b'[' => {
                self.pos += 1;
                self.parse_csi_event()
            }
            b']' => {
                self.pos += 1;
                self.parse_osc_event()
            }
            b'7' => {
                self.pos += 1;
                Some(Event::SaveCursor)
            }
            b'8' => {
                self.pos += 1;
                Some(Event::RestoreCursor)
            }
            b'(' | b')' => {
                // Charset designation — skip.
                let start = self.pos - 1;
                self.pos += 1; // skip ( or )
                if self.pos < self.input.len() {
                    self.pos += 1; // skip designator byte
                }
                let raw = String::from_utf8_lossy(&self.input[start..self.pos]);
                Some(Event::RawEscape(Cow::Owned(raw.into_owned())))
            }
            _ => {
                let start = self.pos - 1;
                self.pos += 1;
                let raw = String::from_utf8_lossy(&self.input[start..self.pos]);
                Some(Event::RawEscape(Cow::Owned(raw.into_owned())))
            }
        }
    }

    fn parse_csi_event(&mut self) -> Option<Event<'a>> {
        let mut params = String::new();
        let mut private = false;

        if self.pos < self.input.len() && self.input[self.pos] == b'?' {
            private = true;
            self.pos += 1;
        }

        while self.pos < self.input.len() {
            let b = self.input[self.pos];
            if b.is_ascii_digit() || b == b';' || b == b':' {
                params.push(b as char);
                self.pos += 1;
            } else {
                break;
            }
        }

        if self.pos >= self.input.len() {
            let raw = format!("\x1b[{}{}", if private { "?" } else { "" }, params);
            return Some(Event::RawEscape(Cow::Owned(raw)));
        }

        let term = self.input[self.pos];
        self.pos += 1;

        if private {
            return match term {
                b'h' if params == "25" => Some(Event::CursorVisibility(true)),
                b'l' if params == "25" => Some(Event::CursorVisibility(false)),
                _ => {
                    let raw = format!("\x1b[?{}{}", params, term as char);
                    Some(Event::RawEscape(Cow::Owned(raw)))
                }
            };
        }

        match term {
            b'm' => {
                self.apply_sgr_event(&params);
                // SGR events don't emit text — recurse to get the next real event,
                // but first push a SetStyle/ResetStyle event.
                if self.style.is_empty() && !params.is_empty() {
                    Some(Event::ResetStyle)
                } else {
                    Some(Event::SetStyle(self.style.clone()))
                }
            }
            b'A' => Some(Event::CursorMove {
                direction: CursorDirection::Up,
                count: parse_u32(&params, 1),
            }),
            b'B' => Some(Event::CursorMove {
                direction: CursorDirection::Down,
                count: parse_u32(&params, 1),
            }),
            b'C' => Some(Event::CursorMove {
                direction: CursorDirection::Forward,
                count: parse_u32(&params, 1),
            }),
            b'D' => Some(Event::CursorMove {
                direction: CursorDirection::Back,
                count: parse_u32(&params, 1),
            }),
            b'H' | b'f' => {
                let parts: Vec<&str> = params.split(';').collect();
                let row = parts.first().and_then(|s| s.parse().ok()).unwrap_or(1);
                let col = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                Some(Event::CursorPosition { row, col })
            }
            b'J' => {
                let n = parse_u32(&params, 0);
                let mode = match n {
                    1 => EraseMode::ToBeginning,
                    2 => EraseMode::All,
                    _ => EraseMode::ToEnd,
                };
                Some(Event::EraseDisplay(mode))
            }
            b'K' => {
                let n = parse_u32(&params, 0);
                let mode = match n {
                    1 => EraseMode::ToBeginning,
                    2 => EraseMode::All,
                    _ => EraseMode::ToEnd,
                };
                Some(Event::EraseLine(mode))
            }
            b's' => Some(Event::SaveCursor),
            b'u' => Some(Event::RestoreCursor),
            b'r' => {
                let parts: Vec<&str> = params.split(';').collect();
                let top = parts.first().and_then(|s| s.parse().ok()).unwrap_or(1);
                let bottom = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(24);
                Some(Event::ScrollRegion { top, bottom })
            }
            _ => {
                let raw = format!("\x1b[{}{}", params, term as char);
                Some(Event::RawEscape(Cow::Owned(raw)))
            }
        }
    }

    fn parse_osc_event(&mut self) -> Option<Event<'a>> {
        let mut osc_buf = Vec::new();
        while self.pos < self.input.len() {
            if self.input[self.pos] == 0x07 {
                self.pos += 1;
                break;
            }
            if self.input[self.pos] == 0x1b
                && self.pos + 1 < self.input.len()
                && self.input[self.pos + 1] == b'\\'
            {
                self.pos += 2;
                break;
            }
            osc_buf.push(self.input[self.pos]);
            self.pos += 1;
        }

        let osc_str = String::from_utf8_lossy(&osc_buf).into_owned();

        // OSC 8 hyperlink.
        if let Some(rest) = osc_str.strip_prefix("8;")
            && let Some(semi_pos) = rest.find(';')
        {
            let url = &rest[semi_pos + 1..];
            if url.is_empty() {
                // Close — return raw.
                return Some(Event::RawEscape(Cow::Owned(
                    "\x1b]8;;\x07".to_string(),
                )));
            }
            // Collect link text until closing OSC 8.
            let mut link_text = String::new();
            while self.pos < self.input.len() {
                if self.input[self.pos] == 0x1b
                    && self.pos + 1 < self.input.len()
                    && self.input[self.pos + 1] == b']'
                {
                    let inner_start = self.pos + 2;
                    let mut inner_end = inner_start;
                    let mut inner = Vec::new();
                    while inner_end < self.input.len() {
                        if self.input[inner_end] == 0x07 {
                            inner_end += 1;
                            break;
                        }
                        if self.input[inner_end] == 0x1b
                            && inner_end + 1 < self.input.len()
                            && self.input[inner_end + 1] == b'\\'
                        {
                            inner_end += 2;
                            break;
                        }
                        inner.push(self.input[inner_end]);
                        inner_end += 1;
                    }
                    let inner_str = String::from_utf8_lossy(&inner);
                    if inner_str.starts_with("8;") {
                        self.pos = inner_end;
                        break;
                    }
                }
                if self.input[self.pos] == b'\n' {
                    break;
                }
                link_text.push(self.input[self.pos] as char);
                self.pos += 1;
            }
            return Some(Event::Hyperlink {
                url: url.to_string(),
                text: link_text,
                style: self.style.clone(),
            });
        }

        Some(Event::RawEscape(Cow::Owned(format!(
            "\x1b]{}\x07",
            osc_str
        ))))
    }

    fn apply_sgr_event(&mut self, params: &str) {
        if params.is_empty() {
            self.style = Style::default();
            return;
        }
        let codes: Vec<&str> = params.split(';').collect();
        let mut i = 0;
        while i < codes.len() {
            let code = codes[i].trim();
            match code {
                "0" | "" => self.style = Style::default(),
                "1" => self.style.bold = true,
                "2" => self.style.dim = true,
                "3" => self.style.italic = true,
                "4" => self.style.underline = true,
                "5" => self.style.blink = true,
                "6" => self.style.rapid_blink = true,
                "7" => self.style.reverse = true,
                "8" => self.style.hidden = true,
                "9" => self.style.strikethrough = true,
                "21" => self.style.double_underline = true,
                "22" => {
                    self.style.bold = false;
                    self.style.dim = false;
                }
                "23" => self.style.italic = false,
                "24" => {
                    self.style.underline = false;
                    self.style.double_underline = false;
                }
                "25" => {
                    self.style.blink = false;
                    self.style.rapid_blink = false;
                }
                "27" => self.style.reverse = false,
                "28" => self.style.hidden = false,
                "29" => self.style.strikethrough = false,
                "39" => self.style.fg = None,
                "49" => self.style.bg = None,
                "53" => self.style.overline = true,
                "55" => self.style.overline = false,
                "59" => self.style.underline_color = None,
                c @ ("30" | "31" | "32" | "33" | "34" | "35" | "36" | "37") => {
                    let n: u8 = c.parse().unwrap();
                    self.style.fg = Some(Color::Standard(n - 30));
                }
                c @ ("40" | "41" | "42" | "43" | "44" | "45" | "46" | "47") => {
                    let n: u8 = c.parse().unwrap();
                    self.style.bg = Some(Color::Standard(n - 40));
                }
                c @ ("90" | "91" | "92" | "93" | "94" | "95" | "96" | "97") => {
                    let n: u8 = c.parse().unwrap();
                    self.style.fg = Some(Color::Bright(n - 90));
                }
                c @ ("100" | "101" | "102" | "103" | "104" | "105" | "106" | "107") => {
                    let n: u8 = c.parse().unwrap();
                    self.style.bg = Some(Color::Bright(n - 100));
                }
                "38" => {
                    if let Some(color) = parse_ext_color(&codes, &mut i) {
                        self.style.fg = Some(color);
                    }
                }
                "48" => {
                    if let Some(color) = parse_ext_color(&codes, &mut i) {
                        self.style.bg = Some(color);
                    }
                }
                "58" => {
                    if let Some(color) = parse_ext_color(&codes, &mut i) {
                        self.style.underline_color = Some(color);
                    }
                }
                _ => {} // Unknown — skip silently in event mode.
            }
            i += 1;
        }
    }
}

fn parse_ext_color(codes: &[&str], i: &mut usize) -> Option<Color> {
    if *i + 1 >= codes.len() {
        return None;
    }
    match codes[*i + 1].trim() {
        "5" => {
            if *i + 2 < codes.len() {
                let n: u8 = codes[*i + 2].trim().parse().unwrap_or(0);
                *i += 2;
                Some(Color::Palette(n))
            } else {
                *i += 1;
                None
            }
        }
        "2" => {
            if *i + 4 < codes.len() {
                let r: u8 = codes[*i + 2].trim().parse().unwrap_or(0);
                let g: u8 = codes[*i + 3].trim().parse().unwrap_or(0);
                let b: u8 = codes[*i + 4].trim().parse().unwrap_or(0);
                *i += 4;
                Some(Color::Rgb(r, g, b))
            } else {
                *i += 1;
                None
            }
        }
        _ => None,
    }
}

fn parse_u32(s: &str, default: u32) -> u32 {
    if s.is_empty() {
        default
    } else {
        s.parse().unwrap_or(default)
    }
}

/// Parse `input` and return a streaming iterator of events.
pub fn events(input: &[u8]) -> EventIter<'_> {
    EventIter::new(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_events_plain_text() {
        let evs: Vec<_> = events(b"Hello").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::Text { text, .. } if text == "Hello")));
    }

    #[test]
    fn test_events_bold() {
        let evs: Vec<_> = events(b"\x1b[1mBold\x1b[0m").collect();
        assert!(evs.iter().any(|e| matches!(e, Event::SetStyle(s) if s.bold)));
        assert!(evs.iter().any(|e| matches!(e, Event::Text { text, style, .. } if text == "Bold" && style.bold)));
        assert!(evs.iter().any(|e| matches!(e, Event::ResetStyle)));
    }

    #[test]
    fn test_events_cursor_move() {
        let evs: Vec<_> = events(b"\x1b[5A").collect();
        assert!(evs.iter().any(|e| matches!(
            e,
            Event::CursorMove {
                direction: CursorDirection::Up,
                count: 5,
            }
        )));
    }

    #[test]
    fn test_events_hyperlink() {
        let evs: Vec<_> =
            events(b"\x1b]8;;https://example.com\x07Click\x1b]8;;\x07").collect();
        assert!(evs
            .iter()
            .any(|e| matches!(e, Event::Hyperlink { url, text, .. } if url == "https://example.com" && text == "Click")));
    }
}
