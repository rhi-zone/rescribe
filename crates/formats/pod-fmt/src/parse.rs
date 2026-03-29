use crate::ast::{Block, DefinitionItem, Diagnostic, Inline, PodDoc, Span};

/// Parse a POD string into a [`PodDoc`], returning diagnostics for any issues.
pub fn parse(input: &str) -> (PodDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.parse_blocks();
    let diagnostics = std::mem::take(&mut p.diagnostics);
    (PodDoc { blocks, span: Span::NONE }, diagnostics)
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    in_pod: bool,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { lines: input.lines().collect(), pos: 0, in_pod: false, diagnostics: Vec::new() }
    }

    fn parse_blocks(&mut self) -> Vec<Block> {
        let mut nodes = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            // Check for =pod or =head* to enter POD mode
            if line.starts_with("=pod") || line.starts_with("=head") {
                self.in_pod = true;
            }

            // =over also enters POD mode
            if line.starts_with("=over") {
                self.in_pod = true;
            }

            // Check for =cut to exit POD mode
            if line.starts_with("=cut") {
                self.in_pod = false;
                self.pos += 1;
                continue;
            }

            // If not in POD mode and we haven't seen a command, skip
            if !self.in_pod && !line.starts_with('=') {
                self.pos += 1;
                continue;
            }

            // Process POD content
            if line.starts_with('=') {
                if let Some(block) = self.parse_command(line) {
                    nodes.push(block);
                }
                // parse_command may advance pos itself (for =over, =begin)
                // If not, we advance here.
                continue;
            }

            // Verbatim paragraph (starts with whitespace)
            if line.starts_with(' ') || line.starts_with('\t') {
                nodes.push(self.parse_verbatim());
                continue;
            }

            // Skip blank lines
            if line.trim().is_empty() {
                self.pos += 1;
                continue;
            }

            // Ordinary paragraph
            nodes.push(self.parse_paragraph());
        }

        nodes
    }

    fn parse_command(&mut self, line: &str) -> Option<Block> {
        // =head1 through =head6
        if let Some(rest) = line.strip_prefix("=head")
            && let Some(level_char) = rest.chars().next()
            && let Some(level) = level_char.to_digit(10)
            && (1..=6).contains(&level)
        {
            let title = rest.get(1..)?.trim();
            let inlines = parse_inline(title);
            self.pos += 1;
            return Some(Block::Heading { level, inlines, span: Span::NONE });
        }

        // =over / =item / =back
        if line.starts_with("=over") {
            return Some(self.parse_list());
        }

        // =pod is just a marker, skip
        if line.starts_with("=pod") {
            self.pos += 1;
            return None;
        }

        // =begin FORMAT ... =end FORMAT
        if line.starts_with("=begin") {
            return Some(self.parse_begin_end());
        }

        // =end without matching =begin
        if line.starts_with("=end") {
            self.diagnostics.push(Diagnostic::warning("=end without matching =begin", Span::NONE));
            self.pos += 1;
            return None;
        }

        // =for FORMAT text
        if line.starts_with("=for") {
            return Some(self.parse_for());
        }

        // =encoding ENC
        if let Some(rest) = line.strip_prefix("=encoding") {
            let encoding = rest.trim().to_string();
            self.pos += 1;
            return Some(Block::Encoding { encoding, span: Span::NONE });
        }

        // =back without matching =over
        if line.starts_with("=back") {
            self.diagnostics.push(Diagnostic::warning("=back without matching =over", Span::NONE));
            self.pos += 1;
            return None;
        }

        // =item outside =over
        if line.starts_with("=item") {
            self.diagnostics.push(Diagnostic::warning("=item outside =over", Span::NONE));
            self.pos += 1;
            return None;
        }

        // Unknown command
        self.pos += 1;
        None
    }

    fn parse_begin_end(&mut self) -> Block {
        let first_line = self.lines[self.pos];
        let format = first_line
            .strip_prefix("=begin")
            .unwrap_or("")
            .trim()
            .to_string();
        self.pos += 1;

        let mut content = String::new();
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];
            if line.starts_with("=end") {
                self.pos += 1;
                break;
            }
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(line);
            self.pos += 1;
        }

        Block::RawBlock { format, content, span: Span::NONE }
    }

    fn parse_for(&mut self) -> Block {
        let first_line = self.lines[self.pos];
        let rest = first_line.strip_prefix("=for").unwrap_or("").trim();
        // First word is format, rest is content
        let (format, content) = if let Some(space_pos) = rest.find(char::is_whitespace) {
            (rest[..space_pos].to_string(), rest[space_pos..].trim().to_string())
        } else {
            (rest.to_string(), String::new())
        };
        self.pos += 1;
        Block::ForBlock { format, content, span: Span::NONE }
    }

    fn parse_list(&mut self) -> Block {
        self.pos += 1; // Skip =over

        // Peek ahead to determine list type
        let mut is_ordered = false;
        let mut is_definition = false;
        for i in self.pos..self.lines.len() {
            let line = self.lines[i];
            if line.starts_with("=item") {
                let item_content = line.strip_prefix("=item").unwrap_or("").trim();
                if item_content.starts_with('*') {
                    // bullet list
                    break;
                } else if item_content.starts_with(|c: char| c.is_ascii_digit()) {
                    is_ordered = true;
                    break;
                } else if !item_content.is_empty() {
                    is_definition = true;
                    break;
                }
                break;
            }
            if line.starts_with("=back") || line.starts_with("=cut") {
                break;
            }
        }

        if is_definition {
            return self.parse_definition_list();
        }

        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.starts_with("=back") {
                self.pos += 1;
                break;
            }

            if line.starts_with("=cut") {
                break;
            }

            if line.starts_with("=item") {
                let item_content = line.strip_prefix("=item").unwrap_or("").trim();

                // Check if it's a numbered item
                if item_content.starts_with(|c: char| c.is_ascii_digit()) {
                    is_ordered = true;
                }

                // Collect item content from following paragraphs
                self.pos += 1;
                let mut item_blocks = Vec::new();

                // Parse the item marker itself if it has text after the number/bullet
                let marker_text = if let Some(stripped) = item_content.strip_prefix('*') {
                    stripped.trim()
                } else if let Some(dot_pos) = item_content.find('.') {
                    item_content.get(dot_pos + 1..).unwrap_or("").trim()
                } else {
                    item_content
                };

                if !marker_text.is_empty() {
                    let inlines = parse_inline(marker_text);
                    item_blocks.push(Block::Paragraph { inlines, span: Span::NONE });
                }

                // Collect following paragraphs until next =item or =back
                while self.pos < self.lines.len() {
                    let inner_line = self.lines[self.pos];

                    if inner_line.starts_with("=item") || inner_line.starts_with("=back") {
                        break;
                    }

                    if inner_line.starts_with("=cut") {
                        break;
                    }

                    if inner_line.trim().is_empty() {
                        self.pos += 1;
                        continue;
                    }

                    // Handle nested =over list inside an item.
                    if inner_line.starts_with("=over") {
                        item_blocks.push(self.parse_list());
                        continue;
                    }

                    // Other POD commands (=pod, =begin, =end, =for, etc.) — skip.
                    if inner_line.starts_with('=') {
                        self.pos += 1;
                        continue;
                    }

                    if inner_line.starts_with(' ') || inner_line.starts_with('\t') {
                        item_blocks.push(self.parse_verbatim());
                    } else {
                        item_blocks.push(self.parse_paragraph());
                    }
                }

                items.push(item_blocks);
            } else {
                self.pos += 1;
            }
        }

        Block::List { ordered: is_ordered, items, span: Span::NONE }
    }

    fn parse_definition_list(&mut self) -> Block {
        let mut items = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.starts_with("=back") {
                self.pos += 1;
                break;
            }

            if line.starts_with("=cut") {
                break;
            }

            if line.starts_with("=item") {
                let term_text = line.strip_prefix("=item").unwrap_or("").trim();
                let term = parse_inline(term_text);
                self.pos += 1;

                let mut desc_blocks = Vec::new();

                while self.pos < self.lines.len() {
                    let inner_line = self.lines[self.pos];

                    if inner_line.starts_with("=item") || inner_line.starts_with("=back") {
                        break;
                    }

                    if inner_line.starts_with("=cut") {
                        break;
                    }

                    if inner_line.trim().is_empty() {
                        self.pos += 1;
                        continue;
                    }

                    if inner_line.starts_with("=over") {
                        desc_blocks.push(self.parse_list());
                        continue;
                    }

                    if inner_line.starts_with('=') {
                        self.pos += 1;
                        continue;
                    }

                    if inner_line.starts_with(' ') || inner_line.starts_with('\t') {
                        desc_blocks.push(self.parse_verbatim());
                    } else {
                        desc_blocks.push(self.parse_paragraph());
                    }
                }

                items.push(DefinitionItem { term, desc: desc_blocks });
            } else {
                self.pos += 1;
            }
        }

        Block::DefinitionList { items, span: Span::NONE }
    }

    fn parse_verbatim(&mut self) -> Block {
        let mut content = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            // Verbatim ends when we hit a non-indented line
            if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
                break;
            }

            // Remove one level of indentation (typically 4 spaces or 1 tab)
            let stripped = if let Some(rest) = line.strip_prefix("    ") {
                rest
            } else if let Some(rest) = line.strip_prefix('\t') {
                rest
            } else {
                line
            };

            content.push_str(stripped);
            content.push('\n');
            self.pos += 1;
        }

        Block::CodeBlock { content: content.trim_end().to_string(), span: Span::NONE }
    }

    fn parse_paragraph(&mut self) -> Block {
        let mut text = String::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            // Paragraph ends on blank line, command, or verbatim start
            if line.trim().is_empty()
                || line.starts_with('=')
                || line.starts_with(' ')
                || line.starts_with('\t')
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
}

/// Parse POD formatting codes into inline elements.
pub fn parse_inline(text: &str) -> Vec<Inline> {
    let mut inlines = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Check for formatting codes: X<...>
        if i + 1 < chars.len() && chars[i + 1] == '<' {
            let code = chars[i];
            if matches!(code, 'B' | 'I' | 'C' | 'L' | 'F' | 'S' | 'U' | 'E' | 'X' | 'Z') {
                // Check for double angle brackets C<< ... >>
                let (content, end_pos) = if i + 2 < chars.len() && chars[i + 2] == '<' {
                    // Double angle brackets with space padding - start from first <
                    find_double_bracket_content(&chars, i + 1)
                } else {
                    // Single angle bracket
                    find_single_bracket_content(&chars, i + 2)
                };

                if let Some((content, end)) = content.zip(end_pos) {
                    // Flush text buffer before structured inlines
                    if !current.is_empty()
                        && !matches!(code, 'E')
                    {
                        inlines.push(Inline::Text(current.clone(), Span::NONE));
                        current.clear();
                    }

                    match code {
                        'B' => {
                            let inner = parse_inline(&content);
                            inlines.push(Inline::Bold(inner, Span::NONE));
                        }
                        'I' => {
                            let inner = parse_inline(&content);
                            inlines.push(Inline::Italic(inner, Span::NONE));
                        }
                        'F' => {
                            let inner = parse_inline(&content);
                            inlines.push(Inline::Filename(inner, Span::NONE));
                        }
                        'U' => {
                            let inner = parse_inline(&content);
                            inlines.push(Inline::Underline(inner, Span::NONE));
                        }
                        'C' => {
                            inlines.push(Inline::Code(content, Span::NONE));
                        }
                        'L' => {
                            // Link: L<name> or L<text|url> or L<text|name/"section">
                            let (url, label) = if let Some(pipe_pos) = content.find('|') {
                                (
                                    content[pipe_pos + 1..].to_string(),
                                    content[..pipe_pos].to_string(),
                                )
                            } else {
                                (content.clone(), content)
                            };
                            inlines.push(Inline::Link { url, label, span: Span::NONE });
                        }
                        'S' => {
                            let inner = parse_inline(&content);
                            inlines.push(Inline::NonBreaking(inner, Span::NONE));
                        }
                        'E' => {
                            // Escape: E<lt>, E<gt>, E<amp>, E<sol>, E<verbar>, etc.
                            let resolved = resolve_entity(&content);
                            if let Some(s) = resolved {
                                current.push_str(&s);
                            }
                            i = end;
                            continue;
                        }
                        'X' => {
                            inlines.push(Inline::IndexEntry(content, Span::NONE));
                        }
                        'Z' => {
                            inlines.push(Inline::Null(Span::NONE));
                        }
                        _ => {}
                    }

                    i = end;
                    continue;
                }
            }
        }

        current.push(chars[i]);
        i += 1;
    }

    if !current.is_empty() {
        inlines.push(Inline::Text(current, Span::NONE));
    }

    inlines
}

/// Resolve a POD entity name to its string value.
fn resolve_entity(name: &str) -> Option<String> {
    match name {
        "lt" => Some("<".into()),
        "gt" => Some(">".into()),
        "amp" => Some("&".into()),
        "sol" => Some("/".into()),
        "verbar" => Some("|".into()),
        "quot" => Some("\"".into()),
        "apos" => Some("'".into()),
        _ => {
            // Try hex (E<0x263A>)
            if let Some(hex) = name.strip_prefix("0x")
                && let Ok(code) = u32::from_str_radix(hex, 16)
                && let Some(c) = char::from_u32(code)
            {
                return Some(c.to_string());
            }
            // Try decimal (E<169>)
            if let Ok(code) = name.parse::<u32>()
                && let Some(c) = char::from_u32(code)
            {
                return Some(c.to_string());
            }
            None
        }
    }
}

fn find_single_bracket_content(chars: &[char], start: usize) -> (Option<String>, Option<usize>) {
    let mut i = start;
    let mut content = String::new();
    let mut depth = 1;

    while i < chars.len() {
        if chars[i] == '<' {
            depth += 1;
            content.push(chars[i]);
        } else if chars[i] == '>' {
            depth -= 1;
            if depth == 0 {
                return (Some(content), Some(i + 1));
            }
            content.push(chars[i]);
        } else {
            content.push(chars[i]);
        }
        i += 1;
    }

    (None, None)
}

fn find_double_bracket_content(chars: &[char], start: usize) -> (Option<String>, Option<usize>) {
    // Count opening brackets
    let mut bracket_count = 0;
    let mut i = start;
    while i < chars.len() && chars[i] == '<' {
        bracket_count += 1;
        i += 1;
    }

    // Skip leading space
    if i < chars.len() && chars[i] == ' ' {
        i += 1;
    }

    // Find closing brackets (same count followed by optional space)
    let mut content = String::new();
    while i < chars.len() {
        // Check for closing sequence: space + N closing brackets
        if chars[i] == ' ' && i + bracket_count < chars.len() {
            let mut all_closing = true;
            for j in 0..bracket_count {
                if chars[i + 1 + j] != '>' {
                    all_closing = false;
                    break;
                }
            }
            if all_closing {
                return (Some(content), Some(i + 1 + bracket_count));
            }
        }

        // Also check for direct closing (without space) for simpler cases
        if chars[i] == '>' {
            let mut closing_count = 0;
            let mut j = i;
            while j < chars.len() && chars[j] == '>' {
                closing_count += 1;
                j += 1;
            }
            if closing_count >= bracket_count {
                return (Some(content.trim_end().to_string()), Some(i + bracket_count));
            }
        }

        content.push(chars[i]);
        i += 1;
    }

    (None, None)
}
