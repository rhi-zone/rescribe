//! ANSI parser — infallible, returns (AnsiDoc, Vec<Diagnostic>).

use crate::ast::{
    AnsiDoc, AnsiNode, Color, CursorDirection, Diagnostic, EraseMode, Severity, Span, Style,
};

/// Parse ANSI-formatted bytes into an [`AnsiDoc`].
///
/// Always succeeds — malformed or unrecognised sequences produce diagnostics
/// instead of hard errors.
pub fn parse(input: &[u8]) -> (AnsiDoc, Vec<Diagnostic>) {
    let mut nodes = Vec::new();
    let mut diagnostics = Vec::new();
    let mut style = Style::default();
    let mut pos = 0;
    let mut text_start = pos;
    let mut text_buf = String::new();

    while pos < input.len() {
        let b = input[pos];

        if b == 0x1b {
            // Flush accumulated plain text.
            if !text_buf.is_empty() {
                nodes.push(AnsiNode::Text {
                    text: std::mem::take(&mut text_buf),
                    style: style.clone(),
                    span: Span::new(text_start, pos),
                });
            }

            let esc_start = pos;
            pos += 1;
            if pos >= input.len() {
                // ESC at end of input.
                diagnostics.push(Diagnostic {
                    span: Span::new(esc_start, pos),
                    severity: Severity::Warning,
                    message: "ESC at end of input".into(),
                    code: "E001",
                });
                nodes.push(AnsiNode::RawEscape {
                    content: "\x1b".into(),
                    span: Span::new(esc_start, pos),
                });
                text_start = pos;
                continue;
            }

            match input[pos] {
                b'[' => {
                    // CSI sequence.
                    pos += 1;
                    let (node, new_pos) =
                        parse_csi(input, pos, esc_start, &mut style, &mut diagnostics);
                    if let Some(n) = node {
                        nodes.push(n);
                    }
                    pos = new_pos;
                }
                b']' => {
                    // OSC sequence.
                    pos += 1;
                    let (node, new_pos) =
                        parse_osc(input, pos, esc_start, &style, &mut diagnostics);
                    if let Some(n) = node {
                        nodes.push(n);
                    }
                    pos = new_pos;
                }
                b'(' | b')' => {
                    // Charset designation — skip the next byte.
                    let end = (pos + 2).min(input.len());
                    let raw: String = input[esc_start..end].iter().map(|&b| b as char).collect();
                    nodes.push(AnsiNode::RawEscape {
                        content: raw,
                        span: Span::new(esc_start, end),
                    });
                    pos = end;
                }
                b'7' => {
                    nodes.push(AnsiNode::SaveCursor {
                        span: Span::new(esc_start, pos + 1),
                    });
                    pos += 1;
                }
                b'8' => {
                    nodes.push(AnsiNode::RestoreCursor {
                        span: Span::new(esc_start, pos + 1),
                    });
                    pos += 1;
                }
                _ => {
                    // Unknown ESC sequence — preserve raw.
                    let raw: String =
                        input[esc_start..pos + 1].iter().map(|&b| b as char).collect();
                    diagnostics.push(Diagnostic {
                        span: Span::new(esc_start, pos + 1),
                        severity: Severity::Warning,
                        message: format!("unknown ESC sequence: {:?}", raw),
                        code: "E002",
                    });
                    nodes.push(AnsiNode::RawEscape {
                        content: raw,
                        span: Span::new(esc_start, pos + 1),
                    });
                    pos += 1;
                }
            }
            text_start = pos;
        } else if b == b'\n' {
            // Flush text, then emit newline.
            if !text_buf.is_empty() {
                nodes.push(AnsiNode::Text {
                    text: std::mem::take(&mut text_buf),
                    style: style.clone(),
                    span: Span::new(text_start, pos),
                });
            }
            nodes.push(AnsiNode::Newline {
                span: Span::new(pos, pos + 1),
            });
            pos += 1;
            text_start = pos;
        } else {
            if text_buf.is_empty() {
                text_start = pos;
            }
            // Accumulate plain text (as UTF-8 if valid, lossy otherwise).
            text_buf.push(b as char);
            pos += 1;
        }
    }

    // Flush trailing text.
    if !text_buf.is_empty() {
        nodes.push(AnsiNode::Text {
            text: text_buf,
            style: style.clone(),
            span: Span::new(text_start, pos),
        });
    }

    let doc_span = Span::new(0, input.len());
    (AnsiDoc { nodes, span: doc_span }, diagnostics)
}

/// Convenience wrapper that accepts `&str`.
pub fn parse_str(input: &str) -> (AnsiDoc, Vec<Diagnostic>) {
    parse(input.as_bytes())
}

/// Strip all ANSI escape sequences from text, returning plain text only.
pub fn strip_ansi(text: &str) -> String {
    let (doc, _) = parse(text.as_bytes());
    let mut out = String::new();
    for node in &doc.nodes {
        match node {
            AnsiNode::Text { text, .. } => out.push_str(text),
            AnsiNode::Newline { .. } => out.push('\n'),
            AnsiNode::Hyperlink { text, .. } => out.push_str(text),
            _ => {}
        }
    }
    out
}

// ── CSI parser ────────────────────────────────────────────────────────────────

fn parse_csi(
    input: &[u8],
    start: usize,
    esc_start: usize,
    style: &mut Style,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Option<AnsiNode>, usize) {
    let mut pos = start;
    let mut params = String::new();
    let mut private_mode = false;

    // Check for private mode marker (e.g., CSI ? ...)
    if pos < input.len() && input[pos] == b'?' {
        private_mode = true;
        pos += 1;
    }

    // Collect parameter bytes (digits, semicolons, colons).
    while pos < input.len() {
        let b = input[pos];
        if b.is_ascii_digit() || b == b';' || b == b':' {
            params.push(b as char);
            pos += 1;
        } else if b.is_ascii_alphabetic() {
            break;
        } else {
            // Unexpected byte — terminate.
            break;
        }
    }

    if pos >= input.len() {
        // Truncated CSI.
        let raw: String = input[esc_start..pos].iter().map(|&b| b as char).collect();
        diagnostics.push(Diagnostic {
            span: Span::new(esc_start, pos),
            severity: Severity::Warning,
            message: "truncated CSI sequence".into(),
            code: "E003",
        });
        return (
            Some(AnsiNode::RawEscape {
                content: raw,
                span: Span::new(esc_start, pos),
            }),
            pos,
        );
    }

    let terminator = input[pos];
    pos += 1;
    let span = Span::new(esc_start, pos);

    if private_mode {
        // Handle private-mode sequences.
        match terminator {
            b'h' if params == "25" => {
                return (Some(AnsiNode::CursorVisibility { visible: true, span }), pos);
            }
            b'l' if params == "25" => {
                return (
                    Some(AnsiNode::CursorVisibility {
                        visible: false,
                        span,
                    }),
                    pos,
                );
            }
            _ => {
                let raw: String = input[esc_start..pos].iter().map(|&b| b as char).collect();
                diagnostics.push(Diagnostic {
                    span,
                    severity: Severity::Info,
                    message: format!("unrecognised private CSI: {:?}", raw),
                    code: "E004",
                });
                return (Some(AnsiNode::RawEscape { content: raw, span }), pos);
            }
        }
    }

    match terminator {
        b'm' => {
            // SGR.
            apply_sgr(&params, style, diagnostics, span);
            (None, pos)
        }
        b'A' => {
            let n = parse_single_param(&params, 1);
            (
                Some(AnsiNode::CursorMove {
                    direction: CursorDirection::Up,
                    count: n,
                    span,
                }),
                pos,
            )
        }
        b'B' => {
            let n = parse_single_param(&params, 1);
            (
                Some(AnsiNode::CursorMove {
                    direction: CursorDirection::Down,
                    count: n,
                    span,
                }),
                pos,
            )
        }
        b'C' => {
            let n = parse_single_param(&params, 1);
            (
                Some(AnsiNode::CursorMove {
                    direction: CursorDirection::Forward,
                    count: n,
                    span,
                }),
                pos,
            )
        }
        b'D' => {
            let n = parse_single_param(&params, 1);
            (
                Some(AnsiNode::CursorMove {
                    direction: CursorDirection::Back,
                    count: n,
                    span,
                }),
                pos,
            )
        }
        b'H' | b'f' => {
            let parts: Vec<&str> = params.split(';').collect();
            let row = parts.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            let col = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            (Some(AnsiNode::CursorPosition { row, col, span }), pos)
        }
        b'J' => {
            let n = parse_single_param(&params, 0);
            let mode = match n {
                1 => EraseMode::ToBeginning,
                2 => EraseMode::All,
                _ => EraseMode::ToEnd,
            };
            (Some(AnsiNode::EraseDisplay { mode, span }), pos)
        }
        b'K' => {
            let n = parse_single_param(&params, 0);
            let mode = match n {
                1 => EraseMode::ToBeginning,
                2 => EraseMode::All,
                _ => EraseMode::ToEnd,
            };
            (Some(AnsiNode::EraseLine { mode, span }), pos)
        }
        b's' => (Some(AnsiNode::SaveCursor { span }), pos),
        b'u' => (Some(AnsiNode::RestoreCursor { span }), pos),
        b'r' => {
            let parts: Vec<&str> = params.split(';').collect();
            let top = parts.first().and_then(|s| s.parse().ok()).unwrap_or(1);
            let bottom = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(24);
            (Some(AnsiNode::ScrollRegion { top, bottom, span }), pos)
        }
        _ => {
            let raw: String = input[esc_start..pos].iter().map(|&b| b as char).collect();
            diagnostics.push(Diagnostic {
                span,
                severity: Severity::Info,
                message: format!("unrecognised CSI terminator: {:?}", terminator as char),
                code: "E005",
            });
            (Some(AnsiNode::RawEscape { content: raw, span }), pos)
        }
    }
}

// ── OSC parser ────────────────────────────────────────────────────────────────

fn parse_osc(
    input: &[u8],
    start: usize,
    esc_start: usize,
    style: &Style,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Option<AnsiNode>, usize) {
    let mut pos = start;
    let mut osc_buf = Vec::new();

    // Collect bytes until BEL (0x07) or ST (ESC \).
    while pos < input.len() {
        if input[pos] == 0x07 {
            pos += 1;
            break;
        }
        if input[pos] == 0x1b && pos + 1 < input.len() && input[pos + 1] == b'\\' {
            pos += 2;
            break;
        }
        osc_buf.push(input[pos]);
        pos += 1;
    }

    let osc_str = String::from_utf8_lossy(&osc_buf).into_owned();
    let span = Span::new(esc_start, pos);

    // Check for OSC 8 (hyperlink).
    if let Some(rest) = osc_str.strip_prefix("8;") {
        // Format: 8;params;url
        if let Some(semi_pos) = rest.find(';') {
            let url = &rest[semi_pos + 1..];
            if url.is_empty() {
                // This is a hyperlink close — return nothing (the opening
                // Hyperlink node already captured the text).
                return (None, pos);
            }
            // Hyperlink open: collect text until closing OSC 8.
            let mut link_text = String::new();
            let _text_start = pos;
            while pos < input.len() {
                if input[pos] == 0x1b
                    && pos + 1 < input.len()
                    && input[pos + 1] == b']'
                {
                    // Potential OSC close — peek for "8;;\a" or "8;;\x1b\\"
                    let osc_inner_start = pos + 2;
                    let mut osc_end = osc_inner_start;
                    let mut inner = Vec::new();
                    while osc_end < input.len() {
                        if input[osc_end] == 0x07 {
                            osc_end += 1;
                            break;
                        }
                        if input[osc_end] == 0x1b
                            && osc_end + 1 < input.len()
                            && input[osc_end + 1] == b'\\'
                        {
                            osc_end += 2;
                            break;
                        }
                        inner.push(input[osc_end]);
                        osc_end += 1;
                    }
                    let inner_str = String::from_utf8_lossy(&inner);
                    if inner_str.starts_with("8;") {
                        pos = osc_end;
                        break;
                    }
                }
                if input[pos] == b'\n' {
                    break;
                }
                link_text.push(input[pos] as char);
                pos += 1;
            }
            let full_span = Span::new(esc_start, pos);
            return (
                Some(AnsiNode::Hyperlink {
                    url: url.to_string(),
                    text: link_text,
                    style: style.clone(),
                    span: full_span,
                }),
                pos,
            );
        }
    }

    // Unknown OSC — preserve raw.
    diagnostics.push(Diagnostic {
        span,
        severity: Severity::Info,
        message: format!("unrecognised OSC sequence: {}", osc_str),
        code: "E006",
    });
    let raw: String = input[esc_start..pos].iter().map(|&b| b as char).collect();
    (Some(AnsiNode::RawEscape { content: raw, span }), pos)
}

// ── SGR application ───────────────────────────────────────────────────────────

fn apply_sgr(params: &str, style: &mut Style, diagnostics: &mut Vec<Diagnostic>, span: Span) {
    if params.is_empty() {
        *style = Style::default();
        return;
    }
    let codes: Vec<&str> = params.split(';').collect();
    let mut i = 0;
    while i < codes.len() {
        let code = codes[i].trim();
        match code {
            "0" | "" => *style = Style::default(),
            "1" => style.bold = true,
            "2" => style.dim = true,
            "3" => style.italic = true,
            "4" => style.underline = true,
            "5" => style.blink = true,
            "6" => style.rapid_blink = true,
            "7" => style.reverse = true,
            "8" => style.hidden = true,
            "9" => style.strikethrough = true,
            "21" => style.double_underline = true,
            "22" => {
                style.bold = false;
                style.dim = false;
            }
            "23" => style.italic = false,
            "24" => {
                style.underline = false;
                style.double_underline = false;
            }
            "25" => {
                style.blink = false;
                style.rapid_blink = false;
            }
            "27" => style.reverse = false,
            "28" => style.hidden = false,
            "29" => style.strikethrough = false,
            "39" => style.fg = None,
            "49" => style.bg = None,
            "53" => style.overline = true,
            "55" => style.overline = false,
            "59" => style.underline_color = None,
            // Standard foreground.
            c @ ("30" | "31" | "32" | "33" | "34" | "35" | "36" | "37") => {
                let n: u8 = c.parse().unwrap();
                style.fg = Some(Color::Standard(n - 30));
            }
            // Standard background.
            c @ ("40" | "41" | "42" | "43" | "44" | "45" | "46" | "47") => {
                let n: u8 = c.parse().unwrap();
                style.bg = Some(Color::Standard(n - 40));
            }
            // Bright foreground.
            c @ ("90" | "91" | "92" | "93" | "94" | "95" | "96" | "97") => {
                let n: u8 = c.parse().unwrap();
                style.fg = Some(Color::Bright(n - 90));
            }
            // Bright background.
            c @ ("100" | "101" | "102" | "103" | "104" | "105" | "106" | "107") => {
                let n: u8 = c.parse().unwrap();
                style.bg = Some(Color::Bright(n - 100));
            }
            // Extended foreground: 38;5;n or 38;2;r;g;b
            "38" => {
                if let Some(color) = parse_extended_color(&codes, &mut i) {
                    style.fg = Some(color);
                }
            }
            // Extended background: 48;5;n or 48;2;r;g;b
            "48" => {
                if let Some(color) = parse_extended_color(&codes, &mut i) {
                    style.bg = Some(color);
                }
            }
            // Underline color: 58;5;n or 58;2;r;g;b
            "58" => {
                if let Some(color) = parse_extended_color(&codes, &mut i) {
                    style.underline_color = Some(color);
                }
            }
            _ => {
                diagnostics.push(Diagnostic {
                    span,
                    severity: Severity::Info,
                    message: format!("unknown SGR parameter: {}", code),
                    code: "E007",
                });
            }
        }
        i += 1;
    }
}

/// Parse 5;n or 2;r;g;b after an SGR 38/48/58 code.
fn parse_extended_color(codes: &[&str], i: &mut usize) -> Option<Color> {
    if *i + 1 >= codes.len() {
        return None;
    }
    match codes[*i + 1].trim() {
        "5" => {
            // 256-color: next param is the index.
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
            // True color: r;g;b.
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

fn parse_single_param(params: &str, default: u32) -> u32 {
    if params.is_empty() {
        default
    } else {
        params.parse().unwrap_or(default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_text() {
        let (doc, diags) = parse(b"Hello");
        assert!(diags.is_empty());
        assert_eq!(doc.nodes.len(), 1);
        assert!(matches!(&doc.nodes[0], AnsiNode::Text { text, .. } if text == "Hello"));
    }

    #[test]
    fn test_bold() {
        let (doc, _) = parse(b"\x1b[1mBold\x1b[0m");
        let texts: Vec<_> = doc
            .nodes
            .iter()
            .filter_map(|n| match n {
                AnsiNode::Text { text, style, .. } => Some((text.as_str(), style.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].0, "Bold");
        assert!(texts[0].1.bold);
    }

    #[test]
    fn test_256_color() {
        let (doc, _) = parse(b"\x1b[38;5;196mRed\x1b[0m");
        if let AnsiNode::Text { style, .. } = &doc.nodes[0] {
            assert_eq!(style.fg, Some(Color::Palette(196)));
        } else {
            panic!("expected text node");
        }
    }

    #[test]
    fn test_truecolor() {
        let (doc, _) = parse(b"\x1b[38;2;255;128;0mOrange\x1b[0m");
        if let AnsiNode::Text { style, .. } = &doc.nodes[0] {
            assert_eq!(style.fg, Some(Color::Rgb(255, 128, 0)));
        } else {
            panic!("expected text node");
        }
    }

    #[test]
    fn test_cursor_movement() {
        let (doc, _) = parse(b"\x1b[5A");
        assert!(matches!(
            &doc.nodes[0],
            AnsiNode::CursorMove {
                direction: CursorDirection::Up,
                count: 5,
                ..
            }
        ));
    }

    #[test]
    fn test_hyperlink() {
        let (doc, _) = parse(b"\x1b]8;;https://example.com\x07Click\x1b]8;;\x07");
        assert!(matches!(
            &doc.nodes[0],
            AnsiNode::Hyperlink { url, text, .. }
            if url == "https://example.com" && text == "Click"
        ));
    }

    #[test]
    fn test_strip_ansi() {
        assert_eq!(strip_ansi("\x1b[1mBold\x1b[0m"), "Bold");
        assert_eq!(strip_ansi("\x1b[31mRed\x1b[0m"), "Red");
        assert_eq!(strip_ansi("Plain text"), "Plain text");
    }
}
