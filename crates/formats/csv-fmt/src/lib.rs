//! CSV (Comma-Separated Values) parser, AST, and emitter.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-csv` and `rescribe-write-csv` as thin adapter layers.

pub mod ast;
pub mod emit;
pub mod parse;

pub use ast::{Cell, CsvDoc, Diagnostic, Row, Severity, Span};
pub use emit::emit;
pub use parse::parse;

#[cfg(test)]
mod tests {
    use super::*;

    fn rows_as_strings(doc: &CsvDoc) -> Vec<Vec<String>> {
        doc.rows.iter().map(|r| r.cells.iter().map(|c| c.value.clone()).collect()).collect()
    }

    #[test]
    fn test_parse_simple() {
        let (doc, _) = parse("a,b,c\n1,2,3");
        let rows = rows_as_strings(&doc);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["a", "b", "c"]);
        assert_eq!(rows[1], vec!["1", "2", "3"]);
    }

    #[test]
    fn test_parse_quoted() {
        let (doc, _) = parse("name,value\n\"hello, world\",42");
        let rows = rows_as_strings(&doc);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1][0], "hello, world");
        assert_eq!(rows[1][1], "42");
    }

    #[test]
    fn test_parse_escaped_quotes() {
        let (doc, _) = parse("a,b\n\"say \"\"hello\"\"\",test");
        let rows = rows_as_strings(&doc);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1][0], "say \"hello\"");
    }

    #[test]
    fn test_emit_simple() {
        let doc = CsvDoc {
            rows: vec![
                Row {
                    cells: vec![
                        Cell { value: "a".to_string(), span: Span::NONE },
                        Cell { value: "b".to_string(), span: Span::NONE },
                        Cell { value: "c".to_string(), span: Span::NONE },
                    ],
                    span: Span::NONE,
                },
                Row {
                    cells: vec![
                        Cell { value: "1".to_string(), span: Span::NONE },
                        Cell { value: "2".to_string(), span: Span::NONE },
                        Cell { value: "3".to_string(), span: Span::NONE },
                    ],
                    span: Span::NONE,
                },
            ],
            span: Span::NONE,
        };
        let output = emit(&doc);
        assert!(output.contains("a,b,c"));
        assert!(output.contains("1,2,3"));
    }

    #[test]
    fn test_roundtrip() {
        let input = "a,b,c\n\"hello, world\",test,\"say \"\"hi\"\"\"";
        let (doc, _) = parse(input);
        let output = emit(&doc);
        let (doc2, _) = parse(&output);
        assert_eq!(rows_as_strings(&doc), rows_as_strings(&doc2));
    }

    #[test]
    fn test_strip_spans() {
        let (doc, _) = parse("a,b\n1,2");
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
