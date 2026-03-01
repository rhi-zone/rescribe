//! ANSI terminal text parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-ansi` and `rescribe-write-ansi` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct AnsiError(pub String);

impl std::fmt::Display for AnsiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ANSI error: {}", self.0)
    }
}

impl std::error::Error for AnsiError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed ANSI document.
#[derive(Debug, Clone, Default)]
pub struct AnsiDoc {
    pub blocks: Vec<Block>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Paragraph {
        inlines: Vec<Inline>,
    },
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    CodeBlock {
        language: Option<String>,
        content: String,
    },
    Blockquote {
        children: Vec<Block>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    ListItem {
        children: Vec<Block>,
    },
    Table {
        rows: Vec<TableRow>,
    },
    TableRow {
        cells: Vec<TableCell>,
    },
    TableCell {
        inlines: Vec<Inline>,
    },
    TableHeader {
        cells: Vec<TableCell>,
    },
    TableBody {
        rows: Vec<TableRow>,
    },
    TableFoot {
        rows: Vec<TableRow>,
    },
    HorizontalRule,
    Div {
        children: Vec<Block>,
    },
    Span {
        inlines: Vec<Inline>,
    },
    RawBlock {
        content: String,
    },
    RawInline {
        content: String,
    },
    DefinitionList {
        items: Vec<DefinitionItem>,
    },
    DefinitionTerm {
        inlines: Vec<Inline>,
    },
    DefinitionDesc {
        children: Vec<Block>,
    },
    Figure {
        children: Vec<Block>,
    },
}

/// A table row.
#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

/// A table cell.
#[derive(Debug, Clone)]
pub struct TableCell {
    pub inlines: Vec<Inline>,
}

/// A definition item (term + description).
#[derive(Debug, Clone)]
pub struct DefinitionItem {
    pub term: Vec<Inline>,
    pub desc: Vec<Block>,
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Underline(Vec<Inline>),
    Strikethrough(Vec<Inline>),
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// ANSI SGR (Select Graphic Rendition) codes
#[derive(Default, Clone)]
struct Style {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
}

/// Parse ANSI text into an [`AnsiDoc`].
pub fn parse(input: &str) -> Result<AnsiDoc, AnsiError> {
    let mut result = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Empty line
        if line.is_empty() || strip_ansi(line).is_empty() {
            i += 1;
            continue;
        }

        // Collect paragraph lines
        let (para_lines, end) = collect_paragraph(&lines, i);
        if !para_lines.is_empty() {
            let text = para_lines.join(" ");
            let inlines = parse_inline(&text);
            result.push(Block::Paragraph { inlines });
        }
        i = end;
    }

    Ok(AnsiDoc { blocks: result })
}

fn collect_paragraph<'a>(lines: &[&'a str], start: usize) -> (Vec<&'a str>, usize) {
    let mut para_lines = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i];
        if line.is_empty() || strip_ansi(line).is_empty() {
            break;
        }
        para_lines.push(line);
        i += 1;
    }

    (para_lines, i)
}

/// Strip ANSI escape sequences from text
pub fn strip_ansi(text: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
            // Skip until 'm' or end of sequence
            i += 2;
            while i < chars.len() && !chars[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i < chars.len() {
                i += 1; // Skip the terminating letter
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut current = String::new();
    let mut style = Style::default();

    while i < chars.len() {
        // Check for ANSI escape sequence
        if chars[i] == '\x1b' && i + 1 < chars.len() && chars[i + 1] == '[' {
            // Flush current text
            if !current.is_empty() {
                nodes.push(create_styled_inline(&current, &style));
                current.clear();
            }

            // Parse escape sequence
            i += 2; // Skip ESC [
            let mut params = String::new();
            while i < chars.len() && !chars[i].is_ascii_alphabetic() {
                params.push(chars[i]);
                i += 1;
            }

            if i < chars.len() {
                let cmd = chars[i];
                i += 1;

                if cmd == 'm' {
                    // SGR command
                    for code in params.split(';') {
                        match code.trim() {
                            "0" | "" => style = Style::default(), // Reset
                            "1" => style.bold = true,
                            "3" => style.italic = true,
                            "4" => style.underline = true,
                            "9" => style.strikethrough = true,
                            "22" => style.bold = false,
                            "23" => style.italic = false,
                            "24" => style.underline = false,
                            "29" => style.strikethrough = false,
                            _ => {} // Ignore colors and other codes
                        }
                    }
                }
            }
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    // Flush remaining text
    if !current.is_empty() {
        nodes.push(create_styled_inline(&current, &style));
    }

    nodes
}

fn create_styled_inline(text: &str, style: &Style) -> Inline {
    let mut inline = Inline::Text(text.to_string());

    // Apply styles from innermost to outermost
    if style.strikethrough {
        inline = Inline::Strikethrough(vec![inline]);
    }
    if style.underline {
        inline = Inline::Underline(vec![inline]);
    }
    if style.italic {
        inline = Inline::Italic(vec![inline]);
    }
    if style.bold {
        inline = Inline::Bold(vec![inline]);
    }

    inline
}

// ── Builder ───────────────────────────────────────────────────────────────────

// ANSI escape codes
pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const DIM: &str = "\x1b[2m";
pub const ITALIC: &str = "\x1b[3m";
pub const UNDERLINE: &str = "\x1b[4m";
pub const STRIKETHROUGH: &str = "\x1b[9m";

// Colors
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";

// Background colors
pub const BG_BLACK: &str = "\x1b[40m";

/// Build an ANSI string from an [`AnsiDoc`].
pub fn build(doc: &AnsiDoc) -> String {
    let mut ctx = BuildContext::new();
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.output
}

struct BuildContext {
    output: String,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Paragraph { inlines } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Heading { level, inlines } => {
            ctx.write(BOLD);
            let color = match level {
                1 => BLUE,
                2 => GREEN,
                3 => YELLOW,
                4 => CYAN,
                _ => MAGENTA,
            };
            ctx.write(color);
            for _ in 0..*level {
                ctx.write("#");
            }
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write(RESET);
            ctx.write("\n\n");
        }

        Block::CodeBlock { language, content } => {
            ctx.write(DIM);
            ctx.write("┌─");
            if let Some(lang) = language {
                ctx.write(" ");
                ctx.write(lang);
                ctx.write(" ");
            }
            ctx.write("─\n");
            ctx.write(RESET);

            ctx.write(BG_BLACK);
            ctx.write(CYAN);

            for line in content.lines() {
                ctx.write(DIM);
                ctx.write("│ ");
                ctx.write(RESET);
                ctx.write(BG_BLACK);
                ctx.write(CYAN);
                ctx.write(line);
                ctx.write(RESET);
                ctx.write("\n");
            }

            ctx.write(RESET);
            ctx.write(DIM);
            ctx.write("└─\n");
            ctx.write(RESET);
            ctx.write("\n");
        }

        Block::Blockquote { children } => {
            for line in collect_block_text(children).lines() {
                ctx.write(DIM);
                ctx.write("│ ");
                ctx.write(RESET);
                ctx.write(ITALIC);
                ctx.write(line);
                ctx.write(RESET);
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::List { ordered, items } => {
            for (idx, item_blocks) in items.iter().enumerate() {
                if *ordered {
                    ctx.write(YELLOW);
                    ctx.write(&format!("{}. ", idx + 1));
                    ctx.write(RESET);
                } else {
                    ctx.write(GREEN);
                    ctx.write("• ");
                    ctx.write(RESET);
                }
                for block in item_blocks {
                    build_block(block, ctx);
                }
            }
            ctx.write("\n");
        }

        Block::ListItem { children } => {
            for child in children {
                build_block(child, ctx);
            }
        }

        Block::Table { rows } => {
            let col_widths = calculate_column_widths(rows);
            emit_table_border(&col_widths, ctx, '┌', '┬', '┐');

            let mut is_header = true;
            for row in rows {
                ctx.write("│");
                for (i, cell) in row.cells.iter().enumerate() {
                    let width = col_widths.get(i).copied().unwrap_or(1);
                    if is_header {
                        ctx.write(BOLD);
                    }
                    ctx.write(" ");
                    let text = collect_inline_text(&cell.inlines);
                    ctx.write(&text);
                    for _ in text.len()..width {
                        ctx.write(" ");
                    }
                    if is_header {
                        ctx.write(RESET);
                    }
                    ctx.write(" │");
                }
                ctx.write("\n");

                if is_header && rows.len() > 1 {
                    emit_table_border(&col_widths, ctx, '├', '┼', '┤');
                    is_header = false;
                }
            }

            emit_table_border(&col_widths, ctx, '└', '┴', '┘');
            ctx.write("\n");
        }

        Block::TableRow { cells } => {
            ctx.write("│");
            for cell in cells {
                ctx.write(" ");
                build_inlines(&cell.inlines, ctx);
                ctx.write(" │");
            }
            ctx.write("\n");
        }

        Block::TableCell { inlines } => {
            build_inlines(inlines, ctx);
        }

        Block::TableHeader { cells } => {
            for cell in cells {
                build_inlines(&cell.inlines, ctx);
            }
        }

        Block::TableBody { rows } => {
            for row in rows {
                build_block(
                    &Block::TableRow {
                        cells: row.cells.clone(),
                    },
                    ctx,
                );
            }
        }

        Block::TableFoot { rows } => {
            for row in rows {
                build_block(
                    &Block::TableRow {
                        cells: row.cells.clone(),
                    },
                    ctx,
                );
            }
        }

        Block::HorizontalRule => {
            ctx.write(DIM);
            ctx.write("────────────────────────────────────────");
            ctx.write(RESET);
            ctx.write("\n\n");
        }

        Block::Div { children } => {
            for child in children {
                build_block(child, ctx);
            }
        }

        Block::Span { inlines } => {
            build_inlines(inlines, ctx);
        }

        Block::RawBlock { content } => {
            ctx.write(content);
            ctx.write("\n");
        }

        Block::RawInline { content } => {
            ctx.write(content);
        }

        Block::DefinitionList { items } => {
            for item in items {
                build_inlines(&item.term, ctx);
                ctx.write("\n");
                for desc_block in &item.desc {
                    ctx.write("  ");
                    build_block(desc_block, ctx);
                }
            }
            ctx.write("\n");
        }

        Block::DefinitionTerm { inlines } => {
            ctx.write(BOLD);
            build_inlines(inlines, ctx);
            ctx.write(RESET);
            ctx.write("\n");
        }

        Block::DefinitionDesc { children } => {
            ctx.write("  ");
            for child in children {
                build_block(child, ctx);
            }
        }

        Block::Figure { children } => {
            for child in children {
                build_block(child, ctx);
            }
        }
    }
}

fn emit_table_border(widths: &[usize], ctx: &mut BuildContext, left: char, mid: char, right: char) {
    ctx.write(DIM);
    ctx.write(&left.to_string());
    for (i, w) in widths.iter().enumerate() {
        for _ in 0..(*w + 2) {
            ctx.write("─");
        }
        if i < widths.len() - 1 {
            ctx.write(&mid.to_string());
        }
    }
    ctx.write(&right.to_string());
    ctx.write(RESET);
    ctx.write("\n");
}

fn calculate_column_widths(rows: &[TableRow]) -> Vec<usize> {
    let num_cols = rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
    let mut widths = vec![1; num_cols];

    for row in rows {
        for (i, cell) in row.cells.iter().enumerate() {
            let text = collect_inline_text(&cell.inlines);
            if text.len() > widths[i] {
                widths[i] = text.len();
            }
        }
    }
    widths
}

#[allow(dead_code)]
fn collect_inline_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        match inline {
            Inline::Text(s) => text.push_str(s),
            Inline::Bold(c)
            | Inline::Italic(c)
            | Inline::Underline(c)
            | Inline::Strikethrough(c) => {
                text.push_str(&collect_inline_text(c));
            }
        }
    }
    text
}

fn collect_block_text(blocks: &[Block]) -> String {
    let mut text = String::new();
    for block in blocks {
        if let Block::Paragraph { inlines } = block {
            text.push_str(&collect_inline_text(inlines));
            text.push('\n');
        }
    }
    text
}

fn build_inlines(inlines: &[Inline], ctx: &mut BuildContext) {
    for inline in inlines {
        build_inline(inline, ctx);
    }
}

fn build_inline(inline: &Inline, ctx: &mut BuildContext) {
    match inline {
        Inline::Text(s) => ctx.write(s),

        Inline::Bold(children) => {
            ctx.write(BOLD);
            build_inlines(children, ctx);
            ctx.write(RESET);
        }

        Inline::Italic(children) => {
            ctx.write(ITALIC);
            build_inlines(children, ctx);
            ctx.write(RESET);
        }

        Inline::Underline(children) => {
            ctx.write(UNDERLINE);
            build_inlines(children, ctx);
            ctx.write(RESET);
        }

        Inline::Strikethrough(children) => {
            ctx.write(STRIKETHROUGH);
            build_inlines(children, ctx);
            ctx.write(RESET);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_plain_text() {
        let doc = parse("Hello world").unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse("\x1b[1mBold text\x1b[0m").unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("\x1b[3mItalic text\x1b[0m").unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_underline() {
        let doc = parse("\x1b[4mUnderlined\x1b[0m").unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_strip_ansi() {
        assert_eq!(strip_ansi("\x1b[1mBold\x1b[0m"), "Bold");
        assert_eq!(strip_ansi("\x1b[31mRed\x1b[0m"), "Red");
        assert_eq!(strip_ansi("Plain text"), "Plain text");
    }

    #[test]
    fn test_combined_styles() {
        let doc = parse("\x1b[1;3mBold and italic\x1b[0m").unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_build_simple() {
        let doc = AnsiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello world".into())],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("Hello world"));
    }

    #[test]
    fn test_build_bold() {
        let doc = AnsiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("bold"));
        assert!(output.contains(BOLD));
    }

    #[test]
    fn test_build_italic() {
        let doc = AnsiDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".into())])],
            }],
        };
        let output = build(&doc);
        assert!(output.contains("italic"));
        assert!(output.contains(ITALIC));
    }

    #[allow(dead_code)]
    fn collect_inline_text(inlines: &[Inline]) -> String {
        let mut out = String::new();
        for inline in inlines {
            match inline {
                Inline::Text(s) => out.push_str(s),
                Inline::Bold(c)
                | Inline::Italic(c)
                | Inline::Underline(c)
                | Inline::Strikethrough(c) => out.push_str(&super::collect_inline_text(c)),
            }
        }
        out
    }
}
