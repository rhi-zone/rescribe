//! Texinfo parser — infallible, returns diagnostics instead of errors.

use crate::ast::{Block, Diagnostic, Inline, Span, TexinfoDoc};

/// Parse a Texinfo string into a [`TexinfoDoc`].
///
/// This function is infallible: instead of returning `Err`, any parse issues
/// are reported as [`Diagnostic`] values in the second element of the tuple.
pub fn parse(input: &str) -> (TexinfoDoc, Vec<Diagnostic>) {
    let mut p = Parser::new(input);
    p.run();
    (
        TexinfoDoc {
            title: p.title,
            blocks: p.blocks,
            span: Span::NONE,
        },
        p.diagnostics,
    )
}

struct Parser<'a> {
    input: &'a str,
    title: Option<String>,
    blocks: Vec<Block>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            title: None,
            blocks: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn run(&mut self) {
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
                    inlines: parse_inline(rest.trim()),
                    span: Span::NONE,
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@unnumbered ") {
                self.blocks.push(Block::Heading {
                    level: 1,
                    inlines: parse_inline(rest.trim()),
                    span: Span::NONE,
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@appendix ") {
                self.blocks.push(Block::Heading {
                    level: 1,
                    inlines: parse_inline(rest.trim()),
                    span: Span::NONE,
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@section ") {
                self.blocks.push(Block::Heading {
                    level: 2,
                    inlines: parse_inline(rest.trim()),
                    span: Span::NONE,
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@unnumberedsec ") {
                self.blocks.push(Block::Heading {
                    level: 2,
                    inlines: parse_inline(rest.trim()),
                    span: Span::NONE,
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@appendixsec ") {
                self.blocks.push(Block::Heading {
                    level: 2,
                    inlines: parse_inline(rest.trim()),
                    span: Span::NONE,
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@subsection ") {
                self.blocks.push(Block::Heading {
                    level: 3,
                    inlines: parse_inline(rest.trim()),
                    span: Span::NONE,
                });
                i += 1;
                continue;
            }
            if let Some(rest) = line.strip_prefix("@subsubsection ") {
                self.blocks.push(Block::Heading {
                    level: 4,
                    inlines: parse_inline(rest.trim()),
                    span: Span::NONE,
                });
                i += 1;
                continue;
            }

            // Handle lists
            if line.starts_with("@itemize") || line.starts_with("@enumerate") {
                let ordered = line.starts_with("@enumerate");
                let (list_block, end_line) = parse_list(&lines, i, ordered);
                self.blocks.push(list_block);
                i = end_line;
                continue;
            }

            // Handle definition lists (@table)
            if line.starts_with("@table") {
                let (def_list_block, end_line) = parse_definition_list(&lines, i);
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
                let (code_block, end_line) = parse_code_block(&lines, i, end_marker);
                self.blocks.push(code_block);
                i = end_line;
                continue;
            }

            // Handle quotations
            if line.starts_with("@quotation") {
                let (quote_block, end_line) = parse_quotation(&lines, i);
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
            let (para_lines, end_line) = collect_paragraph(&lines, i);
            if !para_lines.is_empty() {
                let para_text = para_lines.join(" ");
                let inline_nodes = parse_inline(&para_text);
                if !inline_nodes.is_empty() {
                    self.blocks.push(Block::Paragraph {
                        inlines: inline_nodes,
                        span: Span::NONE,
                    });
                }
            }
            // .max(i + 1) prevents infinite loop when collect_paragraph returns
            // end_line == i (happens when the first line starts with an unknown @directive)
            i = end_line.max(i + 1);
        }
    }
}

fn collect_paragraph<'b>(lines: &[&'b str], start: usize) -> (Vec<&'b str>, usize) {
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

fn parse_list(lines: &[&str], start: usize, ordered: bool) -> (Block, usize) {
    let mut items = Vec::new();
    let mut i = start + 1; // Skip @itemize/@enumerate line
    let mut current_item: Vec<String> = Vec::new();

    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("@end itemize") || line.starts_with("@end enumerate") {
            // Flush current item
            if !current_item.is_empty() {
                let text = current_item.join(" ");
                items.push(parse_inline(&text));
            }
            return (
                Block::List {
                    ordered,
                    items,
                    span: Span::NONE,
                },
                i + 1,
            );
        }

        if line.starts_with("@item") {
            // Flush previous item
            if !current_item.is_empty() {
                let text = current_item.join(" ");
                items.push(parse_inline(&text));
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
        items.push(parse_inline(&text));
    }

    (
        Block::List {
            ordered,
            items,
            span: Span::NONE,
        },
        i,
    )
}

fn parse_definition_list(lines: &[&str], start: usize) -> (Block, usize) {
    let mut items = Vec::new();
    let mut i = start + 1; // Skip @table line
    let mut current_term: Option<String> = None;
    let mut current_desc: Vec<String> = Vec::new();

    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("@end table") {
            // Flush current entry
            if let Some(term) = current_term.take() {
                let term_inlines = parse_inline(&term);
                let mut desc_blocks = Vec::new();
                if !current_desc.is_empty() {
                    let desc_text = current_desc.join(" ");
                    let desc_inlines = parse_inline(&desc_text);
                    desc_blocks.push(Block::Paragraph {
                        inlines: desc_inlines,
                        span: Span::NONE,
                    });
                    current_desc.clear();
                }
                items.push((term_inlines, desc_blocks));
            }
            return (Block::DefinitionList { items, span: Span::NONE }, i + 1);
        }

        if line.starts_with("@item ") {
            // Flush previous entry
            if let Some(term) = current_term.take() {
                let term_inlines = parse_inline(&term);
                let mut desc_blocks = Vec::new();
                if !current_desc.is_empty() {
                    let desc_text = current_desc.join(" ");
                    let desc_inlines = parse_inline(&desc_text);
                    desc_blocks.push(Block::Paragraph {
                        inlines: desc_inlines,
                        span: Span::NONE,
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

    (Block::DefinitionList { items, span: Span::NONE }, i)
}

fn parse_code_block(lines: &[&str], start: usize, end_marker: &str) -> (Block, usize) {
    let mut code_lines = Vec::new();
    let mut i = start + 1; // Skip @example/@verbatim line

    while i < lines.len() {
        let line = lines[i];

        if line.trim() == end_marker {
            let content = code_lines.join("\n");
            return (Block::CodeBlock { content, span: Span::NONE }, i + 1);
        }

        code_lines.push(line);
        i += 1;
    }

    let content = code_lines.join("\n");
    (Block::CodeBlock { content, span: Span::NONE }, i)
}

fn parse_quotation(lines: &[&str], start: usize) -> (Block, usize) {
    let mut quote_lines = Vec::new();
    let mut i = start + 1; // Skip @quotation line

    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("@end quotation") {
            let text = quote_lines.join(" ");
            let inline = parse_inline(&text);
            return (
                Block::Blockquote {
                    children: vec![Block::Paragraph {
                        inlines: inline,
                        span: Span::NONE,
                    }],
                    span: Span::NONE,
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
    let inline = parse_inline(&text);
    (
        Block::Blockquote {
            children: vec![Block::Paragraph {
                inlines: inline,
                span: Span::NONE,
            }],
            span: Span::NONE,
        },
        i,
    )
}

pub(crate) fn parse_inline(text: &str) -> Vec<Inline> {
    let mut nodes = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '@' && i + 1 < chars.len() {
            // Check for inline commands
            if let Some((node, end_pos)) = try_parse_inline_command(&chars, i) {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
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
        nodes.push(Inline::Text(current, Span::NONE));
    }

    nodes
}

fn try_parse_inline_command(chars: &[char], start: usize) -> Option<(Inline, usize)> {
    // Collect command name
    let mut cmd = String::new();
    let mut i = start + 1; // Skip @

    while i < chars.len() && chars[i].is_ascii_alphabetic() {
        cmd.push(chars[i]);
        i += 1;
    }

    // Handle no-brace commands (e.g. @*)
    if cmd.is_empty() && i < chars.len() && chars[i] == '*' {
        return Some((Inline::LineBreak { span: Span::NONE }, i + 1));
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

    // If depth > 0 here, the closing } was never found — treat as literal text.
    if depth > 0 {
        return None;
    }
    let content: String = chars[content_start..i - 1].iter().collect();

    let node = match cmd.as_str() {
        "emph" | "i" => Inline::Emphasis(parse_inline(&content), Span::NONE),

        "strong" | "b" => Inline::Strong(parse_inline(&content), Span::NONE),

        "sup" => Inline::Superscript(parse_inline(&content), Span::NONE),
        "sub" => Inline::Subscript(parse_inline(&content), Span::NONE),

        "code" | "samp" | "kbd" | "key" | "file" | "command" | "option" | "env" => {
            Inline::Code(content, Span::NONE)
        }

        "var" | "dfn" => Inline::Emphasis(parse_inline(&content), Span::NONE),

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
                children: parse_inline(text),
                span: Span::NONE,
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
                children: parse_inline(text),
                span: Span::NONE,
            }
        }

        "xref" | "pxref" | "ref" => {
            // Cross-reference - just use the node name as link
            let parts: Vec<&str> = content.splitn(2, ',').collect();
            let node_name = parts[0].trim();
            Inline::Link {
                url: format!("#{}", node_name),
                children: parse_inline(node_name),
                span: Span::NONE,
            }
        }

        "acronym" | "abbr" => {
            let parts: Vec<&str> = content.splitn(2, ',').collect();
            Inline::Text(parts[0].trim().to_string(), Span::NONE)
        }

        "sc" => {
            // Small caps - just use text as is
            Inline::Text(content, Span::NONE)
        }

        "footnote" => Inline::FootnoteDef {
            content: parse_inline(&content),
            span: Span::NONE,
        },

        _ => {
            // Unknown command - return None to treat as literal text
            return None;
        }
    };

    Some((node, i))
}
