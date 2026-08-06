//! AST↔`rescribe::Document` translation for TSV.
//!
//! This module only translates between [`TsvDoc`](crate::TsvDoc) and
//! rescribe's `Document` IR — no TSV tokenizing/parsing/emitting happens
//! here (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`). Enabled by the `rescribe` feature. tsv-fmt has no
//! separate reader/writer API modes (parse/emit are always compiled), so
//! unlike other `-fmt` crates this module isn't additionally gated on a
//! per-mode feature — `rescribe` alone gates both directions.

use crate::{Cell, Row, Span, TsvDoc};
use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node, ParseError};
use rescribe_std::{node, prop};

/// Parse TSV input into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &rescribe_core::ParseOptions::default())
}

/// Parse TSV input into a document with options.
pub fn parse_with_options(
    input: &str,
    _options: &rescribe_core::ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, _diags) = crate::parse(input);

    let mut table = Node::new(node::TABLE);
    let mut is_first_row = true;

    for row in &doc.rows {
        let mut table_row = Node::new(node::TABLE_ROW);

        for cell in &row.cells {
            let cell_kind = if is_first_row {
                node::TABLE_HEADER
            } else {
                node::TABLE_CELL
            };
            let cell_node = Node::new(cell_kind)
                .child(Node::new(node::TEXT).prop(prop::CONTENT, cell.value.trim()));
            table_row = table_row.child(cell_node);
        }

        table = table.child(table_row);
        is_first_row = false;
    }

    let result = Document {
        content: Node::new(node::DOCUMENT).child(table),
        resources: Default::default(),
        metadata: Default::default(),
        source: None,
    };

    Ok(ConversionResult::ok(result))
}

/// Emit a document to TSV.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document to TSV with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    // Find first table in document
    if let Some(table) = find_table(&doc.content) {
        let mut rows = Vec::new();
        for row in &table.children {
            if row.kind.as_str() == node::TABLE_ROW {
                let cells: Vec<Cell> = row
                    .children
                    .iter()
                    .map(|n| Cell {
                        value: get_text_content(n),
                        span: Span::NONE,
                    })
                    .collect();
                rows.push(Row {
                    cells,
                    span: Span::NONE,
                });
            }
        }
        let tsv_doc = TsvDoc {
            rows,
            span: Span::NONE,
        };
        let output = crate::emit(&tsv_doc);
        Ok(ConversionResult::ok(output.into_bytes()))
    } else {
        // No table found; return empty TSV
        Ok(ConversionResult::ok(Vec::new()))
    }
}

fn find_table(node: &Node) -> Option<&Node> {
    if node.kind.as_str() == node::TABLE {
        return Some(node);
    }
    for child in &node.children {
        if let Some(table) = find_table(child) {
            return Some(table);
        }
    }
    None
}

fn get_text_content(node: &Node) -> String {
    let mut text = String::new();
    collect_text(node, &mut text);
    text
}

fn collect_text(node: &Node, output: &mut String) {
    if node.kind.as_str() == node::TEXT
        && let Some(content) = node.props.get_str(prop::CONTENT)
    {
        output.push_str(content);
    }
    for child in &node.children {
        collect_text(child, output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(input: &str) -> Document {
        parse(input).unwrap().value
    }

    fn emit_str(doc: &Document) -> String {
        String::from_utf8(emit(doc).unwrap().value).unwrap()
    }

    #[test]
    fn test_parse_simple_tsv() {
        let input = "Name\tAge\tCity\nAlice\t30\tNew York\nBob\t25\tLondon";
        let doc = parse_str(input);
        let table = &doc.content.children[0];
        assert_eq!(table.kind.as_str(), node::TABLE);
        assert_eq!(table.children.len(), 3); // 3 rows
    }

    #[test]
    fn test_parse_quoted_fields() {
        let input = "Name\tDescription\n\"Item\"\t\"Has\ttabs\"";
        let doc = parse_str(input);
        let table = &doc.content.children[0];
        let data_row = &table.children[1];
        assert_eq!(data_row.children.len(), 2);
    }

    #[test]
    fn test_emit_simple_table() {
        let doc = Document {
            content: Node::new(node::DOCUMENT).child(
                Node::new(node::TABLE)
                    .child(
                        Node::new(node::TABLE_ROW)
                            .child(
                                Node::new(node::TABLE_HEADER)
                                    .child(Node::new(node::TEXT).prop(prop::CONTENT, "A")),
                            )
                            .child(
                                Node::new(node::TABLE_HEADER)
                                    .child(Node::new(node::TEXT).prop(prop::CONTENT, "B")),
                            ),
                    )
                    .child(
                        Node::new(node::TABLE_ROW)
                            .child(
                                Node::new(node::TABLE_CELL)
                                    .child(Node::new(node::TEXT).prop(prop::CONTENT, "1")),
                            )
                            .child(
                                Node::new(node::TABLE_CELL)
                                    .child(Node::new(node::TEXT).prop(prop::CONTENT, "2")),
                            ),
                    ),
            ),
            resources: Default::default(),
            metadata: Default::default(),
            source: None,
        };
        let output = emit_str(&doc);
        assert!(output.contains("A\tB"));
        assert!(output.contains("1\t2"));
    }
}
