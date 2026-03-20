#![no_main]

//! Fountain roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with Fountain-compatible constructs,
//! emits them to Fountain text via rescribe-write-fountain, parses them back
//! via rescribe-read-fountain, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: Fountain has many structural markers that
//! are context-sensitive:
//! - Scene headings: lines starting with INT. / EXT. / EST. / I/E
//! - Transitions: all-caps lines ending in "TO:"
//! - Characters: all-caps lines
//! - Section/Synopsis markers: # and =
//! - Centered: >text<
//! - Lyric: ~
//! - Forced action: !
//! - Notes: [[ ]]
//! - Forced scene heading: .
//! - Forced character: @
//! - Dual dialogue: ^
//! - Page break: ===
//!
//! FuzzInlineKind: Plain only — Fountain has no block inline markup in action
//! lines that would survive a roundtrip cleanly.
//!
//! FuzzBlock: Action paragraphs only — these roundtrip safely as long as text
//! doesn't trigger structural re-interpretation.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use rescribe_core::{Document, Node};
use rescribe_std::{node, prop};

// ── Fuzz-friendly inline types ────────────────────────────────────────────────

#[derive(Arbitrary, Debug)]
struct FuzzInline {
    text: String,
}

// ── Fuzz-friendly block types ─────────────────────────────────────────────────

#[derive(Arbitrary, Debug)]
enum FuzzBlock {
    Action { inlines: Vec<FuzzInline> },
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Sanitise text: strip characters that would be re-interpreted as structural
/// Fountain markup on the parse-back trip.
///
/// Stripped characters / patterns:
/// - ASCII control characters (0x00–0x1f) including newlines
/// - `*` `_` — emphasis markers
/// - `@` — forced character prefix
/// - `!` — forced action prefix
/// - `#` — section prefix
/// - `=` — synopsis prefix / page-break (===)
/// - `>` `<` — centered text markers / transition prefix
/// - `~` — lyric prefix
/// - `^` — dual dialogue marker
/// - `[` `]` — note delimiters [[...]]
/// - `(` `)` — parenthetical markers
/// - `.` — forced scene heading prefix / abbreviation in INT. EXT.
/// - `\` — escape
///
/// Leading structural patterns stripped in a stable loop:
/// - Lines starting with INT./EXT./EST./I/E → scene heading
/// - ALL-CAPS → character cue
/// - Lines ending with TO: (all caps) → transition
/// - "CUT TO:" / "FADE" prefix → transition
fn sanitise(s: &str) -> Option<String> {
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                // All ASCII control chars 0x00–0x1f (includes \n \r \t)
                '\x00'..='\x1f'
                    // Fountain structural / inline markers
                    | '*' | '_' | '@' | '!' | '#' | '=' | '>' | '<' | '~'
                    | '^' | '[' | ']' | '(' | ')' | '.' | '\\'
            )
        })
        .collect();

    // Trim and strip structural patterns until stable.
    let mut out = out.trim().to_string();
    loop {
        let prev = out.clone();
        out = out.trim().to_string();

        // Strip lines that look like scene headings (INT / EXT / EST / I/E prefix).
        // We already stripped '.' so "INT." becomes "INT" — still strip it.
        for prefix in &["INT ", "EXT ", "EST ", "I/E", "INT", "EXT", "EST"] {
            let upper = out.to_uppercase();
            if upper.starts_with(prefix) {
                out = out[prefix.len()..].trim_start().to_string();
            }
        }

        // Strip transitions: line that ends with "TO:" in uppercase.
        // We already stripped '.' above, but "TO:" → "TO" in output — still safe.
        // Check original-case ending with uppercase only.
        if out == out.to_uppercase() && !out.is_empty() && out.to_uppercase().ends_with("TO") {
            // Could look like a transition — remove trailing "TO"
            let stripped = out[..out.len() - 2].trim_end().to_string();
            if !stripped.is_empty() {
                out = stripped;
            } else {
                out = String::new();
            }
        }

        // Strip all-caps text that would be parsed as a character cue.
        // A character cue is all-caps with at least one letter.
        // If the entire string is uppercase alphabetic+spaces, it would be
        // parsed as a character — lowercase it.
        if !out.is_empty()
            && out
                .chars()
                .all(|c| c.is_uppercase() || c.is_whitespace() || c.is_ascii_digit())
            && out.chars().any(|c| c.is_alphabetic())
        {
            out = out.to_lowercase();
        }

        // Strip leading "CUT TO" or "FADE" that remain after other stripping.
        for prefix in &["cut to", "fade out", "fade in", "smash to"] {
            if out.to_lowercase().starts_with(prefix) {
                out = out[prefix.len()..].trim_start().to_string();
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
    inlines
        .iter()
        .filter_map(|fi| {
            let text = sanitise(&fi.text)?;
            Some(Node::new(node::TEXT).prop(prop::CONTENT, text))
        })
        .collect()
}

// ── Fuzz target ───────────────────────────────────────────────────────────────

fuzz_target!(|blocks: Vec<FuzzBlock>| {
    let content_nodes: Vec<Node> = blocks
        .iter()
        .filter_map(|b| match b {
            FuzzBlock::Action { inlines } => {
                let children = make_para_children(inlines);
                if children.is_empty() {
                    None
                } else {
                    Some(
                        Node::new(node::PARAGRAPH)
                            .prop("fountain:type", "action")
                            .children(children),
                    )
                }
            }
        })
        .collect();

    if content_nodes.is_empty() {
        return;
    }

    let doc = Document::new().with_content(Node::new(node::DOCUMENT).children(content_nodes));

    // Emit to Fountain bytes — must not panic.
    let Ok(emit_result) = rescribe_write_fountain::emit(&doc) else {
        return;
    };
    let _fountain_for_debug = std::str::from_utf8(&emit_result.value)
        .unwrap_or("")
        .to_string();

    // Parse back — must not panic.
    let Ok(fountain_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_fountain::parse(fountain_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "Fountain roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  fountain: {_fountain_for_debug:?}"
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
