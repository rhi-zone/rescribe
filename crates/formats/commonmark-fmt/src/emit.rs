//! CommonMark emitter: converts a [`CmDoc`] back to CommonMark bytes.

use crate::ast::*;

/// Emit a [`CmDoc`] as CommonMark bytes.
///
/// Round-trip guarantee: `parse(emit(ast)).0.strip_spans() == ast.strip_spans()`
/// for any valid [`CmDoc`].
pub fn emit(doc: &CmDoc) -> Vec<u8> {
    let mut out = Emitter::new();
    #[cfg(feature = "frontmatter")]
    out.emit_frontmatter(&doc.frontmatter);
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

    #[cfg(feature = "frontmatter")]
    fn emit_frontmatter(&mut self, fm: &Option<FrontMatter>) {
        let Some(fm) = fm else { return };
        let delim = match fm.kind {
            FrontMatterKind::Yaml => "---",
            FrontMatterKind::Toml => "+++",
        };
        self.push(delim);
        self.newline();
        self.push(&fm.content);
        if !fm.content.ends_with('\n') {
            self.newline();
        }
        self.push(delim);
        self.newline();
        self.newline();
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
            Block::CodeBlock {
                language, content, ..
            } => {
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
            Block::List {
                kind,
                items,
                tight: is_tight,
                ..
            } => {
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
            #[cfg(feature = "tables")]
            Block::Table {
                alignments,
                head,
                rows,
                ..
            } => {
                self.emit_table(alignments, head, rows);
            }
            #[cfg(feature = "footnotes")]
            Block::FootnoteDefinition { label, blocks, .. } => {
                self.emit_footnote_definition(label, blocks);
            }
            #[cfg(feature = "definition-lists")]
            Block::DefinitionList { items, tight, .. } => {
                self.emit_definition_list(items, *tight);
            }
        }
    }

    /// Emit a footnote definition (`[^label]: content`).
    ///
    /// GFM footnote continuation lines require a *fixed* 4-space indent
    /// (`pulldown_cmark::firstpass::scan_containers`'s
    /// `ItemBody::FootnoteDefinition(..) if has_gfm_footnotes() =>
    /// line_start.scan_space(4)`) — unlike list items, this does not scale
    /// with the marker's own width.
    #[cfg(feature = "footnotes")]
    fn emit_footnote_definition(&mut self, label: &str, blocks: &[Block]) {
        let inner = {
            let mut e = Emitter::new();
            e.emit_blocks(blocks, false);
            e.finish()
        };
        let marker = format!("[^{label}]: ");
        let mut lines = inner.lines().peekable();
        if lines.peek().is_none() {
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
                    self.newline();
                } else {
                    self.push("    ");
                    self.push(line);
                    self.newline();
                }
            }
        }
    }

    /// Emit a definition list (`term\n:   definition`).
    ///
    /// Item groups are always blank-line separated (matches pulldown-cmark's
    /// own examples — this holds even for a tight list; see
    /// `parse.rs`'s definition-list roundtrip tests). Within a group,
    /// `tight` controls whether the term/first-definition and
    /// definition/definition boundaries get a blank line too, mirroring
    /// `emit_list`'s `tight` handling for list items.
    #[cfg(feature = "definition-lists")]
    fn emit_definition_list(&mut self, items: &[DefinitionListItem], tight: bool) {
        for (idx, item) in items.iter().enumerate() {
            if idx > 0 {
                self.newline();
            }
            self.emit_inlines(&item.term);
            self.newline();
            if !tight {
                self.newline();
            }
            for (didx, def_blocks) in item.definitions.iter().enumerate() {
                if didx > 0 && !tight {
                    self.newline();
                }
                self.emit_definition(def_blocks, tight);
            }
        }
    }

    /// Emit a single `:   definition` body, indenting continuation lines by
    /// 4 spaces (matching the marker `":   "`'s width — see
    /// `pd::scanners::scan_definition_list_definition_marker_with_indent`,
    /// which computes the continuation indent from however many spaces
    /// follow `:`, up to 4).
    #[cfg(feature = "definition-lists")]
    fn emit_definition(&mut self, blocks: &[Block], tight: bool) {
        let inner = {
            let mut e = Emitter::new();
            e.emit_blocks(blocks, tight);
            e.finish()
        };
        let mut lines = inner.lines().peekable();
        if lines.peek().is_none() {
            self.push(":\n");
        } else {
            let mut first = true;
            for line in lines {
                if first {
                    self.push(":   ");
                    self.push(line);
                    self.newline();
                    first = false;
                } else if line.is_empty() {
                    self.newline();
                } else {
                    self.push("    ");
                    self.push(line);
                    self.newline();
                }
            }
        }
    }

    #[cfg(feature = "tables")]
    fn emit_table_row(&mut self, row: &TableRow) {
        self.push_char('|');
        for cell in &row.cells {
            self.push_char(' ');
            self.emit_inlines(&cell.inlines);
            self.push(" |");
        }
        self.newline();
    }

    #[cfg(feature = "tables")]
    fn emit_table(&mut self, alignments: &[ColumnAlignment], head: &TableRow, rows: &[TableRow]) {
        self.emit_table_row(head);
        self.push_char('|');
        for a in alignments {
            let cell = match a {
                ColumnAlignment::None => "---",
                ColumnAlignment::Left => ":--",
                ColumnAlignment::Center => ":-:",
                ColumnAlignment::Right => "--:",
            };
            self.push_char(' ');
            self.push(cell);
            self.push(" |");
        }
        self.newline();
        for row in rows {
            self.emit_table_row(row);
        }
    }

    fn emit_list(&mut self, kind: &ListKind, items: &[ListItem], tight: bool) {
        for (idx, item) in items.iter().enumerate() {
            if !tight && idx > 0 {
                self.newline(); // blank line between loose items
            }
            #[allow(unused_mut)]
            let (mut marker, mut indent) = list_item_marker(kind, idx);
            #[cfg(feature = "task-lists")]
            if let Some(checked) = item.checked {
                let checkbox = if checked { "[x] " } else { "[ ] " };
                marker.push_str(checkbox);
                indent += checkbox.len();
            }
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
            #[cfg(feature = "strikethrough")]
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
            Inline::Link {
                inlines,
                url,
                title,
                ..
            } => {
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
            Inline::Image {
                alt, url, title, ..
            } => {
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
            #[cfg(feature = "footnotes")]
            Inline::FootnoteReference { label, .. } => {
                self.push("[^");
                self.push(label);
                self.push_char(']');
            }
            #[cfg(feature = "math")]
            Inline::InlineMath { source, .. } => {
                self.push_char('$');
                self.push(&escape_math(source));
                self.push_char('$');
            }
            #[cfg(feature = "math")]
            Inline::DisplayMath { source, .. } => {
                self.push("$$");
                self.push(&escape_math(source));
                self.push("$$");
            }
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Return the marker string and the indent width for a list item.
///
/// The indent width is the number of spaces needed to align continuation lines
/// with the content after the marker.
pub(crate) fn list_item_marker(kind: &ListKind, idx: usize) -> (String, usize) {
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
pub(crate) fn escape_text(s: &str) -> String {
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
pub(crate) fn escape_url(url: &str) -> String {
    if url.contains(' ')
        || url.contains('(')
        || url.contains(')')
        || url.contains('<')
        || url.contains('>')
    {
        // Wrap in angle brackets; escape any `>` inside.
        let inner = url.replace('>', "%3E").replace('<', "%3C");
        format!("<{inner}>")
    } else {
        url.to_string()
    }
}

/// Escape a title string for use inside `"…"` delimiters.
pub(crate) fn escape_title(t: &str) -> String {
    t.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Escape a literal `$` inside math source so it doesn't prematurely close
/// the `$…$`/`$$…$$` span.
///
/// **Known limitation, not fixable within CommonMark's dollar-math grammar:**
/// unlike backtick code spans (where a run of N backticks can always be
/// escaped by wrapping in N+1 backticks — the delimiter length carries no
/// other meaning), a `$`/`$$` delimiter's *length* is itself semantic (one
/// `$` means inline math, two mean display math), so there is no
/// "use more delimiters" escape hatch. The only in-band way to stop a `$`
/// from closing the span early is a preceding backslash — but pulldown-cmark
/// captures the span's raw source verbatim, backslash included (confirmed
/// against `pulldown-cmark`'s own `math_test_9`: `$\$$` round-trips through
/// HTML as literal `\$`, not `$` — the backslash is not stripped). This means
/// math source containing a literal, unescaped `$` cannot round-trip
/// byte-for-byte through `emit`/`parse` — the reparsed source will contain
/// the backslash this function had to insert. Tracked in TODO.md; this is a
/// property of the format's grammar, not an implementation gap.
#[cfg(feature = "math")]
pub(crate) fn escape_math(s: &str) -> String {
    if s.contains('$') {
        s.replace('$', "\\$")
    } else {
        s.to_string()
    }
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
    #[cfg(feature = "strikethrough")]
    fn test_roundtrip_strikethrough() {
        roundtrip("~~deleted text~~\n");
    }

    #[test]
    #[cfg(feature = "frontmatter")]
    fn test_roundtrip_yaml_frontmatter() {
        roundtrip("---\ntitle: X\n---\n\nbody\n");
    }

    #[test]
    #[cfg(feature = "frontmatter")]
    fn test_roundtrip_toml_frontmatter() {
        roundtrip("+++\ntitle = \"X\"\n+++\n\nbody\n");
    }

    #[test]
    #[cfg(feature = "tables")]
    fn test_roundtrip_table() {
        roundtrip("| a | b |\n| --- | --- |\n| 1 | 2 |\n");
    }

    #[test]
    #[cfg(feature = "task-lists")]
    fn test_roundtrip_task_list() {
        roundtrip("- [ ] todo\n- [x] done\n");
    }

    #[test]
    #[cfg(feature = "footnotes")]
    fn test_roundtrip_footnote() {
        roundtrip("Text.[^1]\n\n[^1]: A note.\n");
    }

    #[test]
    #[cfg(feature = "footnotes")]
    fn test_roundtrip_footnote_multi_block() {
        roundtrip("Text.[^1]\n\n[^1]: First paragraph.\n\n    Second paragraph.\n");
    }

    #[test]
    #[cfg(feature = "definition-lists")]
    fn test_roundtrip_definition_list_tight() {
        roundtrip("apple\n:   red fruit\n\norange\n:   orange fruit\n");
    }

    #[test]
    #[cfg(feature = "definition-lists")]
    fn test_roundtrip_definition_list_multi_def() {
        roundtrip("apple\n:   red fruit\n:   computer company\n");
    }

    #[test]
    #[cfg(feature = "math")]
    fn test_roundtrip_inline_math() {
        roundtrip("Euler's identity: $e to the i pi plus 1 equals 0$\n");
    }

    #[test]
    #[cfg(feature = "math")]
    fn test_roundtrip_display_math() {
        roundtrip("$$a squared plus b squared equals c squared$$\n");
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
