#![no_main]

//! txt2tags roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with txt2tags-compatible constructs,
//! emits them to txt2tags text via rescribe-write-t2t, parses them back via
//! rescribe-read-t2t, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: txt2tags inline markup uses double-char
//! delimiters (**bold**, //italic//, ``code``, --strike--, __underline__).
//! Plain text containing these characters may be re-parsed as markup on the
//! return trip. The sanitiser strips them so the corpus exercises structural
//! variations rather than inline-markup edge cases.
//!
//! FuzzInlineKind: Plain, Bold, Italic, Code only (simpler subset that
//! roundtrips cleanly without edge cases in ordered-list detection).

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
    Heading { level: u8, inlines: Vec<FuzzInline> },
    BulletList { items: Vec<Vec<FuzzInline>> },
    OrderedList { items: Vec<Vec<FuzzInline>> },
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Sanitise text: strip ASCII control characters and txt2tags inline markup
/// delimiters that would be re-interpreted on parse-back.
/// Returns None for empty results.
fn sanitise(s: &str) -> Option<String> {
    // Strip characters that are part of t2t inline markup or structural triggers.
    // '*' → **bold**, '/' → //italic//, '`' → ``code``,
    // '-' → --strikethrough-- (also avoids 20× '-' horizontal rule),
    // '_' → __underline__,
    // '[' ']' → links/images,
    // '{' '}' '\\' '^' '%' → other t2t special chars,
    // '+' → ordered list marker / numbered heading,
    // '=' → unnumbered heading / horizontal rule,
    // '\t' → blockquote (tab-indented lines),
    // '"' → raw block delimiter """,
    // '|' → table row.
    // ':' → prevents http:// URL auto-detection (when plain text starting with
    //        'h' or 'H' is followed by adjacent italic //text//, the emitted
    //        combination could look like "http://text//" which the parser
    //        then re-reads as a URL link rather than text+italic).
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                // All ASCII control chars 0x00–0x1f
                '\x00'..='\x1f'
                    // t2t inline markup delimiters
                    | '*' | '/' | '`' | '-' | '_' | '\''
                    // Link / image brackets
                    | '[' | ']'
                    // Other t2t special chars
                    | '{' | '}' | '\\' | '^' | '%'
                    // Structural markers that appear inline
                    | '+' | '=' | '"' | '|'
                    // ':' filtered to prevent URL auto-detection edge cases:
                    // plain text "http:" adjacent to italic "//x//" looks like a URL
                    | ':'
            )
        })
        .collect();

    // Normalize: trim + strip leading structural markers until stable.
    let mut out = out.trim().to_string();
    loop {
        let prev = out.clone();
        out = out.trim().to_string();

        // t2t heading markers: lines starting with '=' or '+' chars.
        // (Already filtered out '=' and '+' above, but be safe for future changes.)
        while out.starts_with('=') {
            out = out.trim_start_matches('=').trim_start().to_string();
        }
        while out.starts_with('+') {
            out = out.trim_start_matches('+').trim_start().to_string();
        }

        // Unordered list marker: "- item" (already filtered '-')
        // Ordered list marker: "+ item" (already filtered '+')
        // Comment line: "% ..." (already filtered '%')

        // Strip trailing '.' to prevent numbered list detection heuristics.
        while out.ends_with('.') {
            out.pop();
            out = out.trim_end().to_string();
        }

        out = out.trim().to_string();

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
    let leaf = Node::new(node::TEXT).prop(prop::CONTENT, text.clone());
    Some(match fi.kind {
        FuzzInlineKind::Plain => leaf,
        FuzzInlineKind::Bold => Node::new(node::STRONG).child(leaf),
        FuzzInlineKind::Italic => Node::new(node::EMPHASIS).child(leaf),
        // CODE uses prop::CONTENT directly (not a child TEXT node) — this is
        // how the t2t reader produces code nodes, so the writer reads it back.
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
                // Levels 1–5: '=' to '====='.
                let lvl = i64::from(*level % 5) + 1;
                Some(
                    Node::new(node::HEADING)
                        .prop(prop::LEVEL, lvl)
                        .children(children),
                )
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

    // Emit to txt2tags bytes — must not panic.
    let Ok(emit_result) = rescribe_write_t2t::emit(&doc) else {
        return;
    };
    let _t2t_for_debug = std::str::from_utf8(&emit_result.value)
        .unwrap_or("")
        .to_string();

    // Parse back — must not panic.
    let Ok(t2t_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_t2t::parse(t2t_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "txt2tags roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  t2t: {_t2t_for_debug:?}"
    );
});

fn extract_text(node: &Node) -> String {
    let mut text = String::new();
    // TEXT nodes store content in prop::CONTENT; CODE nodes also use prop::CONTENT.
    if node.kind.as_str() == node::TEXT || node.kind.as_str() == node::CODE {
        if let Some(content) = node.props.get_str(prop::CONTENT) {
            text.push_str(content);
        }
    }
    for child in &node.children {
        text.push_str(&extract_text(child));
    }
    text
}
