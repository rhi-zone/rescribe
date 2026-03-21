//! TSV parser.

use crate::ast::{Cell, Diagnostic, Row, Span, TsvDoc};

/// Parse a TSV string into a [`TsvDoc`]. Infallible — never panics or returns an error.
pub fn parse(input: &str) -> (TsvDoc, Vec<Diagnostic>) {
    let mut rows = Vec::new();

    for line in input.lines() {
        if line.trim().is_empty() {
            continue;
        }

        let cells = parse_tsv_line(line);
        rows.push(Row { cells, span: Span::NONE });
    }

    (TsvDoc { rows, span: Span::NONE }, Vec::new())
}

fn parse_tsv_line(line: &str) -> Vec<Cell> {
    // TSV is simpler than CSV - just split on tabs
    // Quoted fields can contain tabs and newlines
    let mut cells = Vec::new();
    let mut current = String::new();
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
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '\t' if !in_quotes => {
                cells.push(Cell { value: current.clone(), span: Span::NONE });
                current.clear();
            }
            _ => {
                current.push(c);
            }
        }
    }

    cells.push(Cell { value: current, span: Span::NONE });
    cells
}
