use crate::ast::{Block, Diagnostic, Inline, PodDoc, Span};

/// Parse a POD string into a [`PodDoc`], returning diagnostics for any issues.
pub fn parse(input: &str) -> (PodDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    let blocks = p.parse_blocks();
    (PodDoc { blocks, span: Span::NONE }, vec![])
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    in_pod: bool,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { lines: input.lines().collect(), pos: 0, in_pod: false }
    }

    fn parse_blocks(&mut self) -> Vec<Block> {
        let mut nodes = Vec::new();

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            // Check for =pod or =head* to enter POD mode
            if line.starts_with("=pod") || line.starts_with("=head") {
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
                self.pos += 1;
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
            return Some(Block::Heading { level, inlines, span: Span::NONE });
        }

        // =over / =item / =back
        if line.starts_with("=over") {
            return Some(self.parse_list());
        }

        // =pod is just a marker, skip
        if line.starts_with("=pod") {
            return None;
        }

        // =begin / =end / =for - format-specific content
        if line.starts_with("=begin") || line.starts_with("=end") || line.starts_with("=for") {
            // Skip format-specific content for now
            return None;
        }

        // =encoding - metadata
        if line.starts_with("=encoding") {
            return None;
        }

        None
    }

    fn parse_list(&mut self) -> Block {
        let mut items = Vec::new();
        self.pos += 1; // Skip =over

        // Determine if it's ordered (numbered items) or not
        let mut is_ordered = false;

        while self.pos < self.lines.len() {
            let line = self.lines[self.pos];

            if line.starts_with("=back") {
                self.pos += 1;
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

                    if inner_line.trim().is_empty() {
                        self.pos += 1;
                        continue;
                    }

                    // Handle nested =over list inside an item.
                    if inner_line.starts_with("=over") {
                        item_blocks.push(self.parse_list());
                        continue;
                    }

                    // Other POD commands (=cut, =pod, =begin, =end, =for, etc.) — skip.
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
                    // For escape codes (E, X, Z, S), don't flush buffer - just accumulate
                    let is_escape = matches!(code, 'E' | 'X' | 'Z' | 'S');

                    if !is_escape && !current.is_empty() {
                        inlines.push(Inline::Text(current.clone(), Span::NONE));
                        current.clear();
                    }

                    match code {
                        'B' => {
                            let inner = parse_inline(&content);
                            inlines.push(Inline::Bold(inner, Span::NONE));
                        }
                        'I' | 'F' => {
                            // F<> (filename) is typically rendered as italic
                            let inner = parse_inline(&content);
                            inlines.push(Inline::Italic(inner, Span::NONE));
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
                            // Non-breaking spaces - just accumulate the text
                            current.push_str(&content);
                        }
                        'E' => {
                            // Escape: E<lt>, E<gt>, E<amp>, E<sol>, E<verbar>, etc.
                            let escaped = match content.as_str() {
                                "lt" => "<",
                                "gt" => ">",
                                "amp" => "&",
                                "sol" => "/",
                                "verbar" => "|",
                                "quot" => "\"",
                                "apos" => "'",
                                _ => {
                                    // Try numeric (E<0x201E> or E<8222>)
                                    if let Some(hex) = content.strip_prefix("0x")
                                        && let Ok(code) = u32::from_str_radix(hex, 16)
                                        && let Some(c) = char::from_u32(code)
                                    {
                                        current.push(c);
                                        i = end;
                                        continue;
                                    } else if let Ok(code) = content.parse::<u32>()
                                        && let Some(c) = char::from_u32(code)
                                    {
                                        current.push(c);
                                        i = end;
                                        continue;
                                    }
                                    ""
                                }
                            };
                            if !escaped.is_empty() {
                                current.push_str(escaped);
                            }
                            i = end;
                            continue;
                        }
                        'X' | 'Z' => {
                            // X<> is an index entry (invisible)
                            // Z<> is a null element (invisible)
                            i = end;
                            continue;
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
