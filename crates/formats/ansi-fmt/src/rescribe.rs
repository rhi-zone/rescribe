//! AST↔`rescribe::Document` translation for ANSI terminal text.
//!
//! Enabled by the `rescribe` feature; each direction is additionally gated
//! on the reader/writer mode feature it depends on, so enabling `rescribe`
//! alone (with no mode feature) compiles nothing.
//!
//! # Mapping
//!
//! ANSI is a terminal escape sequence format, not a document format. The
//! reader (`read::parse`) is a genuine AST→IR translation: it walks
//! [`AnsiDoc`](crate::AnsiDoc) (produced by `crate::parse`) and maps it onto
//! rescribe's `Document`:
//! - Control sequences (cursor move, erase, etc.) → top-level `raw_block`
//!   with `ansi:*` props
//! - Text content (text, hyperlinks, raw escapes) → grouped into
//!   `paragraph` nodes, split on blank lines
//!
//! The writer (`write::emit`) is **not** a translation through
//! [`AnsiDoc`](crate::AnsiDoc)/`crate::emit`. `AnsiDoc`'s AST models only
//! low-level terminal constructs (styled text runs, cursor moves, hyperlinks)
//! and has no representation for the document-structure nodes rescribe's IR
//! carries — headings, lists, tables, block quotes. Rendering those to a
//! terminal (`#` heading prefixes, `•` bullets, `│` table borders) requires
//! generating bytes that have no equivalent `AnsiNode` variant to express, so
//! this direction walks the rescribe `Document` tree directly and hand-emits
//! ANSI escape sequences, pre-dating (and not fixed by) this migration.
//!
//! **Known design debt:** this makes `write::emit`'s production code contain
//! byte-level ANSI emission logic inside the `rescribe` feature module,
//! which is exactly what CLAUDE.md's "the `rescribe` feature module must
//! never contain parsing or writing logic" rule prohibits. The violation
//! predates this migration (it lived in `rescribe-write-ansi` before);
//! moving it here relocates it without fixing it. Closing this gap for real
//! requires extending `ansi_fmt::AnsiDoc`/`AnsiNode` with document-structure
//! variants (or a higher-level layout AST layered on top of it) so
//! `write::emit` can become `Document -> AnsiDoc -> crate::emit` like the
//! read direction — out of scope for this mechanical migration.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use crate::{AnsiDoc, AnsiNode, Color, Style};
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_format_api::Parse as _;
    use rescribe_std::{node, prop};

    /// Parse ANSI-formatted text into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse ANSI-formatted text with options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (ansi_doc, _diagnostics) = AnsiDoc::parse(input.as_bytes());

        let blocks = build_document_nodes(&ansi_doc);

        let document = Document {
            content: Node::new(node::DOCUMENT).children(blocks),
            resources: Default::default(),
            metadata: Default::default(),
            source: None,
        };

        Ok(ConversionResult::ok(document))
    }

    /// Flush accumulated inline nodes as a paragraph, clearing the buffer.
    fn flush_para(inline_buf: &mut Vec<&AnsiNode>, result: &mut Vec<Node>) {
        // Remove trailing newlines
        while matches!(inline_buf.last(), Some(AnsiNode::Newline { .. })) {
            inline_buf.pop();
        }
        if !inline_buf.is_empty() {
            let inlines: Vec<Node> = inline_buf.iter().map(|n| ansi_node_to_inline(n)).collect();
            result.push(Node::new(node::PARAGRAPH).children(inlines));
            inline_buf.clear();
        }
    }

    /// Build the top-level block list from AnsiDoc nodes.
    ///
    /// - Control sequences (cursor, erase, etc.) → `raw_block` with `ansi:type` props
    /// - Text, hyperlinks, raw escapes → accumulated into paragraphs (split on 2+ newlines)
    fn build_document_nodes(doc: &AnsiDoc) -> Vec<Node> {
        let mut result: Vec<Node> = Vec::new();
        let mut inline_buf: Vec<&AnsiNode> = Vec::new();
        let mut consecutive_newlines: usize = 0;
        // Tracks the running SGR style as we scan, independently of ansi-fmt's
        // own AST — used only to decide whether a ResetStyle node is redundant
        // (see the ResetStyle arm below). ansi-fmt's parse() now emits a
        // SetStyle/ResetStyle node for every source SGR escape group,
        // unconditionally, mirroring events() (see ast.rs's SetStyle/ResetStyle
        // doc comments) — the "is this reset otherwise unobservable" decision
        // that node emission used to make now lives here instead.
        let mut running_style = Style::default();

        for node in &doc.nodes {
            match node {
                AnsiNode::Newline { .. } => {
                    consecutive_newlines += 1;
                    if consecutive_newlines == 1 {
                        // Single newline: keep in current paragraph as LINE_BREAK
                        inline_buf.push(node);
                    } else if consecutive_newlines == 2 {
                        // Double newline: paragraph boundary — flush
                        flush_para(&mut inline_buf, &mut result);
                    }
                    // 3+ consecutive newlines: already flushed, ignore extras
                }

                // A non-resetting SGR group carries no independent IR content —
                // the resulting style is already captured on the next
                // Text/Hyperlink node's own `style` field, which
                // ansi_node_to_inline reads directly. Track it only to keep
                // running_style accurate for the ResetStyle decision below.
                AnsiNode::SetStyle { style, .. } => {
                    running_style = style.clone();
                }

                // An explicit SGR reset is redundant (dropped, no IR node) when
                // the running style was already non-empty beforehand — that
                // transition is fully captured by whatever Text/Hyperlink node
                // comes next carrying the now-default style. It's only
                // preserved as a raw_inline when the running style was already
                // empty beforehand, i.e. the reset is otherwise unobservable
                // (e.g. a trailing `\x1b[0m` after an unrecognized/no-op SGR
                // code — see the adv-unknown-sgr fixture).
                AnsiNode::ResetStyle { .. } => {
                    let was_redundant = !running_style.is_empty();
                    running_style = Style::default();
                    if !was_redundant {
                        consecutive_newlines = 0;
                        inline_buf.push(node);
                    }
                }

                AnsiNode::Text { style, .. } => {
                    running_style = style.clone();
                    consecutive_newlines = 0;
                    inline_buf.push(node);
                }

                AnsiNode::Hyperlink { style, .. } => {
                    running_style = style.clone();
                    consecutive_newlines = 0;
                    inline_buf.push(node);
                }

                AnsiNode::RawEscape { .. } => {
                    consecutive_newlines = 0;
                    inline_buf.push(node);
                }

                // Control sequences: flush pending paragraph, then emit raw_block
                _ => {
                    consecutive_newlines = 0;
                    flush_para(&mut inline_buf, &mut result);
                    result.push(ansi_node_to_raw_block(node));
                }
            }
        }

        // Flush any remaining inline content
        flush_para(&mut inline_buf, &mut result);

        result
    }

    /// Convert a control-sequence AnsiNode to a top-level raw_block with ansi: props.
    fn ansi_node_to_raw_block(n: &AnsiNode) -> Node {
        match n {
            AnsiNode::CursorMove {
                direction, count, ..
            } => {
                use crate::CursorDirection;
                let type_str = match direction {
                    CursorDirection::Up => "cursor-up",
                    CursorDirection::Down => "cursor-down",
                    CursorDirection::Forward => "cursor-forward",
                    CursorDirection::Back => "cursor-back",
                };
                Node::new(node::RAW_BLOCK)
                    .prop("ansi:type", type_str)
                    .prop("ansi:count", *count as i64)
            }

            AnsiNode::CursorPosition { row, col, .. } => Node::new(node::RAW_BLOCK)
                .prop("ansi:type", "cursor-position")
                .prop("ansi:row", *row as i64)
                .prop("ansi:col", *col as i64),

            AnsiNode::EraseDisplay { mode, .. } => {
                use crate::EraseMode;
                let mode_str = match mode {
                    EraseMode::ToEnd => "to-end",
                    EraseMode::ToBeginning => "to-beginning",
                    EraseMode::All => "all",
                };
                Node::new(node::RAW_BLOCK)
                    .prop("ansi:type", "erase-display")
                    .prop("ansi:mode", mode_str)
            }

            AnsiNode::EraseLine { mode, .. } => {
                use crate::EraseMode;
                let mode_str = match mode {
                    EraseMode::ToEnd => "to-end",
                    EraseMode::ToBeginning => "to-beginning",
                    EraseMode::All => "all",
                };
                Node::new(node::RAW_BLOCK)
                    .prop("ansi:type", "erase-line")
                    .prop("ansi:mode", mode_str)
            }

            AnsiNode::CursorVisibility { visible, .. } => {
                let type_str = if *visible {
                    "cursor-show"
                } else {
                    "cursor-hide"
                };
                Node::new(node::RAW_BLOCK).prop("ansi:type", type_str)
            }

            AnsiNode::SaveCursor { .. } => {
                Node::new(node::RAW_BLOCK).prop("ansi:type", "save-cursor")
            }

            AnsiNode::RestoreCursor { .. } => {
                Node::new(node::RAW_BLOCK).prop("ansi:type", "restore-cursor")
            }

            AnsiNode::ScrollRegion { top, bottom, .. } => Node::new(node::RAW_BLOCK)
                .prop("ansi:type", "scroll-region")
                .prop("ansi:top", *top as i64)
                .prop("ansi:bottom", *bottom as i64),

            // Fallback: shouldn't happen since we route Text/Hyperlink/RawEscape to inline path
            _ => Node::new(node::RAW_BLOCK).prop(prop::FORMAT, "ansi"),
        }
    }

    /// Convert a single inline AnsiNode to a rescribe Node.
    fn ansi_node_to_inline(n: &AnsiNode) -> Node {
        match n {
            AnsiNode::Text { text, style, .. } => {
                if style.is_empty() {
                    return Node::new(node::TEXT).prop(prop::CONTENT, text.clone());
                }

                // Build from innermost (text) outward.
                // Order (outermost → innermost): strong > emphasis > strikeout > underline > span > text
                let mut inner = Node::new(node::TEXT).prop(prop::CONTENT, text.clone());

                // Non-semantic span properties (color, dim, blink, etc.)
                let needs_span = style.fg.is_some()
                    || style.bg.is_some()
                    || style.underline_color.is_some()
                    || style.dim
                    || style.blink
                    || style.rapid_blink
                    || style.reverse
                    || style.hidden
                    || style.overline
                    || style.double_underline;

                if needs_span {
                    let mut span = Node::new(node::SPAN);
                    if let Some(ref fg) = style.fg {
                        span = span.prop("style:color", color_to_string(fg));
                    }
                    if let Some(ref bg) = style.bg {
                        span = span.prop("style:background-color", color_to_string(bg));
                    }
                    if let Some(ref uc) = style.underline_color {
                        span = span.prop("style:underline-color", color_to_string(uc));
                    }
                    if style.dim {
                        span = span.prop("style:dim", true);
                    }
                    if style.blink || style.rapid_blink {
                        span = span.prop("style:blink", true);
                    }
                    if style.reverse {
                        span = span.prop("style:reverse", true);
                    }
                    if style.hidden {
                        span = span.prop("style:hidden", true);
                    }
                    if style.overline {
                        span = span.prop("style:overline", true);
                    }
                    if style.double_underline {
                        span = span.prop("style:double-underline", true);
                    }
                    inner = span.child(inner);
                }

                // Semantic wrappers: applied innermost to outermost
                if style.underline {
                    inner = Node::new(node::UNDERLINE).child(inner);
                }
                if style.strikethrough {
                    inner = Node::new(node::STRIKEOUT).child(inner);
                }
                if style.italic {
                    inner = Node::new(node::EMPHASIS).child(inner);
                }
                if style.bold {
                    inner = Node::new(node::STRONG).child(inner);
                }

                inner
            }

            AnsiNode::Newline { .. } => Node::new(node::LINE_BREAK),

            AnsiNode::Hyperlink { url, text, .. } => Node::new(node::LINK)
                .prop(prop::URL, url.clone())
                .child(Node::new(node::TEXT).prop(prop::CONTENT, text.clone())),

            // Raw escapes: preserve as raw_inline with format = "ansi"
            AnsiNode::RawEscape { content, .. } => Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "ansi")
                .prop(prop::CONTENT, content.clone()),

            // Explicit SGR reset with no (or no-op) style change to carry it —
            // preserve the literal bytes the same way RawEscape does, so a
            // trailing/no-op `\x1b[0m` round-trips through the IR too.
            AnsiNode::ResetStyle { .. } => Node::new(node::RAW_INLINE)
                .prop(prop::FORMAT, "ansi")
                .prop(prop::CONTENT, "\x1b[0m"),

            // Control sequences shouldn't reach here (handled in build_document_nodes),
            // but provide a safe fallback.
            _ => Node::new(node::RAW_INLINE).prop(prop::FORMAT, "ansi"),
        }
    }

    fn color_to_string(color: &Color) -> String {
        match color {
            Color::Standard(n) => {
                let names = [
                    "ansi-black",
                    "ansi-red",
                    "ansi-green",
                    "ansi-yellow",
                    "ansi-blue",
                    "ansi-magenta",
                    "ansi-cyan",
                    "ansi-white",
                ];
                names
                    .get(*n as usize)
                    .copied()
                    .unwrap_or("ansi-unknown")
                    .to_string()
            }
            Color::Bright(n) => {
                let names = [
                    "ansi-bright-black",
                    "ansi-bright-red",
                    "ansi-bright-green",
                    "ansi-bright-yellow",
                    "ansi-bright-blue",
                    "ansi-bright-magenta",
                    "ansi-bright-cyan",
                    "ansi-bright-white",
                ];
                names
                    .get(*n as usize)
                    .copied()
                    .unwrap_or("ansi-bright-unknown")
                    .to_string()
            }
            Color::Palette(n) => format!("ansi-palette-{}", n),
            Color::Rgb(r, g, b) => format!("#{:02x}{:02x}{:02x}", r, g, b),
            Color::Default => "ansi-default".to_string(),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_parse_plain_text() {
            let result = parse("Hello world").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_bold() {
            let result = parse("\x1b[1mBold text\x1b[0m").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_italic() {
            let result = parse("\x1b[3mItalic text\x1b[0m").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_underline() {
            let result = parse("\x1b[4mUnderlined\x1b[0m").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_combined_styles() {
            let result = parse("\x1b[1;3mBold and italic\x1b[0m").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_hyperlink() {
            let result = parse("\x1b]8;;https://example.com\x07click here\x1b]8;;\x07").unwrap();
            assert!(!result.value.content.children.is_empty());
        }

        #[test]
        fn test_parse_multiline() {
            let result = parse("Line one\n\nLine two").unwrap();
            // Should produce two paragraphs.
            assert_eq!(result.value.content.children.len(), 2);
        }

        #[test]
        fn test_cursor_move_becomes_raw_block() {
            let result = parse("\x1b[4D").unwrap();
            let doc = result.value;
            assert_eq!(doc.content.children.len(), 1);
            assert_eq!(doc.content.children[0].kind.as_str(), node::RAW_BLOCK);
            assert_eq!(
                doc.content.children[0].props.get_str("ansi:type"),
                Some("cursor-back")
            );
            assert_eq!(doc.content.children[0].props.get_int("ansi:count"), Some(4));
        }
    }
}

// Not a translation through `AnsiDoc`/`crate::emit` — see the module-level
// doc comment above for why (and the design debt this represents).
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use rescribe_core::{
        ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, Severity,
        WarningKind,
    };
    use rescribe_std::{node, prop};

    /// Emit a document as ANSI-formatted text.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document as ANSI-formatted text with custom options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let mut ctx = EmitContext::new();
        for child in &doc.content.children {
            emit_block(child, &mut ctx);
        }
        Ok(ConversionResult::with_warnings(ctx.output, ctx.warnings))
    }

    // ── Context ───────────────────────────────────────────────────────────────

    struct EmitContext {
        output: Vec<u8>,
        warnings: Vec<FidelityWarning>,
    }

    impl EmitContext {
        fn new() -> Self {
            Self {
                output: Vec::new(),
                warnings: Vec::new(),
            }
        }

        fn push(&mut self, s: &str) {
            self.output.extend_from_slice(s.as_bytes());
        }

        fn warn(&mut self, kind: WarningKind, msg: impl Into<String>) {
            self.warnings
                .push(FidelityWarning::new(Severity::Minor, kind, msg.into()));
        }
    }

    // ── Block emission ───────────────────────────────────────────────────────

    fn emit_block(n: &Node, ctx: &mut EmitContext) {
        match n.kind.as_str() {
            node::DOCUMENT => {
                for child in &n.children {
                    emit_block(child, ctx);
                }
            }

            node::PARAGRAPH => {
                for child in &n.children {
                    emit_inline(child, ctx);
                }
                ctx.push("\n\n");
            }

            node::HEADING => {
                let level = n.props.get_int(prop::LEVEL).unwrap_or(1) as usize;
                let prefix = "#".repeat(level);
                ctx.push("\x1b[1m");
                ctx.push(&prefix);
                ctx.push(" ");
                for child in &n.children {
                    emit_inline(child, ctx);
                }
                ctx.push("\x1b[0m");
                ctx.push("\n\n");
            }

            node::CODE_BLOCK => {
                let lang = n.props.get_str(prop::LANGUAGE).unwrap_or("");
                let content = n.props.get_str(prop::CONTENT).unwrap_or("");
                if !lang.is_empty() {
                    ctx.push("\x1b[2m");
                    ctx.push(lang);
                    ctx.push("\x1b[0m");
                    ctx.push("\n");
                }
                ctx.push(content);
                ctx.push("\n\n");
            }

            node::BLOCKQUOTE => {
                // Emit children, but prefix each line with "│ ".
                // Simple approach: collect content, then prefix lines.
                let mut sub = EmitContext::new();
                for child in &n.children {
                    emit_block(child, &mut sub);
                }
                ctx.warnings.extend(sub.warnings);
                let text = String::from_utf8_lossy(&sub.output);
                for line in text.lines() {
                    ctx.push("│ ");
                    ctx.push(line);
                    ctx.push("\n");
                }
                ctx.push("\n");
            }

            node::LIST => {
                let ordered = n.props.get_bool(prop::ORDERED).unwrap_or(false);
                let mut index = 1usize;
                for child in &n.children {
                    if child.kind.as_str() == node::LIST_ITEM {
                        emit_list_item(child, ordered, index, ctx);
                        index += 1;
                    } else {
                        emit_block(child, ctx);
                    }
                }
                ctx.push("\n");
            }

            node::LIST_ITEM => {
                // Standalone list item (not inside LIST): use bullet.
                emit_list_item(n, false, 1, ctx);
            }

            node::TABLE => {
                for child in &n.children {
                    emit_block(child, ctx);
                }
                ctx.push("\n");
            }

            node::TABLE_ROW => {
                for child in &n.children {
                    ctx.push("│ ");
                    for inline in &child.children {
                        emit_inline(inline, ctx);
                    }
                    ctx.push(" ");
                }
                ctx.push("│\n");
            }

            node::TABLE_CELL | node::TABLE_HEADER => {
                ctx.push("│ ");
                for child in &n.children {
                    emit_inline(child, ctx);
                }
                ctx.push(" │\n");
            }

            node::TABLE_HEAD | node::TABLE_BODY | node::TABLE_FOOT => {
                for child in &n.children {
                    emit_block(child, ctx);
                }
            }

            node::HORIZONTAL_RULE => {
                ctx.push("───────────────────────────────────────────────────────\n\n");
            }

            node::DIV | node::FIGURE => {
                for child in &n.children {
                    emit_block(child, ctx);
                }
            }

            node::SPAN => {
                // Block-level span: emit inline content + newline.
                for child in &n.children {
                    emit_inline(child, ctx);
                }
                ctx.push("\n\n");
            }

            node::RAW_BLOCK => {
                let format = n.props.get_str(prop::FORMAT).unwrap_or("");
                let content = n.props.get_str(prop::CONTENT).unwrap_or("");
                if format == "ansi" || format.is_empty() {
                    ctx.push(content);
                }
                // Other formats: silently drop (they are format-specific raw content).
            }

            node::DEFINITION_LIST => {
                for child in &n.children {
                    emit_block(child, ctx);
                }
                ctx.push("\n");
            }

            node::DEFINITION_TERM => {
                ctx.push("\x1b[1m");
                for child in &n.children {
                    emit_inline(child, ctx);
                }
                ctx.push("\x1b[0m");
                ctx.push("\n");
            }

            node::DEFINITION_DESC => {
                ctx.push("  ");
                for child in &n.children {
                    emit_block(child, ctx);
                }
            }

            _ => {
                // Unknown block: try to render children, warn.
                let has_children = !n.children.is_empty();
                if has_children {
                    for child in &n.children {
                        emit_block(child, ctx);
                    }
                } else {
                    // Leaf unknown node: try inline rendering.
                    emit_inline(n, ctx);
                    ctx.push("\n");
                }
                ctx.warn(
                    WarningKind::UnsupportedNode(n.kind.as_str().to_string()),
                    format!("Unknown block node type for ANSI: {}", n.kind.as_str()),
                );
            }
        }
    }

    fn emit_list_item(n: &Node, ordered: bool, index: usize, ctx: &mut EmitContext) {
        let bullet = if ordered {
            format!("{}. ", index)
        } else {
            "• ".to_string()
        };
        ctx.push(&bullet);
        for child in &n.children {
            // If child is a paragraph, emit its inlines without the trailing newlines.
            if child.kind.as_str() == node::PARAGRAPH {
                for inline in &child.children {
                    emit_inline(inline, ctx);
                }
            } else {
                emit_inline(child, ctx);
            }
        }
        ctx.push("\n");
    }

    // ── Inline emission ──────────────────────────────────────────────────────

    fn emit_inline(n: &Node, ctx: &mut EmitContext) {
        match n.kind.as_str() {
            node::TEXT => {
                let content = n.props.get_str(prop::CONTENT).unwrap_or("");
                ctx.push(content);
            }

            node::STRONG => {
                ctx.push("\x1b[1m");
                for child in &n.children {
                    emit_inline(child, ctx);
                }
                ctx.push("\x1b[0m");
            }

            node::EMPHASIS => {
                ctx.push("\x1b[3m");
                for child in &n.children {
                    emit_inline(child, ctx);
                }
                ctx.push("\x1b[0m");
            }

            node::UNDERLINE => {
                ctx.push("\x1b[4m");
                for child in &n.children {
                    emit_inline(child, ctx);
                }
                ctx.push("\x1b[0m");
            }

            node::STRIKEOUT => {
                ctx.push("\x1b[9m");
                for child in &n.children {
                    emit_inline(child, ctx);
                }
                ctx.push("\x1b[0m");
            }

            node::CODE => {
                // Dim for inline code.
                ctx.push("\x1b[2m");
                let content = n.props.get_str(prop::CONTENT).unwrap_or("");
                ctx.push(content);
                ctx.push("\x1b[0m");
            }

            node::LINK => {
                let url = n.props.get_str(prop::URL).unwrap_or("");
                // Text content from children or CONTENT prop.
                let has_children = !n.children.is_empty();
                if has_children {
                    for child in &n.children {
                        emit_inline(child, ctx);
                    }
                } else if let Some(content) = n.props.get_str(prop::CONTENT) {
                    ctx.push(content);
                } else {
                    ctx.push(url);
                }
                if !url.is_empty() {
                    ctx.push(" (");
                    ctx.push(url);
                    ctx.push(")");
                }
            }

            node::IMAGE => {
                let alt = n.props.get_str(prop::ALT).unwrap_or("Image");
                ctx.push("[");
                ctx.push(alt);
                ctx.push("]");
            }

            node::LINE_BREAK => {
                ctx.push("\n");
            }

            node::SOFT_BREAK => {
                ctx.push(" ");
            }

            node::SPAN => {
                // Apply style from properties.
                let bold = n.props.get_bool("style:bold").unwrap_or(false);
                let italic = n.props.get_bool("style:italic").unwrap_or(false);
                let underline = n.props.get_bool("style:underline").unwrap_or(false);
                let strikethrough = n.props.get_bool("style:strikethrough").unwrap_or(false);
                let dim = n.props.get_bool("style:dim").unwrap_or(false);
                let fg_color = n.props.get_str("style:color");

                let any_style =
                    bold || italic || underline || strikethrough || dim || fg_color.is_some();

                if any_style {
                    let mut codes: Vec<&str> = Vec::new();
                    if bold {
                        codes.push("1");
                    }
                    if dim {
                        codes.push("2");
                    }
                    if italic {
                        codes.push("3");
                    }
                    if underline {
                        codes.push("4");
                    }
                    if strikethrough {
                        codes.push("9");
                    }
                    let sgr = format!("\x1b[{}m", codes.join(";"));
                    ctx.push(&sgr);
                }

                // Content can be in prop or children.
                if let Some(content) = n.props.get_str(prop::CONTENT) {
                    ctx.push(content);
                }
                for child in &n.children {
                    emit_inline(child, ctx);
                }

                if any_style {
                    ctx.push("\x1b[0m");
                }
            }

            node::RAW_INLINE => {
                let format = n.props.get_str(prop::FORMAT).unwrap_or("");
                let content = n.props.get_str(prop::CONTENT).unwrap_or("");
                if format == "ansi" || format.is_empty() {
                    ctx.push(content);
                }
                // Other formats: silently drop.
            }

            node::SUBSCRIPT | node::SUPERSCRIPT => {
                // No terminal representation; emit content as-is.
                for child in &n.children {
                    emit_inline(child, ctx);
                }
            }

            node::FOOTNOTE_REF => {
                let label = n.props.get_str(prop::LABEL).unwrap_or("");
                ctx.push("[");
                ctx.push(label);
                ctx.push("]");
            }

            node::FOOTNOTE_DEF => {
                let label = n.props.get_str(prop::LABEL).unwrap_or("");
                ctx.push("[");
                ctx.push(label);
                ctx.push("] ");
                for child in &n.children {
                    emit_inline(child, ctx);
                }
            }

            node::SMALL_CAPS | node::ALL_CAPS => {
                for child in &n.children {
                    emit_inline(child, ctx);
                }
            }

            node::QUOTED => {
                let quote_type = n.props.get_str(prop::QUOTE_TYPE).unwrap_or("double");
                let (left, right) = if quote_type == "single" {
                    ("\u{2018}", "\u{2019}")
                } else {
                    ("\u{201C}", "\u{201D}")
                };
                ctx.push(left);
                for child in &n.children {
                    emit_inline(child, ctx);
                }
                ctx.push(right);
            }

            "math_inline" | "math_display" => {
                let source = n.props.get_str("math:source").unwrap_or("");
                ctx.push(source);
            }

            _ => {
                // Unknown inline: emit children.
                for child in &n.children {
                    emit_inline(child, ctx);
                }
                ctx.warn(
                    WarningKind::UnsupportedNode(n.kind.as_str().to_string()),
                    format!("Unknown inline node type for ANSI: {}", n.kind.as_str()),
                );
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rescribe_std::builder::*;

        fn emit_str(doc: &Document) -> String {
            let result = emit(doc).unwrap();
            String::from_utf8(result.value).unwrap()
        }

        #[test]
        fn test_emit_paragraph() {
            let doc = doc(|d| d.para(|p| p.text("Hello, world!")));
            let output = emit_str(&doc);
            assert!(output.contains("Hello, world!"));
        }

        #[test]
        fn test_emit_heading() {
            let doc = doc(|d| d.heading(1, |h| h.text("Title")));
            let output = emit_str(&doc);
            assert!(output.contains("# Title"));
            assert!(output.contains("\x1b[1m"));
        }

        #[test]
        fn test_emit_bold() {
            let doc = doc(|d| d.para(|p| p.strong(|s| s.text("bold"))));
            let output = emit_str(&doc);
            assert!(output.contains("bold"));
            assert!(output.contains("\x1b[1m"));
        }

        #[test]
        fn test_emit_italic() {
            let doc = doc(|d| d.para(|p| p.em(|e| e.text("italic"))));
            let output = emit_str(&doc);
            assert!(output.contains("italic"));
            assert!(output.contains("\x1b[3m"));
        }

        #[test]
        fn test_emit_code() {
            let doc = doc(|d| d.para(|p| p.code("code")));
            let output = emit_str(&doc);
            assert!(output.contains("code"));
        }

        #[test]
        fn test_emit_code_block() {
            let doc = doc(|d| d.code_block_lang("fn main() {}", "rust"));
            let output = emit_str(&doc);
            assert!(output.contains("rust"));
            assert!(output.contains("fn main() {}"));
        }

        #[test]
        fn test_emit_link() {
            let doc = doc(|d| d.para(|p| p.link("https://example.com", |l| l.text("click"))));
            let output = emit_str(&doc);
            assert!(output.contains("click"));
            assert!(output.contains("https://example.com"));
        }

        #[test]
        fn test_emit_list() {
            let doc = doc(|d| d.bullet_list(|l| l.item(|i| i.text("one")).item(|i| i.text("two"))));
            let output = emit_str(&doc);
            assert!(output.contains("one"));
            assert!(output.contains("two"));
            assert!(output.contains("•"));
        }

        #[test]
        fn test_emit_horizontal_rule() {
            let doc = doc(|d| d.hr());
            let output = emit_str(&doc);
            assert!(output.contains("───"));
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::{parse, parse_with_options};
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::{emit, emit_with_options};
