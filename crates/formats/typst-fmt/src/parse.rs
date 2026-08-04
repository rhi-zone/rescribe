//! AST reader: thin wrap of `typst_syntax::parse` plus a tree walk that
//! builds this crate's own [`TypstDoc`].
//!
//! `typst-syntax`'s only from-scratch parse entry points (`parse`,
//! `parse_code`, `parse_math`) require the full input `&str` — there is no
//! chunk-fed incremental parse API upstream (`reparse`/`Source::edit` are
//! edit-based *re*parsing of an already-built tree, an editor use case, not
//! applicable to parsing from nothing). This module is therefore the one
//! place `typst_syntax::parse` is called from scratch; [`crate::batch`]'s
//! `StreamingParser` and [`crate::events`]'s `EventIter` both build on top
//! of it rather than duplicating a from-scratch parse.
//!
//! The walk below relocates the AST-building logic that used to live
//! directly in `rescribe-read-typst` (a CLAUDE.md "adapter must not contain
//! parsing logic" violation) — same construct mapping, now producing
//! [`crate::ast::Block`]/[`crate::ast::Inline`] instead of rescribe `Node`s.

use typst_syntax::ast::{AstNode, Expr, Markup};

use crate::ast::{Block, Diagnostic, Inline, Span, TypstDoc};

/// Parse Typst source into this crate's [`TypstDoc`].
///
/// Infallible: syntax errors from `typst-syntax` are reported as
/// [`Diagnostic`]s, never a panic, and the walk still produces whatever
/// structure it can from the (possibly error-recovered) tree.
pub fn parse(input: &str) -> (TypstDoc, Vec<Diagnostic>) {
    let root = typst_syntax::parse(input);

    let mut diags = Vec::new();
    for err in root.errors() {
        let range = err.span.range().unwrap_or_default();
        diags.push(Diagnostic {
            message: err.message.to_string(),
            span: Span {
                start: range.start,
                end: range.end,
            },
        });
    }

    let blocks = match root.cast::<Markup>() {
        Some(markup) => convert_markup_to_blocks(markup),
        None => {
            diags.push(Diagnostic {
                message: "failed to cast parse root to Markup".to_owned(),
                span: Span::NONE,
            });
            Vec::new()
        }
    };

    let span = Span {
        start: 0,
        end: input.len(),
    };
    (TypstDoc { blocks, span }, diags)
}

/// Convert a `Markup` node to a list of block-level constructs.
///
/// Typst has no explicit paragraph node; consecutive inline exprs are
/// grouped into paragraphs, separated by `Parbreak`.
fn convert_markup_to_blocks(markup: Markup) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut inline_buf: Vec<Inline> = Vec::new();

    for expr in markup.exprs() {
        match expr {
            Expr::Parbreak(_) => {
                flush_paragraph(&mut inline_buf, &mut blocks);
            }
            Expr::Heading(h) => {
                flush_paragraph(&mut inline_buf, &mut blocks);
                let level = h.depth().get().min(255) as u8;
                let body = convert_markup_to_inlines(h.body());
                blocks.push(Block::Heading { level, body });
            }
            Expr::ListItem(item) => {
                flush_paragraph(&mut inline_buf, &mut blocks);
                let item_block = convert_list_item_body(item.body());
                blocks.push(Block::List {
                    ordered: false,
                    items: vec![item_block],
                });
            }
            Expr::EnumItem(item) => {
                flush_paragraph(&mut inline_buf, &mut blocks);
                let item_block = convert_list_item_body(item.body());
                blocks.push(Block::List {
                    ordered: true,
                    items: vec![item_block],
                });
            }
            Expr::TermItem(item) => {
                flush_paragraph(&mut inline_buf, &mut blocks);
                let term = convert_markup_to_inlines(item.term());
                let desc = convert_markup_to_inlines(item.description());
                blocks.push(Block::DefinitionList(vec![(term, desc)]));
            }
            Expr::Raw(raw) if raw.block() => {
                flush_paragraph(&mut inline_buf, &mut blocks);
                let content = raw_lines_to_string(raw);
                let lang = raw
                    .lang()
                    .map(|l| l.to_untyped().text().to_string())
                    .filter(|l| !l.is_empty());
                blocks.push(Block::CodeBlock { lang, content });
            }
            Expr::FuncCall(call) => {
                let callee_text = call.callee().to_untyped().text().to_string();
                if is_inline_func(callee_text.as_str()) {
                    if let Some(n) = convert_func_call_inline(call) {
                        inline_buf.push(n);
                    }
                } else {
                    flush_paragraph(&mut inline_buf, &mut blocks);
                    if let Some(block) = convert_func_call_block(call) {
                        blocks.push(block);
                    }
                }
            }
            other => {
                inline_buf.extend(convert_expr_to_inlines(other));
            }
        }
    }

    flush_paragraph(&mut inline_buf, &mut blocks);
    merge_adjacent_lists(blocks)
}

fn raw_lines_to_string(raw: typst_syntax::ast::Raw) -> String {
    raw.lines()
        .map(|t| t.get().as_str().to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}

fn flush_paragraph(inline_buf: &mut Vec<Inline>, blocks: &mut Vec<Block>) {
    if inline_buf.is_empty() {
        return;
    }
    let all_whitespace = inline_buf
        .iter()
        .all(|n| matches!(n, Inline::Text(t) if t.trim().is_empty()));
    if all_whitespace {
        inline_buf.clear();
        return;
    }
    blocks.push(Block::Paragraph(std::mem::take(inline_buf)));
}

fn convert_markup_to_inlines(markup: Markup) -> Vec<Inline> {
    let mut nodes = Vec::new();
    for expr in markup.exprs() {
        nodes.extend(convert_expr_to_inlines(expr));
    }
    nodes
}

fn convert_expr_to_inlines(expr: Expr) -> Vec<Inline> {
    match expr {
        Expr::Text(t) => vec![Inline::Text(t.get().as_str().to_owned())],
        Expr::Space(_) => vec![Inline::Text(" ".to_owned())],
        Expr::Linebreak(_) => vec![Inline::LineBreak],
        Expr::SmartQuote(q) => {
            let ch = if q.double() { "\"" } else { "'" };
            vec![Inline::Text(ch.to_owned())]
        }
        Expr::Escape(e) => {
            let text = e.to_untyped().text().to_string();
            let content = text.strip_prefix('\\').map(str::to_owned).unwrap_or(text);
            vec![Inline::Text(content)]
        }
        Expr::Shorthand(s) => vec![Inline::Text(s.to_untyped().text().to_string())],
        Expr::Strong(s) => vec![Inline::Strong(convert_markup_to_inlines(s.body()))],
        Expr::Emph(e) => vec![Inline::Emph(convert_markup_to_inlines(e.body()))],
        Expr::Raw(raw) => vec![Inline::Code(raw_lines_to_string(raw))],
        Expr::Link(link) => {
            let url = link.get().as_str().to_owned();
            vec![Inline::Link {
                url: url.clone(),
                body: vec![Inline::Text(url)],
            }]
        }
        Expr::Equation(eq) => {
            let math_source = eq.to_untyped().text().to_string();
            let src = math_source.trim_matches('$').trim().to_owned();
            if eq.block() {
                vec![Inline::Raw(format!("$ {src} $"))]
            } else {
                vec![Inline::MathInline(src)]
            }
        }
        Expr::FuncCall(call) => convert_func_call_inline(call).into_iter().collect(),
        Expr::Parbreak(_)
        | Expr::Heading(_)
        | Expr::ListItem(_)
        | Expr::EnumItem(_)
        | Expr::TermItem(_) => vec![],
        other => {
            let text = other.to_untyped().text().to_string();
            if text.is_empty() {
                vec![]
            } else {
                vec![Inline::Raw(text)]
            }
        }
    }
}

fn convert_list_item_body(body: Markup) -> Vec<Block> {
    vec![Block::Paragraph(convert_markup_to_inlines(body))]
}

fn is_inline_func(name: &str) -> bool {
    matches!(
        name,
        "sub"
            | "super"
            | "underline"
            | "strike"
            | "emph"
            | "strong"
            | "footnote"
            | "link"
            | "linebreak"
    )
}

/// Handle a Typst function call that appears in inline (paragraph) context.
fn convert_func_call_inline(call: typst_syntax::ast::FuncCall) -> Option<Inline> {
    let func_name = call.callee().to_untyped().text().to_string();
    match func_name.as_str() {
        "link" => {
            let url = first_str_arg(call.args());
            let body = first_content_arg(call.args());
            match (url, body) {
                (Some(u), Some(b)) => Some(Inline::Link { url: u, body: b }),
                (Some(u), None) => Some(Inline::Link {
                    url: u.clone(),
                    body: vec![Inline::Text(u)],
                }),
                _ => None,
            }
        }
        "footnote" => Some(Inline::Footnote(
            first_content_arg(call.args()).unwrap_or_default(),
        )),
        "sub" => Some(Inline::Subscript(
            first_content_arg(call.args()).unwrap_or_default(),
        )),
        "super" => Some(Inline::Superscript(
            first_content_arg(call.args()).unwrap_or_default(),
        )),
        "underline" => Some(Inline::Underline(
            first_content_arg(call.args()).unwrap_or_default(),
        )),
        "strike" => Some(Inline::Strike(
            first_content_arg(call.args()).unwrap_or_default(),
        )),
        "emph" => Some(Inline::Emph(
            first_content_arg(call.args()).unwrap_or_default(),
        )),
        "strong" => Some(Inline::Strong(
            first_content_arg(call.args()).unwrap_or_default(),
        )),
        "linebreak" => Some(Inline::LineBreak),
        _ => {
            let text = call.to_untyped().text().to_string();
            if text.is_empty() {
                None
            } else {
                Some(Inline::Raw(text))
            }
        }
    }
}

/// Handle a Typst function call that appears at block level.
fn convert_func_call_block(call: typst_syntax::ast::FuncCall) -> Option<Block> {
    let func_name = call.callee().to_untyped().text().to_string();
    match func_name.as_str() {
        "image" => first_str_arg(call.args()).map(|url| Block::Image { url }),
        "raw" => Some(Block::CodeBlock {
            lang: None,
            content: first_str_arg(call.args()).unwrap_or_default(),
        }),
        "quote" => Some(Block::Quote(vec![Block::Paragraph(
            first_content_arg(call.args()).unwrap_or_default(),
        )])),
        "figure" => {
            let mut caption: Option<Vec<Inline>> = None;
            let mut first_pos: Option<Block> = None;
            for arg in call.args().items() {
                match arg {
                    typst_syntax::ast::Arg::Named(named) if named.name().as_str() == "caption" => {
                        if let Expr::ContentBlock(cb) = named.expr() {
                            caption = Some(convert_markup_to_inlines(cb.body()));
                        }
                    }
                    typst_syntax::ast::Arg::Pos(expr) if first_pos.is_none() => {
                        first_pos = convert_func_call_expr_to_block(expr);
                    }
                    _ => {}
                }
            }
            Some(Block::Figure {
                body: first_pos.map(Box::new),
                caption,
            })
        }
        "table" => {
            let mut columns: i64 = 1;
            let mut cells: Vec<Vec<Inline>> = Vec::new();
            for arg in call.args().items() {
                match arg {
                    typst_syntax::ast::Arg::Named(named) if named.name().as_str() == "columns" => {
                        if let Expr::Int(i) = named.expr() {
                            columns = i.get();
                        }
                    }
                    typst_syntax::ast::Arg::Pos(Expr::ContentBlock(cb)) => {
                        cells.push(convert_markup_to_inlines(cb.body()));
                    }
                    _ => {}
                }
            }
            let cols = columns.max(1) as usize;
            let rows: Vec<Vec<Vec<Inline>>> = cells.chunks(cols).map(<[_]>::to_vec).collect();
            Some(Block::Table {
                columns: cols,
                rows,
            })
        }
        "linebreak" => None, // block context: not meaningful, drop silently as before
        "emph" => Some(Block::Paragraph(vec![Inline::Emph(
            first_content_arg(call.args()).unwrap_or_default(),
        )])),
        "strong" => Some(Block::Paragraph(vec![Inline::Strong(
            first_content_arg(call.args()).unwrap_or_default(),
        )])),
        _ => {
            let text = call.to_untyped().text().to_string();
            Some(Block::Raw(text))
        }
    }
}

fn first_str_arg(args: typst_syntax::ast::Args) -> Option<String> {
    for arg in args.items() {
        if let typst_syntax::ast::Arg::Pos(Expr::Str(s)) = arg {
            return Some(s.get().to_string());
        }
    }
    None
}

fn first_content_arg(args: typst_syntax::ast::Args) -> Option<Vec<Inline>> {
    for arg in args.items() {
        if let typst_syntax::ast::Arg::Pos(Expr::ContentBlock(cb)) = arg {
            return Some(convert_markup_to_inlines(cb.body()));
        }
    }
    None
}

fn convert_func_call_expr_to_block(expr: Expr) -> Option<Block> {
    match expr {
        Expr::FuncCall(call) => convert_func_call_block(call),
        Expr::ContentBlock(cb) => Some(Block::Paragraph(convert_markup_to_inlines(cb.body()))),
        other => {
            let inlines = convert_expr_to_inlines(other);
            if inlines.is_empty() {
                None
            } else {
                Some(Block::Paragraph(inlines))
            }
        }
    }
}

/// Merge adjacent `List` blocks with the same `ordered` value, and adjacent
/// `DefinitionList` blocks — individual items arrive as separate
/// single-item blocks because Typst's flat markup sequence gives us one
/// item per step.
fn merge_adjacent_lists(blocks: Vec<Block>) -> Vec<Block> {
    let mut result: Vec<Block> = Vec::new();

    for block in blocks {
        match block {
            Block::List { ordered, items } => {
                if let Some(Block::List {
                    ordered: last_ordered,
                    items: last_items,
                }) = result.last_mut()
                    && *last_ordered == ordered
                {
                    last_items.extend(items);
                    continue;
                }
                result.push(Block::List { ordered, items });
            }
            Block::DefinitionList(entries) => {
                if let Some(Block::DefinitionList(last_entries)) = result.last_mut() {
                    last_entries.extend(entries);
                    continue;
                }
                result.push(Block::DefinitionList(entries));
            }
            other => result.push(other),
        }
    }

    result
}
