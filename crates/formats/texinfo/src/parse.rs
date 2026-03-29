//! Texinfo parser — infallible, returns diagnostics instead of errors.

use crate::ast::{
    Block, CodeBlockVariant, CrossRefKind, Diagnostic, HeadingKind, Inline, MenuEntry, Span,
    SymbolKind, TableRow, TexinfoDoc,
};

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
                || line.starts_with("@author ")
            {
                i += 1;
                continue;
            }

            // Handle node definitions (skip the @node line itself)
            if line.starts_with("@node ") || line == "@node" {
                i += 1;
                continue;
            }

            // Handle @noindent
            if line == "@noindent" || line.starts_with("@noindent ") {
                self.blocks.push(Block::NoIndent { span: Span::NONE });
                i += 1;
                continue;
            }

            // Handle headings
            if let Some((level, kind, rest)) = try_heading(line) {
                self.blocks.push(Block::Heading {
                    level,
                    kind,
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

            // Handle multitable
            if line.starts_with("@multitable") {
                let (table_block, end_line) = parse_multitable(&lines, i);
                self.blocks.push(table_block);
                i = end_line;
                continue;
            }

            // Handle code blocks
            if let Some(variant) = try_code_block_start(line) {
                let end_marker = code_block_end_marker(&variant);
                let (code_block, end_line) = parse_code_block(&lines, i, &end_marker, variant);
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

            // Handle @menu
            if line.starts_with("@menu") {
                let (menu_block, end_line) = parse_menu(&lines, i);
                self.blocks.push(menu_block);
                i = end_line;
                continue;
            }

            // Handle @float
            if line.starts_with("@float") {
                let (float_block, end_line) = parse_float(&lines, i);
                self.blocks.push(float_block);
                i = end_line;
                continue;
            }

            // Handle conditional blocks
            if let Some(env_name) = try_conditional_start(line) {
                let (raw_block, end_line) = parse_raw_block(&lines, i, &env_name);
                self.blocks.push(raw_block);
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

fn try_heading(line: &str) -> Option<(u8, HeadingKind, &str)> {
    let heading_cmds: &[(&str, u8, HeadingKind)] = &[
        ("@chapter ", 1, HeadingKind::Numbered),
        ("@unnumbered ", 1, HeadingKind::Unnumbered),
        ("@appendix ", 1, HeadingKind::Appendix),
        ("@section ", 2, HeadingKind::Numbered),
        ("@unnumberedsec ", 2, HeadingKind::Unnumbered),
        ("@appendixsec ", 2, HeadingKind::Appendix),
        ("@subsection ", 3, HeadingKind::Numbered),
        ("@unnumberedsubsec ", 3, HeadingKind::Unnumbered),
        ("@appendixsubsec ", 3, HeadingKind::Appendix),
        ("@subsubsection ", 4, HeadingKind::Numbered),
        ("@unnumberedsubsubsec ", 4, HeadingKind::Unnumbered),
        ("@appendixsubsubsec ", 4, HeadingKind::Appendix),
    ];
    for (prefix, level, kind) in heading_cmds {
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some((*level, kind.clone(), rest));
        }
    }
    None
}

fn try_code_block_start(line: &str) -> Option<CodeBlockVariant> {
    if line.starts_with("@example") {
        Some(CodeBlockVariant::Example)
    } else if line.starts_with("@smallexample") {
        Some(CodeBlockVariant::SmallExample)
    } else if line.starts_with("@verbatim") {
        Some(CodeBlockVariant::Verbatim)
    } else if line.starts_with("@lisp") {
        Some(CodeBlockVariant::Lisp)
    } else if line.starts_with("@display") {
        Some(CodeBlockVariant::Display)
    } else if line.starts_with("@format") {
        Some(CodeBlockVariant::Format)
    } else {
        None
    }
}

fn code_block_end_marker(variant: &CodeBlockVariant) -> String {
    match variant {
        CodeBlockVariant::Example => "@end example".to_string(),
        CodeBlockVariant::SmallExample => "@end smallexample".to_string(),
        CodeBlockVariant::Verbatim => "@end verbatim".to_string(),
        CodeBlockVariant::Lisp => "@end lisp".to_string(),
        CodeBlockVariant::Display => "@end display".to_string(),
        CodeBlockVariant::Format => "@end format".to_string(),
    }
}

fn try_conditional_start(line: &str) -> Option<String> {
    let conditionals = &[
        "@iftex", "@ifhtml", "@ifinfo", "@ifplaintext", "@ifnottex", "@ifnothtml",
        "@ifnotinfo", "@ifnotplaintext",
    ];
    for cond in conditionals {
        if line.starts_with(cond)
            && (line.len() == cond.len() || line.as_bytes().get(cond.len()) == Some(&b' '))
        {
            return Some(cond[1..].to_string()); // strip leading @
        }
    }
    None
}

fn collect_paragraph<'b>(lines: &[&'b str], start: usize) -> (Vec<&'b str>, usize) {
    let mut para_lines = Vec::new();
    let mut i = start;

    while i < lines.len() {
        let line = lines[i].trim();

        // Stop at empty line or block-level command
        if line.is_empty() {
            break;
        }
        if line.starts_with('@') && !is_inline_command_at_start(line) {
            break;
        }

        para_lines.push(line);
        i += 1;
    }

    (para_lines, i)
}

/// Returns true if the line starts with an inline @-command (not a block command).
fn is_inline_command_at_start(line: &str) -> bool {
    let inline_prefixes = &[
        "@code{", "@emph{", "@strong{", "@uref{", "@url{", "@xref{", "@pxref{", "@ref{",
        "@samp{", "@var{", "@file{", "@dfn{", "@kbd{", "@key{", "@acronym{", "@email{",
        "@command{", "@option{", "@env{", "@cite{", "@abbr{", "@sc{", "@r{", "@i{", "@b{",
        "@t{", "@w{", "@footnote{", "@anchor{", "@dots{", "@enddots{", "@minus{",
        "@copyright{", "@registeredsymbol{", "@LaTeX{", "@TeX{", "@tie{", "@image{",
        "@sup{", "@sub{",
    ];
    for prefix in inline_prefixes {
        if line.starts_with(prefix) {
            return true;
        }
    }
    false
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

        if line.starts_with("@item ") || line == "@item" {
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

            let rest = line.strip_prefix("@item").unwrap().trim();
            current_term = Some(rest.to_string());
        } else if !line.is_empty() && !line.starts_with("@c ") && !line.starts_with("@itemx ") {
            current_desc.push(line.to_string());
        }

        i += 1;
    }

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
        }
        items.push((term_inlines, desc_blocks));
    }

    (Block::DefinitionList { items, span: Span::NONE }, i)
}

fn parse_multitable(lines: &[&str], start: usize) -> (Block, usize) {
    let mut rows = Vec::new();
    let mut i = start + 1; // Skip @multitable line

    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("@end multitable") {
            return (Block::Table { rows, span: Span::NONE }, i + 1);
        }

        if line.starts_with("@headitem ") || line.starts_with("@item ") {
            let is_header = line.starts_with("@headitem ");
            let rest = if is_header {
                line.strip_prefix("@headitem ").unwrap()
            } else {
                line.strip_prefix("@item ").unwrap()
            };
            let cells: Vec<Vec<Inline>> = rest
                .split("@tab")
                .map(|cell| parse_inline(cell.trim()))
                .collect();
            rows.push(TableRow { is_header, cells });
        }

        i += 1;
    }

    (Block::Table { rows, span: Span::NONE }, i)
}

fn parse_code_block(
    lines: &[&str],
    start: usize,
    end_marker: &str,
    variant: CodeBlockVariant,
) -> (Block, usize) {
    let mut code_lines = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];

        if line.trim() == end_marker {
            let content = code_lines.join("\n");
            return (
                Block::CodeBlock {
                    variant,
                    content,
                    span: Span::NONE,
                },
                i + 1,
            );
        }

        code_lines.push(line);
        i += 1;
    }

    let content = code_lines.join("\n");
    (
        Block::CodeBlock {
            variant,
            content,
            span: Span::NONE,
        },
        i,
    )
}

fn parse_quotation(lines: &[&str], start: usize) -> (Block, usize) {
    let mut quote_lines = Vec::new();
    let mut i = start + 1;

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

fn parse_menu(lines: &[&str], start: usize) -> (Block, usize) {
    let mut entries = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i].trim();

        if line.starts_with("@end menu") {
            return (Block::Menu { entries, span: Span::NONE }, i + 1);
        }

        // Menu entries look like: * Node Name:: Description
        if let Some(rest) = line.strip_prefix("* ")
            && let Some(idx) = rest.find("::")
        {
            let node = rest[..idx].trim().to_string();
            let desc = rest[idx + 2..].trim();
            let description = if desc.is_empty() {
                None
            } else {
                Some(desc.to_string())
            };
            entries.push(MenuEntry { node, description });
        }

        i += 1;
    }

    (Block::Menu { entries, span: Span::NONE }, i)
}

fn parse_float(lines: &[&str], start: usize) -> (Block, usize) {
    // @float [type][,label]
    let first_line = lines[start].trim();
    let args = first_line
        .strip_prefix("@float")
        .unwrap_or("")
        .trim();
    let (float_type, label) = if args.is_empty() {
        (None, None)
    } else {
        let parts: Vec<&str> = args.splitn(2, ',').collect();
        let ft = if parts[0].trim().is_empty() {
            None
        } else {
            Some(parts[0].trim().to_string())
        };
        let lb = parts.get(1).map(|s| s.trim().to_string());
        (ft, lb)
    };

    let mut children = Vec::new();
    let mut i = start + 1;

    // Simple: collect paragraphs inside @float
    let mut para_lines: Vec<String> = Vec::new();
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("@end float") {
            if !para_lines.is_empty() {
                let text = para_lines.join(" ");
                let inlines = parse_inline(&text);
                children.push(Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                });
            }
            return (
                Block::Float {
                    float_type,
                    label,
                    children,
                    span: Span::NONE,
                },
                i + 1,
            );
        }
        if line.is_empty() {
            if !para_lines.is_empty() {
                let text = para_lines.join(" ");
                let inlines = parse_inline(&text);
                children.push(Block::Paragraph {
                    inlines,
                    span: Span::NONE,
                });
                para_lines.clear();
            }
        } else {
            para_lines.push(line.to_string());
        }
        i += 1;
    }

    (
        Block::Float {
            float_type,
            label,
            children,
            span: Span::NONE,
        },
        i,
    )
}

fn parse_raw_block(lines: &[&str], start: usize, env_name: &str) -> (Block, usize) {
    let end_marker = format!("@end {}", env_name);
    let mut content_lines = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i].trim();
        if line == end_marker {
            return (
                Block::RawBlock {
                    environment: env_name.to_string(),
                    content: content_lines.join("\n"),
                    span: Span::NONE,
                },
                i + 1,
            );
        }
        content_lines.push(lines[i]);
        i += 1;
    }

    (
        Block::RawBlock {
            environment: env_name.to_string(),
            content: content_lines.join("\n"),
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
            // Check for @@ @{ @} escapes
            if chars[i + 1] == '@' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                current.push('@');
                i += 2;
                continue;
            }
            if chars[i + 1] == '{' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                current.push('{');
                i += 2;
                continue;
            }
            if chars[i + 1] == '}' {
                if !current.is_empty() {
                    nodes.push(Inline::Text(current.clone(), Span::NONE));
                    current.clear();
                }
                current.push('}');
                i += 2;
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
        "emph" => Inline::Emphasis(parse_inline(&content), Span::NONE),
        "i" => Inline::DirectItalic(parse_inline(&content), Span::NONE),
        "strong" => Inline::Strong(parse_inline(&content), Span::NONE),
        "b" => Inline::DirectBold(parse_inline(&content), Span::NONE),

        "sup" => Inline::Superscript(parse_inline(&content), Span::NONE),
        "sub" => Inline::Subscript(parse_inline(&content), Span::NONE),

        "code" => Inline::Code(content, Span::NONE),
        "samp" => Inline::Samp(content, Span::NONE),
        "kbd" => Inline::Kbd(content, Span::NONE),
        "key" => Inline::Key(content, Span::NONE),
        "file" => Inline::File(content, Span::NONE),
        "command" => Inline::Command(content, Span::NONE),
        "option" => Inline::Option(content, Span::NONE),
        "env" => Inline::Env(content, Span::NONE),
        "t" => Inline::DirectTypewriter(content, Span::NONE),

        "var" => Inline::Var(parse_inline(&content), Span::NONE),
        "dfn" => Inline::Dfn(parse_inline(&content), Span::NONE),
        "cite" => Inline::Cite(content, Span::NONE),
        "r" => Inline::Roman(content, Span::NONE),
        "sc" => Inline::SmallCaps(content, Span::NONE),

        "uref" | "url" => {
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
            let address = parts[0].trim().to_string();
            let text = parts.get(1).map(|s| s.trim().to_string());
            Inline::Email {
                address,
                text,
                span: Span::NONE,
            }
        }

        "xref" => {
            let parts: Vec<&str> = content.splitn(2, ',').collect();
            let node_name = parts[0].trim().to_string();
            let text = parts.get(1).map(|s| s.trim().to_string());
            Inline::CrossRef {
                kind: CrossRefKind::Xref,
                node: node_name,
                text,
                span: Span::NONE,
            }
        }

        "pxref" => {
            let parts: Vec<&str> = content.splitn(2, ',').collect();
            let node_name = parts[0].trim().to_string();
            let text = parts.get(1).map(|s| s.trim().to_string());
            Inline::CrossRef {
                kind: CrossRefKind::Pxref,
                node: node_name,
                text,
                span: Span::NONE,
            }
        }

        "ref" => {
            let parts: Vec<&str> = content.splitn(2, ',').collect();
            let node_name = parts[0].trim().to_string();
            let text = parts.get(1).map(|s| s.trim().to_string());
            Inline::CrossRef {
                kind: CrossRefKind::Ref,
                node: node_name,
                text,
                span: Span::NONE,
            }
        }

        "anchor" => Inline::Anchor {
            name: content.trim().to_string(),
            span: Span::NONE,
        },

        "acronym" => {
            let parts: Vec<&str> = content.splitn(2, ',').collect();
            let abbrev = parts[0].trim().to_string();
            let expansion = parts.get(1).map(|s| s.trim().to_string());
            Inline::Acronym {
                abbrev,
                expansion,
                span: Span::NONE,
            }
        }

        "abbr" => {
            let parts: Vec<&str> = content.splitn(2, ',').collect();
            let abbrev = parts[0].trim().to_string();
            let expansion = parts.get(1).map(|s| s.trim().to_string());
            Inline::Abbr {
                abbrev,
                expansion,
                span: Span::NONE,
            }
        }

        "w" => Inline::NoBreak(content, Span::NONE),

        "footnote" => Inline::FootnoteDef {
            content: parse_inline(&content),
            span: Span::NONE,
        },

        "image" => {
            let parts: Vec<&str> = content.splitn(5, ',').collect();
            let file = parts[0].trim().to_string();
            let width = parts.get(1).and_then(|s| {
                let s = s.trim();
                if s.is_empty() { None } else { Some(s.to_string()) }
            });
            let height = parts.get(2).and_then(|s| {
                let s = s.trim();
                if s.is_empty() { None } else { Some(s.to_string()) }
            });
            let alt = parts.get(3).and_then(|s| {
                let s = s.trim();
                if s.is_empty() { None } else { Some(s.to_string()) }
            });
            let extension = parts.get(4).and_then(|s| {
                let s = s.trim();
                if s.is_empty() { None } else { Some(s.to_string()) }
            });
            Inline::Image {
                file,
                width,
                height,
                alt,
                extension,
                span: Span::NONE,
            }
        }

        // Symbol commands
        "dots" => Inline::Symbol(SymbolKind::Dots, Span::NONE),
        "enddots" => Inline::Symbol(SymbolKind::EndDots, Span::NONE),
        "minus" => Inline::Symbol(SymbolKind::Minus, Span::NONE),
        "copyright" => Inline::Symbol(SymbolKind::Copyright, Span::NONE),
        "registeredsymbol" => Inline::Symbol(SymbolKind::Registered, Span::NONE),
        "LaTeX" => Inline::Symbol(SymbolKind::LaTeX, Span::NONE),
        "TeX" => Inline::Symbol(SymbolKind::TeX, Span::NONE),
        "tie" => Inline::Symbol(SymbolKind::Tie, Span::NONE),

        _ => {
            // Unknown command - return None to treat as literal text
            return None;
        }
    };

    Some((node, i))
}
