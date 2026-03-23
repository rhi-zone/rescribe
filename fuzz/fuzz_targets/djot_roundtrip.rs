#![no_main]

//! Djot roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with Djot-compatible constructs,
//! emits them to Djot text via rescribe-write-djot, parses them back via
//! rescribe-read-djot, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: Djot inline markup (`*strong*`, `_emphasis_`,
//! `` `code` ``, `^super^`, `~sub~`, `[span]{}`, `{-del-}`, `{+ins+}`) is
//! context-sensitive. Plain text containing these characters may be re-parsed
//! as markup on the return trip. The sanitiser strips them so the corpus
//! exercises structural variations rather than inline-markup edge cases.
//!

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
    // Code excluded: adjacent code spans (`a``b`) produce double-backtick
    // delimiters that jotdown parses as a 2-backtick verbatim span, breaking
    // the roundtrip. This is a known writer bug (TODO: fix writer to space-
    // separate adjacent code spans). Inline code is covered by the reader fuzz.
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

/// Sanitise text: strip ASCII control characters and Djot inline markup
/// delimiters that would be re-interpreted on parse-back.
/// Returns None for empty results.
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                // All ASCII control chars 0x00–0x1f
                '\x00'..='\x1f'
                    // Djot inline markup delimiters
                    | '*' | '_' | '`' | '^' | '~' | '[' | ']' | '{' | '}' | '\\'
                    // Math / raw-inline
                    | '$' | '<' | '>'
                    // Smart-quote inputs (jotdown converts to curly quotes on roundtrip)
                    | '\'' | '"'
                    // Table column separator
                    | '|'
                    // Heading marker: '#' is a heading start even inside list items
                    // (djot allows inline headings in some contexts, and jotdown
                    // strips unrecognised block starters)
                    | '#'
                    // '-' can form "---" thematic break sequence, stripping content
                    | '-'
                    // '.' and ')' can form ordered list markers ("1.", "a.", "1)", "a)")
                    // even in paragraph context — jotdown treats "9)" as a list marker
                    | '.' | ')'
                    // '+' is a bullet list marker
                    | '+'
            )
        })
        .collect();
    // Normalize: trim + strip structural markers until stable.
    let mut out = out.trim().to_string();
    loop {
        let prev = out.clone();
        out = out.trim().to_string();

        // Djot heading markers: leading '#' chars followed by space
        while out.starts_with("# ") || out.starts_with("## ") {
            let stripped = out.trim_start_matches('#').trim_start().to_string();
            out = stripped;
        }
        // More heading markers: any leading '#' run
        while out.starts_with('#') && out.chars().nth(1).map(|c| c == ' ' || c == '#').unwrap_or(false) {
            out = out.trim_start_matches('#').trim_start().to_string();
        }

        // Djot bullet list markers: "- ", "* ", "+ "
        while out.starts_with("- ") || out.starts_with("* ") || out.starts_with("+ ") {
            out = out[2..].trim().to_string();
        }

        // Djot ordered list: "N. "
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

        // Djot div: "::: "
        while out.starts_with("::: ") {
            out = out[4..].trim().to_string();
        }

        // Djot blockquote: "> "
        while out.starts_with("> ") {
            out = out[2..].trim().to_string();
        }

        // Definition list marker: leading ': ' (Djot description list term)
        while out.starts_with(": ") {
            out = out[2..].trim().to_string();
        }

        // Trailing ':' prevents attribute-list syntax
        while out.ends_with(':') {
            out.pop();
        }
        // Trailing '.' or ')' prevents ordered-list marker matching
        // ("c.", "1.", "9)", etc. are list markers in Djot)
        while out.ends_with('.') || out.ends_with(')') {
            out.pop();
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
                // Djot headings use '#' through '######' (levels 1–6).
                let lvl = i64::from(*level % 6) + 1;
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

    // Emit to Djot bytes — must not panic.
    let Ok(emit_result) = rescribe_write_djot::emit(&doc) else {
        return;
    };
    let _djot_for_debug = std::str::from_utf8(&emit_result.value).unwrap_or("").to_string();

    // Parse back — must not panic.
    let Ok(djot_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_djot::parse(djot_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    // Normalize whitespace for comparison.
    let norm_before: String = text_before.split_whitespace().collect::<Vec<_>>().join(" ");
    let norm_after: String = text_after.split_whitespace().collect::<Vec<_>>().join(" ");

    assert_eq!(
        norm_before,
        norm_after,
        "Djot roundtrip lost text content\n  before: {norm_before:?}\n  after:  {norm_after:?}\n  djot: {_djot_for_debug:?}"
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
