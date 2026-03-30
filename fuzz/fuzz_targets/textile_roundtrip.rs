#![no_main]

//! Textile roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with Textile-compatible constructs,
//! emits them to Textile text via rescribe-write-textile, parses them back via
//! rescribe-read-textile, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: Textile inline markup (`*bold*`, `_italic_`,
//! `@code@`, `^super^`, `~sub~`, `+underline+`, `-strikethrough-`) is
//! context-sensitive. Plain text containing these characters may be re-parsed
//! as markup on the return trip. The sanitiser strips them so the corpus
//! exercises structural variations rather than inline-markup edge cases.
//!
//! FuzzInlineKind: Plain, Bold, Italic, Code only (safe subset).
//! Strikethrough/Underline/Superscript/Subscript excluded: `-`, `+`, `^`, `~`
//! in plain text trigger spurious re-parsing.

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
    // Strikethrough: `-text-` — `-` in plain text triggers false matches
    // Underline: `+text+` — `+` in plain text triggers false matches
    // Superscript: `^text^` — `^` stripped, false matches
    // Subscript: `~text~` — `~` stripped, false matches
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

/// Sanitise text: strip ASCII control characters and Textile inline markup
/// delimiters that would be re-interpreted on parse-back.
/// Returns None for empty results.
fn sanitise(s: &str) -> Option<String> {
    // Strip characters that Textile uses as inline markup or structural markers.
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                // All ASCII control chars 0x00–0x1f
                '\x00'..='\x1f'
                    // Textile inline markup delimiters
                    | '*' | '_' | '@' | '+' | '-' | '^' | '~'
                    // Link/image syntax
                    | '"' | '!' | '['| ']' | '{' | '}' | '%' | '?'
                    // Table cell separator
                    | '|'
                    // Hash can start ordered list items
                    | '#'
                    // Single quote (can appear in textile span syntax)
                    | '\''
                    // '.' filtered: `pre.`, `bc.`, `bq.`, `h1.`..`h6.` are all parsed
                    // as block markers without requiring a trailing space, so any text
                    // containing '.' could produce ambiguous leading sequences.
                    | '.'
                    // ':' and ';' at line start trigger definition list in Textile
                    | ':' | ';'
                    // '==' triggers raw HTML passthrough; '=' also used in table alignment
                    | '='
                    // '$' can interact with Textile span markers in unexpected ways
                    | '$'
                    // '(' ')' used in Textile CSS class/id notation and span syntax
                    | '(' | ')'
                    // DEL (0x7f) control character
                    | '\x7f'
            )
        })
        .collect();

    // Normalize: trim + strip structural markers until stable.
    let mut out = out.trim().to_string();
    loop {
        let prev = out.clone();
        out = out.trim().to_string();

        // Textile heading markers: h1. through h6. at start of line
        for level in 1..=6u8 {
            let prefix = format!("h{}.", level);
            while out.starts_with(&prefix) {
                let stripped = out[prefix.len()..].trim().to_string();
                out = stripped;
            }
        }

        // Textile block markers — strip both with-space and bare dot forms.
        // The parser checks starts_with("pre.") not starts_with("pre. "), so
        // we must strip the bare prefix too.
        for marker in &[
            "bq..", "bq. ", "bq.",
            "bc..", "bc. ", "bc.",
            "pre..", "pre. ", "pre.",
            "fn.", "notextile. ", "p. ", "p.",
        ] {
            while out.starts_with(marker) {
                out = out[marker.len()..].trim().to_string();
            }
        }

        // Textile unordered list marker (already stripped '#' above, but keep loop stable)
        while out.starts_with("* ") || out.starts_with("** ") {
            let skip = if out.starts_with("** ") { 3 } else { 2 };
            out = out[skip..].trim().to_string();
        }

        // Trailing '.' followed by space might start a block modifier
        while out.ends_with(". ") {
            out = out[..out.len() - 2].trim().to_string();
        }
        // Trailing lone '.'
        while out.ends_with('.') && out.len() > 1 {
            let without = out[..out.len() - 1].trim().to_string();
            // Only strip if it could be a block modifier (short prefix)
            if without.len() <= 4 {
                out = without;
            } else {
                break;
            }
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

fn make_para_children(inlines: &[FuzzInline]) -> Vec<Node> {
    // Textile inline markup has word-boundary requirements:
    //   Opening marker: must NOT be preceded by alphanumeric (`G*bold*` fails)
    //   Closing marker: must NOT be followed by alphanumeric (`_word_next` fails)
    //
    // Strategy: process inlines one-by-one. Track whether the last emitted node
    // was formatted. When transitioning plain↔formatted, skip the plain inline if
    // its boundary character (last char before formatted, or first char after formatted)
    // is alphanumeric. This avoids bad adjacency in the emitted output.
    let is_formatted = |kind: &FuzzInlineKind| {
        matches!(kind, FuzzInlineKind::Bold | FuzzInlineKind::Italic | FuzzInlineKind::Code)
    };

    let mut result: Vec<Node> = Vec::new();
    // Track the last character of the last emitted inline's text content.
    // Used to check "opening marker preceded by alphanumeric" (Rule 1).
    let mut last_char: Option<char> = None;
    // Track whether the last emitted inline was formatted (for closing-marker rule).
    let mut last_was_formatted = false;

    for fi in inlines {
        let Some(clean) = sanitise(&fi.text) else {
            continue;
        };

        let this_is_formatted = is_formatted(&fi.kind);

        if this_is_formatted {
            // Rule 1: opening delimiter must not be preceded by alphanumeric.
            // If the previous emitted inline ended with alphanumeric, skip this one.
            if last_char.map(|c| c.is_alphanumeric()).unwrap_or(false) {
                continue;
            }
        } else {
            // Plain text.
            // Rule 2: closing delimiter must not be followed by alphanumeric.
            // If the previous inline was formatted and this text starts with alphanumeric, skip.
            if last_was_formatted
                && clean.chars().next().map(|c| c.is_alphanumeric()).unwrap_or(false)
            {
                continue;
            }
        }

        // Track last char of this inline's text content for the next iteration.
        last_char = if this_is_formatted {
            // After a formatted inline, the emitted closer is the delimiter char
            // (e.g. `*` or `_`). The parser sees the char AFTER the close delimiter.
            // For Rule 1 of the NEXT inline, we care about what char precedes the NEXT
            // opening delimiter. After a formatted close, the last emitted char is the
            // close delimiter itself, which is NOT alphanumeric.
            None // closing delimiter is not alphanumeric — safe for next opening
        } else {
            clean.chars().last()
        };
        last_was_formatted = this_is_formatted;

        let leaf = Node::new(node::TEXT).prop(prop::CONTENT, clean.clone());
        let node = match fi.kind {
            FuzzInlineKind::Plain => leaf,
            FuzzInlineKind::Bold => Node::new(node::STRONG).child(leaf),
            FuzzInlineKind::Italic => Node::new(node::EMPHASIS).child(leaf),
            FuzzInlineKind::Code => Node::new(node::CODE).prop(prop::CONTENT, clean),
        };
        result.push(node);
    }
    result
}

fn make_list_item(inlines: &[FuzzInline]) -> Option<Node> {
    // For list items, use only plain children (no space-prefix complications).
    // The Textile list parser trims content, so leading spaces introduced by the
    // space-prefix fix would be lost. Using plain-only inlines avoids the issue.
    let children: Vec<Node> = inlines
        .iter()
        .filter_map(|fi| {
            let clean = sanitise(&fi.text)?;
            // Reject content starting with space — would be trimmed by list parser
            if clean.starts_with(' ') {
                return None;
            }
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
                // Levels 1–6 (h1. through h6.)
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

    // Emit to Textile bytes — must not panic.
    let Ok(emit_result) = rescribe_write_textile::emit(&doc) else {
        return;
    };
    let _textile_for_debug = std::str::from_utf8(&emit_result.value).unwrap_or("").to_string();

    // Parse back — must not panic.
    let Ok(textile_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_textile::parse(textile_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "Textile roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  textile: {_textile_for_debug:?}"
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
