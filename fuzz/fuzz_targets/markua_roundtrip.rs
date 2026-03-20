#![no_main]

//! Markua roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with Markua-compatible constructs,
//! emits them to Markua text via rescribe-write-markua, parses them back via
//! rescribe-read-markua, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: Markua (like Markdown) uses * ** ~~ ` [ ] ( )
//! ! # > - as inline/block markup. Text containing these is re-parsed as
//! markup on the return trip. The sanitiser strips them so the corpus exercises
//! structural variations rather than inline-markup edge cases.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rescribe_core::{Document, Node};
use rescribe_std::{node, prop};

// ── Fuzz-friendly inline types ────────────────────────────────────────────────

#[derive(Arbitrary, Debug)]
enum FuzzInlineKind {
    Plain,
    Bold,
    // Italic excluded from mixed-inline paragraphs: adjacent *italic* and
    // **bold** produce `*a***b**` which is ambiguous — the re-parse yields
    // different structure. This is an inherent Markua/Markdown limitation,
    // not a roundtrip bug. Italic-only paragraphs are fine; see FuzzBlock.
    Strikethrough,
}

/// An italic-only paragraph with no adjacent Bold spans.
#[derive(Arbitrary, Debug)]
struct FuzzItalicPara {
    texts: Vec<String>,
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
    ItalicPara { para: FuzzItalicPara },
    Heading { level: u8, inlines: Vec<FuzzInline> },
    BulletList { items: Vec<Vec<FuzzInline>> },
    OrderedList { items: Vec<Vec<FuzzInline>> },
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Sanitise text: strip characters that would be re-interpreted as Markua
/// markup on the parse-back trip.
///
/// Markua is Markdown-based; characters stripped:
/// - ASCII control characters 0x00–0x1f (includes `\n`, `\r`, `\t`)
/// - `*` `_` — bold/italic delimiters and scene-break chars
/// - `~` — strikethrough delimiter
/// - `` ` `` — inline code delimiter
/// - `[` `]` `(` `)` `!` — link and image syntax
/// - `#` — ATX heading marker (at line start, but strip everywhere for safety)
/// - `>` — blockquote and special-block marker (A>, W>, etc.)
/// - `-` `+` — unordered list markers and scene break
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                '\x00'..='\x1f'
                    | '*' | '_' | '~' | '`' | '[' | ']' | '(' | ')' | '!'
                    | '#' | '>' | '-' | '+'
            )
        })
        .collect();

    let mut out = out.trim().to_string();

    // Strip patterns that become structural on the parse-back trip.
    loop {
        let prev = out.clone();
        out = out.trim().to_string();
        // Ordered list prefix: N. text or N) text
        loop {
            let trimmed = out.trim_start_matches(|c: char| c.is_ascii_digit()).to_string();
            if trimmed.starts_with(". ") {
                let cut = out.len() - trimmed.len() + 2;
                out = out[cut..].trim().to_string();
            } else if trimmed == "." {
                // Bare "N." with no content — parser treats it as an ordered list
                // marker and emits nothing; strip the whole string.
                out = String::new();
                break;
            } else if trimmed.starts_with(") ") {
                let cut = out.len() - trimmed.len() + 2;
                out = out[cut..].trim().to_string();
            } else if trimmed == ")" {
                // Bare "N)" with no content — same issue.
                out = String::new();
                break;
            } else {
                break;
            }
        }
        if out == prev {
            break;
        }
    }

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
        FuzzInlineKind::Strikethrough => Node::new(node::STRIKEOUT).child(leaf),
    })
}

fn make_para_children(inlines: &[FuzzInline]) -> Vec<Node> {
    inlines.iter().filter_map(make_inline).collect()
}

fn make_italic_para(para: &FuzzItalicPara) -> Option<Node> {
    // Concatenate all texts into one string, then wrap in a single Emphasis node.
    // Multiple adjacent *italic* spans would emit *a**b* which creates *** ambiguity;
    // one span avoids that entirely.
    let combined: String = para.texts.iter().filter_map(|t| sanitise(t)).collect::<Vec<_>>().join(" ");
    let combined = combined.trim().to_string();
    if combined.is_empty() {
        return None;
    }
    let leaf = Node::new(node::TEXT).prop(prop::CONTENT, combined);
    let em = Node::new(node::EMPHASIS).child(leaf);
    Some(Node::new(node::PARAGRAPH).child(em))
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
            FuzzBlock::ItalicPara { para } => make_italic_para(para),
            FuzzBlock::Heading { level, inlines } => {
                let children = make_para_children(inlines);
                if children.is_empty() {
                    return None;
                }
                let lvl = i64::from(*level % 6) + 1; // 1–6
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

    // Emit to Markua bytes — must not panic.
    let Ok(emit_result) = rescribe_write_markua::emit(&doc) else {
        return;
    };
    let _markua_for_debug = std::str::from_utf8(&emit_result.value)
        .unwrap_or("")
        .to_string();

    // Parse back — must not panic.
    let Ok(markua_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_markua::parse(markua_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "Markua roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  markua: {_markua_for_debug:?}"
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
