//! Typst reader for rescribe.
//!
//! Parses Typst markup into rescribe documents using the official `typst-syntax` crate.

use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
use rescribe_std::{node, prop};
use typst_syntax::ast::{AstNode, Expr, Markup};

/// Parse Typst source into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse Typst source with custom options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let root = typst_syntax::parse(input);
    let markup = root
        .cast::<Markup>()
        .ok_or_else(|| ParseError::Invalid("Failed to cast root to Markup".to_owned()))?;

    let children = convert_markup_to_blocks(markup, input);
    let doc_node = Node::new(node::DOCUMENT).children(children);
    let doc = Document::new().with_content(doc_node);
    Ok(ConversionResult::ok(doc))
}

/// Convert a `Markup` node to a list of block-level rescribe nodes.
///
/// Typst does not have explicit paragraph nodes; consecutive inline exprs are
/// grouped into paragraphs, separated by `Parbreak`.
fn convert_markup_to_blocks(markup: Markup, source: &str) -> Vec<Node> {
    let mut blocks: Vec<Node> = Vec::new();
    let mut inline_buf: Vec<Node> = Vec::new();

    for expr in markup.exprs() {
        match expr {
            // --- Block-level elements ---
            Expr::Parbreak(_) => {
                flush_paragraph(&mut inline_buf, &mut blocks);
            }
            Expr::Heading(h) => {
                flush_paragraph(&mut inline_buf, &mut blocks);
                let level = h.depth().get() as i64;
                let body_children = convert_markup_to_inlines(h.body(), source);
                blocks.push(
                    Node::new(node::HEADING)
                        .prop(prop::LEVEL, level)
                        .children(body_children),
                );
            }
            Expr::ListItem(item) => {
                // Each list item arrives as a top-level expr; collect them then merge.
                flush_paragraph(&mut inline_buf, &mut blocks);
                let list_item = convert_list_item_body(item.body(), source);
                blocks.push(
                    Node::new(node::LIST)
                        .prop(prop::ORDERED, false)
                        .children(vec![list_item]),
                );
            }
            Expr::EnumItem(item) => {
                flush_paragraph(&mut inline_buf, &mut blocks);
                let list_item = convert_list_item_body(item.body(), source);
                blocks.push(
                    Node::new(node::LIST)
                        .prop(prop::ORDERED, true)
                        .children(vec![list_item]),
                );
            }
            Expr::Raw(raw) if raw.block() => {
                flush_paragraph(&mut inline_buf, &mut blocks);
                let content: String = raw
                    .lines()
                    .map(|t| t.get().as_str().to_owned())
                    .collect::<Vec<_>>()
                    .join("\n");
                let lang_opt = raw.lang().map(|l| l.to_untyped().text().to_string());
                let mut n = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content);
                if let Some(lang) = lang_opt
                    && !lang.is_empty()
                {
                    n = n.prop(prop::LANGUAGE, lang);
                }
                blocks.push(n);
            }
            Expr::FuncCall(call) => {
                flush_paragraph(&mut inline_buf, &mut blocks);
                if let Some(block) = convert_func_call(call, source) {
                    blocks.push(block);
                }
            }

            // --- Inline elements (gathered into paragraph buffer) ---
            other => {
                inline_buf.extend(convert_expr_to_inlines(other, source));
            }
        }
    }

    flush_paragraph(&mut inline_buf, &mut blocks);
    merge_adjacent_lists(blocks)
}

fn flush_paragraph(inline_buf: &mut Vec<Node>, blocks: &mut Vec<Node>) {
    if inline_buf.is_empty() {
        return;
    }
    // Don't create paragraphs that contain only whitespace text nodes.
    let all_whitespace = inline_buf.iter().all(|n| {
        n.kind.as_str() == node::TEXT
            && n.props
                .get_str(prop::CONTENT)
                .map(|s| s.trim().is_empty())
                .unwrap_or(true)
    });
    if all_whitespace {
        inline_buf.clear();
        return;
    }
    blocks.push(Node::new(node::PARAGRAPH).children(inline_buf.drain(..)));
}

/// Convert a `Markup` body into a flat list of inline rescribe nodes.
fn convert_markup_to_inlines(markup: Markup, source: &str) -> Vec<Node> {
    let mut nodes = Vec::new();
    for expr in markup.exprs() {
        nodes.extend(convert_expr_to_inlines(expr, source));
    }
    nodes
}

/// Convert a single `Expr` to inline rescribe nodes.
fn convert_expr_to_inlines(expr: Expr, source: &str) -> Vec<Node> {
    match expr {
        Expr::Text(t) => {
            vec![Node::new(node::TEXT).prop(prop::CONTENT, t.get().as_str())]
        }
        Expr::Space(_) => {
            vec![Node::new(node::TEXT).prop(prop::CONTENT, " ")]
        }
        Expr::Linebreak(_) => {
            vec![Node::new(node::LINE_BREAK)]
        }
        Expr::SmartQuote(q) => {
            let ch = if q.double() { "\"" } else { "'" };
            vec![Node::new(node::TEXT).prop(prop::CONTENT, ch)]
        }
        Expr::Escape(e) => {
            let text = e.to_untyped().text().to_string();
            // The escape source includes the backslash; strip it.
            let content = if let Some(stripped) = text.strip_prefix('\\') {
                stripped.to_owned()
            } else {
                text
            };
            vec![Node::new(node::TEXT).prop(prop::CONTENT, content)]
        }
        Expr::Shorthand(s) => {
            let text = s.to_untyped().text().to_string();
            vec![Node::new(node::TEXT).prop(prop::CONTENT, text)]
        }
        Expr::Strong(s) => {
            let children = convert_markup_to_inlines(s.body(), source);
            vec![Node::new(node::STRONG).children(children)]
        }
        Expr::Emph(e) => {
            let children = convert_markup_to_inlines(e.body(), source);
            vec![Node::new(node::EMPHASIS).children(children)]
        }
        Expr::Raw(raw) => {
            let content: String = raw
                .lines()
                .map(|t| t.get().as_str().to_owned())
                .collect::<Vec<_>>()
                .join("\n");
            vec![Node::new(node::CODE).prop(prop::CONTENT, content)]
        }
        Expr::Link(link) => {
            let url = link.get().as_str().to_owned();
            vec![
                Node::new(node::LINK)
                    .prop(prop::URL, url.clone())
                    .children(vec![Node::new(node::TEXT).prop(prop::CONTENT, url)]),
            ]
        }
        Expr::Equation(eq) => {
            let math_source = eq.to_untyped().text().to_string();
            // Strip surrounding $ delimiters.
            let src = math_source.trim_matches('$').trim().to_owned();
            if eq.block() {
                vec![Node::new("math_block").prop("math:source", src)]
            } else {
                vec![Node::new("math_inline").prop("math:source", src)]
            }
        }
        Expr::FuncCall(call) => {
            if let Some(n) = convert_func_call(call, source) {
                vec![n]
            } else {
                vec![]
            }
        }
        // Block-level things shouldn't appear at inline level, but be safe.
        Expr::Parbreak(_) | Expr::Heading(_) | Expr::ListItem(_) | Expr::EnumItem(_) => vec![],
        // Everything else: emit raw with source text if non-empty.
        other => {
            let text = other.to_untyped().text().to_string();
            if text.is_empty() {
                vec![]
            } else {
                vec![
                    Node::new(node::RAW_BLOCK)
                        .prop(prop::FORMAT, "typst")
                        .prop(prop::CONTENT, text),
                ]
            }
        }
    }
}

/// Wrap a Markup body in a `LIST_ITEM` node containing a paragraph.
fn convert_list_item_body(body: Markup, source: &str) -> Node {
    let children = convert_markup_to_inlines(body, source);
    Node::new(node::LIST_ITEM).children(vec![Node::new(node::PARAGRAPH).children(children)])
}

/// Handle common Typst built-in function calls at block level.
///
/// Returns `None` for unknown functions that should be silently skipped.
fn convert_func_call(call: typst_syntax::ast::FuncCall, source: &str) -> Option<Node> {
    // The callee for simple identifiers is an Ident node; its text() is the name.
    let callee_node = call.callee().to_untyped();
    let func_name = callee_node.text().as_str();

    match func_name {
        "image" => {
            let url = first_str_arg(call.args());
            let mut n = Node::new(node::IMAGE);
            if let Some(u) = url {
                n = n.prop(prop::URL, u);
            }
            Some(n)
        }
        "link" => {
            let url = first_str_arg(call.args());
            let body_markup = first_content_arg(call.args(), source);
            let mut n = Node::new(node::LINK);
            if let Some(ref u) = url {
                n = n.prop(prop::URL, u.clone());
            }
            if let Some(children) = body_markup {
                n = n.children(children);
            } else if let Some(u) = url {
                n = n.children(vec![Node::new(node::TEXT).prop(prop::CONTENT, u)]);
            }
            Some(n)
        }
        "raw" => {
            let content = first_str_arg(call.args()).unwrap_or_default();
            Some(Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content))
        }
        "quote" => {
            let body = first_content_arg(call.args(), source).unwrap_or_default();
            Some(
                Node::new(node::BLOCKQUOTE)
                    .children(vec![Node::new(node::PARAGRAPH).children(body)]),
            )
        }
        "figure" => Some(Node::new(node::FIGURE)),
        "table" => Some(Node::new(node::TABLE)),
        "linebreak" => Some(Node::new(node::LINE_BREAK)),
        "emph" => {
            let body = first_content_arg(call.args(), source).unwrap_or_default();
            Some(Node::new(node::EMPHASIS).children(body))
        }
        "strong" => {
            let body = first_content_arg(call.args(), source).unwrap_or_default();
            Some(Node::new(node::STRONG).children(body))
        }
        _ => {
            // Unknown function — emit as raw block.
            let text = call.to_untyped().text().to_string();
            Some(
                Node::new(node::RAW_BLOCK)
                    .prop(prop::FORMAT, "typst")
                    .prop(prop::CONTENT, text),
            )
        }
    }
}

/// Extract the first positional string literal argument from a function's args.
fn first_str_arg(args: typst_syntax::ast::Args) -> Option<String> {
    for arg in args.items() {
        if let typst_syntax::ast::Arg::Pos(Expr::Str(s)) = arg {
            return Some(s.get().to_string());
        }
    }
    None
}

/// Extract the first content-block argument (returns inline nodes).
fn first_content_arg(args: typst_syntax::ast::Args, source: &str) -> Option<Vec<Node>> {
    for arg in args.items() {
        if let typst_syntax::ast::Arg::Pos(Expr::ContentBlock(cb)) = arg {
            return Some(convert_markup_to_inlines(cb.body(), source));
        }
    }
    None
}

/// Merge adjacent `LIST` nodes with the same `ordered` value.
///
/// Individual list items arrive as separate single-item LIST blocks because
/// Typst's flat markup sequence gives us one ListItem/EnumItem per step.
fn merge_adjacent_lists(blocks: Vec<Node>) -> Vec<Node> {
    let mut result: Vec<Node> = Vec::new();

    for block in blocks {
        if block.kind.as_str() == node::LIST {
            let ordered = block.props.get_bool(prop::ORDERED).unwrap_or(false);
            if let Some(last) = result.last_mut()
                && last.kind.as_str() == node::LIST
                && last.props.get_bool(prop::ORDERED).unwrap_or(false) == ordered
            {
                last.children.extend(block.children);
                continue;
            }
        }
        result.push(block);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(input: &str) -> Document {
        parse(input).unwrap().value
    }

    #[test]
    fn test_parse_heading() {
        let doc = parse_str("= Title");
        let heading = &doc.content.children[0];
        assert_eq!(heading.kind.as_str(), node::HEADING);
        assert_eq!(heading.props.get_int(prop::LEVEL), Some(1));
    }

    #[test]
    fn test_parse_heading_levels() {
        let doc = parse_str("= Level 1\n\n== Level 2\n\n=== Level 3");
        assert_eq!(doc.content.children[0].props.get_int(prop::LEVEL), Some(1));
        assert_eq!(doc.content.children[1].props.get_int(prop::LEVEL), Some(2));
        assert_eq!(doc.content.children[2].props.get_int(prop::LEVEL), Some(3));
    }

    #[test]
    fn test_parse_paragraph() {
        let doc = parse_str("Hello world!");
        let para = &doc.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
    }

    #[test]
    fn test_parse_bold() {
        let doc = parse_str("This is *bold* text.");
        let para = &doc.content.children[0];
        let strong = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::STRONG);
        assert!(strong.is_some(), "Expected a strong node in paragraph");
    }

    #[test]
    fn test_parse_italic() {
        let doc = parse_str("This is _italic_ text.");
        let para = &doc.content.children[0];
        let emph = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::EMPHASIS);
        assert!(emph.is_some(), "Expected an emphasis node in paragraph");
    }

    #[test]
    fn test_parse_code() {
        let doc = parse_str("Use `code` here.");
        let para = &doc.content.children[0];
        let code = para.children.iter().find(|c| c.kind.as_str() == node::CODE);
        assert!(code.is_some(), "Expected a code node in paragraph");
    }

    #[test]
    fn test_parse_code_block() {
        let doc = parse_str("```rust\nfn main() {}\n```");
        let code = &doc.content.children[0];
        assert_eq!(code.kind.as_str(), node::CODE_BLOCK);
        assert_eq!(code.props.get_str(prop::LANGUAGE), Some("rust"));
    }

    #[test]
    fn test_parse_list() {
        let doc = parse_str("- Item 1\n- Item 2");
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(false));
        assert_eq!(list.children.len(), 2);
    }

    #[test]
    fn test_parse_ordered_list() {
        let doc = parse_str("+ First\n+ Second");
        let list = &doc.content.children[0];
        assert_eq!(list.kind.as_str(), node::LIST);
        assert_eq!(list.props.get_bool(prop::ORDERED), Some(true));
    }

    #[test]
    fn test_parse_image() {
        let doc = parse_str("#image(\"photo.png\")");
        let img = &doc.content.children[0];
        assert_eq!(img.kind.as_str(), node::IMAGE);
        assert_eq!(img.props.get_str(prop::URL), Some("photo.png"));
    }

    #[test]
    fn test_parse_math_inline() {
        let doc = parse_str("Here is $x^2$ math.");
        let para = &doc.content.children[0];
        let math = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == "math_inline");
        assert!(math.is_some(), "Expected a math_inline node");
    }

    #[test]
    fn test_parse_link() {
        let doc = parse_str("Visit https://typst.app for info.");
        let para = &doc.content.children[0];
        let link = para.children.iter().find(|c| c.kind.as_str() == node::LINK);
        assert!(link.is_some(), "Expected a link node");
    }
}
