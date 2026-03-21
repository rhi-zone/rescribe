//! TSV (Tab-Separated Values) reader for rescribe.
//!
//! Parses TSV data into rescribe's document IR as a table.

use rescribe_core::{ConversionResult, Document, ParseError, ParseOptions};
use rescribe_std::{Node, node, prop};

/// Parse TSV input into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse TSV input into a document with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, _diags) = tsv_fmt::parse(input);

    let mut table = Node::new(node::TABLE);
    let mut is_first_row = true;

    for row in &doc.rows {
        let mut table_row = Node::new(node::TABLE_ROW);

        for cell in &row.cells {
            let cell_kind = if is_first_row { node::TABLE_HEADER } else { node::TABLE_CELL };
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(input: &str) -> Document {
        parse(input).unwrap().value
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
}
