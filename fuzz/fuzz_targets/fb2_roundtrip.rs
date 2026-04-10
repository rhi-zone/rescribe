#![no_main]

//! FB2 roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with FB2-supported constructs,
//! emits them to FB2 XML bytes via rescribe-write-fb2, parses them back
//! via rescribe-read-fb2, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! # Known limitations
//!
//! - Lists are excluded: the FB2 writer emits list items as plain `<p>` elements
//!   with bullet/number markers prepended to the text, so the text content changes.
//! - Headings are excluded: the writer wraps heading content in `<title><p>…</p></title>`
//!   and nesting level determines the heading level; a heading emitted in one flat
//!   section would parse back as level 1 regardless of original level.

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
    Strikeout,
    Code,
    Subscript,
    Superscript,
}

#[derive(Arbitrary, Debug)]
struct FuzzInline {
    text: String,
    kind: FuzzInlineKind,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Sanitise text: strip NUL, CR, XML-illegal control chars, and lone surrogates.
/// Returns None for empty or whitespace-only results.
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(*c, '\0' | '\r' | '\x01'..='\x08' | '\x0b' | '\x0c' | '\x0e'..='\x1f')
        })
        .collect();
    if out.trim().is_empty() { None } else { Some(out) }
}

fn make_inline(fi: &FuzzInline) -> Option<Node> {
    let text = sanitise(&fi.text)?;
    let leaf = Node::new(node::TEXT).prop(prop::CONTENT, text.clone());
    Some(match fi.kind {
        FuzzInlineKind::Plain => leaf,
        FuzzInlineKind::Bold => Node::new(node::STRONG).child(leaf),
        FuzzInlineKind::Italic => Node::new(node::EMPHASIS).child(leaf),
        FuzzInlineKind::Strikeout => Node::new(node::STRIKEOUT).child(leaf),
        // FB2 code inline: writer reads from prop::CONTENT, reader also produces prop::CONTENT.
        FuzzInlineKind::Code => Node::new(node::CODE).prop(prop::CONTENT, text),
        FuzzInlineKind::Subscript => Node::new(node::SUBSCRIPT).child(leaf),
        FuzzInlineKind::Superscript => Node::new(node::SUPERSCRIPT).child(leaf),
    })
}

fn make_para_children(inlines: &[FuzzInline]) -> Vec<Node> {
    inlines.iter().filter_map(make_inline).collect()
}

// ── Fuzz target ───────────────────────────────────────────────────────────────

fuzz_target!(|inlines_batches: Vec<Vec<FuzzInline>>| {
    // Each inner Vec is one paragraph.
    let content_nodes: Vec<Node> = inlines_batches
        .iter()
        .filter_map(|inlines| {
            let children = make_para_children(inlines);
            if children.is_empty() {
                None
            } else {
                Some(Node::new(node::PARAGRAPH).children(children))
            }
        })
        .collect();

    if content_nodes.is_empty() {
        return;
    }

    let doc = Document::new().with_content(Node::new(node::DOCUMENT).children(content_nodes));

    // Emit to FB2 bytes — must not panic.
    let Ok(emit_result) = rescribe_write_fb2::emit(&doc) else {
        return;
    };

    // Parse back — must not panic.
    let Ok(xml_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_fb2::parse(xml_str) else {
        return;
    };

    // All visible text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "FB2 roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}"
    );
});

fn extract_text(node: &Node) -> String {
    let mut text = String::new();
    if node.kind.as_str() == node::TEXT {
        if let Some(content) = node.props.get_str(prop::CONTENT) {
            text.push_str(content);
        }
    } else if node.kind.as_str() == node::CODE {
        // Inline code stores text in prop::CONTENT.
        if let Some(content) = node.props.get_str(prop::CONTENT) {
            text.push_str(content);
        }
    }
    for child in &node.children {
        text.push_str(&extract_text(child));
    }
    text
}
