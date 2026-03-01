//! Texinfo parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-texinfo` and `rescribe-write-texinfo` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct TexinfoError(pub String);

impl std::fmt::Display for TexinfoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Texinfo error: {}", self.0)
    }
}

impl std::error::Error for TexinfoError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed Texinfo document.
#[derive(Debug, Clone, Default)]
pub struct TexinfoDoc {
    pub title: Option<String>,
    pub blocks: Vec<Block>,
}

/// Block-level element.
#[derive(Debug, Clone)]
pub enum Block {
    Heading {
        level: u8,
        inlines: Vec<Inline>,
    },
    Paragraph {
        inlines: Vec<Inline>,
    },
    CodeBlock {
        content: String,
    },
    Blockquote {
        children: Vec<Block>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Inline>>,
    },
    DefinitionList {
        items: Vec<(Vec<Inline>, Vec<Block>)>,
    },
    HorizontalRule,
}

/// Inline element.
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Code(String),
    Link { url: String, children: Vec<Inline> },
    Superscript(Vec<Inline>),
    Subscript(Vec<Inline>),
    LineBreak,
    SoftBreak,
    FootnoteDef { content: Vec<Inline> },
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a Texinfo string into a [`TexinfoDoc`].
pub fn parse(input: &str) -> Result<TexinfoDoc, TexinfoError> {
    let mut p = Parser::new(input);
    p.parse()?;
    Ok(TexinfoDoc {
        title: p.title,
        blocks: p.blocks,
    })
}

struct Parser<'a> {
    input: &'a str,
    title: Option<String>,
    blocks: Vec<Block>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            title: None,
            blocks: Vec::new(),
        }
    }

    fn parse(&mut self) -> Result<(), TexinfoError> {
        let lines: Vec<&str> = self.input.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim_start();

            // Skip comments
            if line.starts_with("@c ") || line.starts_with("@comment ") || line == "@c" {
                i += 1;
                continue;
            }

            // Handle @settitle
            if let Some(title) = line.strip_prefix("@settitle ") {
                self.title = Some(title.trim().to_string());
                i += 1;
                continue;
            }

            // Skip directives we don't process
            if line.starts_with("@set ")
                || line.starts_with("@clear ")
                || line.starts_with("@include ")
                || line.starts_with("@setfilename ")
                || line.starts_with("@copying")
                || line.starts_with("@end copying")
                || line.starts_with("@titlepage")
                || line.starts_with("@end titlepage")
                || line.starts_with("@contents")
                || line.starts_with("@shortcontents")
                || line.starts_with("@summarycontents")
                || line.starts_with("@top")
                || line.starts_with("@bye")
                || line.starts_with("@dircategory")
                || line.starts_with("@direntry")
                || line.starts_with("@end direntry")
                || line.starts_with("\\input ")
            {
                i += 1;
                continue;
            }

            // Handle node definitions (skip the @node line itself)
            if line.starts_with("@node ") {
                i += 1;
                continue;
            }

            // Handle headings
            if let Some(rest) = line.strip_prefix("@chapter ") {
                self.blocks.push(Block::Heading {
                    level: 1,
                    inlines: self.parse_inline(rest.trim()),
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@unnumbered ") {
                self.blocks.push(Block::Heading {
                    level: 1,
                    inlines: self.parse_inline(rest.trim()),
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@appendix ") {
                self.blocks.push(Block::Heading {
                    level: 1,
                    inlines: self.parse_inline(rest.trim()),
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@section ") {
                self.blocks.push(Block::Heading {
                    level: 2,
                    inlines: self.parse_inline(rest.trim()),
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@unnumberedsec ") {
                self.blocks.push(Block::Heading {
                    level: 2,
                    inlines: self.parse_inline(rest.trim()),
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@appendixsec ") {
                self.blocks.push(Block::Heading {
                    level: 2,
                    inlines: self.parse_inline(rest.trim()),
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@subsection ") {
                self.blocks.push(Block::Heading {
                    level: 3,
                    inlines: self.parse_inline(rest.trim()),
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@subsubsection ") {
                self.blocks.push(Block::Heading {
                    level: 4,
                    inlines: self.parse_inline(rest.trim()),
                });
                i += 1;
                continue;
            }

            // Handle lists
            if line.starts_with("@itemize") || line.starts_with("@enumerate") {
                let ordered = line.starts_with("@enumerate");
                let (list_block, end_line) = self.parse_list(&lines, i, ordered);
                self.blocks.push(list_block);
                i = end_line;
                continue;
            }

            // Handle definition lists (@table)
            if line.starts_with("@table") {
                let (def_list_block, end_line) = self.parse_definition_list(&lines, i);
                self.blocks.push(def_list_block);
                i = end_line;
                continue;
            }

            // Handle code blocks
            if line.starts_with("@example") || line.starts_with("@verbatim") {
                let end_marker = if line.starts_with("@example") {
                    "@end example"
                } else {
                    "@end verbatim"
                };
                let (code_block, end_line) = self.parse_code_block(&lines, i, end_marker);
                self.blocks.push(code_block);
                i = end_line;
                continue;
            }

            // Handle quotations
            if line.starts_with("@quotation") {
                let (quote_block, end_line) = self.parse_quotation(&lines, i);
                self.blocks.push(quote_block);
                i = end_line;
                continue;
            }

            // Empty lines
            if line.is_empty() {
                i += 1;
                continue;
            }

            // Regular paragraph
            let (para_lines, end_line) = self.collect_paragraph(&lines, i);
            if !para_lines.is_empty() {
                let para_text = para_lines.join(" ");
                let inline_nodes = self.parse_inline(&para_text);
                if !inline_nodes.is_empty() {
                    self.blocks.push(Block::Paragraph {
                        inlines: inline_nodes,
                    });
                }
            }
            i = end_line;
        }

        Ok(())
    }

    fn collect_paragraph<'b>(&self, lines: &[&'b str], start: usize) -> (Vec<&'b str>, usize) {
        let mut para_lines = Vec::new();
        let mut i = start;

        while i < lines.len() {
            let line = lines[i].trim();

            // Stop at empty line or command
            if line.is_empty()
                || line.starts_with('@')
                    && !line.starts_with("@code{")
                    && !line.starts_with("@emph{")
                    && !line.starts_with("@strong{")
                    && !line.starts_with("@uref{")
                    && !line.starts_with("@url{")
                    && !line.starts_with("@xref{")
                    && !line.starts_with("@pxref{")
                    && !line.starts_with("@ref{")
                    && !line.starts_with("@samp{")
                    && !line.starts_with("@var{")
                    && !line.starts_with("@file{")
                    && !line.starts_with("@dfn{")
                    && !line.starts_with("@kbd{")
                    && !line.starts_with("@key{")
                    && !line.starts_with("@acronym{")
                    && !line.starts_with("@email{")
            {
                break;
            }

            para_lines.push(line);
            i += 1;
        }

        (para_lines, i)
    }

    fn parse_list(&self, lines: &[&str], start: usize, ordered: bool) -> (Block, usize) {
        let mut items = Vec::new();
        let mut i = start + 1; // Skip @itemize/@enumerate line
        let mut current_item: Vec<String> = Vec::new();

        while i < lines.len() {
            let line = lines[i].trim();

            if line.starts_with("@end itemize") || line.starts_with("@end enumerate") {
                // Flush current item
                if !current_item.is_empty() {
                    let text = current_item.join(" ");
                    items.push(self.parse_inline(&text));
                }
                return (Block::List { ordered, items }, i + 1);
            }

            if line.starts_with("@item") {
                // Flush previous item
                if !current_item.is_empty() {
                    let text = current_item.join(" ");
                    items.push(self.parse_inline(&text));
                    current_item.clear();
                }

                // Get content after @item
                let rest = line.strip_prefix("@item").unwrap().trim();
                if !rest.is_empty() {
                    current_item.push(rest.to_string());
                }
            } else if !line.is_empty() && !line.starts_with("@c ") {
                current_item.push(line.to_string());
            }

            i += 1;
        }

        // No end marker found - return what we have
        if !current_item.is_empty() {
            let text = current_item.join(" ");
            items.push(self.parse_inline(&text));
        }

        (Block::List { ordered, items }, i)
    }

    fn parse_definition_list(&self, lines: &[&str], start: usize) -> (Block, usize) {
        let mut items = Vec::new();
        let mut i = start + 1; // Skip @table line
        let mut current_term: Option<String> = None;
        let mut current_desc: Vec<String> = Vec::new();

        while i < lines.len() {
            let line = lines[i].trim();

            if line.starts_with("@end table") {
                // Flush current entry
                if let Some(term) = current_term.take() {
                    let term_inlines = self.parse_inline(&term);
                    let mut desc_blocks = Vec::new();
                    if !current_desc.is_empty() {
                        let desc_text = current_desc.join(" ");
                        let desc_inlines = self.parse_inline(&desc_text);
                        desc_blocks.push(Block::Paragraph {
                            inlines: desc_inlines,
                        });
                        current_desc.clear();
                    }
                    items.push((term_inlines, desc_blocks));
                }
                return (Block::DefinitionList { items }, i + 1);
            }

            if line.starts_with("@item ") {
                // Flush previous entry
                if let Some(term) = current_term.take() {
                    let term_inlines = self.parse_inline(&term);
                    let mut desc_blocks = Vec::new();
                    if !current_desc.is_empty() {
                        let desc_text = current_desc.join(" ");
                        let desc_inlines = self.parse_inline(&desc_text);
                        desc_blocks.push(Block::Paragraph {
                            inlines: desc_inlines,
                        });
                        current_desc.clear();
                    }
                    items.push((term_inlines, desc_blocks));
                }

                let rest = line.strip_prefix("@item ").unwrap().trim();
                current_term = Some(rest.to_string());
            } else if !line.is_empty() && !line.starts_with("@c ") && !line.starts_with("@itemx ") {
                current_desc.push(line.to_string());
            }

            i += 1;
        }

        (Block::DefinitionList { items }, i)
    }

    fn parse_code_block(&self, lines: &[&str], start: usize, end_marker: &str) -> (Block, usize) {
        let mut code_lines = Vec::new();
        let mut i = start + 1; // Skip @example/@verbatim line

        while i < lines.len() {
            let line = lines[i];

            if line.trim() == end_marker {
                let content = code_lines.join("\n");
                return (Block::CodeBlock { content }, i + 1);
            }

            code_lines.push(line);
            i += 1;
        }

        let content = code_lines.join("\n");
        (Block::CodeBlock { content }, i)
    }

    fn parse_quotation(&self, lines: &[&str], start: usize) -> (Block, usize) {
        let mut quote_lines = Vec::new();
        let mut i = start + 1; // Skip @quotation line

        while i < lines.len() {
            let line = lines[i].trim();

            if line.starts_with("@end quotation") {
                let text = quote_lines.join(" ");
                let inline = self.parse_inline(&text);
                return (
                    Block::Blockquote {
                        children: vec![Block::Paragraph { inlines: inline }],
                    },
                    i + 1,
                );
            }

            if !line.is_empty() {
                quote_lines.push(line);
            }

            i += 1;
        }

        let text = quote_lines.join(" ");
        let inline = self.parse_inline(&text);
        (
            Block::Blockquote {
                children: vec![Block::Paragraph { inlines: inline }],
            },
            i,
        )
    }

    fn parse_inline(&self, text: &str) -> Vec<Inline> {
        let mut nodes = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '@' && i + 1 < chars.len() {
                // Check for inline commands
                if let Some((node, end_pos)) = self.try_parse_inline_command(&chars, i) {
                    if !current.is_empty() {
                        nodes.push(Inline::Text(current.clone()));
                        current.clear();
                    }
                    nodes.push(node);
                    i = end_pos;
                    continue;
                }
            }

            current.push(chars[i]);
            i += 1;
        }

        if !current.is_empty() {
            nodes.push(Inline::Text(current));
        }

        nodes
    }

    fn try_parse_inline_command(&self, chars: &[char], start: usize) -> Option<(Inline, usize)> {
        // Collect command name
        let mut cmd = String::new();
        let mut i = start + 1; // Skip @

        while i < chars.len() && chars[i].is_ascii_alphabetic() {
            cmd.push(chars[i]);
            i += 1;
        }

        // Check if followed by {
        if i >= chars.len() || chars[i] != '{' {
            return None;
        }

        // Find matching }
        let content_start = i + 1;
        let mut depth = 1;
        i += 1;

        while i < chars.len() && depth > 0 {
            match chars[i] {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            i += 1;
        }

        let content: String = chars[content_start..i - 1].iter().collect();

        let node = match cmd.as_str() {
            "emph" | "i" => Inline::Emphasis(self.parse_inline(&content)),

            "strong" | "b" => Inline::Strong(self.parse_inline(&content)),

            "code" | "samp" | "kbd" | "key" | "file" | "command" | "option" | "env" => {
                Inline::Code(content)
            }

            "var" | "dfn" => Inline::Emphasis(self.parse_inline(&content)),

            "uref" | "url" => {
                // Format: @uref{url} or @uref{url, text}
                let parts: Vec<&str> = content.splitn(2, ',').collect();
                let url = parts[0].trim();
                let text = if parts.len() > 1 {
                    parts[1].trim()
                } else {
                    url
                };
                Inline::Link {
                    url: url.to_string(),
                    children: self.parse_inline(text),
                }
            }

            "email" => {
                let parts: Vec<&str> = content.splitn(2, ',').collect();
                let email = parts[0].trim();
                let text = if parts.len() > 1 {
                    parts[1].trim()
                } else {
                    email
                };
                Inline::Link {
                    url: format!("mailto:{}", email),
                    children: self.parse_inline(text),
                }
            }

            "xref" | "pxref" | "ref" => {
                // Cross-reference - just use the node name as link
                let parts: Vec<&str> = content.splitn(2, ',').collect();
                let node_name = parts[0].trim();
                Inline::Link {
                    url: format!("#{}", node_name),
                    children: self.parse_inline(node_name),
                }
            }

            "acronym" | "abbr" => {
                let parts: Vec<&str> = content.splitn(2, ',').collect();
                Inline::Text(parts[0].trim().to_string())
            }

            "sc" => {
                // Small caps - just use text as is
                Inline::Text(content)
            }

            "footnote" => Inline::FootnoteDef {
                content: self.parse_inline(&content),
            },

            _ => {
                // Unknown command - return None to treat as literal text
                return None;
            }
        };

        Some((node, i))
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a Texinfo string from a [`TexinfoDoc`].
pub fn build(doc: &TexinfoDoc) -> String {
    let mut ctx = BuildContext::new();

    // Write header
    ctx.write("\\input texinfo\n");
    ctx.write("@setfilename output.info\n");

    // Write title if present
    if let Some(title) = &doc.title {
        ctx.write("@settitle ");
        ctx.write(title);
        ctx.write("\n");
    }

    ctx.write("\n@node Top\n");

    if let Some(title) = &doc.title {
        ctx.write("@top ");
        ctx.write(title);
        ctx.write("\n\n");
    }

    // Write blocks
    for block in &doc.blocks {
        build_block(block, &mut ctx);
    }

    // Write footer
    ctx.write("\n@bye\n");

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

    fn write_escaped(&mut self, s: &str) {
        for c in s.chars() {
            match c {
                '@' => self.write("@@"),
                '{' => self.write("@{"),
                '}' => self.write("@}"),
                _ => self.output.push(c),
            }
        }
    }
}

fn build_block(block: &Block, ctx: &mut BuildContext) {
    match block {
        Block::Heading { level, inlines } => {
            let command = match level {
                1 => "@chapter",
                2 => "@section",
                3 => "@subsection",
                4 => "@subsubsection",
                _ => "@subsubsection",
            };

            ctx.write(command);
            ctx.write(" ");
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::Paragraph { inlines } => {
            build_inlines(inlines, ctx);
            ctx.write("\n\n");
        }

        Block::CodeBlock { content } => {
            ctx.write("@example\n");
            ctx.write(content);
            if !content.ends_with('\n') {
                ctx.write("\n");
            }
            ctx.write("@end example\n\n");
        }

        Block::Blockquote { children } => {
            ctx.write("@quotation\n");
            for child in children {
                if let Block::Paragraph { inlines } = child {
                    build_inlines(inlines, ctx);
                    ctx.write("\n");
                } else {
                    build_block(child, ctx);
                }
            }
            ctx.write("@end quotation\n\n");
        }

        Block::List { ordered, items } => {
            if *ordered {
                ctx.write("@enumerate\n");
            } else {
                ctx.write("@itemize @bullet\n");
            }

            for item in items {
                ctx.write("@item ");
                build_inlines(item, ctx);
                ctx.write("\n");
            }

            if *ordered {
                ctx.write("@end enumerate\n\n");
            } else {
                ctx.write("@end itemize\n\n");
            }
        }

        Block::DefinitionList { items } => {
            ctx.write("@table @asis\n");

            for (term, desc_blocks) in items {
                ctx.write("@item ");
                build_inlines(term, ctx);
                ctx.write("\n");

                for desc_block in desc_blocks {
                    if let Block::Paragraph { inlines } = desc_block {
                        build_inlines(inlines, ctx);
                        ctx.write("\n");
                    } else {
                        build_block(desc_block, ctx);
                    }
                }
            }

            ctx.write("@end table\n\n");
        }

        Block::HorizontalRule => {
            ctx.write("\n@sp 1\n@noindent\n@center * * *\n@sp 1\n\n");
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
        Inline::Text(s) => ctx.write_escaped(s),

        Inline::Strong(children) => {
            ctx.write("@strong{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Emphasis(children) => {
            ctx.write("@emph{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Code(s) => {
            ctx.write("@code{");
            ctx.write_escaped(s);
            ctx.write("}");
        }

        Inline::Link { url, children } => {
            if url.starts_with("mailto:") {
                let email = url.strip_prefix("mailto:").unwrap_or(url);
                ctx.write("@email{");
                ctx.write(email);
                if !children.is_empty() {
                    ctx.write(", ");
                    build_inlines(children, ctx);
                }
                ctx.write("}");
            } else if url.starts_with('#') {
                // Internal reference
                let node_name = url.strip_prefix('#').unwrap_or(url);
                ctx.write("@ref{");
                ctx.write(node_name);
                ctx.write("}");
            } else {
                ctx.write("@uref{");
                ctx.write(url);
                if !children.is_empty() {
                    ctx.write(", ");
                    build_inlines(children, ctx);
                }
                ctx.write("}");
            }
        }

        Inline::Superscript(children) => {
            ctx.write("@sup{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::Subscript(children) => {
            ctx.write("@sub{");
            build_inlines(children, ctx);
            ctx.write("}");
        }

        Inline::LineBreak => {
            ctx.write("@*\n");
        }

        Inline::SoftBreak => {
            ctx.write(" ");
        }

        Inline::FootnoteDef { content } => {
            ctx.write("@footnote{");
            build_inlines(content, ctx);
            ctx.write("}");
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let input = r#"@chapter Introduction
This is the introduction paragraph.

@section Getting Started
Here is how to get started."#;

        let doc = parse(input).unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_headings() {
        let input = r#"@chapter Chapter One
@section Section One
@subsection Subsection One
@subsubsection Sub-subsection"#;

        let doc = parse(input).unwrap();
        assert_eq!(doc.blocks.len(), 4);
    }

    #[test]
    fn test_parse_emphasis() {
        let input = r#"This is @emph{emphasized} and @strong{bold} text."#;

        let doc = parse(input).unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_code() {
        let input = r#"Use @code{printf} to print."#;

        let doc = parse(input).unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_list() {
        let input = r#"@itemize
@item First item
@item Second item
@end itemize"#;

        let doc = parse(input).unwrap();
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::List { .. }));
    }

    #[test]
    fn test_parse_enumerate() {
        let input = r#"@enumerate
@item First
@item Second
@end enumerate"#;

        let doc = parse(input).unwrap();
        assert!(!doc.blocks.is_empty());
        let block = &doc.blocks[0];
        assert!(matches!(block, Block::List { ordered: true, .. }));
    }

    #[test]
    fn test_parse_example() {
        let input = r#"@example
int main() {
    return 0;
}
@end example"#;

        let doc = parse(input).unwrap();
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::CodeBlock { .. }));
    }

    #[test]
    fn test_parse_url() {
        let input = r#"Visit @uref{https://example.com, Example Site}."#;

        let doc = parse(input).unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_parse_quotation() {
        let input = r#"@quotation
This is a quoted passage.
@end quotation"#;

        let doc = parse(input).unwrap();
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_skip_comments() {
        let input = r#"@c This is a comment
This is visible.
@comment Another comment
Still visible."#;

        let doc = parse(input).unwrap();
        assert!(!doc.blocks.is_empty());
    }

    #[test]
    fn test_build_header() {
        let doc = TexinfoDoc {
            title: Some("Test".to_string()),
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.starts_with("\\input texinfo"));
        assert!(out.ends_with("@bye\n"));
    }

    #[test]
    fn test_build_paragraph() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Hello, world!".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("Hello, world!"));
    }

    #[test]
    fn test_build_strong() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Strong(vec![Inline::Text("bold".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("@strong{bold}"));
    }

    #[test]
    fn test_build_emphasis() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Emphasis(vec![Inline::Text("italic".into())])],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("@emph{italic}"));
    }

    #[test]
    fn test_build_code() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Code("printf".into())],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("@code{printf}"));
    }

    #[test]
    fn test_build_link() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Link {
                    url: "https://example.com".to_string(),
                    children: vec![Inline::Text("Example".into())],
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("@uref{https://example.com, Example}"));
    }

    #[test]
    fn test_build_list() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::List {
                ordered: false,
                items: vec![
                    vec![Inline::Text("one".into())],
                    vec![Inline::Text("two".into())],
                ],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("@itemize @bullet"));
        assert!(out.contains("@item one"));
        assert!(out.contains("@item two"));
        assert!(out.contains("@end itemize"));
    }

    #[test]
    fn test_build_enumerate() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::List {
                ordered: true,
                items: vec![
                    vec![Inline::Text("first".into())],
                    vec![Inline::Text("second".into())],
                ],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("@enumerate"));
        assert!(out.contains("@item first"));
        assert!(out.contains("@end enumerate"));
    }

    #[test]
    fn test_build_code_block() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::CodeBlock {
                content: "int main() {}".into(),
            }],
        };
        let out = build(&doc);
        assert!(out.contains("@example"));
        assert!(out.contains("int main() {}"));
        assert!(out.contains("@end example"));
    }

    #[test]
    fn test_build_blockquote() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Blockquote {
                children: vec![Block::Paragraph {
                    inlines: vec![Inline::Text("Quoted text".into())],
                }],
            }],
        };
        let out = build(&doc);
        assert!(out.contains("@quotation"));
        assert!(out.contains("Quoted text"));
        assert!(out.contains("@end quotation"));
    }

    #[test]
    fn test_escape_special_chars() {
        let doc = TexinfoDoc {
            title: None,
            blocks: vec![Block::Paragraph {
                inlines: vec![Inline::Text("Use @{braces}".into())],
            }],
        };
        let out = build(&doc);
        // @ -> @@, { -> @{, } -> @}
        assert!(out.contains("Use @@@{braces@}"));
    }

    #[test]
    fn test_parse_settitle() {
        let input = "@settitle My Book\n\n@chapter Introduction\nContent here.";
        let doc = parse(input).unwrap();
        assert_eq!(doc.title, Some("My Book".to_string()));
    }
}
