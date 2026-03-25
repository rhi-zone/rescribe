//! CommonMark emitter: converts a [`CmDoc`] back to CommonMark bytes.

use crate::ast::*;

/// Emit a [`CmDoc`] as CommonMark bytes.
///
/// Round-trip guarantee: `parse(emit(ast)).0.strip_spans() == ast.strip_spans()`
/// for any valid [`CmDoc`].
pub fn emit(doc: &CmDoc) -> Vec<u8> {
    let mut out = Emitter::new();
    out.emit_blocks(&doc.blocks, false);
    out.finish().into_bytes()
}

// ── Emitter ──────────────────────────────────────────────────────────────────

struct Emitter {
    buf: String,
}

impl Emitter {
    fn new() -> Self {
        Emitter { buf: String::new() }
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

    /// Emit a sequence of blocks.
    ///
    /// `tight` controls whether list items suppress the trailing blank line.
    /// When false (top-level or blockquote or loose list), blocks are separated
    /// by blank lines.  When true (tight list item), paragraphs have no trailing
    /// blank line.
    fn emit_blocks(&mut self, blocks: &[Block], tight: bool) {
        for (i, block) in blocks.iter().enumerate() {
            if i > 0 && !tight {
                self.newline(); // blank line separator
            }
            self.emit_block(block, tight);
        }
    }

    fn emit_block(&mut self, block: &Block, tight: bool) {
        match block {
            Block::Paragraph { inlines, .. } => {
                self.emit_inlines(inlines);
                self.newline();
                if !tight {
                    // top-level paragraphs end with a blank line; the blank line
                    // is emitted as the separator in emit_blocks, so just ensure
                    // we end with exactly one newline here (already done above).
                    // The inter-block blank line is added by emit_blocks.
                }
            }
            Block::Heading { level, inlines, .. } => {
                for _ in 0..*level {
                    self.push_char('#');
                }
                self.push_char(' ');
                self.emit_inlines(inlines);
                self.newline();
            }
            Block::CodeBlock { language, content, .. } => {
                // Choose fence style: use ~~~ if content contains ```
                let (fence_open, fence_close) = if content.contains("```") {
                    ("~~~", "~~~")
                } else {
                    ("```", "```")
                };
                self.push(fence_open);
                if let Some(lang) = language {
                    self.push(lang);
                }
                self.newline();
                self.push(content);
                // pulldown-cmark always includes a trailing newline in content
                if !content.ends_with('\n') {
                    self.newline();
                }
                self.push(fence_close);
                self.newline();
            }
            Block::HtmlBlock { content, .. } => {
                self.push(content);
                if !content.ends_with('\n') {
                    self.newline();
                }
            }
            Block::Blockquote { blocks, .. } => {
                // Emit inner blocks to a buffer, then prefix each line with `> `.
                let inner = {
                    let mut inner_emitter = Emitter::new();
                    inner_emitter.emit_blocks(blocks, false);
                    inner_emitter.finish()
                };
                for line in inner.lines() {
                    self.push("> ");
                    self.push(line);
                    self.newline();
                }
                // If inner was empty or ended without a newline, ensure we have one.
                if inner.is_empty() {
                    self.push(">\n");
                }
            }
            Block::List { kind, items, tight: is_tight, .. } => {
                self.emit_list(kind, items, *is_tight);
                // No extra newline here — each item already ends with '\n', and
                // inter-block blank lines are added by emit_blocks when !tight.
                // An extra '\n' here would create a blank continuation line when
                // this list is inside a tight parent list item (roundtrip bug).
            }
            Block::ThematicBreak { .. } => {
                self.push("---");
                self.newline();
            }
        }
    }

    fn emit_list(&mut self, kind: &ListKind, items: &[ListItem], tight: bool) {
        for (idx, item) in items.iter().enumerate() {
            if !tight && idx > 0 {
                self.newline(); // blank line between loose items
            }
            let (marker, indent) = list_item_marker(kind, idx);
            // Emit item blocks into a buffer.
            let inner = {
                let mut e = Emitter::new();
                e.emit_blocks(&item.blocks, tight);
                e.finish()
            };

            // First line gets the marker; subsequent lines get spaces to align.
            let indent_str = " ".repeat(indent);
            let mut lines = inner.lines().peekable();
            if lines.peek().is_none() {
                // Empty item — emit just the marker so the item is preserved.
                self.push(&marker);
                self.newline();
            } else {
                let mut first = true;
                for line in lines {
                    if first {
                        self.push(&marker);
                        self.push(line);
                        self.newline();
                        first = false;
                    } else if line.is_empty() {
                        // blank continuation line — don't add trailing spaces
                        self.newline();
                    } else {
                        self.push(&indent_str);
                        self.push(line);
                        self.newline();
                    }
                }
            }
        }
    }

    fn emit_inlines(&mut self, inlines: &[Inline]) {
        for inline in inlines {
            self.emit_inline(inline);
        }
    }

    fn emit_inline(&mut self, inline: &Inline) {
        match inline {
            Inline::Text { content, .. } => {
                self.push(&escape_text(content));
            }
            Inline::SoftBreak { .. } => {
                self.newline();
            }
            Inline::HardBreak { .. } => {
                self.push("  \n");
            }
            Inline::Emphasis { inlines, .. } => {
                self.push_char('*');
                self.emit_inlines(inlines);
                self.push_char('*');
            }
            Inline::Strong { inlines, .. } => {
                self.push("**");
                self.emit_inlines(inlines);
                self.push("**");
            }
            Inline::Strikethrough { inlines, .. } => {
                self.push("~~");
                self.emit_inlines(inlines);
                self.push("~~");
            }
            Inline::Code { content, .. } => {
                // If content contains a backtick, wrap in double backticks and
                // pad with spaces.
                if content.contains('`') {
                    self.push("`` ");
                    self.push(content);
                    self.push(" ``");
                } else {
                    self.push_char('`');
                    self.push(content);
                    self.push_char('`');
                }
            }
            Inline::HtmlInline { content, .. } => {
                self.push(content);
            }
            Inline::Link { inlines, url, title, .. } => {
                self.push_char('[');
                self.emit_inlines(inlines);
                self.push("](");
                self.push(&escape_url(url));
                if let Some(t) = title {
                    self.push(" \"");
                    self.push(&escape_title(t));
                    self.push_char('"');
                }
                self.push_char(')');
            }
            Inline::Image { alt, url, title, .. } => {
                self.push("![");
                self.push(&escape_text(alt));
                self.push("](");
                self.push(&escape_url(url));
                if let Some(t) = title {
                    self.push(" \"");
                    self.push(&escape_title(t));
                    self.push_char('"');
                }
                self.push_char(')');
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Return the marker string and the indent width for a list item.
///
/// The indent width is the number of spaces needed to align continuation lines
/// with the content after the marker.
fn list_item_marker(kind: &ListKind, idx: usize) -> (String, usize) {
    match kind {
        ListKind::Unordered { marker } => {
            let m = format!("{marker} ");
            let indent = m.len();
            (m, indent)
        }
        ListKind::Ordered { start, marker } => {
            let n = start + idx as u64;
            let suffix = match marker {
                OrderedMarker::Period => '.',
                OrderedMarker::Paren => ')',
            };
            let m = format!("{n}{suffix} ");
            let indent = m.len();
            (m, indent)
        }
    }
}

/// Escape text content so it round-trips through CommonMark.
///
/// We only escape characters that pulldown-cmark will actually reinterpret as
/// syntax. The key ones are delimiters for inline constructs:
/// - `\` — the escape character itself
/// - `*` — emphasis/strong
/// - `_` — emphasis/strong
/// - `` ` `` — code span
/// - `[` — link/image open
/// - `~` — strikethrough (GFM)
/// - `<` — autolink or raw HTML tag
///
/// We deliberately do NOT escape `.`, `!`, `(`, `)`, `#`, `-`, `+`, `>`, `&`
/// etc. Those characters are only special in specific positional contexts
/// (start of line, or adjacent to certain other chars) that won't arise when
/// the text is emitted inline. Over-escaping causes pulldown-cmark to split a
/// single Text event into two (e.g. "text\." → ["text", "."]) which breaks
/// the roundtrip equality check.
fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        if matches!(c, '\\' | '*' | '_' | '`' | '[' | '~' | '<') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Escape a URL for use inside `(…)` destination.
///
/// CommonMark destinations don't need heavy escaping — only literal `)` and
/// spaces need special treatment. We wrap in angle brackets if the URL contains
/// spaces or parentheses, which is the safest approach.
fn escape_url(url: &str) -> String {
    if url.contains(' ') || url.contains('(') || url.contains(')') || url.contains('<') || url.contains('>') {
        // Wrap in angle brackets; escape any `>` inside.
        let inner = url.replace('>', "%3E").replace('<', "%3C");
        format!("<{inner}>")
    } else {
        url.to_string()
    }
}

/// Escape a title string for use inside `"…"` delimiters.
fn escape_title(t: &str) -> String {
    t.replace('\\', "\\\\").replace('"', "\\\"")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    fn roundtrip(input: &str) {
        let (ast, _) = parse(input.as_bytes());
        let out = emit(&ast);
        let (ast2, _) = parse(&out);
        assert_eq!(
            ast.strip_spans(),
            ast2.strip_spans(),
            "roundtrip failed for: {:?}\nemitted: {:?}",
            input,
            String::from_utf8_lossy(&out),
        );
    }

    #[test]
    fn test_roundtrip_paragraph() {
        roundtrip("Hello, world!\n");
    }

    #[test]
    fn test_roundtrip_heading() {
        roundtrip("# Heading 1\n\n## Heading 2\n");
    }

    #[test]
    fn test_roundtrip_emphasis() {
        roundtrip("This is *emphasized* and **strong** text.\n");
    }

    #[test]
    fn test_roundtrip_strikethrough() {
        roundtrip("~~deleted text~~\n");
    }

    #[test]
    fn test_roundtrip_inline_code() {
        roundtrip("Use `code` here.\n");
    }

    #[test]
    fn test_roundtrip_code_block() {
        roundtrip("```rust\nfn main() {}\n```\n");
    }

    #[test]
    fn test_roundtrip_code_block_no_lang() {
        roundtrip("```\nsome code\n```\n");
    }

    #[test]
    fn test_roundtrip_blockquote() {
        roundtrip("> A quoted paragraph.\n");
    }

    #[test]
    fn test_roundtrip_blockquote_nested() {
        roundtrip("> > Nested quote.\n");
    }

    #[test]
    fn test_roundtrip_tight_list() {
        roundtrip("- one\n- two\n- three\n");
    }

    #[test]
    fn test_roundtrip_loose_list() {
        roundtrip("- one\n\n- two\n\n- three\n");
    }

    #[test]
    fn test_roundtrip_ordered_list() {
        roundtrip("1. first\n2. second\n3. third\n");
    }

    #[test]
    fn test_roundtrip_thematic_break() {
        roundtrip("---\n");
    }

    #[test]
    fn test_roundtrip_link() {
        roundtrip("[text](https://example.com)\n");
    }

    #[test]
    fn test_roundtrip_link_with_title() {
        roundtrip("[text](https://example.com \"My Title\")\n");
    }

    #[test]
    fn test_roundtrip_image() {
        roundtrip("![alt text](img.png)\n");
    }

    #[test]
    fn test_roundtrip_html_block() {
        roundtrip("<div>\ncontent\n</div>\n");
    }

    #[test]
    fn test_roundtrip_inline_html() {
        roundtrip("text <em>inline</em> html\n");
    }

    #[test]
    fn test_roundtrip_hard_break() {
        roundtrip("line one  \nline two\n");
    }

    #[test]
    fn test_roundtrip_nested_emphasis() {
        roundtrip("**bold and *nested* emphasis**\n");
    }

    #[test]
    fn test_roundtrip_multiple_blocks() {
        roundtrip("# Title\n\nA paragraph.\n\n- item one\n- item two\n");
    }

    #[test]
    fn test_roundtrip_nested_tight_list() {
        // Tight outer list whose first item contains a tight inner list.
        // Regression: emit() was adding a spurious blank line after the inner
        // list, causing the outer list to reparse as loose.
        roundtrip("- - x\n- x\n");
    }

    #[test]
    fn test_emit_produces_utf8() {
        let (ast, _) = parse(b"Hello");
        let out = emit(&ast);
        assert!(std::str::from_utf8(&out).is_ok());
    }
}
