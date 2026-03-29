//! Streaming ANSI writer — converts a stream of events to ANSI text.
//!
//! # Example
//! ```no_run
//! use ansi_fmt::writer::Writer;
//! use ansi_fmt::OwnedEvent;
//! use ansi_fmt::ast::Style;
//!
//! let mut w = Writer::new(Vec::<u8>::new());
//! let mut s = Style::default();
//! s.bold = true;
//! w.write_event(OwnedEvent::SetStyle(s.clone()));
//! w.write_event(OwnedEvent::Text { text: "Hello".to_string().into(), style: s });
//! w.write_event(OwnedEvent::ResetStyle);
//! let bytes = w.finish();
//! ```

use crate::ast::{Color, CursorDirection, EraseMode, Style};
use crate::events::OwnedEvent;
use std::io::Write;

/// Streaming ANSI writer.
///
/// Feed events with [`write_event`](Writer::write_event), then call
/// [`finish`](Writer::finish) to flush and recover the sink.
pub struct Writer<W: Write> {
    sink: W,
    current_style: Style,
}

impl<W: Write> Writer<W> {
    pub fn new(sink: W) -> Self {
        Writer {
            sink,
            current_style: Style::default(),
        }
    }

    /// Feed one event to the writer.
    pub fn write_event(&mut self, event: OwnedEvent) {
        match event {
            OwnedEvent::Text { text, style, .. } => {
                self.transition_style(&style);
                let _ = self.sink.write_all(text.as_bytes());
                self.current_style = style;
            }
            OwnedEvent::Newline => {
                let _ = self.sink.write_all(b"\n");
            }
            OwnedEvent::SetStyle(style) => {
                self.transition_style(&style);
                self.current_style = style;
            }
            OwnedEvent::ResetStyle => {
                let _ = self.sink.write_all(b"\x1b[0m");
                self.current_style = Style::default();
            }
            OwnedEvent::CursorMove { direction, count } => {
                let letter = match direction {
                    CursorDirection::Up => 'A',
                    CursorDirection::Down => 'B',
                    CursorDirection::Forward => 'C',
                    CursorDirection::Back => 'D',
                };
                let _ = write!(self.sink, "\x1b[{}{}", count, letter);
            }
            OwnedEvent::CursorPosition { row, col } => {
                let _ = write!(self.sink, "\x1b[{};{}H", row, col);
            }
            OwnedEvent::EraseDisplay(mode) => {
                let n = match mode {
                    EraseMode::ToEnd => 0,
                    EraseMode::ToBeginning => 1,
                    EraseMode::All => 2,
                };
                let _ = write!(self.sink, "\x1b[{}J", n);
            }
            OwnedEvent::EraseLine(mode) => {
                let n = match mode {
                    EraseMode::ToEnd => 0,
                    EraseMode::ToBeginning => 1,
                    EraseMode::All => 2,
                };
                let _ = write!(self.sink, "\x1b[{}K", n);
            }
            OwnedEvent::CursorVisibility(visible) => {
                if visible {
                    let _ = self.sink.write_all(b"\x1b[?25h");
                } else {
                    let _ = self.sink.write_all(b"\x1b[?25l");
                }
            }
            OwnedEvent::SaveCursor => {
                let _ = self.sink.write_all(b"\x1b[s");
            }
            OwnedEvent::RestoreCursor => {
                let _ = self.sink.write_all(b"\x1b[u");
            }
            OwnedEvent::ScrollRegion { top, bottom } => {
                let _ = write!(self.sink, "\x1b[{};{}r", top, bottom);
            }
            OwnedEvent::Hyperlink { url, text, style } => {
                self.transition_style(&style);
                self.current_style = style;
                let _ = write!(self.sink, "\x1b]8;;{}\x07{}\x1b]8;;\x07", url, text);
            }
            OwnedEvent::RawEscape(raw) => {
                let _ = self.sink.write_all(raw.as_bytes());
            }
        }
    }

    fn transition_style(&mut self, target: &Style) {
        if &self.current_style == target {
            return;
        }

        if target.is_empty() {
            let _ = self.sink.write_all(b"\x1b[0m");
            return;
        }

        // Build SGR codes.
        let mut codes = Vec::new();

        // If we need to turn off any attribute, reset first.
        let needs_reset = (self.current_style.bold && !target.bold)
            || (self.current_style.dim && !target.dim)
            || (self.current_style.italic && !target.italic)
            || (self.current_style.underline && !target.underline)
            || (self.current_style.blink && !target.blink)
            || (self.current_style.reverse && !target.reverse)
            || (self.current_style.hidden && !target.hidden)
            || (self.current_style.strikethrough && !target.strikethrough)
            || (self.current_style.overline && !target.overline)
            || (self.current_style.fg.is_some() && target.fg.is_none())
            || (self.current_style.bg.is_some() && target.bg.is_none());

        if needs_reset {
            codes.push("0".to_string());
            append_all_style_codes(target, &mut codes);
        } else {
            if target.bold && !self.current_style.bold {
                codes.push("1".to_string());
            }
            if target.dim && !self.current_style.dim {
                codes.push("2".to_string());
            }
            if target.italic && !self.current_style.italic {
                codes.push("3".to_string());
            }
            if target.underline && !self.current_style.underline {
                codes.push("4".to_string());
            }
            if target.blink && !self.current_style.blink {
                codes.push("5".to_string());
            }
            if target.reverse && !self.current_style.reverse {
                codes.push("7".to_string());
            }
            if target.hidden && !self.current_style.hidden {
                codes.push("8".to_string());
            }
            if target.strikethrough && !self.current_style.strikethrough {
                codes.push("9".to_string());
            }
            if target.overline && !self.current_style.overline {
                codes.push("53".to_string());
            }
            if target.fg != self.current_style.fg
                && let Some(ref c) = target.fg
            {
                append_color_codes(c, true, &mut codes);
            }
            if target.bg != self.current_style.bg
                && let Some(ref c) = target.bg
            {
                append_color_codes(c, false, &mut codes);
            }
        }

        if !codes.is_empty() {
            let _ = write!(self.sink, "\x1b[{}m", codes.join(";"));
        }
    }

    /// Flush and return the underlying sink.
    pub fn finish(mut self) -> W {
        if !self.current_style.is_empty() {
            let _ = self.sink.write_all(b"\x1b[0m");
        }
        self.sink
    }
}

fn append_all_style_codes(style: &Style, codes: &mut Vec<String>) {
    if style.bold {
        codes.push("1".to_string());
    }
    if style.dim {
        codes.push("2".to_string());
    }
    if style.italic {
        codes.push("3".to_string());
    }
    if style.underline {
        codes.push("4".to_string());
    }
    if style.blink {
        codes.push("5".to_string());
    }
    if style.reverse {
        codes.push("7".to_string());
    }
    if style.hidden {
        codes.push("8".to_string());
    }
    if style.strikethrough {
        codes.push("9".to_string());
    }
    if style.overline {
        codes.push("53".to_string());
    }
    if let Some(ref c) = style.fg {
        append_color_codes(c, true, codes);
    }
    if let Some(ref c) = style.bg {
        append_color_codes(c, false, codes);
    }
}

fn append_color_codes(color: &Color, foreground: bool, codes: &mut Vec<String>) {
    let base = if foreground { 30 } else { 40 };
    match color {
        Color::Standard(n) => codes.push(format!("{}", base + n)),
        Color::Bright(n) => codes.push(format!("{}", base + 60 + n)),
        Color::Palette(n) => {
            codes.push(format!("{}", base + 8));
            codes.push("5".to_string());
            codes.push(format!("{}", n));
        }
        Color::Rgb(r, g, b) => {
            codes.push(format!("{}", base + 8));
            codes.push("2".to_string());
            codes.push(format!("{}", r));
            codes.push(format!("{}", g));
            codes.push(format!("{}", b));
        }
        Color::Default => codes.push(format!("{}", base + 9)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Style;

    #[test]
    fn test_writer_plain() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::Text {
            text: "Hello".to_string().into(),
            style: Style::default(),
        });
        let out = w.finish();
        assert_eq!(String::from_utf8_lossy(&out), "Hello");
    }

    #[test]
    fn test_writer_bold() {
        let mut w = Writer::new(Vec::<u8>::new());
        let s = Style {
            bold: true,
            ..Style::default()
        };
        w.write_event(OwnedEvent::Text {
            text: "Bold".to_string().into(),
            style: s,
        });
        let out = w.finish();
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("\x1b[1m"));
        assert!(text.contains("Bold"));
        assert!(text.contains("\x1b[0m"));
    }

    #[test]
    fn test_writer_cursor_move() {
        let mut w = Writer::new(Vec::<u8>::new());
        w.write_event(OwnedEvent::CursorMove {
            direction: CursorDirection::Up,
            count: 3,
        });
        let out = w.finish();
        assert_eq!(String::from_utf8_lossy(&out), "\x1b[3A");
    }
}
