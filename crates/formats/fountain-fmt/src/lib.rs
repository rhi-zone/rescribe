//! Fountain screenplay format parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-fountain` and `rescribe-write-fountain` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

// Re-export the most-used types for convenience.
pub use ast::{Block, Diagnostic, FountainDoc, Severity, Span};

/// Parse a Fountain string into a [`FountainDoc`].
///
/// Parsing is infallible — all input is accepted.  Diagnostics are returned
/// alongside the document for any construct that could not be interpreted.
pub fn parse(input: &str) -> (FountainDoc, Vec<Diagnostic>) {
    parse::parse(input)
}

/// Build a Fountain string from a [`FountainDoc`].
pub fn build(doc: &FountainDoc) -> String {
    emit::emit(doc)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_title_page() {
        let input = "Title: My Screenplay\nAuthor: John Doe\n\nINT. HOUSE - DAY";
        let (doc, _diags) = parse(input);
        assert_eq!(
            doc.metadata.get("title").map(|s| s.as_str()),
            Some("My Screenplay")
        );
        assert_eq!(
            doc.metadata.get("author").map(|s| s.as_str()),
            Some("John Doe")
        );
    }

    #[test]
    fn test_parse_scene_heading() {
        let input = "INT. COFFEE SHOP - DAY";
        let (doc, _diags) = parse(input);
        assert_eq!(doc.blocks.len(), 1);
        assert!(matches!(doc.blocks[0], Block::SceneHeading { .. }));
    }

    #[test]
    fn test_parse_dialogue() {
        let input = "JOHN\nHello, how are you?";
        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::Character { .. }));
        assert_eq!(doc.blocks.len(), 2);
    }

    #[test]
    fn test_parse_action() {
        let input = "The door slowly opens. A figure emerges from the shadows.";
        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::Action { .. }));
    }

    #[test]
    fn test_parse_transition() {
        let input = "CUT TO:";
        let (doc, _diags) = parse(input);
        assert!(!doc.blocks.is_empty());
        assert!(matches!(doc.blocks[0], Block::Transition { .. }));
    }

    #[test]
    fn test_build_simple() {
        let mut doc = FountainDoc::default();
        doc.blocks.push(Block::SceneHeading {
            text: "INT. OFFICE - DAY".to_string(),
            span: Span::NONE,
        });
        doc.blocks.push(Block::Action {
            text: "John enters.".to_string(),
            span: Span::NONE,
        });
        let output = build(&doc);
        assert!(output.contains("INT. OFFICE - DAY"));
        assert!(output.contains("John enters."));
    }

    #[test]
    fn test_build_with_metadata() {
        use std::collections::BTreeMap;
        let mut metadata = BTreeMap::new();
        metadata.insert("title".to_string(), "My Script".to_string());
        metadata.insert("author".to_string(), "Jane Doe".to_string());

        let doc = FountainDoc {
            metadata,
            blocks: vec![Block::Action {
                text: "Fade in.".to_string(),
                span: Span::NONE,
            }],
            span: Span::NONE,
        };

        let output = build(&doc);
        assert!(output.contains("Title: My Script"));
        assert!(output.contains("Author: Jane Doe"));
    }

    #[test]
    fn test_parse_section() {
        let input = "# ACT ONE\n\nINT. HOUSE - DAY";
        let (doc, _diags) = parse(input);
        assert!(
            doc.blocks
                .iter()
                .any(|b| matches!(b, Block::Section { .. }))
        );
    }

    #[test]
    fn test_parse_note() {
        let input = "This is action [[with a note]]";
        let (doc, _diags) = parse(input);
        assert!(doc.blocks.iter().any(|b| matches!(b, Block::Note { .. })));
    }

    #[test]
    fn test_parse_centered() {
        let input = ">CENTERED TEXT<";
        let (doc, _diags) = parse(input);
        assert!(
            doc.blocks
                .iter()
                .any(|b| matches!(b, Block::Centered { .. }))
        );
    }

    #[test]
    fn test_parse_lyric() {
        let input = "~And the music plays on...";
        let (doc, _diags) = parse(input);
        assert!(doc.blocks.iter().any(|b| matches!(b, Block::Lyric { .. })));
    }

    #[test]
    fn test_parse_page_break() {
        let input = "Action\n\n===\n\nMore action";
        let (doc, _diags) = parse(input);
        assert!(doc.blocks.iter().any(|b| matches!(b, Block::PageBreak { .. })));
    }

    #[test]
    fn test_build_transition() {
        let mut doc = FountainDoc::default();
        doc.blocks.push(Block::Transition {
            text: "CUT TO:".to_string(),
            span: Span::NONE,
        });
        let output = build(&doc);
        assert!(output.contains("CUT TO:"));
    }

    #[test]
    fn test_build_character_dual() {
        let mut doc = FountainDoc::default();
        doc.blocks.push(Block::Character {
            name: "JOHN".to_string(),
            dual: true,
            span: Span::NONE,
        });
        let output = build(&doc);
        assert!(output.contains("JOHN ^"));
    }
}
