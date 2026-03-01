//! TSV (Tab-Separated Values) parser, AST, and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-tsv` and `rescribe-write-tsv` as thin adapter layers.

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct TsvError(pub String);

impl std::fmt::Display for TsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TSV error: {}", self.0)
    }
}

impl std::error::Error for TsvError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed TSV document (simple tabular data).
#[derive(Debug, Clone, Default)]
pub struct TsvDoc {
    pub rows: Vec<Vec<String>>,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a TSV string into a [`TsvDoc`].
pub fn parse(input: &str) -> Result<TsvDoc, TsvError> {
    let mut rows = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let fields = parse_tsv_line(line);
        rows.push(fields);
    }

    Ok(TsvDoc { rows })
}

fn parse_tsv_line(line: &str) -> Vec<String> {
    // TSV is simpler than CSV - just split on tabs
    // Quoted fields can contain tabs and newlines
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_quotes => {
                in_quotes = true;
            }
            '"' if in_quotes => {
                // Check for escaped quote
                if chars.peek() == Some(&'"') {
                    current_field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '\t' if !in_quotes => {
                fields.push(current_field);
                current_field = String::new();
            }
            _ => {
                current_field.push(c);
            }
        }
    }

    fields.push(current_field);
    fields
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a TSV string from a [`TsvDoc`].
pub fn build(doc: &TsvDoc) -> String {
    let mut output = String::new();

    for row in &doc.rows {
        let cells: Vec<String> = row.iter().map(|cell| escape_tsv_field(cell)).collect();
        output.push_str(&cells.join("\t"));
        output.push('\n');
    }

    output
}

fn escape_tsv_field(field: &str) -> String {
    // TSV escaping: if field contains tab, newline, or quote, wrap in quotes
    if field.contains('\t') || field.contains('"') || field.contains('\n') {
        let escaped = field.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        field.to_string()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_tsv() {
        let input = "Name\tAge\tCity\nAlice\t30\tNew York\nBob\t25\tLondon";
        let doc = parse(input).unwrap();
        assert_eq!(doc.rows.len(), 3);
        assert_eq!(doc.rows[0], vec!["Name", "Age", "City"]);
        assert_eq!(doc.rows[1], vec!["Alice", "30", "New York"]);
        assert_eq!(doc.rows[2], vec!["Bob", "25", "London"]);
    }

    #[test]
    fn test_parse_quoted_fields() {
        let input = "Name\tDescription\n\"Item\"\t\"Has\ttabs\"";
        let doc = parse(input).unwrap();
        assert_eq!(doc.rows.len(), 2);
        assert_eq!(doc.rows[1][1], "Has\ttabs");
    }

    #[test]
    fn test_parse_tsv_line() {
        assert_eq!(parse_tsv_line("a\tb\tc"), vec!["a", "b", "c"]);
        assert_eq!(parse_tsv_line("\"a\tb\"\tc"), vec!["a\tb", "c"]);
    }

    #[test]
    fn test_build_simple_tsv() {
        let doc = TsvDoc {
            rows: vec![
                vec!["Name".to_string(), "Age".to_string()],
                vec!["Alice".to_string(), "30".to_string()],
            ],
        };
        let output = build(&doc);
        assert!(output.contains("Name\tAge"));
        assert!(output.contains("Alice\t30"));
    }

    #[test]
    fn test_escape_tsv() {
        assert_eq!(escape_tsv_field("hello"), "hello");
        assert_eq!(escape_tsv_field("hello\tworld"), "\"hello\tworld\"");
        assert_eq!(escape_tsv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn test_roundtrip() {
        let input = "A\tB\nC\t\"D\tE\"\n";
        let doc = parse(input).unwrap();
        let output = build(&doc);
        let doc2 = parse(&output).unwrap();
        assert_eq!(doc.rows, doc2.rows);
    }
}
