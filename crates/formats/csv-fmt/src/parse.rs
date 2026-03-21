//! CSV parser.

use crate::ast::{Cell, CsvDoc, Diagnostic, Row, Span};

/// Parse a CSV string into a [`CsvDoc`]. Infallible — never panics or returns an error.
pub fn parse(input: &str) -> (CsvDoc, Vec<Diagnostic>) {
    let mut rows = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let cells = parse_csv_line(line);
        rows.push(Row { cells, span: Span::NONE });
    }

    (CsvDoc { rows, span: Span::NONE }, Vec::new())
}

fn parse_csv_line(line: &str) -> Vec<Cell> {
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
                cells.push(Cell { value: current.trim().to_string(), span: Span::NONE });
                current.clear();
            }
            _ => {
                current.push(c);
            }
        }
    }

    cells.push(Cell { value: current.trim().to_string(), span: Span::NONE });
    cells
}
