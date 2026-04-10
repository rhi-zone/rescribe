#![no_main]

//! EPUB roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with EPUB-supported constructs,
//! emits them to EPUB bytes via rescribe-write-epub, parses them back via
//! rescribe-read-epub, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! # Known limitation
//!
//! h1 headings are NOT included in fuzz input. The epub writer splits
//! documents at h1 boundaries, making each h1 a chapter title stored only
//! in the nav/NCX structure (not the XHTML body). The reader re-inserts
//! chapter titles only for multi-chapter documents and uses the spine item
//! id (not the title text), so h1 text cannot survive the round-trip.
//! All other heading levels (h2–h6) are included in chapter XHTML and do
//! round-trip correctly.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rescribe_core::{Document, Node};
use rescribe_std::{node, prop};

// ── Fuzz-friendly inline types ────────────────────────────────────────────────

#[derive(Arbitrary, Debug)]
enum FuzzInlineKind {
    Plain,
    Bold,
    Italic,
    Underline,
    Strikeout,
    Code,
}

#[derive(Arbitrary, Debug)]
struct FuzzInline {
    text: String,
    kind: FuzzInlineKind,
}

// ── Fuzz-friendly block types ─────────────────────────────────────────────────

#[derive(Arbitrary, Debug)]
enum FuzzBlock {
    Paragraph { inlines: Vec<FuzzInline> },
    // levels 2–6 only: level 1 is reserved for chapter splits (would lose text)
    Heading { level: u8, inlines: Vec<FuzzInline> },
    BulletList { items: Vec<Vec<FuzzInline>> },
    OrderedList { items: Vec<Vec<FuzzInline>> },
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Sanitise text: strip NUL, CR, non-printable ASCII control chars, and
/// newlines that XHTML cannot represent or that the HTML parser will
/// normalize away. Returns None for empty or whitespace-only results.
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| !matches!(*c, '\0' | '\r' | '\n' | '\x01'..='\x08' | '\x0b' | '\x0c' | '\x0e'..='\x1f'))
        .collect();
    // HTML parsers normalize whitespace-only text nodes to empty strings
    if out.trim().is_empty() { None } else { Some(out) }
}

fn make_inline(fi: &FuzzInline) -> Option<Node> {
    let text = sanitise(&fi.text)?;
    let leaf = Node::new(node::TEXT).prop(prop::CONTENT, text.clone());
    Some(match fi.kind {
        FuzzInlineKind::Plain => leaf,
        FuzzInlineKind::Bold => Node::new(node::STRONG).child(leaf),
        FuzzInlineKind::Italic => Node::new(node::EMPHASIS).child(leaf),
        FuzzInlineKind::Underline => Node::new(node::UNDERLINE).child(leaf),
        FuzzInlineKind::Strikeout => Node::new(node::STRIKEOUT).child(leaf),
        // The HTML writer reads inline-code text from prop::CONTENT (not child TEXT nodes).
        // The HTML reader also produces CODE with prop::CONTENT. Use the same convention.
        FuzzInlineKind::Code => Node::new(node::CODE).prop(prop::CONTENT, text),
    })
}

fn make_para_children(inlines: &[FuzzInline]) -> Vec<Node> {
    inlines.iter().filter_map(make_inline).collect()
}

fn make_list_item(inlines: &[FuzzInline]) -> Option<Node> {
    let children = make_para_children(inlines);
    if children.is_empty() {
        return None;
    }
    let para = Node::new(node::PARAGRAPH).children(children);
    Some(Node::new(node::LIST_ITEM).child(para))
}

// ── Fuzz target ───────────────────────────────────────────────────────────────

fuzz_target!(|blocks: Vec<FuzzBlock>| {
    let content_nodes: Vec<Node> = blocks
        .iter()
        .filter_map(|b| match b {
            FuzzBlock::Paragraph { inlines } => {
                let children = make_para_children(inlines);
                if children.is_empty() {
                    None
                } else {
                    Some(Node::new(node::PARAGRAPH).children(children))
                }
            }
            FuzzBlock::Heading { level, inlines } => {
                let children = make_para_children(inlines);
                if children.is_empty() {
                    return None;
                }
                // Levels 2–6 only: level 1 splits chapters and text is lost
                let lvl = i64::from(*level % 5) + 2; // 2–6
                Some(Node::new(node::HEADING).prop(prop::LEVEL, lvl).children(children))
            }
            FuzzBlock::BulletList { items } => {
                let list_items: Vec<Node> =
                    items.iter().filter_map(|i| make_list_item(i)).collect();
                if list_items.is_empty() {
                    None
                } else {
                    Some(
                        Node::new(node::LIST)
                            .prop(prop::ORDERED, false)
                            .children(list_items),
                    )
                }
            }
            FuzzBlock::OrderedList { items } => {
                let list_items: Vec<Node> =
                    items.iter().filter_map(|i| make_list_item(i)).collect();
                if list_items.is_empty() {
                    None
                } else {
                    Some(
                        Node::new(node::LIST)
                            .prop(prop::ORDERED, true)
                            .children(list_items),
                    )
                }
            }
        })
        .collect();

    if content_nodes.is_empty() {
        return;
    }

    let doc = Document::new().with_content(Node::new(node::DOCUMENT).children(content_nodes));

    // Emit to EPUB bytes — must not panic.
    let Ok(emit_result) = rescribe_write_epub::emit(&doc) else {
        return;
    };

    // Parse back — must not panic.
    let Ok(parse_result) = rescribe_read_epub::parse_bytes(&emit_result.value) else {
        return;
    };

    // All visible text content must survive the round-trip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "EPUB roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}"
    );
});

fn extract_text(node: &Node) -> String {
    let mut text = String::new();
    if node.kind.as_str() == node::TEXT {
        if let Some(content) = node.props.get_str(prop::CONTENT) {
            text.push_str(content);
        }
    } else if node.kind.as_str() == node::CODE {
        // Inline code stores its text as prop::CONTENT (not a child TEXT node).
        if let Some(content) = node.props.get_str(prop::CONTENT) {
            text.push_str(content);
        }
    }
    for child in &node.children {
        text.push_str(&extract_text(child));
    }
    text
}
