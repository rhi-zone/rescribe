//! ANSI parser — infallible, returns (AnsiDoc, Vec<Diagnostic>).

use crate::ast::{AnsiDoc, Block, Diagnostic, Inline, Span};

/// Parse ANSI-formatted text into an [`AnsiDoc`].
///
/// Always succeeds — malformed or unrecognised sequences produce diagnostics
/// instead of hard errors.
pub fn parse(input: &str) -> (AnsiDoc, Vec<Diagnostic>) {
    let diagnostics = Vec::new();
    let mut result = Vec::new();

    // Compute byte offsets of each line start for span tracking.
    let offsets = line_byte_offsets(input);
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Skip empty or whitespace-only lines
        if line.is_empty() || strip_ansi(line).trim().is_empty() {
            i += 1;
            continue;
        }

        let start_line = i;
        let (para_lines, end) = collect_paragraph(&lines, i);
        if !para_lines.is_empty() {
            let text = para_lines.join(" ");
            let inlines = parse_inline(&text);
            let span_start = offsets.get(start_line).copied().unwrap_or(0);
            let span_end = offsets.get(end).copied().unwrap_or(input.len());
            result.push(Block::Paragraph {
                inlines,
                span: Span::new(span_start, span_end),
            });
        }
        i = end;
    }

    let doc_span = Span::new(0, input.len());
    (AnsiDoc { blocks: result, span: doc_span }, diagnostics)
}

fn line_byte_offsets(input: &str) -> Vec<usize> {
    let mut offsets = vec![0usize];
    let mut pos = 0;
    for ch in input.chars() {
        pos += ch.len_utf8();
        if ch == '\n' {
            offsets.push(pos);
        }
    }
    offsets
}

fn collect_paragraph<'a>(lines: &[&'a str], start: usize) -> (Vec<&'a str>, usize) {
    let mut para_lines = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        if line.is_empty() || strip_ansi(line).trim().is_empty() {
            break;
        }
        para_lines.push(line);
        i += 1;
    }

    (para_lines, i)
}

/// Strip ANSI escape sequences from text, returning plain text.
pub fn strip_ansi(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Skip until an ASCII alphabetic terminator
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

// ── Inline parser ─────────────────────────────────────────────────────────────

#[derive(Default, Clone)]
struct Style {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
}

fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut chars = text.chars().peekable();
    let mut current = String::new();
    let mut style = Style::default();

    loop {
        match chars.peek().copied() {
            None => break,
            Some('\x1b') => {
                chars.next();
                if chars.peek() == Some(&'[') {
                    chars.next(); // consume '['

                    // Flush current text before style change
                    if !current.is_empty() {
                        nodes.push(create_styled_inline(&current, &style));
                        current.clear();
                    }

                    // Collect parameter digits/semicolons
                    let mut params = String::new();
                    loop {
                        match chars.peek() {
                            Some(&c) if !c.is_ascii_alphabetic() => {
                                params.push(c);
                                chars.next();
                            }
                            _ => break,
                        }
                    }

                    // Consume terminating letter
                    if chars.next() == Some('m') {
                        apply_sgr(&params, &mut style);
                    }
                } else {
                    // ESC not followed by '[' — treat as literal
                    current.push('\x1b');
                }
            }
            Some(c) => {
                chars.next();
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        nodes.push(create_styled_inline(&current, &style));
    }

    nodes
}

fn apply_sgr(params: &str, style: &mut Style) {
    for code in params.split(';') {
        match code.trim() {
            "0" | "" => *style = Style::default(),
            "1" => style.bold = true,
            "3" => style.italic = true,
            "4" => style.underline = true,
            "9" => style.strikethrough = true,
            "22" => style.bold = false,
            "23" => style.italic = false,
            "24" => style.underline = false,
            "29" => style.strikethrough = false,
            _ => {} // Ignore colours and other codes
        }
    }
}

fn create_styled_inline(text: &str, style: &Style) -> Inline {
    let mut inline = Inline::Text(text.to_string(), Span::NONE);

    // Apply styles from innermost to outermost so the AST nesting mirrors
    // the order in which `build_inline` wraps them.
    if style.strikethrough {
        inline = Inline::Strikethrough(vec![inline], Span::NONE);
    }
    if style.underline {
        inline = Inline::Underline(vec![inline], Span::NONE);
    }
    if style.italic {
        inline = Inline::Italic(vec![inline], Span::NONE);
    }
    if style.bold {
        inline = Inline::Bold(vec![inline], Span::NONE);
    }

    inline
}
