#![no_main]

//! ANSI roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents containing paragraphs with styled
//! inlines, emits them to ANSI text via rescribe-write-ansi, parses them back
//! via rescribe-read-ansi, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: ANSI uses `ESC [` sequences. Text containing
//! `\x1b` (ESC) would be re-parsed as a style code on the return trip.
//! Control characters including `\n` split paragraphs.
//!
//! Why only paragraphs: The ANSI emitter adds visual chrome to headings
//! (coloured `#` prefix) and lists (`• ` / `N. ` prefix) that the parser
//! cannot strip — those prefixes become part of the recovered text. Only
//! plain paragraphs round-trip cleanly.

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
    Strikethrough,
}

#[derive(Arbitrary, Debug)]
struct FuzzInline {
    text: String,
    kind: FuzzInlineKind,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Sanitise text: strip characters that would interfere with the ANSI
/// round-trip.
///
/// Characters stripped:
/// - All ASCII control characters 0x00–0x1f (includes `\n`, `\r`, `\t`)
///   — newlines split paragraphs; the parser joins lines with space so
///   multi-line paragraphs collapse.
/// - `\x1b` (ESC) — would be re-interpreted as an ANSI escape sequence.
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| !matches!(*c, '\x00'..='\x1f' | '\x7f'))
        .collect();
    let out = out.trim().to_string();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn make_inline(fi: &FuzzInline) -> Option<Node> {
    let text = sanitise(&fi.text)?;
    let leaf = Node::new(node::TEXT).prop(prop::CONTENT, text);
    Some(match fi.kind {
        FuzzInlineKind::Plain => leaf,
        FuzzInlineKind::Bold => Node::new(node::STRONG).child(leaf),
        FuzzInlineKind::Italic => Node::new(node::EMPHASIS).child(leaf),
        FuzzInlineKind::Underline => Node::new(node::UNDERLINE).child(leaf),
        FuzzInlineKind::Strikethrough => Node::new(node::STRIKEOUT).child(leaf),
    })
}

fn make_para_children(inlines: &[FuzzInline]) -> Vec<Node> {
    inlines.iter().filter_map(make_inline).collect()
}

// ── Fuzz target ───────────────────────────────────────────────────────────────

fuzz_target!(|inlines_list: Vec<Vec<FuzzInline>>| {
    let content_nodes: Vec<Node> = inlines_list
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

    // Emit to ANSI bytes — must not panic.
    let Ok(emit_result) = rescribe_write_ansi::emit(&doc) else {
        return;
    };
    let _ansi_for_debug = std::str::from_utf8(&emit_result.value)
        .unwrap_or("")
        .to_string();

    // Parse back — must not panic.
    let Ok(ansi_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_ansi::parse(ansi_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "ANSI roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  ansi: {_ansi_for_debug:?}"
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
