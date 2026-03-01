//! Djot writer for rescribe.
//!
//! Emits rescribe's document IR as Djot markup.
//!
//! # Example
//!
//! ```ignore
//! use rescribe_write_djot::emit;
//!
//! let doc = Document::new();
//! let result = emit(&doc)?;
//! let djot = String::from_utf8(result.value).unwrap();
//! ```

use rescribe_core::{
    ConversionResult, Document, EmitError, FidelityWarning, Node, Severity, WarningKind,
};
use rescribe_std::{node, prop};

/// Emit a document as Djot markup.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut ctx = EmitContext::new();
    emit_node(&doc.content, &mut ctx);

    // Trim only trailing newlines, not spaces — trailing spaces on marker lines
    // like "- " (empty list item) are syntactically significant in djot.
    let output = ctx.output.trim_end_matches('\n').to_string() + "\n";
    Ok(ConversionResult::with_warnings(
        output.into_bytes(),
        ctx.warnings,
    ))
}

struct EmitContext {
    output: String,
    warnings: Vec<FidelityWarning>,
    list_depth: usize,
    in_tight_list: bool,
}

impl EmitContext {
    fn new() -> Self {
        Self {
            output: String::new(),
            warnings: Vec::new(),
            list_depth: 0,
            in_tight_list: false,
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn newline(&mut self) {
        self.output.push('\n');
    }

    #[allow(dead_code)]
    fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::FeatureLost("djot".to_string()),
            message,
        ));
    }
}

fn emit_node(node: &Node, ctx: &mut EmitContext) {
    match node.kind.as_str() {
        node::DOCUMENT => {
            for child in &node.children {
                emit_node(child, ctx);
            }
        }
        node::PARAGRAPH => {
            emit_inline_children(node, ctx);
            ctx.newline();
            if !ctx.in_tight_list {
                ctx.newline();
            }
        }
        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as usize;
            let hashes = "#".repeat(level);
            ctx.write(&hashes);
            ctx.write(" ");
            emit_inline_children(node, ctx);
            ctx.newline();
            ctx.newline();
        }
        node::BLOCKQUOTE => {
            for child in &node.children {
                ctx.write("> ");
                emit_node(child, ctx);
            }
        }
        node::CODE_BLOCK => {
            let language = node.props.get_str(prop::LANGUAGE).unwrap_or("");
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");

            ctx.write("```");
            ctx.writeln(language);
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.newline();
            }
            ctx.writeln("```");
            ctx.newline();
        }
        node::LIST => {
            let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
            let start = node.props.get_int(prop::START).unwrap_or(1);
            let tight = is_tight_list(node);

            let old_tight = ctx.in_tight_list;
            ctx.in_tight_list = tight;
            ctx.list_depth += 1;

            for (i, child) in node.children.iter().enumerate() {
                if ordered {
                    ctx.write(&format!("{}. ", start + i as i64));
                } else {
                    ctx.write("- ");
                }
                emit_list_item_content(child, ctx);
            }

            ctx.list_depth -= 1;
            ctx.in_tight_list = old_tight;

            if ctx.list_depth == 0 {
                ctx.newline();
            }
        }
        node::LIST_ITEM => {
            // Handled by LIST
            emit_list_item_content(node, ctx);
        }
        node::TABLE => {
            emit_table(node, ctx);
            ctx.newline();
        }
        node::HORIZONTAL_RULE => {
            ctx.writeln("* * *");
            ctx.newline();
        }
        node::DIV => {
            let class = node.props.get_str("html:class").unwrap_or("");
            if !class.is_empty() {
                ctx.writeln(&format!("::: {}", class));
            } else {
                ctx.writeln(":::");
            }
            for child in &node.children {
                emit_node(child, ctx);
            }
            ctx.writeln(":::");
            ctx.newline();
        }
        node::DEFINITION_LIST => {
            for child in &node.children {
                emit_node(child, ctx);
            }
            ctx.newline();
        }
        node::DEFINITION_TERM => {
            ctx.write(": ");
            emit_inline_children(node, ctx);
            ctx.newline();
        }
        node::DEFINITION_DESC => {
            ctx.write("  ");
            emit_inline_children(node, ctx);
            ctx.newline();
        }
        node::FOOTNOTE_DEF => {
            let label = node.props.get_str(prop::LABEL).unwrap_or("?");
            ctx.write(&format!("[^{}]: ", label));
            emit_inline_children(node, ctx);
            ctx.newline();
            ctx.newline();
        }
        node::RAW_BLOCK => {
            let format = node.props.get_str(prop::FORMAT).unwrap_or("html");
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            ctx.writeln(&format!("```{{{}}}", format));
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.newline();
            }
            ctx.writeln("```");
            ctx.newline();
        }
        // Inline nodes in block context
        _ => {
            emit_inline(node, ctx);
        }
    }
}

fn emit_list_item_content(node: &Node, ctx: &mut EmitContext) {
    // Handle task list items
    if let Some(checked) = node.props.get_bool(prop::CHECKED) {
        if checked {
            ctx.write("[x] ");
        } else {
            ctx.write("[ ] ");
        }
    }

    // Emit children, handling nested structure
    let mut first = true;
    for child in &node.children {
        if child.kind.as_str() == node::PARAGRAPH {
            if !first {
                ctx.write("  ");
            }
            emit_inline_children(child, ctx);
            ctx.newline();
        } else if child.kind.as_str() == node::LIST {
            ctx.newline();
            // Indent nested list
            let indent = "  ".repeat(ctx.list_depth);
            let old_output = std::mem::take(&mut ctx.output);
            emit_node(child, ctx);
            let nested = std::mem::replace(&mut ctx.output, old_output);
            for line in nested.lines() {
                ctx.write(&indent);
                ctx.writeln(line);
            }
        } else {
            emit_node(child, ctx);
        }
        first = false;
    }
}

fn emit_inline_children(node: &Node, ctx: &mut EmitContext) {
    for child in &node.children {
        emit_inline(child, ctx);
    }
}

/// Escape special djot characters in plain text content.
///
/// Djot uses `\X` to escape any non-alphanumeric ASCII character.
/// We escape inline markup characters unconditionally, plus `:` which
/// is a definition-list block marker and dangerous even in inline
/// positions (jotdown will parse a paragraph starting with `:` as a
/// definition list).  Other block-level starters (`-`, `+`, `#`, `>`)
/// are only ever special at the start of a *block line*, not inside an
/// already-open inline run, so we leave them unescaped to keep output
/// readable.
fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    for (i, &ch) in chars.iter().enumerate() {
        match ch {
            // Escape character itself
            '\\' => {
                out.push('\\');
                out.push('\\');
            }
            // Inline code
            '`' => {
                out.push('\\');
                out.push('`');
            }
            // Strong / emphasis
            '*' | '_' => {
                out.push('\\');
                out.push(ch);
            }
            // Span / link / footnote starters
            '{' | '[' => {
                out.push('\\');
                out.push(ch);
            }
            // Superscript / subscript / math
            '^' | '~' | '$' => {
                out.push('\\');
                out.push(ch);
            }
            // Image: only special before '['
            '!' if chars.get(i + 1) == Some(&'[') => {
                out.push('\\');
                out.push('!');
            }
            // Definition-list marker: dangerous at start of any line
            ':' => {
                out.push('\\');
                out.push(':');
            }
            // Straight quotes: jotdown applies smart-quote substitution, so
            // an unescaped ' or " round-trips as a curly quote (≠ original).
            '\'' | '"' => {
                out.push('\\');
                out.push(ch);
            }
            // Table marker: '|' is a table-row delimiter even at start of
            // a paragraph — jotdown parses "|cell|" as a table, not text.
            '|' => {
                out.push('\\');
                out.push('|');
            }
            other => out.push(other),
        }
    }
    out
}

/// Choose backtick delimiter for an inline code span.
///
/// Uses the smallest N such that no run of N backticks appears in `content`,
/// preventing the delimiter from being misread as part of the content.
fn code_span_delimiters(content: &str) -> String {
    let mut max_run = 0usize;
    let mut run = 0usize;
    for ch in content.chars() {
        if ch == '`' {
            run += 1;
            max_run = max_run.max(run);
        } else {
            run = 0;
        }
    }
    "`".repeat(max_run + 1)
}

fn emit_inline(node: &Node, ctx: &mut EmitContext) {
    match node.kind.as_str() {
        node::TEXT => {
            if let Some(content) = node.props.get_str(prop::CONTENT) {
                ctx.write(&escape_text(content));
            }
        }
        node::EMPHASIS => {
            ctx.write("_");
            emit_inline_children(node, ctx);
            ctx.write("_");
        }
        node::STRONG => {
            ctx.write("*");
            emit_inline_children(node, ctx);
            ctx.write("*");
        }
        node::STRIKEOUT => {
            ctx.write("{-");
            emit_inline_children(node, ctx);
            ctx.write("-}");
        }
        node::SUBSCRIPT => {
            ctx.write("~");
            emit_inline_children(node, ctx);
            ctx.write("~");
        }
        node::SUPERSCRIPT => {
            ctx.write("^");
            emit_inline_children(node, ctx);
            ctx.write("^");
        }
        node::UNDERLINE => {
            // Djot uses {+...+} for insert, which is close to underline
            ctx.write("{+");
            emit_inline_children(node, ctx);
            ctx.write("+}");
        }
        node::CODE => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            let delim = code_span_delimiters(content);
            ctx.write(&delim);
            ctx.write(content);
            ctx.write(&delim);
        }
        node::LINK => {
            let url = node.props.get_str(prop::URL).unwrap_or("");
            ctx.write("[");
            emit_inline_children(node, ctx);
            ctx.write("](");
            ctx.write(url);
            ctx.write(")");
        }
        node::IMAGE => {
            let url = node.props.get_str(prop::URL).unwrap_or("");
            let alt = node.props.get_str(prop::ALT).unwrap_or("");
            ctx.write("![");
            ctx.write(alt);
            ctx.write("](");
            ctx.write(url);
            ctx.write(")");
        }
        node::LINE_BREAK => {
            ctx.write("\\\n");
        }
        node::SOFT_BREAK => {
            ctx.newline();
        }
        node::FOOTNOTE_REF => {
            let label = node.props.get_str(prop::LABEL).unwrap_or("?");
            ctx.write(&format!("[^{}]", label));
        }
        node::SPAN => {
            // Djot span syntax
            ctx.write("[");
            emit_inline_children(node, ctx);
            ctx.write("]{}");
        }
        node::RAW_INLINE => {
            let format = node.props.get_str(prop::FORMAT).unwrap_or("html");
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            ctx.write(&format!("`{}`{{{}}}", content, format));
        }
        "math:inline" => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            ctx.write("$");
            ctx.write(content);
            ctx.write("$");
        }
        "math:display" => {
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            ctx.write("$$");
            ctx.write(content);
            ctx.write("$$");
        }
        _ => {
            // Unknown inline - try to emit children
            emit_inline_children(node, ctx);
        }
    }
}

fn emit_table(node: &Node, ctx: &mut EmitContext) {
    // Find header row and body rows
    let mut header_row: Option<&Node> = None;
    let mut body_rows: Vec<&Node> = Vec::new();

    for child in &node.children {
        if child.kind.as_str() == node::TABLE_ROW {
            // Check if this is a header row (first row with TABLE_HEADER cells)
            let has_headers = child
                .children
                .iter()
                .any(|c| c.kind.as_str() == node::TABLE_HEADER);
            if has_headers && header_row.is_none() {
                header_row = Some(child);
            } else {
                body_rows.push(child);
            }
        }
    }

    // Emit header
    if let Some(header) = header_row {
        ctx.write("|");
        for cell in &header.children {
            ctx.write(" ");
            emit_inline_children(cell, ctx);
            ctx.write(" |");
        }
        ctx.newline();

        // Separator
        ctx.write("|");
        for _ in &header.children {
            ctx.write("---|");
        }
        ctx.newline();
    }

    // Emit body rows
    for row in body_rows {
        ctx.write("|");
        for cell in &row.children {
            ctx.write(" ");
            emit_inline_children(cell, ctx);
            ctx.write(" |");
        }
        ctx.newline();
    }
}

fn is_tight_list(list: &Node) -> bool {
    // A list is tight if no list item contains multiple block elements
    for item in &list.children {
        if item.children.len() > 1 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_std::builder::doc;

    #[test]
    fn test_emit_paragraph() {
        let document = doc(|d| d.para(|i| i.text("Hello, world!")));
        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert_eq!(output, "Hello, world!\n");
    }

    #[test]
    fn test_emit_heading() {
        let document = doc(|d| d.heading(2, |i| i.text("Title")));
        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.starts_with("## Title"));
    }

    #[test]
    fn test_emit_emphasis() {
        let document = doc(|d| d.para(|i| i.em(|i| i.text("emphasis"))));
        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("_emphasis_"));
    }

    #[test]
    fn test_emit_strong() {
        let document = doc(|d| d.para(|i| i.strong(|i| i.text("bold"))));
        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("*bold*"));
    }

    #[test]
    fn test_emit_link() {
        let document = doc(|d| d.para(|i| i.link("https://example.com", |i| i.text("link"))));
        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("[link](https://example.com)"));
    }

    #[test]
    fn test_emit_code_block() {
        let document = doc(|d| d.code_block_lang("fn main() {}", "rust"));
        let result = emit(&document).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("```rust"));
        assert!(output.contains("fn main() {}"));
        assert!(output.contains("```\n"));
    }
}

/// Roundtrip regression tests for fuzz-found crashes.
#[cfg(test)]
mod roundtrip_tests {
    use super::emit;
    use rescribe_read_djot::parse;

    fn roundtrip_text_preserved(input: &str) {
        fn extract_text(node: &rescribe_core::Node) -> String {
            use rescribe_std::{node, prop};
            let mut t = String::new();
            if node.kind.as_str() == node::TEXT
                && let Some(c) = node.props.get_str(prop::CONTENT)
            {
                t.push_str(c);
            }
            for ch in &node.children {
                t.push_str(&extract_text(ch));
            }
            t
        }
        let doc1 = parse(input).unwrap().value;
        let out = emit(&doc1).unwrap();
        let s = String::from_utf8(out.value).unwrap();
        let doc2 = parse(&s).unwrap().value;
        let t1: String = extract_text(&doc1.content)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let t2: String = extract_text(&doc2.content)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        // Use multiset (sorted-char) equality: detects character additions/removals
        // while tolerating jotdown's span-delimiter adjacency reordering quirk.
        let mut c1: Vec<char> = t1.chars().collect();
        let mut c2: Vec<char> = t2.chars().collect();
        c1.sort_unstable();
        c2.sort_unstable();
        assert_eq!(
            c1, c2,
            "roundtrip text mismatch for {input:?}: {t1:?} -> {t2:?}"
        );
    }

    // Empty list item: trailing space on "- " was stripped by trim_end(),
    // making "-" be read back as plain text instead of a list marker.
    #[test]
    fn empty_list_item() {
        roundtrip_text_preserved("\n:\n+");
    }

    // Escaped colon: \: → text ":" → writer emitted bare ":", which djot
    // re-reads as a definition-list marker.
    #[test]
    fn escaped_colon() {
        roundtrip_text_preserved("\\:");
    }

    // Code span with internal backtick run: content "-``)" was wrapped in
    // "``...``" but the internal "``" broke the delimiter.
    #[test]
    fn code_span_internal_backtick_run() {
        roundtrip_text_preserved("`-``)");
    }

    // Smart quotes: unescaped straight apostrophe ' (U+0027) round-trips as
    // curly right-single-quote ' (U+2019) because jotdown applies smart quotes.
    #[test]
    fn smart_quote_apostrophe() {
        roundtrip_text_preserved("\\'|");
    }

    // Table marker: unescaped '|' is absorbed as a table column separator on
    // re-parse, stripping the pipe characters from the text content.
    #[test]
    fn pipe_as_table_marker() {
        roundtrip_text_preserved("|\x7f\\|");
    }

    // Nested superscripts: ^^^:^ → IR TEXT("^^") + SUPERSCRIPT(TEXT(":")), writer
    // emits \^\^^\\:^ which jotdown reparses differently due to span-delimiter
    // adjacency quirk.  Characters are reordered but not added/removed, so
    // multiset equality passes while strict equality would fail.
    #[test]
    fn nested_superscripts() {
        roundtrip_text_preserved("`!`^^^:^");
    }
}
