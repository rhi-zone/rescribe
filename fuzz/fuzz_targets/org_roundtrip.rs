#![no_main]

//! Org-mode roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with Org-compatible constructs,
//! emits them to Org text via rescribe-write-org, parses them back via
//! rescribe-read-org, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: Org inline markup (`*bold*`, `/italic/`,
//! `=code=`, `_underline_`, `+strikethrough+`, `^{super}`, `_{sub}`) is
//! context-sensitive. Plain text containing these characters may be
//! re-parsed as markup on the return trip. The sanitiser strips them so
//! the corpus exercises structural variations rather than inline edge cases.
//!
//! Excluded FuzzInlineKind:
//! - Underline (`_text_`): `_` also appears in identifiers; causes false
//!   matches in plain text
//! - Strikethrough (`+text+`): `+` is also an unordered list marker
//! - Superscript: builder emits `^{text}` but parser has no `^{` match arm
//!   — known gap in org-fmt parser
//! - Subscript: same — builder emits `_{text}` but parser misses it
//! - MathInline: `$source$` — sanitiser strips `$` but math parsing is
//!   fragile with arbitrary content
//! - FootnoteRef: `[fn:label]` — `[` and `]` stripped by sanitiser
//!
//! Excluded FuzzBlock:
//! - Blockquote: content re-parsed as inline, structural roundtrip is lossy
//! - Table: fragile is_header reconstruction; col_widths indexing issues
//! - HorizontalRule: `-----` — sanitiser strips `-`
//! - DefinitionList: parser has no dedicated handler for `- term :: desc`
//! - Div/Figure/RawBlock/Unknown: not reliably roundtrippable

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
    // Excluded (see module doc):
    // Underline, Strikethrough, Superscript, Subscript, MathInline, FootnoteRef
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
    // Excluded (see module doc):
    // Blockquote, Table, HorizontalRule, DefinitionList, Div, Figure,
    // RawBlock, Unknown
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Sanitise text: strip ASCII control characters and Org inline markup
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
                    // Org inline markup delimiters
                    | '*' | '/' | '_' | '+' | '~' | '=' | '[' | ']'
                    | '^' | '{' | '}' | '$' | '<' | '>' | '#' | '|'
                    // Backslash (line-break marker in Org)
                    | '\\'
                    // '-' filtered to prevent "-----" horizontal rule trigger
                    // (Org: line of 5+ dashes = HorizontalRule)
                    | '-'
                    // '@' filtered to prevent '@@backend:content@@' export snippet:
                    // export snippet nodes are not TEXT nodes, so their content
                    // is lost in extract_text.
                    | '@'
                    // ':' filtered to prevent '::' description-list separator and
                    // ':key:' property drawer patterns from splitting/hiding content.
                    | ':'
            )
        })
        .collect();

    // Normalize: trim + strip structural markers until stable.
    let mut out = out.trim().to_string();
    loop {
        let prev = out.clone();
        out = out.trim().to_string();

        // Org heading markers: leading '*' chars followed by space
        while out.starts_with("* ") {
            out = out[2..].trim().to_string();
        }
        // Strip leading all-star prefix (e.g. "** " or "***" alone)
        while out.starts_with('*') {
            let stripped = out.trim_start_matches('*').trim_start().to_string();
            if stripped == out.trim_start_matches('*').to_string() {
                // No more leading stars with content after
                break;
            }
            out = stripped;
        }

        // Org unordered list markers: "- " or "+ "
        while out.starts_with("- ") || out.starts_with("+ ") {
            out = out[2..].trim().to_string();
        }

        // Org ordered list markers: "N. " or "N) "
        loop {
            let trimmed = out
                .trim_start_matches(|c: char| c.is_ascii_digit())
                .to_string();
            if trimmed.starts_with(". ") || trimmed.starts_with(") ") {
                let cut = out.len() - trimmed.len() + 2;
                out = out[cut..].trim().to_string();
            } else {
                break;
            }
        }

        // Org metadata directives: "#+ "
        while out.starts_with("#+ ") || out.starts_with("#+") {
            out = out.trim_start_matches('#').trim_start().to_string();
        }

        // Strip TODO/DONE keywords (heading strip_heading_metadata removes them)
        if out.starts_with("TODO ") {
            out = out["TODO ".len()..].trim().to_string();
        }
        if out.starts_with("DONE ") {
            out = out["DONE ".len()..].trim().to_string();
        }

        // Strip Org fixed-width block prefix (': text' — line starting with ': '
        // is a fixed-width verbatim line, not a paragraph).
        while out.starts_with(": ") {
            out = out[2..].trim().to_string();
        }

        // Strip trailing ':' (prevents key: metadata patterns)
        while out.ends_with(':') {
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
        // CODE uses prop::CONTENT directly (not a child TEXT node)
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
                // Clamp level to 1..=8 (Org supports up to 8 heading levels)
                let lvl = i64::from(*level % 8) + 1;
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

    // Emit to Org bytes — must not panic.
    let Ok(emit_result) = rescribe_write_org::emit(&doc) else {
        return;
    };
    let _org_for_debug = std::str::from_utf8(&emit_result.value).unwrap_or("").to_string();

    // Parse back — must not panic.
    let Ok(org_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_org::parse(org_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "Org roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  org: {_org_for_debug:?}"
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
