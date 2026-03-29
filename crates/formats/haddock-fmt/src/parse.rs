use crate::ast::{Block, Diagnostic, HaddockDoc, Inline, Span};

/// Parse a Haddock string into a [`HaddockDoc`].
pub fn parse(input: &str) -> (HaddockDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    (HaddockDoc { blocks, span: Span::NONE }, Vec::new())
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            lines: input.lines().collect(),
            pos: 0,
        }
    }

    fn parse(&mut self) -> Vec<Block> {
        let mut blocks = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // Doc-test: >>> expression
            if line.starts_with(">>> ") {
                blocks.push(self.parse_doctest());
                continue;
            }

            // Property annotation: @since, @deprecated, @param, @returns
            if let Some(block) = self.try_parse_property(line) {
                blocks.push(block);
                self.pos += 1;
                continue;
            }

            // @-delimited code block (line is exactly "@" or "@ " to start)
            if line.trim() == "@" {
                blocks.push(self.parse_at_code_block());
                continue;
            }

            // Code block (bird-track style: > prefix)
            if line.starts_with("> ") {
                blocks.push(self.parse_code_block());
                continue;
            }

            // Heading (= to ====)
            if let Some(block) = self.try_parse_heading(line) {
                blocks.push(block);
                self.pos += 1;
                continue;
            }

            // Definition list [term]
            if line.trim_start().starts_with('[')
                && let Some(block) = self.parse_definition_list()
            {
                blocks.push(block);
                continue;
            }

            // Unordered list *
            if line.trim_start().starts_with("* ") {
                blocks.push(self.parse_unordered_list());
                continue;
            }

            // Ordered list (1) or 1.
            if self.is_ordered_list_item(line) {
                blocks.push(self.parse_ordered_list());
                continue;
            }

            // Regular paragraph (or unrecognised line — always advance pos).
            let prev_pos = self.pos;
            blocks.push(self.parse_paragraph());
            // Safety: if parse_paragraph consumed nothing, advance past the line
            // to prevent an infinite loop on lines that look like block markers
            // when trimmed but not when tested against `line.starts_with`.
            if self.pos == prev_pos {
                self.pos += 1;
            }
        }

        blocks
    }

    fn try_parse_heading(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim_start();

        // Count leading = signs
        let level = trimmed.chars().take_while(|&c| c == '=').count();

        if level > 0 && level <= 6 {
            let rest = trimmed[level..].trim();
            // Remove trailing = if present
            let content = rest.trim_end_matches('=').trim();
            let inlines = parse_inline(content);

            return Some(Block::Heading {
                level: level as u8,
                inlines,
                span: Span::NONE,
            });
        }
        None
    }

    fn parse_code_block(&mut self) -> Block {
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !line.starts_with("> ") && !line.trim().is_empty() {
                break;
            }

            if let Some(code_line) = line.strip_prefix("> ") {
                content.push_str(code_line);
                content.push('\n');
            }
            self.pos += 1;
        }

        Block::CodeBlock {
            content: content.trim_end().to_string(),
            span: Span::NONE,
        }
    }

    fn parse_at_code_block(&mut self) -> Block {
        // Skip opening @
        self.pos += 1;
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.trim() == "@" {
                self.pos += 1;
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.pos += 1;
        }

        Block::AtCodeBlock {
            content,
            span: Span::NONE,
        }
    }

    fn parse_doctest(&mut self) -> Block {
        let line = self.lines[self.pos];
        let expression = line[4..].to_string(); // skip ">>> "
        self.pos += 1;

        // Check for result line (non-empty, not starting with >>>)
        let result = if self.pos < self.lines.len() {
            let next = self.lines[self.pos];
            if !next.trim().is_empty() && !next.starts_with(">>> ") && !next.starts_with('=') {
                self.pos += 1;
                Some(next.to_string())
            } else {
                None
            }
        } else {
            None
        };

        Block::DocTest {
            expression,
            result,
            span: Span::NONE,
        }
    }

    fn try_parse_property(&self, line: &str) -> Option<Block> {
        let trimmed = line.trim();

        // @since VERSION
        if let Some(rest) = trimmed.strip_prefix("@since ") {
            return Some(Block::Property {
                key: "since".to_string(),
                name: None,
                description: vec![Inline::Text(rest.trim().to_string(), Span::NONE)],
                span: Span::NONE,
            });
        }
        // @deprecated REASON
        if trimmed == "@deprecated" || trimmed.starts_with("@deprecated ") {
            let desc_text = trimmed
                .strip_prefix("@deprecated")
                .unwrap_or("")
                .trim();
            let description = if desc_text.is_empty() {
                Vec::new()
            } else {
                parse_inline(desc_text)
            };
            return Some(Block::Property {
                key: "deprecated".to_string(),
                name: None,
                description,
                span: Span::NONE,
            });
        }
        // @param NAME DESCRIPTION
        if let Some(rest) = trimmed.strip_prefix("@param ") {
            let rest = rest.trim();
            let (name, desc) = if let Some(space) = rest.find(' ') {
                (rest[..space].to_string(), rest[space + 1..].trim())
            } else {
                (rest.to_string(), "")
            };
            let description = if desc.is_empty() {
                Vec::new()
            } else {
                parse_inline(desc)
            };
            return Some(Block::Property {
                key: "param".to_string(),
                name: Some(name),
                description,
                span: Span::NONE,
            });
        }
        // @returns DESCRIPTION
        if let Some(rest) = trimmed.strip_prefix("@returns ") {
            return Some(Block::Property {
                key: "returns".to_string(),
                name: None,
                description: parse_inline(rest.trim()),
                span: Span::NONE,
            });
        }

        None
    }

    fn is_ordered_list_item(&self, line: &str) -> bool {
        let trimmed = line.trim_start();
        // (1) style
        if trimmed.starts_with('(')
            && let Some(close) = trimmed.find(')')
        {
            let num = &trimmed[1..close];
            if num.chars().all(|c| c.is_ascii_digit()) && !num.is_empty() {
                return true;
            }
        }
        // 1. style
        let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !digits.is_empty() && trimmed[digits.len()..].starts_with(". ") {
            return true;
        }
        false
    }

    fn parse_unordered_list(&mut self) -> Block {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if !trimmed.starts_with("* ") {
                break;
            }

            let content = trimmed[2..].trim();
            let inlines = parse_inline(content);
            items.push(inlines);
            self.pos += 1;
        }

        Block::UnorderedList { items, span: Span::NONE }
    }

    fn parse_ordered_list(&mut self) -> Block {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if !self.is_ordered_list_item(line) {
                break;
            }

            let trimmed = line.trim_start();
            // (N) style
            if trimmed.starts_with('(')
                && let Some(close) = trimmed.find(')')
            {
                let content = trimmed[close + 1..].trim();
                let inlines = parse_inline(content);
                items.push(inlines);
            } else {
                // N. style
                let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
                let content = trimmed[digits.len() + 2..].trim(); // skip "N. "
                let inlines = parse_inline(content);
                items.push(inlines);
            }
            self.pos += 1;
        }

        Block::OrderedList { items, span: Span::NONE }
    }

    fn parse_definition_list(&mut self) -> Option<Block> {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            let trimmed = line.trim_start();

            if !trimmed.starts_with('[') {
                break;
            }

            // Find closing bracket
            if let Some(close) = trimmed.find(']') {
                let term = &trimmed[1..close];
                let desc = trimmed[close + 1..].trim();

                let term_inlines = parse_inline(term);
                let desc_inlines = parse_inline(desc);

                items.push((term_inlines, desc_inlines));
            }
            self.pos += 1;
        }

        if items.is_empty() {
            None
        } else {
            Some(Block::DefinitionList { items, span: Span::NONE })
        }
    }

    fn parse_paragraph(&mut self) -> Block {
        let mut text = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.trim().is_empty() {
                break;
            }

            // Check for block elements
            let trimmed = line.trim_start();
            if trimmed.starts_with('=')
                || trimmed.starts_with("* ")
                || trimmed.starts_with('[')
                || trimmed.starts_with("> ")
                || trimmed.starts_with(">>> ")
                || trimmed == "@"
                || self.is_ordered_list_item(line)
                || self.is_property_line(trimmed)
            {
                break;
            }

            if !text.is_empty() {
                text.push(' ');
            }
            text.push_str(line.trim());
            self.pos += 1;
        }

        let inlines = parse_inline(&text);
        Block::Paragraph { inlines, span: Span::NONE }
    }

    fn is_property_line(&self, trimmed: &str) -> bool {
        trimmed.starts_with("@since ")
            || trimmed == "@deprecated"
            || trimmed.starts_with("@deprecated ")
            || trimmed.starts_with("@param ")
            || trimmed.starts_with("@returns ")
    }
}

pub(crate) fn parse_inline(text: &str) -> Vec<Inline> {
    let mut inlines = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Inline code @...@
        if chars[i] == '@'
            && i + 1 < chars.len()
            && let Some((end, content)) = find_closing(&chars, i + 1, '@')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            inlines.push(Inline::Code(content, Span::NONE));
            i = end + 1;
            continue;
        }

        // Backtick inline code `...`
        if chars[i] == '`'
            && i + 1 < chars.len()
            && let Some((end, content)) = find_closing(&chars, i + 1, '`')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            inlines.push(Inline::Code(content, Span::NONE));
            i = end + 1;
            continue;
        }

        // Bold __...__
        if chars[i] == '_'
            && i + 1 < chars.len()
            && chars[i + 1] == '_'
            && i + 2 < chars.len()
            && let Some((end, content)) = find_double_closing(&chars, i + 2, '_')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Strong(inner, Span::NONE));
            i = end + 2;
            continue;
        }

        // Italic /.../ (but not //)
        if chars[i] == '/'
            && i + 1 < chars.len()
            && chars[i + 1] != '/'
            && (i == 0 || !chars[i - 1].is_alphanumeric())
            && let Some((end, content)) = find_closing(&chars, i + 1, '/')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            let inner = parse_inline(&content);
            inlines.push(Inline::Emphasis(inner, Span::NONE));
            i = end + 1;
            continue;
        }

        // Module reference "Module"
        if chars[i] == '"'
            && i + 1 < chars.len()
        {
            // Try "text"<url> link first
            if let Some((end, link_text, url)) = parse_haddock_link(&chars, i) {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                inlines.push(Inline::Link {
                    url,
                    text: link_text,
                    span: Span::NONE,
                });
                i = end;
                continue;
            }
            // Try module reference "Module.Name"
            if let Some((end, module)) = parse_module_ref(&chars, i) {
                if !current.is_empty() {
                    inlines.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                inlines.push(Inline::ModuleLink {
                    module,
                    span: Span::NONE,
                });
                i = end;
                continue;
            }
        }

        // Identifier reference '...'
        if chars[i] == '\''
            && i + 1 < chars.len()
            && let Some((end, content)) = find_closing(&chars, i + 1, '\'')
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            inlines.push(Inline::Code(content, Span::NONE));
            i = end + 1;
            continue;
        }

        // Raw URL <url>
        if chars[i] == '<'
            && let Some((end, url)) = parse_raw_url(&chars, i)
        {
            if !current.is_empty() {
                inlines.push(Inline::Text(current.clone(), Span::NONE));
                current.clear();
            }
            inlines.push(Inline::Link {
                url: url.clone(),
                text: url,
                span: Span::NONE,
            });
            i = end;
            continue;
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        inlines.push(Inline::Text(current, Span::NONE));
    }

    inlines
}

fn find_closing(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i < chars.len() {
        if chars[i] == marker {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn find_double_closing(chars: &[char], start: usize, marker: char) -> Option<(usize, String)> {
    let mut i = start;
    let mut content = String::new();

    while i + 1 < chars.len() {
        if chars[i] == marker && chars[i + 1] == marker {
            return Some((i, content));
        }
        content.push(chars[i]);
        i += 1;
    }
    None
}

fn parse_haddock_link(chars: &[char], start: usize) -> Option<(usize, String, String)> {
    // "text"<url>
    if chars[start] != '"' {
        return None;
    }

    let mut i = start + 1;
    let mut link_text = String::new();

    // Find closing "
    while i < chars.len() && chars[i] != '"' {
        link_text.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '"' {
        return None;
    }
    i += 1; // skip "

    // Must be followed by <
    if i >= chars.len() || chars[i] != '<' {
        return None;
    }
    i += 1; // skip <

    // Collect URL until >
    let mut url = String::new();
    while i < chars.len() && chars[i] != '>' {
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '>' {
        return None;
    }
    i += 1; // skip >

    Some((i, link_text, url))
}

fn parse_module_ref(chars: &[char], start: usize) -> Option<(usize, String)> {
    // "Module.Name" — must start with uppercase after the quote
    if chars[start] != '"' {
        return None;
    }

    let mut i = start + 1;
    let mut module = String::new();

    // Must start with uppercase letter (module names)
    if i >= chars.len() || !chars[i].is_uppercase() {
        return None;
    }

    while i < chars.len() && chars[i] != '"' {
        module.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '"' {
        return None;
    }

    // Next char must NOT be '<' (that's a link)
    if i + 1 < chars.len() && chars[i + 1] == '<' {
        return None;
    }

    i += 1; // skip closing "

    // Validate: module name must be a valid Haskell module (letters, digits, dots, underscores)
    if module.is_empty()
        || !module.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '_')
    {
        return None;
    }

    Some((i, module))
}

fn parse_raw_url(chars: &[char], start: usize) -> Option<(usize, String)> {
    // <url>
    if chars[start] != '<' {
        return None;
    }

    let mut i = start + 1;
    let mut url = String::new();

    while i < chars.len() && chars[i] != '>' {
        url.push(chars[i]);
        i += 1;
    }

    if i >= chars.len() || chars[i] != '>' {
        return None;
    }
    i += 1;

    // Basic URL validation
    if url.starts_with("http://") || url.starts_with("https://") || url.contains('@') {
        Some((i, url))
    } else {
        None
    }
}
