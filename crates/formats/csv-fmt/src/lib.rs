//! CSV (Comma-Separated Values) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-csv` and `rescribe-write-csv` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct CsvError(pub String);

impl std::fmt::Display for CsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CSV error: {}", self.0)
    }
}

impl std::error::Error for CsvError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed CSV document.
#[derive(Debug, Clone, Default)]
pub struct CsvDoc {
    /// Rows of the CSV, where each row is a vector of cell strings.
    pub rows: Vec<Vec<String>>,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a CSV string into a [`CsvDoc`].
pub fn parse(input: &str) -> Result<CsvDoc, CsvError> {
    let mut rows = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let cells = parse_csv_line(line);
        rows.push(cells);
    }

    Ok(CsvDoc { rows })
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                // Check for escaped quote
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' if !in_quotes => {
                in_quotes = true;
            }
            ',' if !in_quotes => {
                cells.push(current.trim().to_string());
                current.clear();
            }
            _ => {
                current.push(c);
            }
        }
    }

    cells.push(current.trim().to_string());
    cells
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Convert a [`CsvDoc`] into a CSV string.
pub fn build(doc: &CsvDoc) -> String {
    let mut output = String::new();

    for row in &doc.rows {
        let cells: Vec<String> = row.iter().map(|cell| escape_csv_field(cell)).collect();
        output.push_str(&cells.join(","));
        output.push('\n');
    }

    output
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        field.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let result = parse("a,b,c\n1,2,3").unwrap();
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[0], vec!["a", "b", "c"]);
        assert_eq!(result.rows[1], vec!["1", "2", "3"]);
    }

    #[test]
    fn test_parse_quoted() {
        let result = parse("name,value\n\"hello, world\",42").unwrap();
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[1][0], "hello, world");
        assert_eq!(result.rows[1][1], "42");
    }

    #[test]
    fn test_parse_escaped_quotes() {
        let result = parse("a,b\n\"say \"\"hello\"\"\",test").unwrap();
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[1][0], "say \"hello\"");
    }

    #[test]
    fn test_csv_line_parsing() {
        let cells = parse_csv_line("a,b,c");
        assert_eq!(cells, vec!["a", "b", "c"]);

        let cells = parse_csv_line("\"hello, world\",test");
        assert_eq!(cells, vec!["hello, world", "test"]);
    }

    #[test]
    fn test_build_simple() {
        let doc = CsvDoc {
            rows: vec![
                vec!["a".to_string(), "b".to_string(), "c".to_string()],
                vec!["1".to_string(), "2".to_string(), "3".to_string()],
            ],
        };
        let output = build(&doc);
        assert!(output.contains("a,b,c"));
        assert!(output.contains("1,2,3"));
    }

    #[test]
    fn test_escape_csv() {
        assert_eq!(escape_csv_field("hello"), "hello");
        assert_eq!(escape_csv_field("hello, world"), "\"hello, world\"");
        assert_eq!(escape_csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn test_roundtrip() {
        let input = "a,b,c\n\"hello, world\",test,\"say \"\"hi\"\"\"";
        let doc = parse(input).unwrap();
        let output = build(&doc);
        let doc2 = parse(&output).unwrap();
        assert_eq!(doc.rows, doc2.rows);
    }
}
