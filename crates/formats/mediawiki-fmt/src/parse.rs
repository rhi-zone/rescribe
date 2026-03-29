//! MediaWiki parser -- infallible, returns (MediawikiDoc, Vec<Diagnostic>).

use crate::ast::{
    Block, DefinitionItem, Diagnostic, Inline, MediawikiDoc, Span, TableCell, TableRow,
};

/// Returns true if `chars[pos..]` starts with `<tag>` exactly (case-insensitive).
fn match_html_tag(chars: &[char], pos: usize, tag: &str) -> bool {
    let tag_chars: Vec<char> = tag.chars().collect();
    let open_len = 1 + tag_chars.len() + 1;
    if pos + open_len > chars.len() {
        return false;
    }
    chars[pos] == '<'
        && chars[pos + 1..pos + 1 + tag_chars.len()]
            .iter()
            .zip(tag_chars.iter())
            .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
        && chars[pos + 1 + tag_chars.len()] == '>'
}

/// Match a self-closing tag like `<references/>` or `<references />`
fn match_self_closing_tag(chars: &[char], pos: usize, tag: &str) -> Option<usize> {
    let tag_chars: Vec<char> = tag.chars().collect();
    if pos + 2 + tag_chars.len() > chars.len() || chars[pos] != '<' {
        return None;
    }
    if !chars[pos + 1..pos + 1 + tag_chars.len()]
        .iter()
        .zip(tag_chars.iter())
        .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
    {
        return None;
    }
    let after_tag = pos + 1 + tag_chars.len();
    let mut i = after_tag;
    // skip whitespace
    while i < chars.len() && chars[i] == ' ' {
        i += 1;
    }
    if i < chars.len() && chars[i] == '/' {
        i += 1;
        if i < chars.len() && chars[i] == '>' {
            return Some(i + 1);
        }
    }
    None
}

/// Match `<tag ...>` with optional attributes, returns end position after `>`.
fn match_html_tag_with_attrs(chars: &[char], pos: usize, tag: &str) -> Option<usize> {
    let tag_chars: Vec<char> = tag.chars().collect();
    if pos + 2 + tag_chars.len() > chars.len() || chars[pos] != '<' {
        return None;
    }
    if !chars[pos + 1..pos + 1 + tag_chars.len()]
        .iter()
        .zip(tag_chars.iter())
        .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
    {
        return None;
    }
    let after_tag = pos + 1 + tag_chars.len();
    if after_tag >= chars.len() {
        return None;
    }
    // Either '>' immediately or space/attrs then '>'
    if chars[after_tag] == '>' {
        return Some(after_tag + 1);
    }
    if chars[after_tag] == ' ' || chars[after_tag] == '\t' {
        let mut i = after_tag;
        while i < chars.len() && chars[i] != '>' {
            i += 1;
        }
        if i < chars.len() {
            return Some(i + 1);
        }
    }
    None
}

/// Finds the first occurrence of `</tag>` in `chars[start..]`, returns its position
/// (case-insensitive).
fn find_close_html_tag(chars: &[char], start: usize, tag: &str) -> Option<usize> {
    let close: Vec<char> = format!("</{}>", tag).chars().collect();
    let close_len = close.len();
    (start..=chars.len().saturating_sub(close_len)).find(|&pos| {
        chars[pos..pos + close_len]
            .iter()
            .zip(close.iter())
            .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()))
    })
}

/// Extract the `lang="..."` attribute from syntaxhighlight/source tag chars.
fn extract_lang_attr(chars: &[char], start: usize, end: usize) -> Option<String> {
    let s: String = chars[start..end].iter().collect();
    // Find lang="..." or language="..."
    for prefix in &["lang=\"", "language=\"", "lang='", "language='"] {
        if let Some(pos) = s.to_lowercase().find(prefix) {
            let quote = if prefix.ends_with('"') { '"' } else { '\'' };
            let val_start = pos + prefix.len();
            if let Some(val_end) = s[val_start..].find(quote) {
                return Some(s[val_start..val_start + val_end].to_string());
            }
        }
    }
    None
}

/// Parse a MediaWiki string into a [`MediawikiDoc`].
///
/// The parser is infallible: any unrecognised input is treated as a paragraph.
/// Diagnostics (warnings/errors) are returned alongside the document.
pub fn parse(input: &str) -> (MediawikiDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let (blocks, diags) = p.parse();
    (MediawikiDoc { blocks, span: Span::NONE }, diags)
}

struct Parser<'a> {
    input: &'a str,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input }
    }

    fn parse(&mut self) -> (Vec<Block>, Vec<Diagnostic>) {
        let lines: Vec<&str> = self.input.lines().collect();
        let mut i = 0;
        let mut blocks = Vec::new();
        let diags = Vec::new();

        while i < lines.len() {
            let line = lines[i];
            let trimmed = line.trim();

            if trimmed.is_empty() {
                i += 1;
                continue;
            }

            // HTML comments: <!-- ... -->
            if trimmed.starts_with("<!--") {
                let (_, consumed) = self.skip_comment(&lines[i..]);
                i += consumed;
                continue;
            }

            // Magic words: __TOC__, __NOTOC__, __FORCETOC__ -- skip
            if trimmed.starts_with("__")
                && trimmed.ends_with("__")
                && trimmed.len() > 4
                && trimmed[2..trimmed.len() - 2]
                    .chars()
                    .all(|c| c.is_ascii_uppercase())
            {
                i += 1;
                continue;
            }

            // Heading
            if trimmed.starts_with('=')
                && let Some(heading) = self.parse_heading(trimmed)
            {
                blocks.push(heading);
                i += 1;
                continue;
            }

            // Definition list
            if trimmed.starts_with(';') {
                let (dl, consumed) = self.parse_definition_list(&lines[i..]);
                blocks.push(dl);
                i += consumed;
                continue;
            }

            // List
            if trimmed.starts_with('*') || trimmed.starts_with('#') {
                let (list, consumed) = self.parse_list(&lines[i..]);
                blocks.push(list);
                i += consumed;
                continue;
            }

            // Horizontal rule
            if trimmed == "----" || (trimmed.chars().all(|c| c == '-') && trimmed.len() >= 4)
            {
                blocks.push(Block::HorizontalRule);
                i += 1;
                continue;
            }

            // <blockquote>...</blockquote>
            if trimmed.to_lowercase().starts_with("<blockquote") {
                let (block, consumed) = self.parse_blockquote_tag(&lines[i..]);
                blocks.push(block);
                i += consumed;
                continue;
            }

            // <pre>...</pre>
            if trimmed.to_lowercase().starts_with("<pre") {
                let (block, consumed) = self.parse_pre_tag(&lines[i..]);
                blocks.push(block);
                i += consumed;
                continue;
            }

            // <nowiki>...</nowiki> block
            if trimmed.to_lowercase().starts_with("<nowiki>") {
                let (block, consumed) = self.parse_nowiki_block(&lines[i..]);
                blocks.push(block);
                i += consumed;
                continue;
            }

            // <syntaxhighlight> or <source>
            if trimmed.to_lowercase().starts_with("<syntaxhighlight")
                || trimmed.to_lowercase().starts_with("<source")
            {
                let (block, consumed) = self.parse_syntaxhighlight(&lines[i..]);
                blocks.push(block);
                i += consumed;
                continue;
            }

            // <math>...</math> (block-level if it's the only thing on the line)
            if trimmed.to_lowercase().starts_with("<math>")
                && trimmed.to_lowercase().contains("</math>")
            {
                // Treat as inline math inside a paragraph -- fall through
            } else if trimmed.to_lowercase().starts_with("<math>") {
                let (block, consumed) = self.parse_math_block(&lines[i..]);
                blocks.push(block);
                i += consumed;
                continue;
            }

            // <references/> or <references /> -- skip (rendered by ref system)
            if trimmed.to_lowercase().starts_with("<references") {
                i += 1;
                continue;
            }

            // Code block (indented with space)
            if line.starts_with(' ') {
                let (block, consumed) = self.parse_code_block(&lines[i..]);
                blocks.push(block);
                i += consumed;
                continue;
            }

            // Table
            if trimmed.starts_with("{|") {
                let (table, consumed) = self.parse_table(&lines[i..]);
                blocks.push(table);
                i += consumed;
                continue;
            }

            // Regular paragraph
            let (para, consumed) = self.parse_paragraph(&lines[i..]);
            blocks.push(para);
            i += consumed;
        }

        (blocks, diags)
    }

    fn skip_comment(&self, lines: &[&str]) -> ((), usize) {
        let joined = lines.join("\n");
        if let Some(end) = joined.find("-->") {
            let consumed_chars = end + 3;
            // Count newlines up to that point
            let consumed = joined[..consumed_chars].matches('\n').count() + 1;
            ((), consumed)
        } else {
            ((), lines.len())
        }
    }

    fn parse_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim();

        // Count leading `=`
        let level = trimmed.chars().take_while(|&c| c == '=').count();
        if level == 0 || level > 6 {
            return None;
        }

        // Check for matching trailing `=`
        let content = trimmed
            .trim_start_matches('=')
            .trim_end_matches('=')
            .trim();

        let inlines = self.parse_inline(content);
        Some(Block::Heading { level: level as u8, inlines, span: Span::NONE })
    }

    fn parse_definition_list(&self, lines: &[&str]) -> (Block, usize) {
        let mut items = Vec::new();
        let mut consumed = 0;
        let mut pending_term: Option<Vec<Inline>> = None;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }

            if let Some(rest) = trimmed.strip_prefix(';') {
                // Flush previous pending term without description
                if let Some(term) = pending_term.take() {
                    items.push(DefinitionItem { term, desc: Vec::new() });
                }
                let content = rest.trim();
                // Check if term and def are on the same line with ":"
                if let Some(colon_pos) = content.find(':') {
                    let term_text = content[..colon_pos].trim();
                    let desc_text = content[colon_pos + 1..].trim();
                    items.push(DefinitionItem {
                        term: self.parse_inline(term_text),
                        desc: self.parse_inline(desc_text),
                    });
                } else {
                    pending_term = Some(self.parse_inline(content));
                }
                consumed += 1;
            } else if let Some(rest) = trimmed.strip_prefix(':') {
                if pending_term.is_none() {
                    break;
                }
                let desc_text = rest.trim();
                let term = pending_term.take().unwrap();
                items.push(DefinitionItem {
                    term,
                    desc: self.parse_inline(desc_text),
                });
                consumed += 1;
            } else {
                break;
            }
        }

        // Flush any remaining pending term
        if let Some(term) = pending_term {
            items.push(DefinitionItem { term, desc: Vec::new() });
        }

        (Block::DefinitionList { items, span: Span::NONE }, consumed.max(1))
    }

    fn parse_list(&self, lines: &[&str]) -> (Block, usize) {
        let mut items: Vec<Vec<Block>> = Vec::new();
        let mut consumed = 0;
        let first_char = lines[0].trim().chars().next().unwrap_or('*');
        let ordered = first_char == '#';

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }

            // Check if this is a list item with the same marker
            let marker = if ordered { '#' } else { '*' };
            if !trimmed.starts_with(marker) {
                break;
            }

            // For simplicity, flatten nested items
            let content = trimmed.trim_start_matches(marker).trim();
            let inlines = self.parse_inline(content);
            items.push(vec![Block::Paragraph { inlines, span: Span::NONE }]);

            consumed += 1;
        }

        (Block::List { ordered, items, span: Span::NONE }, consumed.max(1))
    }

    fn parse_code_block(&self, lines: &[&str]) -> (Block, usize) {
        let mut content = String::new();
        let mut consumed = 0;

        for line in lines {
            if !line.starts_with(' ') && !line.is_empty() {
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            // Remove one leading space
            content.push_str(line.strip_prefix(' ').unwrap_or(line));
            consumed += 1;
        }

        (Block::CodeBlock { language: None, content, span: Span::NONE }, consumed.max(1))
    }

    fn parse_blockquote_tag(&self, lines: &[&str]) -> (Block, usize) {
        let joined = lines.join("\n");
        let lower = joined.to_lowercase();
        if let Some(end) = lower.find("</blockquote>") {
            let open_end = joined.find('>').unwrap_or(0) + 1;
            let inner = joined[open_end..end].trim();
            let consumed = joined[..end + "</blockquote>".len()]
                .matches('\n')
                .count()
                + 1;
            // Parse inner content as blocks
            let (inner_doc, _) = parse(inner);
            (
                Block::Blockquote { children: inner_doc.blocks, span: Span::NONE },
                consumed,
            )
        } else {
            // Unclosed blockquote -- treat rest as content
            let open_end = joined.find('>').unwrap_or(0) + 1;
            let inner = joined[open_end..].trim();
            let (inner_doc, _) = parse(inner);
            (
                Block::Blockquote { children: inner_doc.blocks, span: Span::NONE },
                lines.len(),
            )
        }
    }

    fn parse_pre_tag(&self, lines: &[&str]) -> (Block, usize) {
        let joined = lines.join("\n");
        let lower = joined.to_lowercase();
        if let Some(end) = lower.find("</pre>") {
            let open_end = joined.find('>').unwrap_or(0) + 1;
            let content = &joined[open_end..end];
            let consumed = joined[..end + "</pre>".len()].matches('\n').count() + 1;
            (Block::PreBlock { content: content.to_string(), span: Span::NONE }, consumed)
        } else {
            let open_end = joined.find('>').unwrap_or(0) + 1;
            (
                Block::PreBlock { content: joined[open_end..].to_string(), span: Span::NONE },
                lines.len(),
            )
        }
    }

    fn parse_nowiki_block(&self, lines: &[&str]) -> (Block, usize) {
        let joined = lines.join("\n");
        let lower = joined.to_lowercase();
        if let Some(end) = lower.find("</nowiki>") {
            let open_end = lower.find("<nowiki>").unwrap_or(0) + "<nowiki>".len();
            let content = &joined[open_end..end];
            let consumed = joined[..end + "</nowiki>".len()].matches('\n').count() + 1;
            (Block::PreBlock { content: content.to_string(), span: Span::NONE }, consumed)
        } else {
            let open_end = "<nowiki>".len();
            (
                Block::PreBlock {
                    content: joined[open_end..].to_string(),
                    span: Span::NONE,
                },
                lines.len(),
            )
        }
    }

    fn parse_syntaxhighlight(&self, lines: &[&str]) -> (Block, usize) {
        let joined = lines.join("\n");
        let lower = joined.to_lowercase();
        // Find the end tag
        let (end_tag, end_pos) =
            if let Some(pos) = lower.find("</syntaxhighlight>") {
                ("</syntaxhighlight>", pos)
            } else if let Some(pos) = lower.find("</source>") {
                ("</source>", pos)
            } else {
                // Unclosed
                let open_end = joined.find('>').unwrap_or(0) + 1;
                return (
                    Block::CodeBlock {
                        language: None,
                        content: joined[open_end..].to_string(),
                        span: Span::NONE,
                    },
                    lines.len(),
                );
            };

        let open_end = joined.find('>').unwrap_or(0) + 1;
        let chars: Vec<char> = joined.chars().collect();
        let lang = extract_lang_attr(&chars, 0, open_end);
        let content = &joined[open_end..end_pos];
        // Strip leading/trailing newline from content
        let content = content.strip_prefix('\n').unwrap_or(content);
        let content = content.strip_suffix('\n').unwrap_or(content);
        let consumed = joined[..end_pos + end_tag.len()].matches('\n').count() + 1;
        (
            Block::CodeBlock { language: lang, content: content.to_string(), span: Span::NONE },
            consumed,
        )
    }

    fn parse_math_block(&self, lines: &[&str]) -> (Block, usize) {
        let joined = lines.join("\n");
        let lower = joined.to_lowercase();
        if let Some(end) = lower.find("</math>") {
            let open_end = "<math>".len();
            let source = joined[open_end..end].trim();
            let consumed = joined[..end + "</math>".len()].matches('\n').count() + 1;
            // Emit as a paragraph containing MathInline
            (
                Block::Paragraph {
                    inlines: vec![Inline::MathInline { source: source.to_string() }],
                    span: Span::NONE,
                },
                consumed,
            )
        } else {
            // Unclosed math
            (
                Block::Paragraph {
                    inlines: vec![Inline::Text(joined)],
                    span: Span::NONE,
                },
                lines.len(),
            )
        }
    }

    fn parse_table(&self, lines: &[&str]) -> (Block, usize) {
        let mut rows = Vec::new();
        let mut consumed = 0;
        let mut caption = None;

        for line in lines {
            let trimmed = line.trim();

            if trimmed == "|}" {
                consumed += 1;
                break;
            }

            if trimmed.starts_with("{|") {
                consumed += 1;
                continue;
            }

            // Table caption: |+ caption text
            if let Some(rest) = trimmed.strip_prefix("|+") {
                let cap_text = rest.trim();
                caption = Some(self.parse_inline(cap_text));
                consumed += 1;
                continue;
            }

            if trimmed.starts_with("|-") {
                // Table row marker
                consumed += 1;
                continue;
            }

            if trimmed.starts_with('|') || trimmed.starts_with('!') {
                // Parse cells in this line
                let is_header = trimmed.starts_with('!');
                let content = trimmed.trim_start_matches(['|', '!']);
                let separator = if is_header { "!!" } else { "||" };
                let cells_str: Vec<&str> = content.split(separator).collect();
                let mut cells = Vec::new();

                for cell_content in cells_str {
                    let inlines = self.parse_inline(cell_content.trim());
                    cells.push(TableCell { is_header, inlines, span: Span::NONE });
                }

                if !cells.is_empty() {
                    rows.push(TableRow { cells, span: Span::NONE });
                }
            }

            consumed += 1;
        }

        (Block::Table { rows, caption, span: Span::NONE }, consumed.max(1))
    }

    fn parse_paragraph(&self, lines: &[&str]) -> (Block, usize) {
        let mut text = String::new();
        let mut consumed = 0;

        for line in lines {
            let trimmed = line.trim();

            // Stop at empty lines, headings, lists, rules, tables, block tags
            if trimmed.is_empty()
                || trimmed.starts_with('=')
                || trimmed.starts_with('*')
                || trimmed.starts_with('#')
                || trimmed.starts_with(';')
                || trimmed == "----"
                || (trimmed.chars().all(|c| c == '-') && trimmed.len() >= 4)
                || trimmed.starts_with("{|")
                || trimmed == "|}"
                || trimmed.starts_with("|-")
                || trimmed.starts_with('|')
                || trimmed.starts_with('!')
                || trimmed.starts_with("<!--")
            {
                break;
            }

            // Stop at block-level HTML tags
            let lower = trimmed.to_lowercase();
            if lower.starts_with("<blockquote")
                || lower.starts_with("<pre")
                || lower.starts_with("<nowiki>")
                || lower.starts_with("<syntaxhighlight")
                || lower.starts_with("<source")
            {
                break;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(trimmed);
            consumed += 1;
        }

        let inlines = self.parse_inline(&text);
        (Block::Paragraph { inlines, span: Span::NONE }, consumed.max(1))
    }

    #[allow(clippy::only_used_in_recursion)]
    fn parse_inline(&self, text: &str) -> Vec<Inline> {
        let mut inlines = Vec::new();
        let mut current_text = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Template: {{...}}
            if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
                if !current_text.is_empty() {
                    inlines.push(Inline::Text(current_text.clone()));
                    current_text.clear();
                }
                let start = i + 2;
                let mut depth = 1;
                let mut end = start;
                while end < chars.len() {
                    if end + 1 < chars.len() && chars[end] == '{' && chars[end + 1] == '{' {
                        depth += 1;
                        end += 2;
                        continue;
                    }
                    if end + 1 < chars.len() && chars[end] == '}' && chars[end + 1] == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        end += 2;
                        continue;
                    }
                    end += 1;
                }
                if depth == 0 {
                    let content: String = chars[start..end].iter().collect();
                    inlines.push(Inline::Template { content });
                    i = end + 2;
                    continue;
                }
                // Unclosed -- treat as text
                current_text.push_str("{{");
                i += 2;
                continue;
            }

            // Bold+Italic: '''''text'''''
            if i + 4 < chars.len()
                && chars[i] == '\''
                && chars[i + 1] == '\''
                && chars[i + 2] == '\''
                && chars[i + 3] == '\''
                && chars[i + 4] == '\''
            {
                // Find closing '''''
                let start = i + 5;
                let mut end = start;
                while end + 4 < chars.len() {
                    if chars[end] == '\''
                        && chars[end + 1] == '\''
                        && chars[end + 2] == '\''
                        && chars[end + 3] == '\''
                        && chars[end + 4] == '\''
                    {
                        break;
                    }
                    end += 1;
                }
                if end + 4 < chars.len() {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let inner: String = chars[start..end].iter().collect();
                    let inner_inlines = self.parse_inline(&inner);
                    inlines.push(Inline::Bold(vec![Inline::Italic(inner_inlines)]));
                    i = end + 5;
                    continue;
                }
            }

            // Bold: '''text'''
            if i + 2 < chars.len()
                && chars[i] == '\''
                && chars[i + 1] == '\''
                && chars[i + 2] == '\''
            {
                if !current_text.is_empty() {
                    inlines.push(Inline::Text(current_text.clone()));
                    current_text.clear();
                }

                // Find closing '''
                let start = i + 3;
                let mut end = start;
                while end + 2 < chars.len() {
                    if chars[end] == '\''
                        && chars[end + 1] == '\''
                        && chars[end + 2] == '\''
                    {
                        break;
                    }
                    end += 1;
                }

                if end + 2 < chars.len() {
                    let inner: String = chars[start..end].iter().collect();
                    let inner_inlines = self.parse_inline(&inner);
                    inlines.push(Inline::Bold(inner_inlines));
                    i = end + 3;
                    continue;
                }
            }

            // Italic: ''text''
            if i + 1 < chars.len() && chars[i] == '\'' && chars[i + 1] == '\'' {
                // Make sure it's not bold
                if i + 2 < chars.len() && chars[i + 2] == '\'' {
                    // This is bold, handled above
                } else {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }

                    // Find closing ''
                    let start = i + 2;
                    let mut end = start;
                    while end + 1 < chars.len() {
                        if chars[end] == '\'' && chars[end + 1] == '\'' {
                            // Make sure it's not '''
                            if end + 2 < chars.len() && chars[end + 2] == '\'' {
                                end += 1;
                                continue;
                            }
                            break;
                        }
                        end += 1;
                    }

                    if end + 1 < chars.len() {
                        let inner: String = chars[start..end].iter().collect();
                        let inner_inlines = self.parse_inline(&inner);
                        inlines.push(Inline::Italic(inner_inlines));
                        i = end + 2;
                        continue;
                    }
                }
            }

            // Internal link: [[Title]] or [[Title|text]]
            if i + 1 < chars.len() && chars[i] == '[' && chars[i + 1] == '[' {
                if !current_text.is_empty() {
                    inlines.push(Inline::Text(current_text.clone()));
                    current_text.clear();
                }

                // Find closing ]]
                let start = i + 2;
                let mut end = start;
                while end + 1 < chars.len() {
                    if chars[end] == ']' && chars[end + 1] == ']' {
                        break;
                    }
                    end += 1;
                }

                if end + 1 < chars.len() {
                    let inner: String = chars[start..end].iter().collect();

                    // Image: [[File:filename|alt]] or [[Image:filename|alt]]
                    let image_prefix = if inner.starts_with("File:") {
                        Some(5usize)
                    } else if inner.starts_with("Image:") {
                        Some(6usize)
                    } else {
                        None
                    };
                    if let Some(prefix_len) = image_prefix {
                        let (url_part, alt_part) = if let Some(pipe_pos) = inner.find('|')
                        {
                            (
                                inner[prefix_len..pipe_pos].to_string(),
                                inner[pipe_pos + 1..].to_string(),
                            )
                        } else {
                            (inner[prefix_len..].to_string(), String::new())
                        };
                        inlines.push(Inline::Image { url: url_part, alt: alt_part });
                        i = end + 2;
                        continue;
                    }

                    let (url, text) = if let Some(pipe_pos) = inner.find('|') {
                        let url = &inner[..pipe_pos];
                        let text = &inner[pipe_pos + 1..];
                        (url.to_string(), text.to_string())
                    } else {
                        (inner.clone(), inner)
                    };

                    inlines.push(Inline::Link { url, text });
                    i = end + 2;
                    continue;
                }
            }

            // External link: [url text]
            if chars[i] == '[' && (i + 1 >= chars.len() || chars[i + 1] != '[') {
                if !current_text.is_empty() {
                    inlines.push(Inline::Text(current_text.clone()));
                    current_text.clear();
                }

                // Find closing ]
                let start = i + 1;
                let mut end = start;
                while end < chars.len() && chars[end] != ']' {
                    end += 1;
                }

                if end < chars.len() {
                    let inner: String = chars[start..end].iter().collect();
                    let parts: Vec<&str> = inner.splitn(2, ' ').collect();
                    let url = parts[0].to_string();
                    let text = if parts.len() > 1 {
                        parts[1].to_string()
                    } else {
                        url.clone()
                    };

                    inlines.push(Inline::Link { url, text });
                    i = end + 1;
                    continue;
                }
            }

            // HTML tag inlines
            if chars[i] == '<' {
                // <br/> or <br />
                let is_br_void = chars.get(i + 1) == Some(&'b')
                    && chars.get(i + 2) == Some(&'r')
                    && chars.get(i + 3) == Some(&'/')
                    && chars.get(i + 4) == Some(&'>');
                let is_br_space = chars.get(i + 1) == Some(&'b')
                    && chars.get(i + 2) == Some(&'r')
                    && chars.get(i + 3) == Some(&' ')
                    && chars.get(i + 4) == Some(&'/')
                    && chars.get(i + 5) == Some(&'>');
                if is_br_void || is_br_space {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    inlines.push(Inline::LineBreak);
                    i += if is_br_void { 5 } else { 6 };
                    continue;
                }

                // <code>...</code>
                if match_html_tag(&chars, i, "code")
                    && let Some(close) = find_close_html_tag(&chars, i + 6, "code")
                {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let inner: String = chars[i + 6..close].iter().collect();
                    inlines.push(Inline::Code(inner));
                    i = close + 7; // "</code>" is 7 chars
                    continue;
                }

                // <tt>...</tt>
                if match_html_tag(&chars, i, "tt")
                    && let Some(close) = find_close_html_tag(&chars, i + 4, "tt")
                {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let inner: String = chars[i + 4..close].iter().collect();
                    inlines.push(Inline::Code(inner));
                    i = close + 5; // "</tt>" is 5 chars
                    continue;
                }

                // <nowiki>...</nowiki> (inline)
                if match_html_tag(&chars, i, "nowiki")
                    && let Some(close) = find_close_html_tag(&chars, i + 8, "nowiki")
                {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let inner: String = chars[i + 8..close].iter().collect();
                    inlines.push(Inline::Nowiki { content: inner });
                    i = close + 9; // "</nowiki>" is 9 chars
                    continue;
                }

                // <math>...</math>
                if match_html_tag(&chars, i, "math")
                    && let Some(close) = find_close_html_tag(&chars, i + 6, "math")
                {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let inner: String = chars[i + 6..close].iter().collect();
                    inlines.push(Inline::MathInline { source: inner });
                    i = close + 7; // "</math>" is 7 chars
                    continue;
                }

                // <ref>...</ref> (footnote)
                if match_html_tag(&chars, i, "ref")
                    && let Some(close) = find_close_html_tag(&chars, i + 5, "ref")
                {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let inner: String = chars[i + 5..close].iter().collect();
                    inlines.push(Inline::FootnoteRef {
                        label: String::new(),
                        content: Some(inner),
                    });
                    i = close + 6; // "</ref>" is 6 chars
                    continue;
                }

                // <ref name="...">...</ref>
                if let Some(tag_end) = match_html_tag_with_attrs(&chars, i, "ref") {
                    // Check for self-closing: <ref name="..." />
                    if let Some(sc_end) = match_self_closing_tag(&chars, i, "ref") {
                        // Extract name from the tag
                        let tag_content: String = chars[i..sc_end].iter().collect();
                        let name = extract_ref_name(&tag_content);
                        if !current_text.is_empty() {
                            inlines.push(Inline::Text(current_text.clone()));
                            current_text.clear();
                        }
                        inlines.push(Inline::FootnoteRef {
                            label: name.unwrap_or_default(),
                            content: None,
                        });
                        i = sc_end;
                        continue;
                    }
                    if let Some(close) = find_close_html_tag(&chars, tag_end, "ref") {
                        let tag_content: String = chars[i..tag_end].iter().collect();
                        let name = extract_ref_name(&tag_content);
                        if !current_text.is_empty() {
                            inlines.push(Inline::Text(current_text.clone()));
                            current_text.clear();
                        }
                        let inner: String = chars[tag_end..close].iter().collect();
                        inlines.push(Inline::FootnoteRef {
                            label: name.unwrap_or_default(),
                            content: Some(inner),
                        });
                        i = close + 6;
                        continue;
                    }
                }

                // <references/> or <references /> (inline occurrence)
                if let Some(sc_end) = match_self_closing_tag(&chars, i, "references") {
                    // Skip it
                    i = sc_end;
                    continue;
                }

                // <del>...</del>
                if match_html_tag(&chars, i, "del")
                    && let Some(close) = find_close_html_tag(&chars, i + 5, "del")
                {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let inner: String = chars[i + 5..close].iter().collect();
                    inlines.push(Inline::Strikeout(self.parse_inline(&inner)));
                    i = close + 6; // "</del>" is 6 chars
                    continue;
                }

                // <sup>...</sup>
                if match_html_tag(&chars, i, "sup")
                    && let Some(close) = find_close_html_tag(&chars, i + 5, "sup")
                {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let inner: String = chars[i + 5..close].iter().collect();
                    inlines.push(Inline::Superscript(self.parse_inline(&inner)));
                    i = close + 6; // "</sup>" is 6 chars
                    continue;
                }

                // <sub>...</sub>
                if match_html_tag(&chars, i, "sub")
                    && let Some(close) = find_close_html_tag(&chars, i + 5, "sub")
                {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let inner: String = chars[i + 5..close].iter().collect();
                    inlines.push(Inline::Subscript(self.parse_inline(&inner)));
                    i = close + 6; // "</sub>" is 6 chars
                    continue;
                }

                // <s>...</s>  (after <sub>/<sup> to avoid prefix conflict)
                if match_html_tag(&chars, i, "s")
                    && let Some(close) = find_close_html_tag(&chars, i + 3, "s")
                {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let inner: String = chars[i + 3..close].iter().collect();
                    inlines.push(Inline::Strikeout(self.parse_inline(&inner)));
                    i = close + 4; // "</s>" is 4 chars
                    continue;
                }

                // <u>...</u>
                if match_html_tag(&chars, i, "u")
                    && let Some(close) = find_close_html_tag(&chars, i + 3, "u")
                {
                    if !current_text.is_empty() {
                        inlines.push(Inline::Text(current_text.clone()));
                        current_text.clear();
                    }
                    let inner: String = chars[i + 3..close].iter().collect();
                    inlines.push(Inline::Underline(self.parse_inline(&inner)));
                    i = close + 4; // "</u>" is 4 chars
                    continue;
                }
            }

            // Regular character
            current_text.push(chars[i]);
            i += 1;
        }

        if !current_text.is_empty() {
            inlines.push(Inline::Text(current_text));
        }

        inlines
    }
}

/// Extract `name="value"` from a ref tag string.
fn extract_ref_name(tag: &str) -> Option<String> {
    for prefix in &["name=\"", "name='", "name =\"", "name ='"] {
        if let Some(pos) = tag.to_lowercase().find(prefix) {
            let quote = if prefix.ends_with('"') { '"' } else { '\'' };
            let val_start = pos + prefix.len();
            if let Some(val_end) = tag[val_start..].find(quote) {
                return Some(tag[val_start..val_start + val_end].to_string());
            }
        }
    }
    None
}
