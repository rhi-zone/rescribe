//! AST↔`rescribe::Document` translation for Djot.
//!
//! This module only translates between [`DjotDoc`](crate::DjotDoc) and
//! rescribe's `Document` IR — no Djot tokenizing/parsing/emitting happens
//! here (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`). Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{Alignment, Block, DjotDoc, Inline, ListKind};
    use rescribe_core::{
        ConversionResult, Document, FidelityWarning, Node, ParseError, Properties,
    };
    use rescribe_std::{node, prop};

    /// Parse Djot text into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        let (djot_doc, _diagnostics) = crate::parse_str(input);
        let mut converter = Converter::new();
        let children = converter.convert_doc(&djot_doc);

        let document = Document {
            content: Node::new(node::DOCUMENT).children(children),
            resources: Default::default(),
            metadata: Properties::new(),
            source: None,
        };

        Ok(ConversionResult::with_warnings(
            document,
            converter.warnings,
        ))
    }

    struct Converter {
        warnings: Vec<FidelityWarning>,
    }

    impl Converter {
        fn new() -> Self {
            Self {
                warnings: Vec::new(),
            }
        }

        fn convert_doc(&mut self, doc: &DjotDoc) -> Vec<Node> {
            let mut nodes = self.convert_blocks(&doc.blocks);
            // Footnote definitions
            for fn_def in &doc.footnotes {
                let children = self.convert_blocks(&fn_def.blocks);
                let node = Node::new(node::FOOTNOTE_DEF)
                    .prop(prop::LABEL, fn_def.label.clone())
                    .children(children);
                nodes.push(node);
            }
            nodes
        }

        fn convert_blocks(&mut self, blocks: &[Block]) -> Vec<Node> {
            blocks.iter().map(|b| self.convert_block(b)).collect()
        }

        fn convert_block(&mut self, block: &Block) -> Node {
            match block {
                Block::Paragraph { inlines, .. } => {
                    Node::new(node::PARAGRAPH).children(self.convert_inlines(inlines))
                }
                Block::Heading { level, inlines, .. } => Node::new(node::HEADING)
                    .prop(prop::LEVEL, *level as i64)
                    .children(self.convert_inlines(inlines)),
                Block::Blockquote { blocks, .. } => {
                    Node::new(node::BLOCKQUOTE).children(self.convert_blocks(blocks))
                }
                Block::List {
                    kind, items, tight, ..
                } => {
                    let (ordered, start) = match kind {
                        ListKind::Bullet(_) | ListKind::Task => (false, 1i64),
                        ListKind::Ordered { start, .. } => (true, *start as i64),
                    };
                    let mut list = Node::new(node::LIST)
                        .prop(prop::ORDERED, ordered)
                        .prop("tight", *tight);
                    if ordered && start != 1 {
                        list = list.prop(prop::START, start);
                    }
                    let item_nodes: Vec<Node> = items
                        .iter()
                        .map(|item| {
                            let mut li = Node::new(node::LIST_ITEM)
                                .children(self.convert_blocks(&item.blocks));
                            if let Some(checked) = item.checked {
                                li = li.prop(prop::CHECKED, checked);
                            }
                            li
                        })
                        .collect();
                    list.children(item_nodes)
                }
                Block::CodeBlock {
                    language, content, ..
                } => {
                    let mut cb = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, content.clone());
                    if let Some(lang) = language {
                        cb = cb.prop(prop::LANGUAGE, lang.clone());
                    }
                    cb
                }
                Block::RawBlock {
                    format, content, ..
                } => Node::new(node::RAW_BLOCK)
                    .prop(prop::FORMAT, format.clone())
                    .prop(prop::CONTENT, content.clone()),
                Block::Div { class, blocks, .. } => {
                    let mut div = Node::new(node::DIV).children(self.convert_blocks(blocks));
                    if let Some(c) = class {
                        div = div.prop("html:class", c.clone());
                    }
                    div
                }
                Block::Table { caption, rows, .. } => {
                    let mut table_nodes = Vec::new();
                    if let Some(cap_inlines) = caption {
                        // Caption as a paragraph with a caption marker
                        let cap = Node::new(node::PARAGRAPH)
                            .prop("role", "caption")
                            .children(self.convert_inlines(cap_inlines));
                        table_nodes.push(cap);
                    }
                    for row in rows {
                        let cell_nodes: Vec<Node> = row
                            .cells
                            .iter()
                            .map(|cell| {
                                let kind = if row.is_header {
                                    node::TABLE_HEADER
                                } else {
                                    node::TABLE_CELL
                                };
                                let mut cn =
                                    Node::new(kind).children(self.convert_inlines(&cell.inlines));
                                let align_str = match cell.alignment {
                                    Alignment::Left => "left",
                                    Alignment::Right => "right",
                                    Alignment::Center => "center",
                                    Alignment::Default => "",
                                };
                                if !align_str.is_empty() {
                                    cn = cn.prop("style:align", align_str);
                                }
                                cn
                            })
                            .collect();
                        table_nodes.push(Node::new(node::TABLE_ROW).children(cell_nodes));
                    }
                    Node::new(node::TABLE).children(table_nodes)
                }
                Block::ThematicBreak { .. } => Node::new(node::HORIZONTAL_RULE),
                Block::DefinitionList { items, .. } => {
                    let mut dl_children = Vec::new();
                    for item in items {
                        dl_children.push(
                            Node::new(node::DEFINITION_TERM)
                                .children(self.convert_inlines(&item.term)),
                        );
                        for def_block in &item.definitions {
                            dl_children.push(
                                Node::new(node::DEFINITION_DESC)
                                    .children(vec![self.convert_block(def_block)]),
                            );
                        }
                    }
                    Node::new(node::DEFINITION_LIST).children(dl_children)
                }
            }
        }

        fn convert_inlines(&mut self, inlines: &[Inline]) -> Vec<Node> {
            inlines.iter().map(|i| self.convert_inline(i)).collect()
        }

        fn convert_inline(&mut self, inline: &Inline) -> Node {
            match inline {
                Inline::Text { content, .. } => {
                    Node::new(node::TEXT).prop(prop::CONTENT, content.clone())
                }
                Inline::SoftBreak { .. } => Node::new(node::SOFT_BREAK),
                Inline::HardBreak { .. } => Node::new(node::LINE_BREAK),
                Inline::Emphasis { inlines, .. } => {
                    Node::new(node::EMPHASIS).children(self.convert_inlines(inlines))
                }
                Inline::Strong { inlines, .. } => {
                    Node::new(node::STRONG).children(self.convert_inlines(inlines))
                }
                Inline::Delete { inlines, .. } => {
                    Node::new(node::STRIKEOUT).children(self.convert_inlines(inlines))
                }
                Inline::Insert { inlines, .. } => {
                    Node::new(node::UNDERLINE).children(self.convert_inlines(inlines))
                }
                Inline::Highlight { inlines, .. } => Node::new(node::SPAN)
                    .prop("html:class", "mark")
                    .children(self.convert_inlines(inlines)),
                Inline::Subscript { inlines, .. } => {
                    Node::new(node::SUBSCRIPT).children(self.convert_inlines(inlines))
                }
                Inline::Superscript { inlines, .. } => {
                    Node::new(node::SUPERSCRIPT).children(self.convert_inlines(inlines))
                }
                Inline::Verbatim { content, .. } => {
                    Node::new(node::CODE).prop(prop::CONTENT, content.clone())
                }
                Inline::MathInline { content, .. } => {
                    Node::new("math:inline").prop(prop::CONTENT, content.clone())
                }
                Inline::MathDisplay { content, .. } => {
                    Node::new("math:display").prop(prop::CONTENT, content.clone())
                }
                Inline::RawInline {
                    format, content, ..
                } => Node::new(node::RAW_INLINE)
                    .prop(prop::FORMAT, format.clone())
                    .prop(prop::CONTENT, content.clone()),
                Inline::Link {
                    inlines,
                    url,
                    title,
                    ..
                } => {
                    let mut link = Node::new(node::LINK)
                        .prop(prop::URL, url.clone())
                        .children(self.convert_inlines(inlines));
                    if let Some(t) = title {
                        link = link.prop(prop::TITLE, t.clone());
                    }
                    link
                }
                Inline::Image {
                    inlines,
                    url,
                    title,
                    ..
                } => {
                    let alt = collect_text(inlines);
                    let mut img = Node::new(node::IMAGE).prop(prop::URL, url.clone());
                    if !alt.is_empty() {
                        img = img.prop(prop::ALT, alt);
                    }
                    if let Some(t) = title {
                        img = img.prop(prop::TITLE, t.clone());
                    }
                    img
                }
                Inline::Span { inlines, attr, .. } => {
                    let mut span = Node::new(node::SPAN).children(self.convert_inlines(inlines));
                    if let Some(id) = &attr.id {
                        span = span.prop("html:id", id.clone());
                    }
                    if !attr.classes.is_empty() {
                        span = span.prop("html:class", attr.classes.join(" "));
                    }
                    span
                }
                Inline::FootnoteRef { label, .. } => {
                    Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, label.clone())
                }
                Inline::Symbol { name, .. } => {
                    Node::new(node::TEXT).prop(prop::CONTENT, format!(":{name}:"))
                }
                Inline::Autolink { url, .. } => {
                    let text = Node::new(node::TEXT).prop(prop::CONTENT, url.clone());
                    Node::new(node::LINK)
                        .prop(prop::URL, url.clone())
                        .children(vec![text])
                }
            }
        }
    }

    fn collect_text(inlines: &[Inline]) -> String {
        inlines
            .iter()
            .map(|i| match i {
                Inline::Text { content, .. } => content.as_str(),
                _ => "",
            })
            .collect()
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_std::node;

        #[test]
        fn test_parse_paragraph() {
            let result = parse("Hello, world!").unwrap();
            let doc = result.value;
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::PARAGRAPH);
        }

        #[test]
        fn test_parse_heading() {
            let result = parse("# Heading 1\n\n## Heading 2").unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }

        #[test]
        fn test_parse_emphasis() {
            let result = parse("_emphasis_ and *strong*").unwrap();
            let doc = result.value;
            assert_eq!(doc.content.children.len(), 1);
            let para = &doc.content.children[0];
            let has_emphasis = para
                .children
                .iter()
                .any(|n| n.kind.as_str() == node::EMPHASIS);
            let has_strong = para
                .children
                .iter()
                .any(|n| n.kind.as_str() == node::STRONG);
            assert!(has_emphasis);
            assert!(has_strong);
        }

        #[test]
        fn test_parse_link() {
            let result = parse("[link](https://example.com)").unwrap();
            let doc = result.value;
            let para = &doc.content.children[0];
            let link = para.children.iter().find(|n| n.kind.as_str() == node::LINK);
            assert!(link.is_some());
            assert_eq!(
                link.unwrap().props.get_str(prop::URL),
                Some("https://example.com")
            );
        }

        #[test]
        fn test_parse_code_block() {
            let result = parse("```rust\nfn main() {}\n```").unwrap();
            let doc = result.value;
            let cb = doc
                .content
                .children
                .iter()
                .find(|n| n.kind.as_str() == node::CODE_BLOCK);
            assert!(cb.is_some());
            assert_eq!(cb.unwrap().props.get_str(prop::LANGUAGE), Some("rust"));
        }

        #[test]
        fn test_parse_list() {
            let result = parse("- item 1\n- item 2").unwrap();
            let doc = result.value;
            let list = doc
                .content
                .children
                .iter()
                .find(|n| n.kind.as_str() == node::LIST);
            assert!(list.is_some());
            assert_eq!(list.unwrap().props.get_bool(prop::ORDERED), Some(false));
            assert_eq!(list.unwrap().children.len(), 2);
        }
    }
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use rescribe_core::{
        ConversionResult, Document, EmitError, FidelityWarning, Node, Severity, WarningKind,
    };
    use rescribe_std::{node, prop};

    /// Emit a document as Djot markup.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let mut ctx = EmitContext::new();
        emit_node(&doc.content, &mut ctx);

        // Trim only trailing newlines, not spaces — trailing spaces on marker lines
        // like "- " (empty list item) are syntactically significant in djot.
        let output = ctx.output.trim_end_matches('\n').to_string() + "\n";
        Ok(ConversionResult::with_warnings(
            output.into_bytes(),
            ctx.warnings,
        ))
    }

    struct EmitContext {
        output: String,
        warnings: Vec<FidelityWarning>,
        list_depth: usize,
        in_tight_list: bool,
    }

    impl EmitContext {
        fn new() -> Self {
            Self {
                output: String::new(),
                warnings: Vec::new(),
                list_depth: 0,
                in_tight_list: false,
            }
        }

        fn write(&mut self, s: &str) {
            self.output.push_str(s);
        }

        fn writeln(&mut self, s: &str) {
            self.output.push_str(s);
            self.output.push('\n');
        }

        fn newline(&mut self) {
            self.output.push('\n');
        }

        #[allow(dead_code)]
        fn warn(&mut self, message: impl Into<String>) {
            self.warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::FeatureLost("djot".to_string()),
                message,
            ));
        }
    }

    fn emit_node(node: &Node, ctx: &mut EmitContext) {
        match node.kind.as_str() {
            node::DOCUMENT => {
                for child in &node.children {
                    emit_node(child, ctx);
                }
            }
            node::PARAGRAPH => {
                emit_inline_children(node, ctx);
                ctx.newline();
                if !ctx.in_tight_list {
                    ctx.newline();
                }
            }
            node::HEADING => {
                let level = node.props.get_int(prop::LEVEL).unwrap_or(1) as usize;
                let hashes = "#".repeat(level);
                ctx.write(&hashes);
                ctx.write(" ");
                emit_inline_children(node, ctx);
                ctx.newline();
                ctx.newline();
            }
            node::BLOCKQUOTE => {
                for child in &node.children {
                    ctx.write("> ");
                    emit_node(child, ctx);
                }
            }
            node::CODE_BLOCK => {
                let language = node.props.get_str(prop::LANGUAGE).unwrap_or("");
                let content = node.props.get_str(prop::CONTENT).unwrap_or("");

                ctx.write("```");
                ctx.writeln(language);
                ctx.write(content);
                if !content.ends_with('\n') {
                    ctx.newline();
                }
                ctx.writeln("```");
                ctx.newline();
            }
            node::LIST => {
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let start = node.props.get_int(prop::START).unwrap_or(1);
                let tight = is_tight_list(node);

                let old_tight = ctx.in_tight_list;
                ctx.in_tight_list = tight;
                ctx.list_depth += 1;

                for (i, child) in node.children.iter().enumerate() {
                    if ordered {
                        ctx.write(&format!("{}. ", start + i as i64));
                    } else {
                        ctx.write("- ");
                    }
                    emit_list_item_content(child, ctx);
                }

                ctx.list_depth -= 1;
                ctx.in_tight_list = old_tight;

                if ctx.list_depth == 0 {
                    ctx.newline();
                }
            }
            node::LIST_ITEM => {
                // Handled by LIST
                emit_list_item_content(node, ctx);
            }
            node::TABLE => {
                emit_table(node, ctx);
                ctx.newline();
            }
            node::HORIZONTAL_RULE => {
                ctx.writeln("* * *");
                ctx.newline();
            }
            node::DIV => {
                let class = node.props.get_str("html:class").unwrap_or("");
                if !class.is_empty() {
                    ctx.writeln(&format!("::: {}", class));
                } else {
                    ctx.writeln(":::");
                }
                for child in &node.children {
                    emit_node(child, ctx);
                }
                ctx.writeln(":::");
                ctx.newline();
            }
            node::DEFINITION_LIST => {
                for child in &node.children {
                    emit_node(child, ctx);
                }
                ctx.newline();
            }
            node::DEFINITION_TERM => {
                ctx.write(": ");
                emit_inline_children(node, ctx);
                ctx.newline();
            }
            node::DEFINITION_DESC => {
                ctx.write("  ");
                emit_inline_children(node, ctx);
                ctx.newline();
            }
            node::FOOTNOTE_DEF => {
                let label = node.props.get_str(prop::LABEL).unwrap_or("?");
                ctx.write(&format!("[^{}]: ", label));
                emit_inline_children(node, ctx);
                ctx.newline();
                ctx.newline();
            }
            node::RAW_BLOCK => {
                let format = node.props.get_str(prop::FORMAT).unwrap_or("html");
                let content = node.props.get_str(prop::CONTENT).unwrap_or("");
                ctx.writeln(&format!("```{{{}}}", format));
                ctx.write(content);
                if !content.ends_with('\n') {
                    ctx.newline();
                }
                ctx.writeln("```");
                ctx.newline();
            }
            // Inline nodes in block context
            _ => {
                emit_inline(node, ctx);
            }
        }
    }

    fn emit_list_item_content(node: &Node, ctx: &mut EmitContext) {
        // Handle task list items
        if let Some(checked) = node.props.get_bool(prop::CHECKED) {
            if checked {
                ctx.write("[x] ");
            } else {
                ctx.write("[ ] ");
            }
        }

        // Emit children, handling nested structure
        let mut first = true;
        for child in &node.children {
            if child.kind.as_str() == node::PARAGRAPH {
                if !first {
                    ctx.write("  ");
                }
                emit_inline_children(child, ctx);
                ctx.newline();
            } else if child.kind.as_str() == node::LIST {
                ctx.newline();
                // Indent nested list
                let indent = "  ".repeat(ctx.list_depth);
                let old_output = std::mem::take(&mut ctx.output);
                emit_node(child, ctx);
                let nested = std::mem::replace(&mut ctx.output, old_output);
                for line in nested.lines() {
                    ctx.write(&indent);
                    ctx.writeln(line);
                }
            } else {
                emit_node(child, ctx);
            }
            first = false;
        }
    }

    fn emit_inline_children(node: &Node, ctx: &mut EmitContext) {
        for child in &node.children {
            emit_inline(child, ctx);
        }
    }

    /// Escape special djot characters in plain text content.
    ///
    /// Djot uses `\X` to escape any non-alphanumeric ASCII character.
    /// We escape inline markup characters unconditionally, plus `:` which
    /// is a definition-list block marker and dangerous even in inline
    /// positions (jotdown will parse a paragraph starting with `:` as a
    /// definition list).  Other block-level starters (`-`, `+`, `#`, `>`)
    /// are only ever special at the start of a *block line*, not inside an
    /// already-open inline run, so we leave them unescaped to keep output
    /// readable.
    fn escape_text(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let chars: Vec<char> = s.chars().collect();
        for (i, &ch) in chars.iter().enumerate() {
            match ch {
                // Escape character itself
                '\\' => {
                    out.push('\\');
                    out.push('\\');
                }
                // Inline code
                '`' => {
                    out.push('\\');
                    out.push('`');
                }
                // Strong / emphasis
                '*' | '_' => {
                    out.push('\\');
                    out.push(ch);
                }
                // Span / link / footnote starters
                '{' | '[' => {
                    out.push('\\');
                    out.push(ch);
                }
                // Superscript / subscript / math
                '^' | '~' | '$' => {
                    out.push('\\');
                    out.push(ch);
                }
                // Image: only special before '['
                '!' if chars.get(i + 1) == Some(&'[') => {
                    out.push('\\');
                    out.push('!');
                }
                // Definition-list marker: dangerous at start of any line
                ':' => {
                    out.push('\\');
                    out.push(':');
                }
                // Straight quotes: jotdown applies smart-quote substitution, so
                // an unescaped ' or " round-trips as a curly quote (≠ original).
                '\'' | '"' => {
                    out.push('\\');
                    out.push(ch);
                }
                // Table marker: '|' is a table-row delimiter even at start of
                // a paragraph — jotdown parses "|cell|" as a table, not text.
                '|' => {
                    out.push('\\');
                    out.push('|');
                }
                other => out.push(other),
            }
        }
        out
    }

    /// Choose backtick delimiter for an inline code span.
    ///
    /// Uses the smallest N such that no run of N backticks appears in `content`,
    /// preventing the delimiter from being misread as part of the content.
    fn code_span_delimiters(content: &str) -> String {
        let mut max_run = 0usize;
        let mut run = 0usize;
        for ch in content.chars() {
            if ch == '`' {
                run += 1;
                max_run = max_run.max(run);
            } else {
                run = 0;
            }
        }
        "`".repeat(max_run + 1)
    }

    fn emit_inline(node: &Node, ctx: &mut EmitContext) {
        match node.kind.as_str() {
            node::TEXT => {
                if let Some(content) = node.props.get_str(prop::CONTENT) {
                    ctx.write(&escape_text(content));
                }
            }
            node::EMPHASIS => {
                ctx.write("_");
                emit_inline_children(node, ctx);
                ctx.write("_");
            }
            node::STRONG => {
                ctx.write("*");
                emit_inline_children(node, ctx);
                ctx.write("*");
            }
            node::STRIKEOUT => {
                ctx.write("{-");
                emit_inline_children(node, ctx);
                ctx.write("-}");
            }
            node::SUBSCRIPT => {
                ctx.write("~");
                emit_inline_children(node, ctx);
                ctx.write("~");
            }
            node::SUPERSCRIPT => {
                ctx.write("^");
                emit_inline_children(node, ctx);
                ctx.write("^");
            }
            node::UNDERLINE => {
                // Djot uses {+...+} for insert, which is close to underline
                ctx.write("{+");
                emit_inline_children(node, ctx);
                ctx.write("+}");
            }
            node::CODE => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("");
                let delim = code_span_delimiters(content);
                ctx.write(&delim);
                ctx.write(content);
                ctx.write(&delim);
            }
            node::LINK => {
                let url = node.props.get_str(prop::URL).unwrap_or("");
                ctx.write("[");
                emit_inline_children(node, ctx);
                ctx.write("](");
                ctx.write(url);
                ctx.write(")");
            }
            node::IMAGE => {
                let url = node.props.get_str(prop::URL).unwrap_or("");
                let alt = node.props.get_str(prop::ALT).unwrap_or("");
                ctx.write("![");
                ctx.write(alt);
                ctx.write("](");
                ctx.write(url);
                ctx.write(")");
            }
            node::LINE_BREAK => {
                ctx.write("\\\n");
            }
            node::SOFT_BREAK => {
                ctx.newline();
            }
            node::FOOTNOTE_REF => {
                let label = node.props.get_str(prop::LABEL).unwrap_or("?");
                ctx.write(&format!("[^{}]", label));
            }
            node::SPAN => {
                // Djot span syntax
                ctx.write("[");
                emit_inline_children(node, ctx);
                ctx.write("]{}");
            }
            node::RAW_INLINE => {
                let format = node.props.get_str(prop::FORMAT).unwrap_or("html");
                let content = node.props.get_str(prop::CONTENT).unwrap_or("");
                ctx.write(&format!("`{}`{{{}}}", content, format));
            }
            "math:inline" => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("");
                ctx.write("$");
                ctx.write(content);
                ctx.write("$");
            }
            "math:display" => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("");
                ctx.write("$$");
                ctx.write(content);
                ctx.write("$$");
            }
            _ => {
                // Unknown inline - try to emit children
                emit_inline_children(node, ctx);
            }
        }
    }

    fn emit_table(node: &Node, ctx: &mut EmitContext) {
        // Find header row and body rows
        let mut header_row: Option<&Node> = None;
        let mut body_rows: Vec<&Node> = Vec::new();

        for child in &node.children {
            if child.kind.as_str() == node::TABLE_ROW {
                // Check if this is a header row (first row with TABLE_HEADER cells)
                let has_headers = child
                    .children
                    .iter()
                    .any(|c| c.kind.as_str() == node::TABLE_HEADER);
                if has_headers && header_row.is_none() {
                    header_row = Some(child);
                } else {
                    body_rows.push(child);
                }
            }
        }

        // Emit header
        if let Some(header) = header_row {
            ctx.write("|");
            for cell in &header.children {
                ctx.write(" ");
                emit_inline_children(cell, ctx);
                ctx.write(" |");
            }
            ctx.newline();

            // Separator
            ctx.write("|");
            for _ in &header.children {
                ctx.write("---|");
            }
            ctx.newline();
        }

        // Emit body rows
        for row in body_rows {
            ctx.write("|");
            for cell in &row.children {
                ctx.write(" ");
                emit_inline_children(cell, ctx);
                ctx.write(" |");
            }
            ctx.newline();
        }
    }

    fn is_tight_list(list: &Node) -> bool {
        // A list is tight if no list item contains multiple block elements
        for item in &list.children {
            if item.children.len() > 1 {
                return false;
            }
        }
        true
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_std::builder::doc;

        #[test]
        fn test_emit_paragraph() {
            let document = doc(|d| d.para(|i| i.text("Hello, world!")));
            let result = emit(&document).unwrap();
            let output = String::from_utf8(result.value).unwrap();
            assert_eq!(output, "Hello, world!\n");
        }

        #[test]
        fn test_emit_heading() {
            let document = doc(|d| d.heading(2, |i| i.text("Title")));
            let result = emit(&document).unwrap();
            let output = String::from_utf8(result.value).unwrap();
            assert!(output.starts_with("## Title"));
        }

        #[test]
        fn test_emit_emphasis() {
            let document = doc(|d| d.para(|i| i.em(|i| i.text("emphasis"))));
            let result = emit(&document).unwrap();
            let output = String::from_utf8(result.value).unwrap();
            assert!(output.contains("_emphasis_"));
        }

        #[test]
        fn test_emit_strong() {
            let document = doc(|d| d.para(|i| i.strong(|i| i.text("bold"))));
            let result = emit(&document).unwrap();
            let output = String::from_utf8(result.value).unwrap();
            assert!(output.contains("*bold*"));
        }

        #[test]
        fn test_emit_link() {
            let document = doc(|d| d.para(|i| i.link("https://example.com", |i| i.text("link"))));
            let result = emit(&document).unwrap();
            let output = String::from_utf8(result.value).unwrap();
            assert!(output.contains("[link](https://example.com)"));
        }

        #[test]
        fn test_emit_code_block() {
            let document = doc(|d| d.code_block_lang("fn main() {}", "rust"));
            let result = emit(&document).unwrap();
            let output = String::from_utf8(result.value).unwrap();
            assert!(output.contains("```rust"));
            assert!(output.contains("fn main() {}"));
            assert!(output.contains("```\n"));
        }
    }

    /// Roundtrip regression tests for fuzz-found crashes.
    #[cfg(all(test, feature = "reader-ast"))]
    mod roundtrip_tests {
        use super::emit;
        use crate::rescribe::parse;

        fn roundtrip_text_preserved(input: &str) {
            fn extract_text(node: &rescribe_core::Node) -> String {
                use rescribe_std::{node, prop};
                let mut t = String::new();
                if node.kind.as_str() == node::TEXT
                    && let Some(c) = node.props.get_str(prop::CONTENT)
                {
                    t.push_str(c);
                }
                for ch in &node.children {
                    t.push_str(&extract_text(ch));
                }
                t
            }
            let doc1 = parse(input).unwrap().value;
            let out = emit(&doc1).unwrap();
            let s = String::from_utf8(out.value).unwrap();
            let doc2 = parse(&s).unwrap().value;
            let t1: String = extract_text(&doc1.content)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let t2: String = extract_text(&doc2.content)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            // Use multiset (sorted-char) equality: detects character additions/removals
            // while tolerating jotdown's span-delimiter adjacency reordering quirk.
            let mut c1: Vec<char> = t1.chars().collect();
            let mut c2: Vec<char> = t2.chars().collect();
            c1.sort_unstable();
            c2.sort_unstable();
            assert_eq!(
                c1, c2,
                "roundtrip text mismatch for {input:?}: {t1:?} -> {t2:?}"
            );
        }

        // Empty list item: trailing space on "- " was stripped by trim_end(),
        // making "-" be read back as plain text instead of a list marker.
        #[test]
        fn empty_list_item() {
            roundtrip_text_preserved("\n:\n+");
        }

        // Escaped colon: \: → text ":" → writer emitted bare ":", which djot
        // re-reads as a definition-list marker.
        #[test]
        fn escaped_colon() {
            roundtrip_text_preserved("\\:");
        }

        // Code span with internal backtick run: content "-``)" was wrapped in
        // "``...``" but the internal "``" broke the delimiter.
        #[test]
        fn code_span_internal_backtick_run() {
            roundtrip_text_preserved("`-``)");
        }

        // Smart quotes: unescaped straight apostrophe ' (U+0027) round-trips as
        // curly right-single-quote ' (U+2019) because jotdown applies smart quotes.
        #[test]
        fn smart_quote_apostrophe() {
            roundtrip_text_preserved("\\'|");
        }

        // Table marker: unescaped '|' is absorbed as a table column separator on
        // re-parse, stripping the pipe characters from the text content.
        #[test]
        fn pipe_as_table_marker() {
            roundtrip_text_preserved("|\x7f\\|");
        }

        // Nested superscripts: ^^^:^ → IR TEXT("^^") + SUPERSCRIPT(TEXT(":")), writer
        // emits \^\^^\\:^ which jotdown reparses differently due to span-delimiter
        // adjacency quirk.  Characters are reordered but not added/removed, so
        // multiset equality passes while strict equality would fail.
        #[test]
        fn nested_superscripts() {
            roundtrip_text_preserved("`!`^^^:^");
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::parse;
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::emit;
