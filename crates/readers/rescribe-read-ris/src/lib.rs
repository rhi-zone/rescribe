//! RIS (Research Information Systems) reader for rescribe.
//!
//! Parses RIS bibliography files into rescribe's document IR.
//! RIS is a standardized tag format for bibliographic citations.
//!
//! # Example
//!
//! ```
//! use rescribe_read_ris::parse;
//!
//! let ris = "TY  - JOUR\nAU  - Smith, John\nTI  - A Great Paper\nER  -";
//! let result = parse(ris).unwrap();
//! let doc = result.value;
//! ```

use rescribe_core::{ConversionResult, Document, Node, ParseError, Properties};
use rescribe_std::{node, prop};

/// Parse RIS text into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &Default::default())
}

/// Parse RIS text with options.
pub fn parse_with_options(
    input: &str,
    _options: &rescribe_core::ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let ris_doc = ris::parse(input).map_err(|e| ParseError::Invalid(e.to_string()))?;

    let mut entries = Vec::new();

    for entry in ris_doc.entries {
        entries.push(entry_to_node(&entry));
    }

    let content = if entries.is_empty() {
        Node::new(node::DOCUMENT)
    } else {
        Node::new(node::DOCUMENT).child(Node::new(node::DEFINITION_LIST).children(entries))
    };

    Ok(ConversionResult::ok(Document {
        content,
        resources: Default::default(),
        metadata: Properties::new(),
        source: None,
    }))
}

fn entry_to_node(entry: &ris::RisEntry) -> Node {
    // Generate a cite key from first author and year
    let cite_key = entry.generate_cite_key();
    let bibtex_type = ris::ris_type_to_bibtex(&entry.entry_type);

    // Create the term (citation key)
    let term = Node::new(node::DEFINITION_TERM)
        .child(Node::new(node::CODE).prop(prop::CONTENT, cite_key.clone()));

    // Build the description content
    let mut desc_children = Vec::new();

    // Entry type badge
    let type_text = format!("[{}] ", bibtex_type);
    desc_children.push(
        Node::new(node::SPAN)
            .prop("html:class", "ris-type")
            .child(Node::new(node::TEXT).prop(prop::CONTENT, type_text)),
    );

    // Authors (AU tag, can be multiple)
    let authors = entry.get_all("AU");
    if !authors.is_empty() {
        let author_text = authors.join("; ");
        desc_children.push(
            Node::new(node::STRONG).child(Node::new(node::TEXT).prop(prop::CONTENT, author_text)),
        );
        desc_children.push(Node::new(node::TEXT).prop(prop::CONTENT, ". "));
    }

    // Title (TI or T1)
    let title = entry.get_first("TI").or_else(|| entry.get_first("T1"));
    if let Some(t) = title {
        desc_children.push(
            Node::new(node::EMPHASIS)
                .child(Node::new(node::TEXT).prop(prop::CONTENT, t.to_string())),
        );
        desc_children.push(Node::new(node::TEXT).prop(prop::CONTENT, ". "));
    }

    // Journal/Publication (JO, JF, or T2)
    let journal = entry
        .get_first("JO")
        .or_else(|| entry.get_first("JF"))
        .or_else(|| entry.get_first("T2"));
    if let Some(j) = journal {
        desc_children.push(Node::new(node::TEXT).prop(prop::CONTENT, j.to_string()));
        desc_children.push(Node::new(node::TEXT).prop(prop::CONTENT, ". "));
    }

    // Volume (VL)
    if let Some(vol) = entry.get_first("VL") {
        desc_children.push(Node::new(node::TEXT).prop(prop::CONTENT, format!("{}.", vol)));
    }

    // Year (PY or Y1)
    let year = entry.get_first("PY").or_else(|| entry.get_first("Y1"));
    if let Some(y) = year {
        // Extract just the year part (format might be YYYY/MM/DD)
        let year_str = y.split('/').next().unwrap_or(y);
        desc_children.push(Node::new(node::TEXT).prop(prop::CONTENT, format!(" ({})", year_str)));
    }

    // Pages (SP - start page, EP - end page)
    if let Some(sp) = entry.get_first("SP") {
        let pages = if let Some(ep) = entry.get_first("EP") {
            format!(", pp. {}-{}", sp, ep)
        } else {
            format!(", p. {}", sp)
        };
        desc_children.push(Node::new(node::TEXT).prop(prop::CONTENT, pages));
    }

    // DOI (DO)
    if let Some(doi) = entry.get_first("DO") {
        desc_children.push(Node::new(node::TEXT).prop(prop::CONTENT, ". "));
        desc_children.push(
            Node::new(node::LINK)
                .prop(prop::URL, format!("https://doi.org/{}", doi))
                .child(Node::new(node::TEXT).prop(prop::CONTENT, format!("doi:{}", doi))),
        );
    }

    // URL (UR)
    if let Some(url) = entry.get_first("UR") {
        desc_children.push(Node::new(node::TEXT).prop(prop::CONTENT, ". "));
        desc_children.push(
            Node::new(node::LINK)
                .prop(prop::URL, url.to_string())
                .child(Node::new(node::TEXT).prop(prop::CONTENT, url.to_string())),
        );
    }

    let desc = Node::new(node::DEFINITION_DESC)
        .prop("ris:type", entry.entry_type.clone())
        .prop("ris:key", cite_key)
        .child(Node::new(node::PARAGRAPH).children(desc_children));

    Node::new("ris:entry").children(vec![term, desc])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_article() {
        let ris = r#"TY  - JOUR
AU  - Smith, John
AU  - Doe, Jane
TI  - A Great Paper
JO  - Nature
PY  - 2020
VL  - 123
SP  - 45
EP  - 67
DO  - 10.1234/nature.2020
ER  -"#;

        let result = parse(ris).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_book() {
        let ris = r#"TY  - BOOK
AU  - Knuth, Donald E.
TI  - The Art of Computer Programming
PY  - 1997
ER  -"#;

        let result = parse(ris).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_multiple() {
        let ris = r#"TY  - JOUR
AU  - First, Author
TI  - First Paper
ER  -
TY  - JOUR
AU  - Second, Author
TI  - Second Paper
ER  -"#;

        let result = parse(ris).unwrap();
        let doc = result.value;
        let def_list = &doc.content.children[0];
        assert_eq!(def_list.children.len(), 2);
    }

    #[test]
    fn test_parse_empty() {
        let ris = "";
        let result = parse(ris).unwrap();
        let doc = result.value;
        assert!(doc.content.children.is_empty());
    }
}
