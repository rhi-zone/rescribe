//! CommonMark parser — wraps pulldown-cmark's offset iterator into the [`CmDoc`] AST.

use crate::ast::{
    Block, CmDoc, Diagnostic, Inline, LinkDef, ListItem, ListKind, OrderedMarker, Severity, Span,
};
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

// ── Frame stack ──────────────────────────────────────────────────────────────

/// One level in the tree-builder stack.
enum Frame {
    /// The root document accumulates top-level blocks.
    Doc { blocks: Vec<Block> },
    Blockquote { blocks: Vec<Block>, start: usize },
    List { kind: ListKind, items: Vec<ListItem>, tight: bool, start: usize },
    Item { blocks: Vec<Block>, tight_para: bool, tight_inlines: Vec<Inline>, start: usize },
    Paragraph { inlines: Vec<Inline>, start: usize },
    Heading { level: u8, inlines: Vec<Inline>, start: usize },
    Emphasis { inlines: Vec<Inline>, start: usize },
    Strong { inlines: Vec<Inline>, start: usize },
    Strikethrough { inlines: Vec<Inline>, start: usize },
    Link { inlines: Vec<Inline>, url: String, title: Option<String>, start: usize },
    /// Accumulates the alt text from the text events inside an image tag.
    Image { alt: String, url: String, title: Option<String>, start: usize },
    /// A buffered HTML block: content is accumulated from consecutive Html events.
    HtmlBlock { content: String, start: usize },
    /// A code block: accumulates a single Text event as content.
    CodeBlock { language: Option<String>, content: String, start: usize },
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Parse CommonMark (plus GFM strikethrough) from a byte slice.
///
/// Always succeeds; non-UTF-8 input produces a single `Warning` diagnostic and
/// an empty document. Any unknown pulldown-cmark events are silently skipped —
/// this is a strict superset of the CommonMark spec and diagnostics are only
/// generated for encoding problems.
pub fn parse(input: &[u8]) -> (CmDoc, Vec<Diagnostic>) {
    let s = match std::str::from_utf8(input) {
        Ok(s) => s,
        Err(_) => {
            return (
                CmDoc { blocks: vec![], link_defs: vec![] },
                vec![Diagnostic {
                    span: Span::NONE,
                    severity: Severity::Warning,
                    message: "input is not valid UTF-8".to_string(),
                    code: "commonmark::invalid-utf8",
                }],
            );
        }
    };
    parse_str(s)
}

/// Parse CommonMark (plus GFM strikethrough) from a `&str`.
pub fn parse_str(input: &str) -> (CmDoc, Vec<Diagnostic>) {
    let opts = Options::ENABLE_STRIKETHROUGH;
    let iter = Parser::new_ext(input, opts).into_offset_iter();

    let mut stack: Vec<Frame> = vec![Frame::Doc { blocks: vec![] }];
    let diagnostics: Vec<Diagnostic> = vec![];

    // pulldown-cmark exposes reference link definitions via a separate API.
    let link_defs = collect_link_defs(input);

    for (event, range) in iter {
        let start = range.start;
        let end = range.end;

        match event {
            // ── Block opens ──────────────────────────────────────────────────
            Event::Start(Tag::Paragraph) => {
                stack.push(Frame::Paragraph { inlines: vec![], start });
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let level_u8 = heading_level_to_u8(level);
                stack.push(Frame::Heading { level: level_u8, inlines: vec![], start });
            }
            Event::Start(Tag::BlockQuote(_)) => {
                stack.push(Frame::Blockquote { blocks: vec![], start });
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(lang) => {
                        let s = lang.trim().to_string();
                        if s.is_empty() { None } else { Some(s) }
                    }
                    CodeBlockKind::Indented => None,
                };
                stack.push(Frame::CodeBlock { language, content: String::new(), start });
            }
            Event::Start(Tag::List(first)) => {
                let kind = match first {
                    None => ListKind::Unordered { marker: '-' },
                    Some(n) => ListKind::Ordered { start: n, marker: OrderedMarker::Period },
                };
                // tight is determined later by whether Item children contain paragraphs
                stack.push(Frame::List { kind, items: vec![], tight: true, start });
            }
            Event::Start(Tag::Item) => {
                stack.push(Frame::Item {
                    blocks: vec![],
                    tight_para: false,
                    tight_inlines: vec![],
                    start,
                });
            }
            Event::Start(Tag::HtmlBlock) => {
                stack.push(Frame::HtmlBlock { content: String::new(), start });
            }

            // ── Inline opens ──────────────────────────────────────────────────
            Event::Start(Tag::Emphasis) => {
                stack.push(Frame::Emphasis { inlines: vec![], start });
            }
            Event::Start(Tag::Strong) => {
                stack.push(Frame::Strong { inlines: vec![], start });
            }
            Event::Start(Tag::Strikethrough) => {
                stack.push(Frame::Strikethrough { inlines: vec![], start });
            }
            Event::Start(Tag::Link { link_type, dest_url, title, .. }) => {
                let raw_url = dest_url.into_string();
                let url = if link_type == pulldown_cmark::LinkType::Email
                    && !raw_url.starts_with("mailto:")
                {
                    format!("mailto:{raw_url}")
                } else {
                    raw_url
                };
                let title = if title.is_empty() { None } else { Some(title.into_string()) };
                stack.push(Frame::Link { inlines: vec![], url, title, start });
            }
            Event::Start(Tag::Image { dest_url, title, .. }) => {
                let url = dest_url.into_string();
                let title = if title.is_empty() { None } else { Some(title.into_string()) };
                stack.push(Frame::Image { alt: String::new(), url, title, start });
            }

            // ── Closes ────────────────────────────────────────────────────────
            Event::End(TagEnd::Paragraph) => {
                let frame = stack.pop();
                if let Some(Frame::Paragraph { inlines, start: s }) = frame {
                    let block = Block::Paragraph { inlines, span: Span { start: s, end } };
                    // If inside an item, mark it as having an explicit paragraph child
                    // (→ loose list).
                    if let Some(Frame::Item { blocks, tight_para, .. }) = stack.last_mut() {
                        *tight_para = true;
                        blocks.push(block);
                    } else {
                        push_block(&mut stack, block);
                    }
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                let frame = stack.pop();
                if let Some(Frame::Heading { level, inlines, start: s }) = frame {
                    let block = Block::Heading { level, inlines, span: Span { start: s, end } };
                    push_block(&mut stack, block);
                }
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                let frame = stack.pop();
                if let Some(Frame::Blockquote { blocks, start: s }) = frame {
                    let block = Block::Blockquote { blocks, span: Span { start: s, end } };
                    push_block(&mut stack, block);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                let frame = stack.pop();
                if let Some(Frame::CodeBlock { language, content, start: s }) = frame {
                    let block = Block::CodeBlock { language, content, span: Span { start: s, end } };
                    push_block(&mut stack, block);
                }
            }
            Event::End(TagEnd::List(_)) => {
                let frame = stack.pop();
                if let Some(Frame::List { kind, items, tight, start: s }) = frame {
                    let block = Block::List { kind, items, tight, span: Span { start: s, end } };
                    push_block(&mut stack, block);
                }
            }
            Event::End(TagEnd::Item) => {
                let frame = stack.pop();
                if let Some(Frame::Item { mut blocks, tight_para, tight_inlines, start: s }) = frame {
                    // Tight list items accumulate inlines directly (no Paragraph wrapper
                    // from pulldown). Wrap them in an implicit paragraph so every item
                    // always has Block children — consistent with loose items.
                    if !tight_inlines.is_empty() {
                        blocks.push(Block::Paragraph {
                            inlines: tight_inlines,
                            span: Span { start: s, end },
                        });
                    }
                    let item = ListItem { blocks, span: Span { start: s, end } };
                    // If this item had explicit paragraphs, mark the parent list as loose.
                    if tight_para
                        && let Some(Frame::List { tight, .. }) = stack.last_mut()
                    {
                        *tight = false;
                    }
                    if let Some(Frame::List { items, .. }) = stack.last_mut() {
                        items.push(item);
                    }
                }
            }
            Event::End(TagEnd::HtmlBlock) => {
                let frame = stack.pop();
                if let Some(Frame::HtmlBlock { content, start: s }) = frame {
                    let block = Block::HtmlBlock { content, span: Span { start: s, end } };
                    push_block(&mut stack, block);
                }
            }
            Event::End(TagEnd::Emphasis) => {
                let frame = stack.pop();
                if let Some(Frame::Emphasis { inlines, start: s }) = frame {
                    let inline = Inline::Emphasis { inlines, span: Span { start: s, end } };
                    push_inline(&mut stack, inline);
                }
            }
            Event::End(TagEnd::Strong) => {
                let frame = stack.pop();
                if let Some(Frame::Strong { inlines, start: s }) = frame {
                    let inline = Inline::Strong { inlines, span: Span { start: s, end } };
                    push_inline(&mut stack, inline);
                }
            }
            Event::End(TagEnd::Strikethrough) => {
                let frame = stack.pop();
                if let Some(Frame::Strikethrough { inlines, start: s }) = frame {
                    let inline = Inline::Strikethrough { inlines, span: Span { start: s, end } };
                    push_inline(&mut stack, inline);
                }
            }
            Event::End(TagEnd::Link) => {
                let frame = stack.pop();
                if let Some(Frame::Link { inlines, url, title, start: s }) = frame {
                    let inline = Inline::Link { inlines, url, title, span: Span { start: s, end } };
                    push_inline(&mut stack, inline);
                }
            }
            Event::End(TagEnd::Image) => {
                let frame = stack.pop();
                if let Some(Frame::Image { alt, url, title, start: s }) = frame {
                    let inline = Inline::Image { alt, url, title, span: Span { start: s, end } };
                    push_inline(&mut stack, inline);
                }
            }

            // ── Leaf events ───────────────────────────────────────────────────
            Event::Text(text) => {
                let s = text.into_string();
                // Text events inside an image frame accumulate the alt text.
                if let Some(Frame::Image { alt, .. }) = stack.last_mut() {
                    alt.push_str(&s);
                } else if let Some(Frame::CodeBlock { content, .. }) = stack.last_mut() {
                    content.push_str(&s);
                } else if let Some(Frame::HtmlBlock { content, .. }) = stack.last_mut() {
                    content.push_str(&s);
                } else {
                    let inline = Inline::Text { content: s, span: Span { start, end } };
                    push_inline(&mut stack, inline);
                }
            }
            Event::Code(text) => {
                let inline = Inline::Code {
                    content: text.into_string(),
                    span: Span { start, end },
                };
                push_inline(&mut stack, inline);
            }
            Event::Html(text) => {
                // Html events are block-level raw HTML; they arrive while HtmlBlock frame is on stack.
                if let Some(Frame::HtmlBlock { content, .. }) = stack.last_mut() {
                    content.push_str(&text);
                } else {
                    // Unexpected Html event outside HtmlBlock frame — treat as HtmlBlock directly.
                    let block =
                        Block::HtmlBlock { content: text.into_string(), span: Span { start, end } };
                    push_block(&mut stack, block);
                }
            }
            Event::InlineHtml(text) => {
                let inline = Inline::HtmlInline {
                    content: text.into_string(),
                    span: Span { start, end },
                };
                push_inline(&mut stack, inline);
            }
            Event::SoftBreak => {
                let inline = Inline::SoftBreak { span: Span { start, end } };
                push_inline(&mut stack, inline);
            }
            Event::HardBreak => {
                let inline = Inline::HardBreak { span: Span { start, end } };
                push_inline(&mut stack, inline);
            }
            Event::Rule => {
                let block = Block::ThematicBreak { span: Span { start, end } };
                push_block(&mut stack, block);
            }

            // ── Ignored events ───────────────────────────────────────────────
            // FootnoteReference, TaskListMarker, Math, etc. are pulldown-cmark
            // extensions we don't model here.
            _ => {}
        }
    }

    // Drain the root Doc frame.
    let blocks = match stack.into_iter().next() {
        Some(Frame::Doc { blocks }) => blocks,
        _ => vec![],
    };

    (CmDoc { blocks, link_defs }, diagnostics)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Push a completed block onto the nearest block-accepting frame.
fn push_block(stack: &mut [Frame], block: Block) {
    for frame in stack.iter_mut().rev() {
        match frame {
            Frame::Doc { blocks }
            | Frame::Blockquote { blocks, .. }
            | Frame::Item { blocks, .. } => {
                blocks.push(block);
                return;
            }
            _ => {}
        }
    }
}

/// Push a completed inline onto the nearest inline-accepting frame.
///
/// For tight list items, pulldown-cmark does not emit `Start/End(Paragraph)` events;
/// inlines arrive with only `Frame::Item` on the stack. We accumulate them in
/// `Frame::Item::tight_inlines` and wrap them in a `Block::Paragraph` at `End(Item)`.
fn push_inline(stack: &mut [Frame], inline: Inline) {
    for frame in stack.iter_mut().rev() {
        let target: &mut Vec<Inline> = match frame {
            Frame::Paragraph { inlines, .. }
            | Frame::Heading { inlines, .. }
            | Frame::Emphasis { inlines, .. }
            | Frame::Strong { inlines, .. }
            | Frame::Strikethrough { inlines, .. }
            | Frame::Link { inlines, .. } => inlines,
            // Tight list item: accumulate inlines for later wrapping in a paragraph.
            Frame::Item { tight_inlines, .. } => tight_inlines,
            // Image alt text is handled before push_inline is called.
            _ => continue,
        };
        // Merge consecutive Text nodes — pulldown-cmark can split a single logical
        // text run into multiple Text events (e.g. backslash escapes).
        if let Inline::Text { content: new_content, span: new_span } = &inline {
            if let Some(Inline::Text { content, span }) = target.last_mut() {
                content.push_str(new_content);
                span.end = new_span.end;
                return;
            }
        }
        target.push(inline);
        return;
    }
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Collect reference link definitions from the input string.
///
/// pulldown-cmark exposes these through [`Parser::reference_definitions`].
fn collect_link_defs(input: &str) -> Vec<LinkDef> {
    let opts = Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(input, opts);
    let defs = parser.reference_definitions();
    let mut out: Vec<LinkDef> = defs
        .iter()
        .map(|(label, def)| LinkDef {
            label: label.to_string(),
            url: def.dest.to_string(),
            title: def.title.as_ref().map(|t| t.to_string()),
        })
        .collect();
    // Sort for deterministic output.
    out.sort_by(|a, b| a.label.cmp(&b.label));
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paragraph() {
        let (doc, diags) = parse(b"Hello, world!");
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(&doc.blocks[0], Block::Paragraph { .. }));
    }

    #[test]
    fn test_heading() {
        let (doc, diags) = parse(b"# Heading 1\n\n## Heading 2\n");
        assert!(diags.is_empty());
        assert_eq!(doc.blocks.len(), 2);
        assert!(matches!(&doc.blocks[0], Block::Heading { level: 1, .. }));
        assert!(matches!(&doc.blocks[1], Block::Heading { level: 2, .. }));
    }

    #[test]
    fn test_emphasis_and_strong() {
        let (doc, diags) = parse(b"*em* and **strong**");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Emphasis { .. })));
            assert!(inlines.iter().any(|i| matches!(i, Inline::Strong { .. })));
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn test_code_block() {
        let (doc, diags) = parse(b"```rust\nfn main() {}\n```\n");
        assert!(diags.is_empty());
        assert!(matches!(
            &doc.blocks[0],
            Block::CodeBlock { language: Some(lang), content, .. }
            if lang == "rust" && content == "fn main() {}\n"
        ));
    }

    #[test]
    fn test_unordered_list() {
        let (doc, diags) = parse(b"- one\n- two\n- three\n");
        assert!(diags.is_empty());
        assert!(matches!(&doc.blocks[0], Block::List { kind: ListKind::Unordered { .. }, .. }));
        if let Block::List { items, .. } = &doc.blocks[0] {
            assert_eq!(items.len(), 3);
        }
    }

    #[test]
    fn test_ordered_list() {
        let (doc, diags) = parse(b"1. first\n2. second\n");
        assert!(diags.is_empty());
        assert!(matches!(
            &doc.blocks[0],
            Block::List { kind: ListKind::Ordered { start: 1, .. }, .. }
        ));
    }

    #[test]
    fn test_blockquote() {
        let (doc, diags) = parse(b"> A quoted paragraph.\n");
        assert!(diags.is_empty());
        assert!(matches!(&doc.blocks[0], Block::Blockquote { .. }));
    }

    #[test]
    fn test_thematic_break() {
        let (doc, diags) = parse(b"---\n");
        assert!(diags.is_empty());
        assert!(matches!(&doc.blocks[0], Block::ThematicBreak { .. }));
    }

    #[test]
    fn test_link() {
        let (doc, diags) = parse(b"[text](https://example.com)\n");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Link { .. })));
        }
    }

    #[test]
    fn test_image() {
        let (doc, diags) = parse(b"![alt text](img.png)\n");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Image { alt, .. } if alt == "alt text")));
        }
    }

    #[test]
    fn test_html_block() {
        let (doc, diags) = parse(b"<div>\ncontent\n</div>\n");
        assert!(diags.is_empty());
        assert!(matches!(&doc.blocks[0], Block::HtmlBlock { .. }));
    }

    #[test]
    fn test_inline_html() {
        let (doc, diags) = parse(b"text <em>inline</em> html\n");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::HtmlInline { .. })));
        }
    }

    #[test]
    fn test_invalid_utf8() {
        let (doc, diags) = parse(b"\xff\xfe");
        assert_eq!(doc.blocks.len(), 0);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "commonmark::invalid-utf8");
    }

    #[test]
    fn test_link_def() {
        let (doc, diags) = parse(b"[link][ref]\n\n[ref]: https://example.com\n");
        assert!(diags.is_empty());
        assert_eq!(doc.link_defs.len(), 1);
        assert_eq!(doc.link_defs[0].url, "https://example.com");
    }

    #[test]
    fn test_strip_spans() {
        let (doc, _) = parse(b"# Hello\n\nA paragraph.\n");
        let stripped = doc.strip_spans();
        for block in &stripped.blocks {
            match block {
                Block::Heading { span, .. } | Block::Paragraph { span, .. } => {
                    assert_eq!(*span, Span::NONE);
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_loose_list() {
        // A blank line between items makes a loose list.
        let (doc, _) = parse(b"- item one\n\n- item two\n");
        if let Block::List { tight, .. } = &doc.blocks[0] {
            assert!(!tight, "list with blank-separated items should be loose");
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_tight_list() {
        let (doc, _) = parse(b"- item one\n- item two\n");
        if let Block::List { tight, .. } = &doc.blocks[0] {
            assert!(*tight, "list without blank lines should be tight");
        } else {
            panic!("expected list");
        }
    }

    #[test]
    fn test_gfm_strikethrough() {
        let (doc, diags) = parse(b"~~deleted~~\n");
        assert!(diags.is_empty());
        if let Block::Paragraph { inlines, .. } = &doc.blocks[0] {
            assert!(inlines.iter().any(|i| matches!(i, Inline::Strikethrough { .. })));
        }
    }
}
