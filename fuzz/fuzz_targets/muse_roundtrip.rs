#![no_main]

//! Muse roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with Muse-compatible constructs,
//! emits them to Muse text via rescribe-write-muse, parses them back via
//! rescribe-read-muse, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: Muse inline markup (`**bold**`, `*italic*`,
//! `=code=`, `[[links]]`) is context-sensitive. Plain text containing these
//! characters may be re-parsed as markup on the return trip. The sanitiser
//! strips them so the corpus exercises structural variations rather than
//! inline-markup edge cases.
//!
//! Structural markers stripped from plain text:
//! - Heading markers: `* ` to `**** ` at start of line
//! - Unordered list: ` - ` at start
//! - Ordered list: ` N. ` at start
//! - Block tags: `<verse>`, `<quote>`, `<example>`, `<src>` at line start
//! - Directive markers: `#` at line start

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

/// Sanitise text: strip ASCII control characters and Muse inline markup
/// delimiters that would be re-interpreted on parse-back.
/// Returns None for empty results.
fn sanitise(s: &str) -> Option<String> {
    // Strip characters that Muse uses as inline markup or structural markers.
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                // All ASCII control chars 0x00–0x1f
                '\x00'..='\x1f'
                    // Muse inline markup delimiters
                    | '*' | '='
                    // Link syntax
                    | '[' | ']'
                    // XML-like tag syntax (verse/quote/example blocks)
                    | '<' | '>'
                    // Quote character (used in various contexts)
                    | '"'
                    // Backslash (escape)
                    | '\\'
            )
        })
        .collect();

    // Normalize: trim + strip structural markers until stable.
    let mut out = out.trim().to_string();
    loop {
        let prev = out.clone();
        out = out.trim().to_string();

        // Muse heading markers: leading '*' chars followed by space
        // (since '*' chars are already stripped from content, this handles
        // any residual patterns from other combinations)
        while out.starts_with("* ")
            || out.starts_with("** ")
            || out.starts_with("*** ")
            || out.starts_with("**** ")
        {
            // Strip up to 4 leading '*' + space
            let stripped = out.trim_start_matches('*').trim_start().to_string();
            out = stripped;
        }

        // Muse unordered list: " - " at start
        if out.starts_with("- ") {
            out = out[2..].trim().to_string();
        }

        // Muse ordered list: " N. " at start — strip leading digits followed by ". "
        loop {
            let trimmed = out
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .to_string();
            if trimmed.starts_with(". ") {
                let cut = out.len() - trimmed.len() + 2;
                out = out[cut..].trim().to_string();
            } else {
                break;
            }
        }

        // Muse definition list trigger: " :: " in the middle
        // Strip " :: " and everything after to prevent def-list misparse
        if let Some(pos) = out.find(" :: ") {
            out = out[..pos].trim().to_string();
        }

        // Directive: '#' at start
        while out.starts_with('#') {
            out = out[1..].trim().to_string();
        }

        // Horizontal rule: 4+ dashes
        while out.starts_with("----") {
            out = out[4..].trim().to_string();
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
        // CODE uses prop::CONTENT directly (not a child TEXT node)
        FuzzInlineKind::Code => Node::new(node::CODE).prop(prop::CONTENT, text),
    })
}

fn make_para_children(inlines: &[FuzzInline]) -> Vec<Node> {
    inlines.iter().filter_map(make_inline).collect()
}

fn make_list_item(inlines: &[FuzzInline]) -> Option<Node> {
    let children: Vec<Node> = inlines
        .iter()
        .filter_map(|fi| {
            let clean = sanitise(&fi.text)?;
            Some(Node::new(node::TEXT).prop(prop::CONTENT, clean))
        })
        .collect();
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
                // Levels 1–4 (* through ****).
                let lvl = i64::from(*level % 4) + 1;
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

    // Emit to Muse bytes — must not panic.
    let Ok(emit_result) = rescribe_write_muse::emit(&doc) else {
        return;
    };
    let _muse_for_debug = std::str::from_utf8(&emit_result.value).unwrap_or("").to_string();

    // Parse back — must not panic.
    let Ok(muse_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_muse::parse(muse_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "Muse roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  muse: {_muse_for_debug:?}"
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
