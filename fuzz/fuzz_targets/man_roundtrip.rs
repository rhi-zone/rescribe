#![no_main]

//! Man page roundtrip fuzz target.
//!
//! Generates arbitrary rescribe Documents with man-page-compatible constructs,
//! emits them to man page text via rescribe-write-man, parses them back via
//! rescribe-read-man, and asserts that text content is preserved.
//!
//! Direction: arbitrary_rescribe_doc → emit → parse → assert text preserved
//!
//! Why restricted character set: troff/roff uses many special characters as
//! structural markers.  Plain text containing `\`, `.`, `'` etc. would be
//! re-interpreted as macros or escapes on the return trip.  The sanitiser
//! strips them so the corpus exercises structural variations rather than
//! escape-sequence edge cases.
//!
//! FuzzInlineKind excludes Link: links are emitted as "text (url)" and the
//! URL parenthetical is not re-parsed as a structured link, breaking roundtrip.
//!
//! FuzzBlock excludes BulletList and OrderedList: man lists are emitted as
//! `.IP \(bu` / `.IP N.` which the parser re-reads as definition lists with
//! the bullet/number as the term text, contaminating the text comparison.
//! Lists are still covered by the man_reader no-panic target.

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
    // BulletList and OrderedList excluded: see module-level comment.
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Sanitise text: strip troff special characters that would be re-interpreted
/// as macros or escape sequences on the parse-back.
/// Returns None for empty results.
fn sanitise(s: &str) -> Option<String> {
    // First pass: strip problematic chars.
    let out: String = s
        .chars()
        .filter(|c| {
            !matches!(
                *c,
                // All ASCII control chars 0x00–0x1f (includes \n, \t)
                '\x00'..='\x1f'
                    // troff escape and macro chars
                    | '\\' | '.' | '\''
                    // troff special chars in text that cause issues
                    | '-' | '"' | '&' | '|'
                    | '[' | ']' | '{' | '}'
                    | '<' | '>' | '#' | '~'
                    | '^' | '*' | '_' | '='
                    | '+' | '`'
            )
        })
        .collect();

    // Normalize: trim + strip structural markers until stable.
    // A line beginning with '.' is a macro line; strip leading dots
    // that would survive the char filter (none should, but be safe).
    let mut out = out.trim().to_string();
    loop {
        let prev = out.clone();
        out = out.trim().to_string();

        // Strip trailing backslash (line continuation in troff) — already
        // filtered, but strip the textual word "\" if it somehow appears.
        while out.ends_with('\\') {
            out.pop();
            out = out.trim_end().to_string();
        }

        // Strip lines that look like troff macro names (shouldn't appear
        // after filtering '.', but strip common ones for safety).
        for prefix in &[
            "TH ", "SH ", "SS ", "PP", "TP", "IP ", "RS", "RE",
            "br", "nf", "fi", "B ", "I ", "BR ", "IR ", "RB ", "RI ",
            "BI ", "IB ", "URL ", "UR ", "UE", "sp",
        ] {
            if out.starts_with(prefix) {
                out = out[prefix.len()..].trim().to_string();
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
    let leaf = Node::new(node::TEXT).prop(prop::CONTENT, text.clone());
    Some(match fi.kind {
        FuzzInlineKind::Plain => leaf,
        FuzzInlineKind::Bold => Node::new(node::STRONG).child(leaf),
        FuzzInlineKind::Italic => Node::new(node::EMPHASIS).child(leaf),
        // CODE: emitted as bold (man has no inline code, writer uses \fB...\fR)
        // Use prop::CONTENT so the writer can read it back consistently.
        FuzzInlineKind::Code => Node::new(node::CODE).prop(prop::CONTENT, text),
    })
}

fn make_para_children(inlines: &[FuzzInline]) -> Vec<Node> {
    inlines.iter().filter_map(make_inline).collect()
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
                // Levels 2–3 (SH / SS); level 1 is used for .TH title.
                let lvl = i64::from(*level % 2) + 2; // 2 or 3
                Some(Node::new(node::HEADING).prop(prop::LEVEL, lvl).children(children))
            }
        })
        .collect();

    if content_nodes.is_empty() {
        return;
    }

    let doc = Document::new().with_content(Node::new(node::DOCUMENT).children(content_nodes));

    // Emit to man page bytes — must not panic.
    let Ok(emit_result) = rescribe_write_man::emit(&doc) else {
        return;
    };
    let _man_for_debug = std::str::from_utf8(&emit_result.value).unwrap_or("").to_string();

    // Parse back — must not panic.
    let Ok(man_str) = std::str::from_utf8(&emit_result.value) else {
        return;
    };
    let Ok(parse_result) = rescribe_read_man::parse(man_str) else {
        return;
    };

    // All text content must survive the roundtrip.
    let text_before = extract_text(&doc.content);
    let text_after = extract_text(&parse_result.value.content);

    assert_eq!(
        text_before,
        text_after,
        "man roundtrip lost text content\n  before: {text_before:?}\n  after:  {text_after:?}\n  man: {_man_for_debug:?}"
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
        // Skip headings: man page headings are emitted as uppercase (.SH/.SS)
        // and the .TH title block is always auto-generated (e.g. "UNTITLED"),
        // so heading text does not survive the roundtrip unmodified.
        if child.kind.as_str() == node::HEADING {
            continue;
        }
        text.push_str(&extract_text(child));
    }
    text
}
