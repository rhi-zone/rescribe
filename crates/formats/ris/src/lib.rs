//! RIS (Research Information Systems) citation format parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-ris` and `rescribe-write-ris` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

pub use ast::*;
pub use emit::emit;
pub use parse::parse;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_article() {
        let ris = r#"TY  - JOUR
AU  - Smith, John
AU  - Doe, Jane
TI  - A Great Paper
JO  - Nature
PY  - 2020
VL  - 123
SP  - 45
EP  - 67
DO  - 10.1234/nature.2020
 ER  -"#;

        let (doc, _diags) = parse(ris);
        assert_eq!(doc.entries.len(), 1);
        let entry = &doc.entries[0];
        assert_eq!(entry.entry_type, "JOUR");
        assert_eq!(entry.get_first("TI"), Some("A Great Paper"));
        assert_eq!(entry.get_all("AU").len(), 2);
    }

    #[test]
    fn test_parse_book() {
        let ris = r#"TY  - BOOK
AU  - Knuth, Donald E.
TI  - The Art of Computer Programming
PY  - 1997
 ER  -"#;

        let (doc, _diags) = parse(ris);
        assert_eq!(doc.entries.len(), 1);
        let entry = &doc.entries[0];
        assert_eq!(entry.entry_type, "BOOK");
    }

    #[test]
    fn test_parse_multiple() {
        let ris = r#"TY  - JOUR
AU  - First, Author
TI  - First Paper
ER  -
TY  - JOUR
AU  - Second, Author
TI  - Second Paper
ER  -"#;

        let (doc, _diags) = parse(ris);
        assert_eq!(doc.entries.len(), 2);
    }

    #[test]
    fn test_parse_empty() {
        let ris = "";
        let (doc, _diags) = parse(ris);
        assert!(doc.entries.is_empty());
    }

    #[test]
    fn test_generate_cite_key() {
        let mut entry = RisEntry::new("JOUR");
        entry.add_field("AU", "Smith, John");
        entry.add_field("PY", "2020");

        let key = entry.generate_cite_key();
        assert_eq!(key, "smith2020");
    }

    #[test]
    fn test_emit_simple() {
        let mut entry = RisEntry::new("JOUR");
        entry.add_field("AU", "Smith, John");
        entry.add_field("TI", "A Paper");

        let doc = RisDoc {
            entries: vec![entry],
            span: Span::NONE,
        };

        let output = emit(&doc);
        assert!(output.contains("TY  - JOUR"));
        assert!(output.contains("AU  - Smith, John"));
        assert!(output.contains("TI  - A Paper"));
        assert!(output.contains("ER  -"));
    }

    #[test]
    fn test_ris_type_to_bibtex() {
        assert_eq!(ris_type_to_bibtex("JOUR"), "article");
        assert_eq!(ris_type_to_bibtex("BOOK"), "book");
        assert_eq!(ris_type_to_bibtex("THES"), "phdthesis");
    }

    #[test]
    fn test_bibtex_type_to_ris() {
        assert_eq!(bibtex_type_to_ris("article"), "JOUR");
        assert_eq!(bibtex_type_to_ris("book"), "BOOK");
        assert_eq!(bibtex_type_to_ris("phdthesis"), "THES");
    }

    #[test]
    fn test_strip_spans() {
        let mut entry = RisEntry::new("JOUR");
        entry.add_field("TI", "Test");
        let mut doc = RisDoc { entries: vec![entry], span: Span::NONE };
        doc.strip_spans();
        assert_eq!(doc.span, Span::NONE);
        assert_eq!(doc.entries[0].span, Span::NONE);
    }
}
