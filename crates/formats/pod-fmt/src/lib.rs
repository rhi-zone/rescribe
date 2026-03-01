//! POD (Plain Old Documentation) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-pod` and `rescribe-write-pod` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct PodError(pub String);

impl std::fmt::Display for PodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "POD error: {}", self.0)
    }
}

impl std::error::Error for PodError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed POD document.
#[derive(Debug, Clone, Default)]
pub struct PodDoc {
    pub blocks: Vec<Block>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Heading {
        level: u32,
        inlines: Vec<Inline>,
    },
    Paragraph {
        inlines: Vec<Inline>,
    },
    CodeBlock {
        content: String,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Bold(Vec<Inline>),
    Italic(Vec<Inline>),
    Underline(Vec<Inline>),
    Code(String),
    Link { url: String, label: String },
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a POD string into a [`PodDoc`].
pub fn parse(input: &str) -> Result<PodDoc, PodError> {
    let mut p = Parser::new(input);
    let blocks = p.parse();
    Ok(PodDoc { blocks })
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    pos: usize,
    in_pod: bool,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            lines: input.lines().collect(),
            pos: 0,
            in_pod: false,
        }
    }

    fn parse(&mut self) -> Vec<Block> {
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
            return Some(Block::Heading { level, inlines });
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
                    item_blocks.push(Block::Paragraph { inlines });
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

        Block::List {
            ordered: is_ordered,
            items,
        }
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

        Block::CodeBlock {
            content: content.trim_end().to_string(),
        }
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
        Block::Paragraph { inlines }
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
            if matches!(
                code,
                'B' | 'I' | 'C' | 'L' | 'F' | 'S' | 'U' | 'E' | 'X' | 'Z'
            ) {
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
                        inlines.push(Inline::Text(current.clone()));
                        current.clear();
                    }

                    match code {
                        'B' => {
                            let inner = parse_inline(&content);
                            inlines.push(Inline::Bold(inner));
                        }
                        'I' | 'F' => {
                            // F<> (filename) is typically rendered as italic
                            let inner = parse_inline(&content);
                            inlines.push(Inline::Italic(inner));
                        }
                        'U' => {
                            let inner = parse_inline(&content);
                            inlines.push(Inline::Underline(inner));
                        }
                        'C' => {
                            inlines.push(Inline::Code(content));
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
                            inlines.push(Inline::Link { url, label });
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
        inlines.push(Inline::Text(current));
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
                return (
                    Some(content.trim_end().to_string()),
                    Some(i + bracket_count),
                );
            }
        }

        content.push(chars[i]);
        i += 1;
    }

    (None, None)
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a POD string from a [`PodDoc`].
pub fn build(doc: &PodDoc) -> String {
    let mut ctx = BuildContext::new();
    ctx.write("=pod\n\n");
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }
    ctx.write("=cut\n");
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
        Block::Heading { level, inlines } => {
            ctx.write(&format!("=head{} ", level));
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Paragraph { inlines } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content } => {
            // Verbatim paragraphs need 4-space indentation
            for line in content.lines() {
                ctx.write("    ");
                ctx.write(line);
                ctx.write("\n");
            }
            ctx.write("\n");
        }

        Block::List { ordered, items } => {
            ctx.write("=over 4\n\n");

            let mut num = 1;
            for item_blocks in items {
                if *ordered {
                    ctx.write(&format!("=item {}. ", num));
                    num += 1;
                } else {
                    ctx.write("=item * ");
                }

                // Emit first paragraph inline with =item
                let mut first = true;
                for item_block in item_blocks {
                    if first && matches!(item_block, Block::Paragraph { .. }) {
                        if let Block::Paragraph { inlines } = item_block {
                            build_inlines(inlines, ctx);
                            ctx.write("\n\n");
                        }
                        first = false;
                    } else {
                        build_block(item_block, ctx);
                    }
                }
            }

            ctx.write("=back\n\n");
        }
    }
}

fn build_inlines(inlines: &[Inline], ctx: &mut BuildContext) {
    for inline in inlines {
        build_inline(inline, ctx);
    }
}

fn build_inline(inline: &Inline, ctx: &mut BuildContext) {
    match inline {
        Inline::Text(s) => {
            // Escape < and > in plain text
            let escaped = s.replace('<', "E<lt>").replace('>', "E<gt>");
            ctx.write(&escaped);
        }

        Inline::Bold(children) => {
            ctx.write("B<");
            build_inlines(children, ctx);
            ctx.write(">");
        }

        Inline::Italic(children) => {
            ctx.write("I<");
            build_inlines(children, ctx);
            ctx.write(">");
        }

        Inline::Underline(children) => {
            ctx.write("U<");
            build_inlines(children, ctx);
            ctx.write(">");
        }

        Inline::Code(content) => {
            // Use double brackets if content contains > or <
            if content.contains('>') || content.contains('<') {
                ctx.write("C<< ");
                ctx.write(content);
                ctx.write(" >>");
            } else {
                ctx.write("C<");
                ctx.write(content);
                ctx.write(">");
            }
        }

        Inline::Link { url, label } => {
            if label.is_empty() || label == url {
                ctx.write("L<");
                ctx.write(url);
                ctx.write(">");
            } else {
                ctx.write("L<");
                ctx.write(label);
                ctx.write("|");
                ctx.write(url);
                ctx.write(">");
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heading() {
        let doc = parse("=head1 NAME\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Heading { .. }));
    }

    #[test]
    fn test_parse_heading_level2() {
        let doc = parse("=head2 DESCRIPTION\n").unwrap();
        if let Block::Heading { level, .. } = &doc.blocks[0] {
            assert_eq!(*level, 2);
        } else {
            panic!("expected heading");
        }
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse("=pod\n\nThis is a paragraph.\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse("=pod\n\nThis is B<bold> text.\n").unwrap();
        if let Block::Paragraph { inlines } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Bold(_))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse("=pod\n\nThis is I<italic> text.\n").unwrap();
        if let Block::Paragraph { inlines } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Italic(_))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_code() {
        let doc = parse("=pod\n\nUse C<my $var> here.\n").unwrap();
        if let Block::Paragraph { inlines } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Code(_))));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_link() {
        let doc = parse("=pod\n\nSee L<perlpod> for details.\n").unwrap();
        if let Block::Paragraph { inlines } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Link { .. })));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_parse_verbatim() {
        let doc = parse("=pod\n\n    print \"Hello\";\n").unwrap();
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_list() {
        let doc = parse("=over\n\n=item * First\n\n=item * Second\n\n=back\n").unwrap();
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 2);
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_parse_escape() {
        let doc = parse("=pod\n\nE<lt>tag E<gt>\n").unwrap();
        if let Block::Paragraph { inlines } = &doc.blocks[0] {
            let text = inlines
                .iter()
                .filter_map(|i| {
                    if let Inline::Text(s) = i {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .collect::<String>();
            assert!(text.contains('<'));
            assert!(text.contains('>'));
        }
    }

    #[test]
    fn test_parse_double_brackets() {
        let doc = parse("=pod\n\nC<< $a <=> $b >>\n").unwrap();
        if let Block::Paragraph { inlines } = &doc.blocks[0] {
            let code = inlines.iter().find(|i| matches!(i, Inline::Code(_)));
            assert!(code.is_some());
            if let Some(Inline::Code(content)) = code {
                assert!(content.contains("<=>"));
            }
        }
    }

    #[test]
    fn test_build_heading() {
        let doc = PodDoc {
            blocks: vec![Block::Heading {
                level: 1,
                inlines: vec![Inline::Text("NAME".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("=head1 NAME"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_bold() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Bold(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("B<bold>"));
    }

    #[test]
    fn test_build_italic() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Italic(vec![Inline::Text("italic".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("I<italic>"));
    }

    #[test]
    fn test_build_code() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("$var".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("C<$var>"));
    }

    #[test]
    fn test_build_code_with_angle_brackets() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("$a <=> $b".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("C<< $a <=> $b >>"));
    }

    #[test]
    fn test_build_link() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "perlpod".into(),
                    label: "perlpod".into(),
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("L<perlpod>"));
    }

    #[test]
    fn test_build_link_with_label() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "perlpod".into(),
                    label: "documentation".into(),
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("L<documentation|perlpod>"));
    }

    #[test]
    fn test_build_list() {
        let doc = PodDoc {
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("one".into())],
                    }],
                    vec![Block::Paragraph {
                        inlines: vec![Inline::Text("two".into())],
                    }],
                ],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("=over"));
        assert!(out.contains("=item * one"));
        assert!(out.contains("=item * two"));
        assert!(out.contains("=back"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = PodDoc {
            blocks: vec![Block::CodeBlock {
                content: "print 'Hello';".into(),
            }],
        };
        let out = build(&doc);
        assert!(out.contains("    print 'Hello';"));
    }

    #[test]
    fn test_build_pod_cut() {
        let doc = PodDoc {
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Content".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.starts_with("=pod"));
        assert!(out.ends_with("=cut\n"));
    }
}
