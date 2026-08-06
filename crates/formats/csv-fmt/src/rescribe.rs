//! AST↔`rescribe::Document` translation for CSV.
//!
//! This module only translates between [`CsvDoc`](crate::CsvDoc) and
//! rescribe's `Document` IR — no CSV tokenizing/parsing/emitting happens
//! here (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`). Enabled by the `rescribe` feature. csv-fmt has no
//! per-API-mode features (unlike other format crates in this workspace), so
//! both directions are gated on `rescribe` alone.
//!
//! # Mapping
//!
//! A `CsvDoc` maps to a single `table` node: each `Row` becomes a
//! `table_row`, and each `Cell` becomes a `table_header` (first row) or
//! `table_cell` (subsequent rows) wrapping a `text` node with the cell's
//! value. The writer reverses this: it finds the first `table` node in the
//! document, walks its `table_row` children, and concatenates the text
//! content of each cell.

#[cfg(feature = "rescribe")]
mod read {
    use rescribe_core::{ConversionResult, Document, Node, ParseError, ParseOptions};
    use rescribe_std::{node, prop};

    /// Parse CSV into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        parse_with_options(input, &ParseOptions::default())
    }

    /// Parse CSV with options.
    pub fn parse_with_options(
        input: &str,
        _options: &ParseOptions,
    ) -> Result<ConversionResult<Document>, ParseError> {
        let (csv_doc, _diags) = crate::parse(input);

        let mut rows = Vec::new();
        let mut is_header = true;

        for row in &csv_doc.rows {
            let cell_nodes: Vec<Node> = row
                .cells
                .iter()
                .map(|cell| {
                    let node_kind = if is_header {
                        node::TABLE_HEADER
                    } else {
                        node::TABLE_CELL
                    };
                    Node::new(node_kind)
                        .child(Node::new(node::TEXT).prop(prop::CONTENT, cell.value.as_str()))
                })
                .collect();

            rows.push(Node::new(node::TABLE_ROW).children(cell_nodes));
            is_header = false;
        }

        let table = Node::new(node::TABLE).children(rows);

        let document = Document {
            content: Node::new(node::DOCUMENT).child(table),
            resources: Default::default(),
            metadata: Default::default(),
            source: None,
        };

        Ok(ConversionResult::ok(document))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_parse_simple() {
            let result = parse("a,b,c\n1,2,3").unwrap();
            assert_eq!(result.value.content.children.len(), 1);
            let table = &result.value.content.children[0];
            assert_eq!(table.kind.as_str(), node::TABLE);
            assert_eq!(table.children.len(), 2);
        }

        #[test]
        fn test_parse_quoted() {
            let result = parse("name,value\n\"hello, world\",42").unwrap();
            assert_eq!(result.value.content.children.len(), 1);
        }

        #[test]
        fn test_parse_escaped_quotes() {
            let result = parse("a,b\n\"say \"\"hello\"\"\",test").unwrap();
            assert_eq!(result.value.content.children.len(), 1);
        }
    }
}

#[cfg(feature = "rescribe")]
mod write {
    use crate::{Cell, CsvDoc, Row, Span};
    use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, Node};
    use rescribe_std::{node, prop};

    /// Emit a document to CSV.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        emit_with_options(doc, &EmitOptions::default())
    }

    /// Emit a document to CSV with options.
    pub fn emit_with_options(
        doc: &Document,
        _options: &EmitOptions,
    ) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        // Find first table in document
        if let Some(table) = find_table(&doc.content) {
            let csv_doc = document_to_csv_doc(table);
            let output = crate::emit(&csv_doc);
            Ok(ConversionResult::ok(output.into_bytes()))
        } else {
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

    fn document_to_csv_doc(table: &Node) -> CsvDoc {
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
        CsvDoc {
            rows,
            span: Span::NONE,
        }
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

        fn emit_str(doc: &Document) -> String {
            String::from_utf8(emit(doc).unwrap().value).unwrap()
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
            assert!(output.contains("A,B"));
            assert!(output.contains("1,2"));
        }
    }
}

#[cfg(feature = "rescribe")]
pub use read::{parse, parse_with_options};
#[cfg(feature = "rescribe")]
pub use write::{emit, emit_with_options};
