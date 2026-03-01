//! CSV writer for rescribe.
//!
//! Serializes rescribe's document IR tables to CSV format.

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
        let output = csv_fmt::build(&csv_doc);
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

fn document_to_csv_doc(table: &Node) -> csv_fmt::CsvDoc {
    let mut rows = Vec::new();
    for row in &table.children {
        if row.kind.as_str() == node::TABLE_ROW {
            let cells: Vec<String> = row.children.iter().map(get_text_content).collect();
            rows.push(cells);
        }
    }
    csv_fmt::CsvDoc { rows }
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
