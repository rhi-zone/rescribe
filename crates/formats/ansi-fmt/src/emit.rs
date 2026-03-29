//! ANSI emitter — converts [`AnsiDoc`] to ANSI-formatted bytes.

use crate::ast::{AnsiDoc, AnsiNode, Color, CursorDirection, EraseMode, Style};

/// Emit an [`AnsiDoc`] as an ANSI-formatted string.
pub fn emit(doc: &AnsiDoc) -> String {
    let mut out = String::new();
    let mut current_style = Style::default();

    for node in &doc.nodes {
        emit_node(node, &mut out, &mut current_style);
    }

    // Reset style at end if non-default.
    if !current_style.is_empty() {
        out.push_str("\x1b[0m");
    }

    out
}

/// Alias for [`emit`].
#[inline]
pub fn build(doc: &AnsiDoc) -> String {
    emit(doc)
}

fn emit_node(node: &AnsiNode, out: &mut String, current_style: &mut Style) {
    match node {
        AnsiNode::Text { text, style, .. } => {
            emit_style_transition(current_style, style, out);
            out.push_str(text);
            *current_style = style.clone();
        }
        AnsiNode::Newline { .. } => {
            out.push('\n');
        }
        AnsiNode::CursorMove {
            direction, count, ..
        } => {
            let letter = match direction {
                CursorDirection::Up => 'A',
                CursorDirection::Down => 'B',
                CursorDirection::Forward => 'C',
                CursorDirection::Back => 'D',
            };
            out.push_str(&format!("\x1b[{}{}", count, letter));
        }
        AnsiNode::CursorPosition { row, col, .. } => {
            out.push_str(&format!("\x1b[{};{}H", row, col));
        }
        AnsiNode::EraseDisplay { mode, .. } => {
            let n = match mode {
                EraseMode::ToEnd => 0,
                EraseMode::ToBeginning => 1,
                EraseMode::All => 2,
            };
            out.push_str(&format!("\x1b[{}J", n));
        }
        AnsiNode::EraseLine { mode, .. } => {
            let n = match mode {
                EraseMode::ToEnd => 0,
                EraseMode::ToBeginning => 1,
                EraseMode::All => 2,
            };
            out.push_str(&format!("\x1b[{}K", n));
        }
        AnsiNode::CursorVisibility { visible, .. } => {
            if *visible {
                out.push_str("\x1b[?25h");
            } else {
                out.push_str("\x1b[?25l");
            }
        }
        AnsiNode::SaveCursor { .. } => {
            out.push_str("\x1b[s");
        }
        AnsiNode::RestoreCursor { .. } => {
            out.push_str("\x1b[u");
        }
        AnsiNode::ScrollRegion { top, bottom, .. } => {
            out.push_str(&format!("\x1b[{};{}r", top, bottom));
        }
        AnsiNode::Hyperlink {
            url, text, style, ..
        } => {
            emit_style_transition(current_style, style, out);
            *current_style = style.clone();
            out.push_str(&format!("\x1b]8;;{}\x07{}\x1b]8;;\x07", url, text));
        }
        AnsiNode::RawEscape { content, .. } => {
            out.push_str(content);
        }
    }
}

/// Emit SGR codes to transition from `from` style to `to` style.
fn emit_style_transition(from: &Style, to: &Style, out: &mut String) {
    if from == to {
        return;
    }

    if to.is_empty() {
        out.push_str("\x1b[0m");
        return;
    }

    // If the target style differs in ways that are hard to turn off
    // individually, reset and re-apply.
    let needs_reset = (from.bold && !to.bold)
        || (from.dim && !to.dim)
        || (from.italic && !to.italic)
        || (from.underline && !to.underline)
        || (from.double_underline && !to.double_underline)
        || (from.blink && !to.blink)
        || (from.rapid_blink && !to.rapid_blink)
        || (from.reverse && !to.reverse)
        || (from.hidden && !to.hidden)
        || (from.strikethrough && !to.strikethrough)
        || (from.overline && !to.overline)
        || (from.fg.is_some() && to.fg.is_none())
        || (from.bg.is_some() && to.bg.is_none())
        || (from.underline_color.is_some() && to.underline_color.is_none());

    if needs_reset {
        // Reset then apply all target attributes.
        let mut codes = vec!["0".to_string()];
        append_style_codes(to, &mut codes);
        out.push_str(&format!("\x1b[{}m", codes.join(";")));
        return;
    }

    // Only add new attributes.
    let mut codes = Vec::new();
    if to.bold && !from.bold {
        codes.push("1".to_string());
    }
    if to.dim && !from.dim {
        codes.push("2".to_string());
    }
    if to.italic && !from.italic {
        codes.push("3".to_string());
    }
    if to.underline && !from.underline {
        codes.push("4".to_string());
    }
    if to.blink && !from.blink {
        codes.push("5".to_string());
    }
    if to.rapid_blink && !from.rapid_blink {
        codes.push("6".to_string());
    }
    if to.reverse && !from.reverse {
        codes.push("7".to_string());
    }
    if to.hidden && !from.hidden {
        codes.push("8".to_string());
    }
    if to.strikethrough && !from.strikethrough {
        codes.push("9".to_string());
    }
    if to.double_underline && !from.double_underline {
        codes.push("21".to_string());
    }
    if to.overline && !from.overline {
        codes.push("53".to_string());
    }
    if to.fg != from.fg
        && let Some(ref c) = to.fg
    {
        append_color_codes(c, true, &mut codes);
    }
    if to.bg != from.bg
        && let Some(ref c) = to.bg
    {
        append_color_codes(c, false, &mut codes);
    }
    if to.underline_color != from.underline_color
        && let Some(ref c) = to.underline_color
    {
        append_underline_color_codes(c, &mut codes);
    }

    if !codes.is_empty() {
        out.push_str(&format!("\x1b[{}m", codes.join(";")));
    }
}

fn append_style_codes(style: &Style, codes: &mut Vec<String>) {
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
    if style.rapid_blink {
        codes.push("6".to_string());
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
    if style.double_underline {
        codes.push("21".to_string());
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
    if let Some(ref c) = style.underline_color {
        append_underline_color_codes(c, codes);
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

fn append_underline_color_codes(color: &Color, codes: &mut Vec<String>) {
    match color {
        Color::Palette(n) => {
            codes.push("58".to_string());
            codes.push("5".to_string());
            codes.push(format!("{}", n));
        }
        Color::Rgb(r, g, b) => {
            codes.push("58".to_string());
            codes.push("2".to_string());
            codes.push(format!("{}", r));
            codes.push(format!("{}", g));
            codes.push(format!("{}", b));
        }
        _ => {
            // Standard/bright/default don't apply to underline color.
        }
    }
}

/// Extract plain text from an [`AnsiDoc`], stripping all formatting.
pub fn collect_text(doc: &AnsiDoc) -> String {
    let mut s = String::new();
    for node in &doc.nodes {
        match node {
            AnsiNode::Text { text, .. } => s.push_str(text),
            AnsiNode::Newline { .. } => s.push('\n'),
            AnsiNode::Hyperlink { text, .. } => s.push_str(text),
            _ => {}
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn test_roundtrip_plain() {
        let (doc, _) = parse(b"Hello world");
        let emitted = emit(&doc);
        assert_eq!(emitted, "Hello world");
    }

    #[test]
    fn test_roundtrip_bold() {
        let (doc, _) = parse(b"\x1b[1mBold\x1b[0m");
        let emitted = emit(&doc);
        let (doc2, _) = parse(emitted.as_bytes());
        assert_eq!(collect_text(&doc), collect_text(&doc2));
        // Structural: doc2 should have a bold text node.
        let texts: Vec<_> = doc2
            .nodes
            .iter()
            .filter_map(|n| match n {
                AnsiNode::Text { text, style, .. } if !text.is_empty() => {
                    Some((text.clone(), style.clone()))
                }
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|(t, s)| t == "Bold" && s.bold));
    }

    #[test]
    fn test_roundtrip_256_color() {
        let (doc, _) = parse(b"\x1b[38;5;196mRed\x1b[0m");
        let emitted = emit(&doc);
        let (doc2, _) = parse(emitted.as_bytes());
        if let AnsiNode::Text { style, .. } = &doc2.nodes[0] {
            assert_eq!(style.fg, Some(Color::Palette(196)));
        } else {
            panic!("expected text");
        }
    }
}
