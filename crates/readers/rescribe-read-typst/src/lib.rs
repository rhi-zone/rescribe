//! Typst reader for rescribe.
//!
//! Parses Typst markup into rescribe documents using the official `typst-syntax` crate.
//!
//! # Features
//! - `syntax` (default): fast parse-only path using `typst-syntax` AST directly
//! - `eval`: full compiler path via the `typst` crate; adds `parse_evaluated()`

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
            Expr::TermItem(item) => {
                flush_paragraph(&mut inline_buf, &mut blocks);
                let term_children = convert_markup_to_inlines(item.term(), source);
                let desc_children = convert_markup_to_inlines(item.description(), source);
                let term_node = Node::new(node::DEFINITION_TERM).children(term_children);
                let desc_node = Node::new(node::DEFINITION_DESC).children(desc_children);
                blocks.push(Node::new(node::DEFINITION_LIST).children(vec![term_node, desc_node]));
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
                // Some functions are inherently inline; route them to the inline
                // buffer rather than flushing the current paragraph.
                let callee_text = call.callee().to_untyped().text().to_string();
                if is_inline_func(callee_text.as_str()) {
                    if let Some(n) = convert_func_call(call, source) {
                        inline_buf.push(n);
                    }
                } else {
                    flush_paragraph(&mut inline_buf, &mut blocks);
                    if let Some(block) = convert_func_call(call, source) {
                        blocks.push(block);
                    }
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
        Expr::Parbreak(_)
        | Expr::Heading(_)
        | Expr::ListItem(_)
        | Expr::EnumItem(_)
        | Expr::TermItem(_) => vec![],
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

/// Returns true if the named Typst function should be treated as inline content.
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
        "footnote" => {
            let body = first_content_arg(call.args(), source).unwrap_or_default();
            Some(Node::new(node::FOOTNOTE_DEF).children(body))
        }
        "sub" => {
            let body = first_content_arg(call.args(), source).unwrap_or_default();
            Some(Node::new(node::SUBSCRIPT).children(body))
        }
        "super" => {
            let body = first_content_arg(call.args(), source).unwrap_or_default();
            Some(Node::new(node::SUPERSCRIPT).children(body))
        }
        "underline" => {
            let body = first_content_arg(call.args(), source).unwrap_or_default();
            Some(Node::new(node::UNDERLINE).children(body))
        }
        "strike" => {
            let body = first_content_arg(call.args(), source).unwrap_or_default();
            Some(Node::new(node::STRIKEOUT).children(body))
        }
        "figure" => {
            let mut figure = Node::new(node::FIGURE);
            let mut caption_children: Option<Vec<Node>> = None;
            let mut first_pos: Option<Node> = None;
            for arg in call.args().items() {
                match arg {
                    typst_syntax::ast::Arg::Named(named) if named.name().as_str() == "caption" => {
                        if let Expr::ContentBlock(cb) = named.expr() {
                            caption_children = Some(convert_markup_to_inlines(cb.body(), source));
                        }
                    }
                    typst_syntax::ast::Arg::Pos(expr) if first_pos.is_none() => {
                        if let Some(n) = convert_func_call_expr(expr, source) {
                            first_pos = Some(n);
                        }
                    }
                    _ => {}
                }
            }
            let mut children = Vec::new();
            if let Some(img) = first_pos {
                children.push(img);
            }
            if let Some(cap) = caption_children {
                children.push(Node::new(node::PARAGRAPH).children(cap));
            }
            figure = figure.children(children);
            Some(figure)
        }
        "table" => {
            let mut columns: i64 = 1;
            let mut cells: Vec<Node> = Vec::new();
            for arg in call.args().items() {
                match arg {
                    typst_syntax::ast::Arg::Named(named) if named.name().as_str() == "columns" => {
                        if let Expr::Int(i) = named.expr() {
                            columns = i.get();
                        }
                    }
                    typst_syntax::ast::Arg::Pos(Expr::ContentBlock(cb)) => {
                        let cell_children = convert_markup_to_inlines(cb.body(), source);
                        cells.push(Node::new(node::TABLE_CELL).children(cell_children));
                    }
                    _ => {}
                }
            }
            // Build rows from flat cell list using column count.
            let rows: Vec<Node> = if columns > 0 {
                cells
                    .chunks(columns as usize)
                    .map(|row_cells| Node::new(node::TABLE_ROW).children(row_cells.to_vec()))
                    .collect()
            } else {
                // Fall back: emit all cells directly.
                cells
            };
            Some(
                Node::new(node::TABLE)
                    .prop("columns", columns)
                    .children(rows),
            )
        }
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

/// Convert a positional `Expr` in a function call position to a rescribe node,
/// used when extracting figure body content.
fn convert_func_call_expr(expr: Expr, source: &str) -> Option<Node> {
    match expr {
        Expr::FuncCall(call) => convert_func_call(call, source),
        Expr::ContentBlock(cb) => {
            let children = convert_markup_to_inlines(cb.body(), source);
            Some(Node::new(node::PARAGRAPH).children(children))
        }
        other => {
            let inlines = convert_expr_to_inlines(other, source);
            if inlines.is_empty() {
                None
            } else {
                Some(Node::new(node::PARAGRAPH).children(inlines))
            }
        }
    }
}

/// Merge adjacent `LIST` nodes with the same `ordered` value, and adjacent
/// `DEFINITION_LIST` nodes.
///
/// Individual list items arrive as separate single-item LIST / DEFINITION_LIST
/// blocks because Typst's flat markup sequence gives us one item per step.
fn merge_adjacent_lists(blocks: Vec<Node>) -> Vec<Node> {
    let mut result: Vec<Node> = Vec::new();

    for block in blocks {
        let kind = block.kind.as_str();
        if kind == node::LIST {
            let ordered = block.props.get_bool(prop::ORDERED).unwrap_or(false);
            if let Some(last) = result.last_mut()
                && last.kind.as_str() == node::LIST
                && last.props.get_bool(prop::ORDERED).unwrap_or(false) == ordered
            {
                last.children.extend(block.children);
                continue;
            }
        } else if kind == node::DEFINITION_LIST
            && let Some(last) = result.last_mut()
            && last.kind.as_str() == node::DEFINITION_LIST
        {
            last.children.extend(block.children);
            continue;
        }
        result.push(block);
    }

    result
}

// ---------------------------------------------------------------------------
// Eval path (requires `eval` feature)
// ---------------------------------------------------------------------------

#[cfg(feature = "eval")]
mod eval_impl {
    use std::path::PathBuf;

    use typst::Feature;
    use typst::diag::{FileError, FileResult};
    use typst::foundations::{Bytes, Datetime};
    use typst::text::{Font, FontBook};
    use typst::utils::LazyHash;
    use typst::{Library, LibraryExt, World};
    use typst_kit::fonts::FontSearcher;
    use typst_syntax::{FileId, Source, VirtualPath};

    use rescribe_core::{
        ConversionResult, Document, FidelityWarning, Node, ParseError, Severity, WarningKind,
    };
    use rescribe_std::{node, prop};

    use typst_html::{HtmlDocument, HtmlElement, HtmlNode};

    /// A minimal `World` implementation for single-file in-memory compilation.
    pub struct MinimalWorld {
        library: LazyHash<Library>,
        font_book: LazyHash<FontBook>,
        fonts: Vec<typst_kit::fonts::FontSlot>,
        source: Source,
        main_id: FileId,
    }

    impl MinimalWorld {
        pub fn new(text: &str) -> Self {
            let library = LazyHash::new(
                Library::builder()
                    .with_features(std::iter::once(Feature::Html).collect())
                    .build(),
            );
            let main_id = FileId::new_fake(VirtualPath::new("input.typ"));
            let source = Source::new(main_id, text.to_string());
            let searched = FontSearcher::new()
                .include_system_fonts(false)
                .search();
            Self {
                library,
                font_book: LazyHash::new(searched.book),
                fonts: searched.fonts,
                source,
                main_id,
            }
        }
    }

    impl World for MinimalWorld {
        fn library(&self) -> &LazyHash<Library> {
            &self.library
        }

        fn book(&self) -> &LazyHash<FontBook> {
            &self.font_book
        }

        fn main(&self) -> FileId {
            self.main_id
        }

        fn source(&self, id: FileId) -> FileResult<Source> {
            if id == self.main_id {
                Ok(self.source.clone())
            } else {
                Err(FileError::NotFound(PathBuf::from(
                    id.vpath().as_rootless_path(),
                )))
            }
        }

        fn file(&self, id: FileId) -> FileResult<Bytes> {
            Err(FileError::NotFound(PathBuf::from(
                id.vpath().as_rootless_path(),
            )))
        }

        fn font(&self, index: usize) -> Option<Font> {
            self.fonts.get(index)?.get()
        }

        fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
            None
        }
    }

    /// Walk the root `HtmlDocument` and produce a flat list of IR block nodes.
    pub fn convert_html_doc_to_nodes(html_doc: &HtmlDocument) -> Vec<Node> {
        // The root is always `<html>` containing `<head>` and `<body>`.
        // We skip head/html wrapper and walk body children directly.
        let mut blocks = Vec::new();
        collect_html_blocks(&html_doc.root, &mut blocks);
        blocks
    }

    /// Recursively collect block-level nodes from an `HtmlElement`.
    fn collect_html_blocks(elem: &HtmlElement, out: &mut Vec<Node>) {
        let tag = elem.tag.resolve();
        let tag_str = tag.as_str();
        match tag_str {
            "html" | "body" => {
                // Transparent containers — descend into children.
                for child in &elem.children {
                    collect_html_node_blocks(child, out);
                }
            }
            "head" => { /* skip head entirely */ }
            _ => {
                if let Some(n) = convert_html_element(elem) {
                    out.push(n);
                }
            }
        }
    }

    fn collect_html_node_blocks(node: &HtmlNode, out: &mut Vec<Node>) {
        match node {
            HtmlNode::Element(elem) => collect_html_blocks(elem, out),
            HtmlNode::Text(text, _) => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    out.push(
                        Node::new(node::PARAGRAPH).children(vec![
                            Node::new(node::TEXT).prop(prop::CONTENT, trimmed),
                        ]),
                    );
                }
            }
            HtmlNode::Tag(_) | HtmlNode::Frame(_) => {}
        }
    }

    /// Convert a single `HtmlElement` to a rescribe `Node`.
    fn convert_html_element(elem: &HtmlElement) -> Option<Node> {
        let tag = elem.tag.resolve();
        let tag_str = tag.as_str();
        match tag_str {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = (tag_str.as_bytes()[1] - b'0') as i64;
                let children = collect_inline_children(elem);
                Some(
                    Node::new(node::HEADING)
                        .prop(prop::LEVEL, level)
                        .children(children),
                )
            }
            "p" => {
                let children = collect_inline_children(elem);
                Some(Node::new(node::PARAGRAPH).children(children))
            }
            "ul" => {
                let items = collect_list_items(elem);
                Some(
                    Node::new(node::LIST)
                        .prop(prop::ORDERED, false)
                        .children(items),
                )
            }
            "ol" => {
                let items = collect_list_items(elem);
                Some(
                    Node::new(node::LIST)
                        .prop(prop::ORDERED, true)
                        .children(items),
                )
            }
            "li" => {
                let children = collect_inline_children(elem);
                Some(
                    Node::new(node::LIST_ITEM)
                        .children(vec![Node::new(node::PARAGRAPH).children(children)]),
                )
            }
            "pre" => {
                let text = extract_text_content(elem);
                Some(Node::new(node::CODE_BLOCK).prop(prop::CONTENT, text))
            }
            "blockquote" => {
                let mut inner = Vec::new();
                for child in &elem.children {
                    collect_html_node_blocks(child, &mut inner);
                }
                if inner.is_empty() {
                    let text = extract_text_content(elem);
                    inner.push(
                        Node::new(node::PARAGRAPH)
                            .children(vec![Node::new(node::TEXT).prop(prop::CONTENT, text)]),
                    );
                }
                Some(Node::new(node::BLOCKQUOTE).children(inner))
            }
            "table" => {
                let rows = collect_table_rows(elem);
                Some(Node::new(node::TABLE).children(rows))
            }
            "figure" => {
                let mut children = Vec::new();
                for child in &elem.children {
                    if let HtmlNode::Element(child_elem) = child {
                        let child_tag = child_elem.tag.resolve();
                        match child_tag.as_str() {
                            "figcaption" => {
                                let cap_children = collect_inline_children(child_elem);
                                children.push(
                                    Node::new(node::PARAGRAPH).children(cap_children),
                                );
                            }
                            _ => {
                                if let Some(n) = convert_html_element(child_elem) {
                                    children.push(n);
                                }
                            }
                        }
                    }
                }
                Some(Node::new(node::FIGURE).children(children))
            }
            "hr" => Some(Node::new(node::HORIZONTAL_RULE)),
            "div" | "section" | "article" | "main" | "header" | "footer" | "nav" | "aside" => {
                // Generic containers: collect block children.
                let mut inner = Vec::new();
                for child in &elem.children {
                    collect_html_node_blocks(child, &mut inner);
                }
                Some(Node::new(node::DIV).children(inner))
            }
            _ => {
                // Try as an inline element wrapped in a paragraph.
                if let Some(inline) = convert_html_inline(elem) {
                    Some(
                        Node::new(node::PARAGRAPH)
                            .children(vec![inline]),
                    )
                } else {
                    None
                }
            }
        }
    }

    /// Convert an `HtmlElement` to an inline rescribe `Node`.
    fn convert_html_inline(elem: &HtmlElement) -> Option<Node> {
        let tag = elem.tag.resolve();
        let tag_str = tag.as_str();
        match tag_str {
            "strong" | "b" => {
                let children = collect_inline_children(elem);
                Some(Node::new(node::STRONG).children(children))
            }
            "em" | "i" => {
                let children = collect_inline_children(elem);
                Some(Node::new(node::EMPHASIS).children(children))
            }
            "code" => {
                let text = extract_text_content(elem);
                Some(Node::new(node::CODE).prop(prop::CONTENT, text))
            }
            "u" => {
                let children = collect_inline_children(elem);
                Some(Node::new(node::UNDERLINE).children(children))
            }
            "s" | "del" => {
                let children = collect_inline_children(elem);
                Some(Node::new(node::STRIKEOUT).children(children))
            }
            "sub" => {
                let children = collect_inline_children(elem);
                Some(Node::new(node::SUBSCRIPT).children(children))
            }
            "sup" => {
                let children = collect_inline_children(elem);
                Some(Node::new(node::SUPERSCRIPT).children(children))
            }
            "a" => {
                let href = elem
                    .attrs
                    .0
                    .iter()
                    .find(|(k, _)| k.resolve().as_str() == "href")
                    .map(|(_, v)| v.as_str().to_owned())
                    .unwrap_or_default();
                let children = collect_inline_children(elem);
                let display = if children.is_empty() {
                    vec![Node::new(node::TEXT).prop(prop::CONTENT, href.clone())]
                } else {
                    children
                };
                Some(
                    Node::new(node::LINK)
                        .prop(prop::URL, href)
                        .children(display),
                )
            }
            "img" => {
                let src = elem
                    .attrs
                    .0
                    .iter()
                    .find(|(k, _)| k.resolve().as_str() == "src")
                    .map(|(_, v)| v.as_str().to_owned())
                    .unwrap_or_default();
                let alt = elem
                    .attrs
                    .0
                    .iter()
                    .find(|(k, _)| k.resolve().as_str() == "alt")
                    .map(|(_, v)| v.as_str().to_owned());
                let mut n = Node::new(node::IMAGE).prop(prop::URL, src);
                if let Some(alt_text) = alt {
                    n = n.prop(prop::ALT, alt_text);
                }
                Some(n)
            }
            "br" => Some(Node::new(node::LINE_BREAK)),
            "span" => {
                let children = collect_inline_children(elem);
                if children.is_empty() {
                    None
                } else {
                    Some(Node::new(node::SPAN).children(children))
                }
            }
            _ => None,
        }
    }

    /// Collect inline children of an element (text + inline elements).
    fn collect_inline_children(elem: &HtmlElement) -> Vec<Node> {
        let mut nodes = Vec::new();
        for child in &elem.children {
            match child {
                HtmlNode::Text(text, _) => {
                    if !text.is_empty() {
                        nodes.push(Node::new(node::TEXT).prop(prop::CONTENT, text.as_str()));
                    }
                }
                HtmlNode::Element(child_elem) => {
                    if let Some(inline) = convert_html_inline(child_elem) {
                        nodes.push(inline);
                    } else if let Some(block) = convert_html_element(child_elem) {
                        // Block nested in inline context — unwrap if paragraph, else raw.
                        nodes.push(block);
                    }
                }
                HtmlNode::Tag(_) | HtmlNode::Frame(_) => {}
            }
        }
        nodes
    }

    /// Extract all text content from an element recursively.
    fn extract_text_content(elem: &HtmlElement) -> String {
        let mut buf = String::new();
        extract_text_recursive(&elem.children, &mut buf);
        buf
    }

    fn extract_text_recursive(children: &[HtmlNode], buf: &mut String) {
        for child in children {
            match child {
                HtmlNode::Text(text, _) => buf.push_str(text.as_str()),
                HtmlNode::Element(elem) => extract_text_recursive(&elem.children, buf),
                HtmlNode::Tag(_) | HtmlNode::Frame(_) => {}
            }
        }
    }

    /// Collect `<li>` items from a list element.
    fn collect_list_items(elem: &HtmlElement) -> Vec<Node> {
        let mut items = Vec::new();
        for child in &elem.children {
            if let HtmlNode::Element(child_elem) = child {
                let tag = child_elem.tag.resolve();
                if tag.as_str() == "li" {
                    let children = collect_inline_children(child_elem);
                    items.push(
                        Node::new(node::LIST_ITEM)
                            .children(vec![Node::new(node::PARAGRAPH).children(children)]),
                    );
                }
            }
        }
        items
    }

    /// Collect table rows from a `<table>` element.
    fn collect_table_rows(elem: &HtmlElement) -> Vec<Node> {
        let mut rows = Vec::new();
        for child in &elem.children {
            if let HtmlNode::Element(child_elem) = child {
                let tag = child_elem.tag.resolve();
                match tag.as_str() {
                    "tr" => {
                        rows.push(convert_table_row(child_elem));
                    }
                    "thead" | "tbody" | "tfoot" => {
                        // Recurse into section containers.
                        for inner in &child_elem.children {
                            if let HtmlNode::Element(row_elem) = inner {
                                let row_tag = row_elem.tag.resolve();
                                if row_tag.as_str() == "tr" {
                                    rows.push(convert_table_row(row_elem));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        rows
    }

    fn convert_table_row(elem: &HtmlElement) -> Node {
        let mut cells = Vec::new();
        for child in &elem.children {
            if let HtmlNode::Element(child_elem) = child {
                let tag = child_elem.tag.resolve();
                let kind = match tag.as_str() {
                    "th" => node::TABLE_HEADER,
                    "td" => node::TABLE_CELL,
                    _ => continue,
                };
                let children = collect_inline_children(child_elem);
                cells.push(Node::new(kind).children(children));
            }
        }
        Node::new(node::TABLE_ROW).children(cells)
    }

    /// Parse Typst source through the full compiler, resolving `#let`, `#for`, `#if`,
    /// show rules, etc., before converting to rescribe IR.
    ///
    /// Falls back to the syntax-only `parse()` result (with a warning attached) if
    /// compilation fails.
    pub fn parse_evaluated(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        let world = MinimalWorld::new(input);
        let result = typst::compile::<HtmlDocument>(&world);

        let warning_msgs: Vec<String> = result
            .warnings
            .iter()
            .map(|w| w.message.to_string())
            .collect();

        match result.output {
            Ok(html_doc) => {
                let blocks = convert_html_doc_to_nodes(&html_doc);
                let doc = Document::new().with_content(
                    Node::new(node::DOCUMENT).children(blocks),
                );
                let mut cr = ConversionResult::ok(doc);
                for w in warning_msgs {
                    cr = cr.warn(FidelityWarning::new(
                        Severity::Info,
                        WarningKind::FeatureLost("typst-compile-warning".to_owned()),
                        w,
                    ));
                }
                Ok(cr)
            }
            Err(errors) => {
                // Compilation failed — fall back to syntax-only parse with warnings.
                let mut cr = super::parse(input)?;
                for e in errors.iter() {
                    cr = cr.warn(FidelityWarning::new(
                        Severity::Major,
                        WarningKind::FeatureLost("typst-compile-error".to_owned()),
                        format!("typst compile error: {}", e.message),
                    ));
                }
                for w in warning_msgs {
                    cr = cr.warn(FidelityWarning::new(
                        Severity::Info,
                        WarningKind::FeatureLost("typst-compile-warning".to_owned()),
                        w,
                    ));
                }
                Ok(cr)
            }
        }
    }
}

/// Parse Typst source through the full typst compiler (resolving `#let`, `#for`, `#if`,
/// show rules, etc.) before converting to the rescribe IR.
///
/// On compilation failure, falls back to the syntax-only result with errors attached as
/// fidelity warnings.
///
/// Only available with the `eval` feature.
#[cfg(feature = "eval")]
pub fn parse_evaluated(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    eval_impl::parse_evaluated(input)
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

    #[test]
    fn test_parse_term_list() {
        let doc = parse_str("/ Rust: A systems language\n/ Python: A scripting language");
        assert_eq!(
            doc.content.children.len(),
            1,
            "Adjacent term items should merge"
        );
        let dl = &doc.content.children[0];
        assert_eq!(dl.kind.as_str(), node::DEFINITION_LIST);
        // Two terms merged: 4 children total (term+desc, term+desc).
        assert_eq!(dl.children.len(), 4);
        assert_eq!(dl.children[0].kind.as_str(), node::DEFINITION_TERM);
        assert_eq!(dl.children[1].kind.as_str(), node::DEFINITION_DESC);
    }

    #[test]
    fn test_parse_footnote() {
        let doc = parse_str("#footnote[A note here]");
        // Footnotes are inline; they end up inside a paragraph.
        let para = &doc.content.children[0];
        assert_eq!(para.kind.as_str(), node::PARAGRAPH);
        let footnote = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::FOOTNOTE_DEF);
        assert!(
            footnote.is_some(),
            "Expected a footnote_def node in paragraph"
        );
        assert!(!footnote.unwrap().children.is_empty());
    }

    #[test]
    fn test_parse_sub_super() {
        let doc = parse_str("#sub[2] and #super[3]");
        let para = &doc.content.children[0];
        let sub = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::SUBSCRIPT);
        assert!(sub.is_some(), "Expected subscript node");
        let sup = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::SUPERSCRIPT);
        assert!(sup.is_some(), "Expected superscript node");
    }

    #[test]
    fn test_parse_underline_strike() {
        let doc = parse_str("#underline[hello] and #strike[world]");
        let para = &doc.content.children[0];
        let u = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::UNDERLINE);
        assert!(u.is_some(), "Expected underline node");
        let s = para
            .children
            .iter()
            .find(|c| c.kind.as_str() == node::STRIKEOUT);
        assert!(s.is_some(), "Expected strikeout node");
    }

    #[test]
    fn test_parse_table() {
        let doc = parse_str("#table(columns: 2, [A], [B], [C], [D])");
        let table = &doc.content.children[0];
        assert_eq!(table.kind.as_str(), node::TABLE);
        // 4 cells / 2 columns = 2 rows
        assert_eq!(table.children.len(), 2, "Expected 2 rows");
        assert_eq!(table.children[0].kind.as_str(), node::TABLE_ROW);
        assert_eq!(
            table.children[0].children.len(),
            2,
            "Expected 2 cells per row"
        );
    }

    #[test]
    fn test_parse_figure() {
        let doc = parse_str("#figure(image(\"cat.png\"), caption: [A cat])");
        let figure = &doc.content.children[0];
        assert_eq!(figure.kind.as_str(), node::FIGURE);
        // First child should be an image.
        assert_eq!(figure.children[0].kind.as_str(), node::IMAGE);
        // Second child should be a paragraph (caption).
        assert_eq!(figure.children[1].kind.as_str(), node::PARAGRAPH);
    }

    #[cfg(feature = "eval")]
    mod eval_tests {
        use super::super::*;

        #[test]
        fn test_parse_evaluated_basic() {
            let result = parse_evaluated("= Hello\n\nWorld paragraph.").unwrap();
            let doc = &result.value;
            // Should have at least a heading and a paragraph.
            assert!(
                doc.content.children.len() >= 2,
                "Expected at least heading + paragraph, got: {:?}",
                doc.content
                    .children
                    .iter()
                    .map(|n| n.kind.as_str())
                    .collect::<Vec<_>>()
            );
        }

        #[test]
        fn test_parse_evaluated_let_binding() {
            // #let resolves at eval time; the text "Alice" should appear.
            let result = parse_evaluated("#let name = \"Alice\"\nHello, #name!").unwrap();
            let doc = &result.value;
            // Walk all text nodes to find "Alice".
            fn has_text(node: &rescribe_core::Node, needle: &str) -> bool {
                if node.kind.as_str() == rescribe_std::node::TEXT {
                    if let Some(content) = node.props.get_str(rescribe_std::prop::CONTENT) {
                        if content.contains(needle) {
                            return true;
                        }
                    }
                }
                node.children.iter().any(|c| has_text(c, needle))
            }
            assert!(
                has_text(&doc.content, "Alice"),
                "Expected evaluated text 'Alice' in document"
            );
        }

        #[test]
        fn test_parse_evaluated_fallback_on_error() {
            // Intentionally broken typst (missing closing brace) should not panic;
            // it should return a ConversionResult (possibly falling back to syntax parse).
            let result = parse_evaluated("= Heading\n\n#let x = {");
            assert!(result.is_ok(), "Should not error even on broken typst");
        }
    }
}
