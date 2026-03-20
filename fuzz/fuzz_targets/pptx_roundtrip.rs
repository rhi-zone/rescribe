#![no_main]

//! PPTX roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with PPTX-supported constructs,
//! emits them to PPTX bytes via rescribe-write-pptx, parses them back via
//! rescribe-read-pptx, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why text-only comparison: PPTX writer uses PresentationBuilder which only
//! supports plain text (no bullet properties, no inline formatting), so
//! structure is necessarily lossy. But all text content must survive.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rescribe_core::{Document, Node};
use rescribe_std::{node, prop};

// ── Fuzz-friendly types ─────────────────────────────────────────────────────

/// Note: BulletList excluded because the writer adds "• " prefix to list items
/// (PresentationBuilder lacks bullet API), making roundtrip text comparison fail.
/// Lists are tested via fixtures instead.
#[derive(Arbitrary, Debug)]
enum FuzzSlideContent {
    Paragraph { text: String },
    Table { rows: Vec<Vec<String>> },
}

#[derive(Arbitrary, Debug)]
struct FuzzSlide {
    title: String,
    content: Vec<FuzzSlideContent>,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Sanitise text: strip NUL, CR, and non-printable ASCII control chars that
/// XML cannot represent. Returns None for empty results.
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                '\0' | '\r' | '\x01'..='\x08' | '\x0b' | '\x0c' | '\x0e'..='\x1f'
            )
        })
        .collect();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn make_text(s: &str) -> Option<Node> {
    let text = sanitise(s)?;
    Some(Node::new(node::TEXT).prop(prop::CONTENT, text))
}

// ── Fuzz target ─────────────────────────────────────────────────────────────

fuzz_target!(|slides: Vec<FuzzSlide>| {
    let slide_nodes: Vec<Node> = slides
        .iter()
        .enumerate()
        .filter_map(|(idx, slide)| {
            let slide_num = (idx + 1) as i64;
            let mut div = Node::new(node::DIV).prop("slide", slide_num);

            // Title → heading level 1
            if let Some(text_node) = make_text(&slide.title) {
                div = div.child(
                    Node::new(node::HEADING)
                        .prop(prop::LEVEL, 1i64)
                        .child(text_node),
                );
            }

            for item in &slide.content {
                match item {
                    FuzzSlideContent::Paragraph { text } => {
                        if let Some(text_node) = make_text(text) {
                            div = div.child(Node::new(node::PARAGRAPH).child(text_node));
                        }
                    }
                    FuzzSlideContent::Table { rows } => {
                        let row_nodes: Vec<Node> = rows
                            .iter()
                            .filter_map(|row| {
                                let cells: Vec<Node> = row
                                    .iter()
                                    .filter_map(|t| {
                                        let text_node = make_text(t)?;
                                        Some(Node::new(node::TABLE_CELL).child(
                                            Node::new(node::PARAGRAPH).child(text_node),
                                        ))
                                    })
                                    .collect();
                                if cells.is_empty() {
                                    None
                                } else {
                                    Some(Node::new(node::TABLE_ROW).children(cells))
                                }
                            })
                            .collect();
                        if !row_nodes.is_empty() {
                            div = div.child(Node::new(node::TABLE).children(row_nodes));
                        }
                    }
                }
            }

            // Skip empty slides
            if div.children.is_empty() {
                return None;
            }
            Some(div)
        })
        .collect();

    if slide_nodes.is_empty() {
        return;
    }

    let doc = Document::new().with_content(Node::new(node::DOCUMENT).children(slide_nodes));

    // Emit to PPTX bytes — must not panic.
    let Ok(emit_result) = rescribe_write_pptx::emit(&doc) else {
        return;
    };

    // Parse back — must not panic.
    let Ok(parse_result) = rescribe_read_pptx::parse(&emit_result.value) else {
        return;
    };

    // All visible text content must survive the roundtrip.
    // Compare as sorted chars because PPTX shape ordering isn't guaranteed
    // (tables/text boxes may be read in different order than written).
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    let mut chars_before: Vec<char> = text_before.chars().collect();
    let mut chars_after: Vec<char> = text_after.chars().collect();
    chars_before.sort();
    chars_after.sort();

    assert_eq!(
        chars_before,
        chars_after,
        "PPTX roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}"
    );
});

fn extract_text(node: &Node) -> String {
    let mut text = String::new();
    if node.kind.as_str() == node::TEXT {
        if let Some(content) = node.props.get_str(prop::CONTENT) {
            text.push_str(content);
        }
    }
    for child in &node.children {
        text.push_str(&extract_text(child));
    }
    text
}
