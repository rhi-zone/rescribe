//! CSV reader for rescribe.
//!
//! Parses CSV (Comma-Separated Values) into rescribe's document IR as a table.

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
    let csv_doc = csv_fmt::parse(input)
        .map_err(|e| ParseError::Invalid(format!("CSV parse error: {}", e)))?;

    let mut rows = Vec::new();
    let mut is_header = true;

    for row in &csv_doc.rows {
        let cell_nodes: Vec<Node> = row
            .iter()
            .map(|cell| {
                let node_kind = if is_header {
                    node::TABLE_HEADER
                } else {
                    node::TABLE_CELL
                };
                Node::new(node_kind).child(Node::new(node::TEXT).prop(prop::CONTENT, cell.as_str()))
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
