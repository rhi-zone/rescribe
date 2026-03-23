//! Djot emitter: converts a `DjotDoc` back to Djot markup.

use crate::ast::*;

/// Emit a `DjotDoc` as a Djot string.
pub fn emit(doc: &DjotDoc) -> String {
    let mut out = Emitter::new();
    out.emit_blocks(&doc.blocks);

    // Emit footnote definitions
    for fn_def in &doc.footnotes {
        out.blank_line();
        out.newline_if_needed();
        out.emit_footnote_def(fn_def);
        if !out.buf.ends_with('\n') {
            out.newline();
        }
    }

    // Emit link definitions
    for link_def in &doc.link_defs {
        out.blank_line();
        out.newline_if_needed();
        out.emit_link_def(link_def);
        if !out.buf.ends_with('\n') {
            out.newline();
        }
    }

    out.finish()
}

struct Emitter {
    buf: String,
    /// Number of blank lines to emit before next content.
    pending_blanks: usize,
}

impl Emitter {
    fn new() -> Self {
        Emitter { buf: String::new(), pending_blanks: 0 }
    }

    fn finish(self) -> String {
        self.buf
    }

    fn push(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    fn push_char(&mut self, c: char) {
        self.buf.push(c);
    }

    fn newline(&mut self) {
        self.buf.push('\n');
    }

    /// Ensure at least one blank line before next output (used between blocks).
    fn blank_line(&mut self) {
        self.pending_blanks = self.pending_blanks.max(1);
    }

    fn newline_if_needed(&mut self) {
        if self.pending_blanks > 0 {
            for _ in 0..self.pending_blanks {
                self.newline();
            }
            self.pending_blanks = 0;
        }
    }

    fn emit_blocks(&mut self, blocks: &[Block]) {
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 {
                self.blank_line();
            }
            self.newline_if_needed();
            self.emit_block(block);
            // Each block ends with a newline so inter-block blank lines work correctly.
            if !self.buf.ends_with('\n') {
                self.newline();
            }
        }
    }

    fn emit_block(&mut self, block: &Block) {
        match block {
            Block::Paragraph { inlines, attr, .. } => {
                self.emit_attr_line(attr);
                self.emit_inlines(inlines);
            }
            Block::Heading { level, inlines, attr, .. } => {
                self.emit_attr_line(attr);
                for _ in 0..*level {
                    self.push_char('#');
                }
                self.push_char(' ');
                self.emit_inlines(inlines);
            }
            Block::Blockquote { blocks, attr, .. } => {
                self.emit_attr_line(attr);
                let inner = {
                    let mut inner_emitter = Emitter::new();
                    inner_emitter.emit_blocks(blocks);
                    inner_emitter.finish()
                };
                for line in inner.lines() {
                    if line.is_empty() {
                        self.push("> ");
                    } else {
                        self.push("> ");
                        self.push(line);
                    }
                    self.newline();
                }
                // Remove trailing newline added by newline()
                if self.buf.ends_with('\n') {
                    self.buf.pop();
                }
            }
            Block::List { kind, items, attr, .. } => {
                self.emit_attr_line(attr);
                self.emit_list(kind, items);
            }
            Block::CodeBlock { language, content, attr, .. } => {
                self.emit_attr_line(attr);
                let lang = language.as_deref().unwrap_or("");
                self.push("```");
                self.push(lang);
                self.newline();
                self.push(content);
                if !content.ends_with('\n') {
                    self.newline();
                }
                self.push("```");
            }
            Block::RawBlock { format, content, attr, .. } => {
                self.emit_attr_line(attr);
                self.push("```=");
                self.push(format);
                self.newline();
                self.push(content);
                if !content.ends_with('\n') {
                    self.newline();
                }
                self.push("```");
            }
            Block::Div { class, blocks, attr, .. } => {
                self.emit_attr_line(attr);
                self.push(":::");
                if let Some(cls) = class {
                    self.push_char(' ');
                    self.push(cls);
                }
                self.newline();
                self.emit_blocks(blocks);
                self.newline();
                self.push(":::");
            }
            Block::Table { caption, rows, .. } => {
                if let Some(cap) = caption {
                    self.push("^ ");
                    self.emit_inlines(cap);
                    self.newline();
                }
                // Find max cells for alignment row
                let max_cells = rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
                // Collect alignment info from first header row
                let alignments: Vec<Alignment> = rows
                    .iter()
                    .find(|r| r.is_header)
                    .map(|r| r.cells.iter().map(|c| c.alignment.clone()).collect())
                    .unwrap_or_default();

                for row in rows {
                    self.emit_table_row(row);
                    self.newline();
                    if row.is_header {
                        // Emit separator row
                        self.push_char('|');
                        for i in 0..max_cells {
                            let align = alignments.get(i).unwrap_or(&Alignment::Default);
                            match align {
                                Alignment::Left => self.push(":---|"),
                                Alignment::Right => self.push("---:|"),
                                Alignment::Center => self.push(":---:|"),
                                Alignment::Default => self.push("----|"),
                            }
                        }
                        self.newline();
                    }
                }
                // Remove trailing newline
                if self.buf.ends_with('\n') {
                    self.buf.pop();
                }
            }
            Block::ThematicBreak { attr, .. } => {
                self.emit_attr_line(attr);
                self.push("* * *");
            }
            Block::DefinitionList { items, attr, .. } => {
                self.emit_attr_line(attr);
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        self.newline();
                    }
                    self.push(": ");
                    self.emit_inlines(&item.term);
                    self.newline();
                    let inner = {
                        let mut e = Emitter::new();
                        e.emit_blocks(&item.definitions);
                        e.finish()
                    };
                    for line in inner.lines() {
                        self.push("  ");
                        self.push(line);
                        self.newline();
                    }
                }
                // Remove trailing newline
                if self.buf.ends_with('\n') {
                    self.buf.pop();
                }
            }
        }
    }

    fn emit_list(&mut self, kind: &ListKind, items: &[ListItem]) {
        for (idx, item) in items.iter().enumerate() {
            if idx > 0 {
                self.newline();
            }
            let marker = list_item_marker(kind, idx as u32);
            self.push(&marker);

            // Checked prefix for task lists
            if let Some(checked) = item.checked {
                if checked {
                    self.push("[x] ");
                } else {
                    self.push("[ ] ");
                }
            }

            let inner = {
                let mut e = Emitter::new();
                e.emit_blocks(&item.blocks);
                e.finish()
            };

            let mut first = true;
            for line in inner.lines() {
                if first {
                    self.push(line);
                    self.newline();
                    first = false;
                } else {
                    self.push("  ");
                    self.push(line);
                    self.newline();
                }
            }
            // Remove trailing newline from last line
            if self.buf.ends_with('\n') {
                self.buf.pop();
            }
        }
    }

    fn emit_table_row(&mut self, row: &TableRow) {
        self.push_char('|');
        for cell in &row.cells {
            self.push_char(' ');
            self.emit_inlines(&cell.inlines);
            self.push(" |");
        }
    }

    fn emit_attr_line(&mut self, attr: &Attr) {
        if attr.is_empty() {
            return;
        }
        self.push(&format_attr(attr));
        self.newline();
    }

    fn emit_inlines(&mut self, inlines: &[Inline]) {
        for inline in inlines {
            self.emit_inline(inline);
        }
    }

    fn emit_inline(&mut self, inline: &Inline) {
        match inline {
            Inline::Text { content, .. } => {
                self.push(content);
            }
            Inline::SoftBreak { .. } => {
                self.newline();
            }
            Inline::HardBreak { .. } => {
                self.push("\\\n");
            }
            Inline::Emphasis { inlines, attr, .. } => {
                self.push_char('_');
                self.emit_inlines(inlines);
                self.push_char('_');
                self.push(&format_attr_inline(attr));
            }
            Inline::Strong { inlines, attr, .. } => {
                self.push_char('*');
                self.emit_inlines(inlines);
                self.push_char('*');
                self.push(&format_attr_inline(attr));
            }
            Inline::Delete { inlines, attr, .. } => {
                self.push("{-");
                self.emit_inlines(inlines);
                self.push("-}");
                self.push(&format_attr_inline(attr));
            }
            Inline::Insert { inlines, attr, .. } => {
                self.push("{+");
                self.emit_inlines(inlines);
                self.push("+}");
                self.push(&format_attr_inline(attr));
            }
            Inline::Highlight { inlines, attr, .. } => {
                self.push("{=");
                self.emit_inlines(inlines);
                self.push("=}");
                self.push(&format_attr_inline(attr));
            }
            Inline::Subscript { inlines, attr, .. } => {
                self.push_char('~');
                self.emit_inlines(inlines);
                self.push_char('~');
                self.push(&format_attr_inline(attr));
            }
            Inline::Superscript { inlines, attr, .. } => {
                self.push_char('^');
                self.emit_inlines(inlines);
                self.push_char('^');
                self.push(&format_attr_inline(attr));
            }
            Inline::Verbatim { content, attr, .. } => {
                let ticks = choose_backticks(content);
                self.push(&ticks);
                // Pad with spaces if content starts/ends with backtick
                if content.starts_with('`') || content.ends_with('`') {
                    self.push_char(' ');
                    self.push(content);
                    self.push_char(' ');
                } else {
                    self.push(content);
                }
                self.push(&ticks);
                self.push(&format_attr_inline(attr));
            }
            Inline::MathInline { content, .. } => {
                let ticks = choose_backticks(content);
                self.push_char('$');
                self.push(&ticks);
                self.push(content);
                self.push(&ticks);
            }
            Inline::MathDisplay { content, .. } => {
                let ticks = choose_backticks(content);
                self.push("$$");
                self.push(&ticks);
                self.push(content);
                self.push(&ticks);
            }
            Inline::RawInline { format, content, .. } => {
                let ticks = choose_backticks(content);
                self.push(&ticks);
                self.push(content);
                self.push(&ticks);
                self.push("{=");
                self.push(format);
                self.push_char('}');
            }
            Inline::Link { inlines, url, title, attr, .. } => {
                self.push_char('[');
                self.emit_inlines(inlines);
                self.push("](");
                self.push(url);
                if let Some(t) = title {
                    self.push(" \"");
                    self.push(t);
                    self.push_char('"');
                }
                self.push_char(')');
                self.push(&format_attr_inline(attr));
            }
            Inline::Image { inlines, url, title, attr, .. } => {
                self.push("![");
                self.emit_inlines(inlines);
                self.push("](");
                self.push(url);
                if let Some(t) = title {
                    self.push(" \"");
                    self.push(t);
                    self.push_char('"');
                }
                self.push_char(')');
                self.push(&format_attr_inline(attr));
            }
            Inline::Span { inlines, attr, .. } => {
                self.push_char('[');
                self.emit_inlines(inlines);
                self.push_char(']');
                self.push(&format_attr_inline(attr));
            }
            Inline::FootnoteRef { label, .. } => {
                self.push("[^");
                self.push(label);
                self.push_char(']');
            }
            Inline::Symbol { name, .. } => {
                self.push_char(':');
                self.push(name);
                self.push_char(':');
            }
            Inline::Autolink { url, is_email, .. } => {
                self.push_char('<');
                if *is_email {
                    // Strip `mailto:` prefix if present
                    if let Some(addr) = url.strip_prefix("mailto:") {
                        self.push(addr);
                    } else {
                        self.push(url);
                    }
                } else {
                    self.push(url);
                }
                self.push_char('>');
            }
        }
    }

    fn emit_footnote_def(&mut self, fn_def: &FootnoteDef) {
        self.push("[^");
        self.push(&fn_def.label);
        self.push("]: ");
        let inner = {
            let mut e = Emitter::new();
            e.emit_blocks(&fn_def.blocks);
            e.finish()
        };
        let mut first = true;
        for line in inner.lines() {
            if first {
                self.push(line);
                self.newline();
                first = false;
            } else {
                self.push("  ");
                self.push(line);
                self.newline();
            }
        }
        if self.buf.ends_with('\n') {
            self.buf.pop();
        }
    }

    fn emit_link_def(&mut self, link_def: &LinkDef) {
        self.push_char('[');
        self.push(&link_def.label);
        self.push("]: ");
        self.push(&link_def.url);
        if let Some(title) = &link_def.title {
            self.push(" \"");
            self.push(title);
            self.push_char('"');
        }
    }
}

// ── Helper functions ─────────────────────────────────────────────────────────

fn list_item_marker(kind: &ListKind, idx: u32) -> String {
    match kind {
        ListKind::Bullet(BulletStyle::Dash) => "- ".to_string(),
        ListKind::Bullet(BulletStyle::Star) => "* ".to_string(),
        ListKind::Bullet(BulletStyle::Plus) => "+ ".to_string(),
        ListKind::Task => "- ".to_string(),
        ListKind::Ordered { style, delimiter, start } => {
            let n = start + idx;
            let num_str = format_ordered_number(n, style);
            match delimiter {
                OrderedDelimiter::Period => format!("{num_str}. "),
                OrderedDelimiter::Paren => format!("{num_str}) "),
                OrderedDelimiter::Enclosed => format!("({num_str}) "),
            }
        }
    }
}

fn format_ordered_number(n: u32, style: &OrderedStyle) -> String {
    match style {
        OrderedStyle::Decimal => n.to_string(),
        OrderedStyle::LowerAlpha => {
            let idx = ((n.saturating_sub(1)) % 26) as u8;
            ((b'a' + idx) as char).to_string()
        }
        OrderedStyle::UpperAlpha => {
            let idx = ((n.saturating_sub(1)) % 26) as u8;
            ((b'A' + idx) as char).to_string()
        }
        OrderedStyle::LowerRoman => to_roman(n).to_lowercase(),
        OrderedStyle::UpperRoman => to_roman(n),
    }
}

fn to_roman(n: u32) -> String {
    let vals = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut result = String::new();
    let mut n = n;
    for (val, sym) in vals {
        while n >= val {
            result.push_str(sym);
            n -= val;
        }
    }
    if result.is_empty() { "0".to_string() } else { result }
}

fn choose_backticks(content: &str) -> String {
    // Count max consecutive backticks in content; use one more
    let mut max = 0;
    let mut current = 0;
    for c in content.chars() {
        if c == '`' {
            current += 1;
            max = max.max(current);
        } else {
            current = 0;
        }
    }
    "`".repeat(max + 1)
}

fn format_attr(attr: &Attr) -> String {
    if attr.is_empty() {
        return String::new();
    }
    let mut parts = Vec::new();
    if let Some(id) = &attr.id {
        parts.push(format!("#{id}"));
    }
    for cls in &attr.classes {
        parts.push(format!(".{cls}"));
    }
    for (k, v) in &attr.kv {
        if v.contains('"') || v.contains(' ') {
            parts.push(format!("{k}=\"{v}\""));
        } else {
            parts.push(format!("{k}={v}"));
        }
    }
    format!("{{{}}}", parts.join(" "))
}

fn format_attr_inline(attr: &Attr) -> String {
    if attr.is_empty() {
        String::new()
    } else {
        format_attr(attr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    #[test]
    fn test_emit_heading() {
        let (doc, _) = parse("## Hello");
        let output = emit(&doc);
        assert!(output.contains("## Hello"));
    }

    #[test]
    fn test_emit_paragraph() {
        let (doc, _) = parse("Hello world");
        let output = emit(&doc);
        assert!(output.contains("Hello world"));
    }

    #[test]
    fn test_emit_code_block() {
        let input = "```rust\nfn main() {}\n```";
        let (doc, _) = parse(input);
        let output = emit(&doc);
        assert!(output.contains("```rust"));
        assert!(output.contains("fn main() {}"));
    }

    #[test]
    fn test_emit_list() {
        let input = "- one\n- two";
        let (doc, _) = parse(input);
        let output = emit(&doc);
        assert!(output.contains("- one"));
        assert!(output.contains("- two"));
    }

    #[test]
    fn test_emit_thematic_break() {
        let input = "---";
        let (doc, _) = parse(input);
        let output = emit(&doc);
        assert!(output.contains("* * *"));
    }
}
