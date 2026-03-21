//! TSV (Tab-Separated Values) parser, AST, and emitter.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-tsv` and `rescribe-write-tsv` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

pub use ast::{Cell, Diagnostic, Row, Severity, Span, TsvDoc};
pub use emit::emit;
pub use parse::parse;

#[cfg(test)]
mod tests {
    use super::*;

    fn rows_as_strings(doc: &TsvDoc) -> Vec<Vec<String>> {
        doc.rows.iter().map(|r| r.cells.iter().map(|c| c.value.clone()).collect()).collect()
    }

    #[test]
    fn test_parse_simple_tsv() {
        let (doc, _) = parse("Name\tAge\tCity\nAlice\t30\tNew York\nBob\t25\tLondon");
        let rows = rows_as_strings(&doc);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["Name", "Age", "City"]);
        assert_eq!(rows[1], vec!["Alice", "30", "New York"]);
        assert_eq!(rows[2], vec!["Bob", "25", "London"]);
    }

    #[test]
    fn test_parse_quoted_fields() {
        let (doc, _) = parse("Name\tDescription\n\"Item\"\t\"Has\ttabs\"");
        let rows = rows_as_strings(&doc);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1][1], "Has\ttabs");
    }

    #[test]
    fn test_build_simple_tsv() {
        let doc = TsvDoc {
            rows: vec![
                Row {
                    cells: vec![
                        Cell { value: "Name".to_string(), span: Span::NONE },
                        Cell { value: "Age".to_string(), span: Span::NONE },
                    ],
                    span: Span::NONE,
                },
                Row {
                    cells: vec![
                        Cell { value: "Alice".to_string(), span: Span::NONE },
                        Cell { value: "30".to_string(), span: Span::NONE },
                    ],
                    span: Span::NONE,
                },
            ],
            span: Span::NONE,
        };
        let output = emit(&doc);
        assert!(output.contains("Name\tAge"));
        assert!(output.contains("Alice\t30"));
    }

    #[test]
    fn test_roundtrip() {
        let input = "A\tB\nC\t\"D\tE\"\n";
        let (doc, _) = parse(input);
        let output = emit(&doc);
        let (doc2, _) = parse(&output);
        assert_eq!(rows_as_strings(&doc), rows_as_strings(&doc2));
    }

    #[test]
    fn test_strip_spans() {
        let (doc, _) = parse("a\tb\n1\t2");
        let stripped = doc.strip_spans();
        assert_eq!(stripped.span, Span::NONE);
        for row in &stripped.rows {
            assert_eq!(row.span, Span::NONE);
            for cell in &row.cells {
                assert_eq!(cell.span, Span::NONE);
            }
        }
    }
}
