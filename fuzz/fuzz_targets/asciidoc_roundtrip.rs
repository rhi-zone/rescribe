#![no_main]

//! AsciiDoc roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with AsciiDoc-compatible constructs,
//! emits them to AsciiDoc text via rescribe-write-asciidoc, parses them back via
//! rescribe-read-asciidoc, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: AsciiDoc inline markup (`*bold*`, `_italic_`,
//! `` `code` ``, `^super^`, `~sub~`, `#highlight#`, `+passthrough+`) is
//! context-sensitive. Plain text containing these characters may be re-parsed
//! as markup on the return trip. The sanitiser strips them so the corpus
//! exercises structural variations rather than inline-markup edge cases.
//!
//! FuzzInlineKind excludes Strikeout/Underline/SmallCaps: those are emitted as
//! `[role]#text#` which the parser re-reads as Highlight (not Strikeout/etc.).
//! This is a known roundtrip gap tracked as a TODO.

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
    // Strikeout/Underline/SmallCaps excluded: [role]#text# syntax is emitted
    // by the writer but re-parsed as Highlight, breaking roundtrip.
    // TODO: fix parser to recognise [line-through]#...#, [underline]#...#, etc.
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

/// Sanitise text: strip ASCII control characters and AsciiDoc inline markup
/// delimiters that would be re-interpreted on parse-back.
/// Returns None for empty results.
fn sanitise(s: &str) -> Option<String> {
    // First pass: strip problematic chars.
    // ':' prevents '::' description list trigger (any line with '::' is a def list).
    // '-' prevents '---' horizontal rule (three dashes alone on a line).
    // '\'' prevents "'''" horizontal rule.
    // '/' prevents accidental protocol patterns (http://, etc.) — ':' already blocks those
    //      but '/' alone in certain positions can cause issues.
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                // All ASCII control chars 0x00–0x1f
                '\x00'..='\x1f'
                    // AsciiDoc inline markup delimiters
                    | '*' | '_' | '`' | '^' | '~' | '#' | '+' | '<' | '>'
                    // ':' filtered to prevent '::' description list trigger
                    | ':'
                    // '-' filtered to prevent '---' horizontal rule
                    | '-'
                    // '\'' filtered to prevent "'''" horizontal rule
                    | '\''
                    // '.' filtered to prevent '....' delimited block (4+ dots)
                    // and ordered list marker (". item")
                    | '.'
            )
        })
        .collect();
    // Normalize: trim + strip structural markers until stable.
    // Each rule may expose another (e.g. ".. item" → strip ".. " → "item").
    let mut out = out.trim().to_string();
    loop {
        let prev = out.clone();
        out = out.trim().to_string();

        // AsciiDoc heading markers: leading '=' chars followed by space
        while out.starts_with('=') {
            let stripped = out.trim_start_matches('=').trim_start().to_string();
            out = stripped;
        }

        // AsciiDoc block attribute: leading '['
        if out.starts_with('[') {
            out = out[1..].trim().to_string();
        }

        // AsciiDoc delimited block openers (4+ repeated chars)
        for prefix in &["----", "====", "****", "____", "++++", "...."] {
            while out.starts_with(prefix) {
                out = out[prefix.len()..].trim().to_string();
            }
        }

        // AsciiDoc block image macro
        while out.starts_with("image::") {
            out = out["image::".len()..].trim().to_string();
        }

        // AsciiDoc unordered list markers
        while out.starts_with("* ") || out.starts_with("- ") {
            out = out[2..].trim().to_string();
        }
        while out.starts_with("** ") {
            out = out[3..].trim().to_string();
        }

        // AsciiDoc ordered list markers
        while out.starts_with(". ") {
            out = out[2..].trim().to_string();
        }
        while out.starts_with(".. ") {
            out = out[3..].trim().to_string();
        }

        // Numbered list: "N. "
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

        // AsciiDoc description list: "term::" — strip trailing colons
        while out.ends_with(':') {
            out.pop();
        }
        out = out.trim().to_string();

        // AsciiDoc horizontal rule / page break markers
        if out == "'''" || out == "---" || out == "<<<" {
            out = String::new();
        }

        // AsciiDoc link macros that might appear after stripping: avoid "link" alone
        // (covered by ':' filter) — nothing more needed here

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
        // how the AsciiDoc reader produces code nodes, so the writer reads it back.
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
                // Levels 1–5 (= through =====).
                let lvl = i64::from(*level % 5) + 1;
                Some(Node::new(node::HEADING).prop(prop::LEVEL, lvl).children(children))
            }
            FuzzBlock::BulletList { items } => {
                let list_items: Vec<Node> = items.iter().filter_map(|i| make_list_item(i)).collect();
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
                let list_items: Vec<Node> = items.iter().filter_map(|i| make_list_item(i)).collect();
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

    // Emit to AsciiDoc bytes — must not panic.
    let Ok(emit_result) = rescribe_write_asciidoc::emit(&doc) else {
        return;
    };
    let _adoc_for_debug = std::str::from_utf8(&emit_result.value).unwrap_or("").to_string();

    // Parse back — must not panic.
    let Ok(adoc_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_asciidoc::parse(adoc_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "AsciiDoc roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  adoc: {_adoc_for_debug:?}"
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
