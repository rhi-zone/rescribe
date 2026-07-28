//! RIS (Research Information Systems) writer for rescribe.
//!
//! Emits `bibliography`/`bibliography_entry`/`bibliography_field` IR nodes
//! (see `rescribe_std::node` and ADR 0005 in the rescribe repo) as RIS
//! format bibliographic data.

use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, PropValue,
};
use rescribe_std::{node, prop};

/// Legacy flat entry kinds, still accepted for backwards compatibility with
/// documents built by hand or by an older reader version (not produced by
/// `rescribe-read-ris` any more, which now emits `bibliography_entry`).
const RIS_ENTRY: &str = "ris:entry";
const BIBTEX_ENTRY: &str = "bibtex:entry";
const CITATION_ENTRY: &str = "citation_entry";

/// Emit a document as RIS.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as RIS with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut ctx = EmitContext::new();
    emit_nodes(&doc.content.children, &mut ctx);
    Ok(ConversionResult::with_warnings(
        ctx.output.into_bytes(),
        ctx.warnings,
    ))
}

struct EmitContext {
    output: String,
    warnings: Vec<FidelityWarning>,
}

impl EmitContext {
    fn new() -> Self {
        Self {
            output: String::new(),
            warnings: Vec::new(),
        }
    }
}

fn emit_nodes(nodes: &[Node], ctx: &mut EmitContext) {
    for node in nodes {
        emit_node(node, ctx);
    }
}

fn emit_node(node: &Node, ctx: &mut EmitContext) {
    match node.kind.as_str() {
        "document" | "definition_list" | node::BIBLIOGRAPHY => emit_nodes(&node.children, ctx),

        node::BIBLIOGRAPHY_ENTRY => emit_bibliography_entry(node, ctx),

        RIS_ENTRY => emit_ris_entry(node, ctx),
        BIBTEX_ENTRY => emit_bibtex_entry(node, ctx),
        CITATION_ENTRY => emit_citation_entry(node, ctx),

        _ => {
            // Check for known entry types
            if is_bibtex_type(node.kind.as_str()) {
                emit_typed_entry(node, ctx);
            } else {
                emit_nodes(&node.children, ctx);
            }
        }
    }
}

fn is_bibtex_type(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "article"
            | "book"
            | "booklet"
            | "conference"
            | "inbook"
            | "incollection"
            | "inproceedings"
            | "manual"
            | "mastersthesis"
            | "misc"
            | "phdthesis"
            | "proceedings"
            | "techreport"
            | "unpublished"
            | "online"
            | "software"
            | "dataset"
    )
}

/// Write a `bibliography_entry` node (see `rescribe-read-ris`'s
/// `entry_to_node`) back to an RIS record. `ris:field` on each
/// `bibliography_field` child (set by every field-producing arm of the
/// reader) names the exact source tag; `field:role` is the fallback for a
/// field built by a non-RIS producer (a cross-format conversion into RIS).
/// `page_first`/`page_last` map directly back to `SP`/`EP` (RIS keeps them
/// as separate tags, unlike BibTeX/CSL-JSON's combined page string, so no
/// recombining is needed); `prop::DATE` becomes `PY` in `YYYY/MM/DD/`
/// form (the RIS date convention — a trailing slash even when month/day
/// are absent).
fn emit_bibliography_entry(node: &Node, ctx: &mut EmitContext) {
    let mut entry = ris::RisEntry::new(node.props.get_str("ris:type").unwrap_or("GEN"));

    if let Some(PropValue::Map(date)) = node.props.get(prop::DATE) {
        entry.add_field("PY", &format_ris_date(date));
    }

    for child in &node.children {
        if child.kind.as_str() != node::BIBLIOGRAPHY_FIELD {
            continue;
        }
        let role = child.props.get_str(prop::FIELD_ROLE).unwrap_or("misc");
        let tag = child
            .props
            .get_str("ris:field")
            .map(str::to_string)
            .unwrap_or_else(|| default_ris_tag(role).to_string());
        let text = flatten_field_text(child);
        if !text.is_empty() {
            entry.add_field(&tag, &text);
        }
    }

    let output = ris::emit(&ris::RisDoc {
        entries: vec![entry],
        span: ris::Span::NONE,
    });
    ctx.output.push_str(&output);
}

/// RIS tag fallback for a `bibliography_field` built by a non-RIS producer
/// (no `ris:field` property to name the exact source tag).
fn default_ris_tag(role: &str) -> &'static str {
    match role {
        "author" => "AU",
        "editor" => "A2",
        "title" => "TI",
        "container_title" => "JO",
        "publisher" => "PB",
        "publisher_location" => "CY",
        "edition" => "ET",
        "volume" => "VL",
        "issue" => "IS",
        "page_first" => "SP",
        "page_last" => "EP",
        "identifier" => "UR",
        _ => "N1",
    }
}

fn format_ris_date(map: &std::collections::HashMap<String, PropValue>) -> String {
    let as_int = |key: &str| match map.get(key) {
        Some(PropValue::Int(i)) => Some(*i),
        _ => None,
    };
    let Some(year) = as_int("year") else {
        return String::new();
    };
    let mut text = format!("{year:04}/");
    if let Some(month) = as_int("month") {
        text.push_str(&format!("{month:02}/"));
        if let Some(day) = as_int("day") {
            text.push_str(&format!("{day:02}/"));
        }
    }
    text
}

/// Concatenate a field's descendant `TEXT` node content (depth-first).
fn flatten_field_text(node: &Node) -> String {
    let mut out = String::new();
    flatten_field_text_into(node, &mut out);
    out
}

fn flatten_field_text_into(node: &Node, out: &mut String) {
    if node.kind.as_str() == node::TEXT
        && let Some(content) = node.props.get_str(prop::CONTENT)
    {
        out.push_str(content);
    }
    for child in &node.children {
        flatten_field_text_into(child, out);
    }
}

fn emit_ris_entry(node: &Node, ctx: &mut EmitContext) {
    let mut entry = ris::RisEntry::new(node.props.get_str("ris:type").unwrap_or("GEN"));

    // Emit all ris: prefixed properties
    for (key, value) in node.props.iter() {
        if let Some(tag) = key.strip_prefix("ris:")
            && tag != "type"
            && tag != "key"
            && let rescribe_core::PropValue::String(s) = value
        {
            entry.add_field(&tag.to_uppercase(), s);
        }
    }

    let output = ris::emit(&ris::RisDoc {
        entries: vec![entry],
        span: ris::Span::NONE,
    });
    ctx.output.push_str(&output);
}

fn emit_bibtex_entry(node: &Node, ctx: &mut EmitContext) {
    let bibtex_type = node.props.get_str("bibtex:type").unwrap_or("misc");
    let ris_type = ris::bibtex_type_to_ris(bibtex_type);
    let mut entry = ris::RisEntry::new(ris_type);

    // Map bibtex fields to RIS tags
    emit_bibtex_fields(node, &mut entry);

    let output = ris::emit(&ris::RisDoc {
        entries: vec![entry],
        span: ris::Span::NONE,
    });
    ctx.output.push_str(&output);
}

fn emit_bibtex_fields(node: &Node, entry: &mut ris::RisEntry) {
    for (key, value) in node.props.iter() {
        if let Some(field) = key.strip_prefix("bibtex:")
            && let rescribe_core::PropValue::String(s) = value
            && let Some(ris_tag) = ris::bibtex_field_to_ris(field)
        {
            entry.add_field(ris_tag, s);
        }
    }
}

fn emit_citation_entry(node: &Node, ctx: &mut EmitContext) {
    let csl_type = node.props.get_str("type").unwrap_or("misc");
    let ris_type = ris::csl_type_to_ris(csl_type);
    let mut entry = ris::RisEntry::new(ris_type);

    // Map CSL fields to RIS
    if let Some(title) = node.props.get_str("title") {
        entry.add_field("TI", title);
    }
    if let Some(author) = node.props.get_str("author") {
        // Authors might be semicolon-separated
        for a in author.split(';') {
            entry.add_field("AU", a.trim());
        }
    }
    if let Some(container) = node.props.get_str("container-title") {
        entry.add_field("JO", container);
    }
    if let Some(issued) = node.props.get_str("issued") {
        entry.add_field("PY", issued);
    }
    if let Some(volume) = node.props.get_str("volume") {
        entry.add_field("VL", volume);
    }
    if let Some(page) = node.props.get_str("page") {
        // Pages might be in format "start-end"
        let parts: Vec<&str> = page.split('-').collect();
        if !parts.is_empty() {
            entry.add_field("SP", parts[0]);
        }
        if parts.len() > 1 {
            entry.add_field("EP", parts[1]);
        }
    }
    if let Some(doi) = node.props.get_str("DOI") {
        entry.add_field("DO", doi);
    }
    if let Some(url) = node.props.get_str("URL") {
        entry.add_field("UR", url);
    }
    if let Some(abs) = node.props.get_str("abstract") {
        entry.add_field("AB", abs);
    }

    let output = ris::emit(&ris::RisDoc {
        entries: vec![entry],
        span: ris::Span::NONE,
    });
    ctx.output.push_str(&output);
}

fn emit_typed_entry(node: &Node, ctx: &mut EmitContext) {
    let bibtex_type = node.kind.as_str().to_lowercase();
    let ris_type = ris::bibtex_type_to_ris(&bibtex_type);
    let mut entry = ris::RisEntry::new(ris_type);

    // Emit standard fields
    let field_mappings = [
        ("author", "AU"),
        ("title", "TI"),
        ("journal", "JO"),
        ("booktitle", "T2"),
        ("year", "PY"),
        ("volume", "VL"),
        ("number", "IS"),
        ("pages", "SP"), // Note: pages needs special handling
        ("publisher", "PB"),
        ("address", "CY"),
        ("doi", "DO"),
        ("url", "UR"),
        ("abstract", "AB"),
        ("keywords", "KW"),
        ("isbn", "SN"),
        ("issn", "SN"),
    ];

    for (prop_name, ris_tag) in field_mappings {
        if let Some(value) = node.props.get_str(prop_name) {
            if prop_name == "author" {
                // Authors might be "and"-separated
                for author in value.split(" and ") {
                    entry.add_field(ris_tag, author.trim());
                }
            } else if prop_name == "pages" {
                // Handle page ranges
                let parts: Vec<&str> = value.split('-').collect();
                if !parts.is_empty() {
                    entry.add_field("SP", parts[0].trim());
                }
                if parts.len() > 1 {
                    entry.add_field("EP", parts[1].trim());
                }
            } else {
                entry.add_field(ris_tag, value);
            }
        }
    }

    let output = ris::emit(&ris::RisDoc {
        entries: vec![entry],
        span: ris::Span::NONE,
    });
    ctx.output.push_str(&output);
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_core::NodeKind;

    fn emit_str(doc: &Document) -> String {
        String::from_utf8(emit(doc).unwrap().value).unwrap()
    }

    #[test]
    fn test_emit_typed_entry() {
        let entry = Node::new(NodeKind::from("article"))
            .prop("author", "Smith, John")
            .prop("title", "A Great Paper")
            .prop("journal", "Nature")
            .prop("year", "2020");

        let doc = Document::new()
            .with_content(Node::new(NodeKind::from("document")).children(vec![entry]));
        let output = emit_str(&doc);

        assert!(output.contains("TY  - JOUR"));
        assert!(output.contains("AU  - Smith, John"));
        assert!(output.contains("TI  - A Great Paper"));
        assert!(output.contains("JO  - Nature"));
        assert!(output.contains("PY  - 2020"));
        assert!(output.contains("ER  -"));
    }

    #[test]
    fn test_emit_pages() {
        let entry = Node::new(NodeKind::from("article"))
            .prop("title", "Test")
            .prop("pages", "123-456");

        let doc = Document::new()
            .with_content(Node::new(NodeKind::from("document")).children(vec![entry]));
        let output = emit_str(&doc);

        assert!(output.contains("SP  - 123"));
        assert!(output.contains("EP  - 456"));
    }

    #[test]
    fn test_emit_multiple_authors() {
        let entry = Node::new(NodeKind::from("article"))
            .prop("title", "Test")
            .prop("author", "Smith, John and Doe, Jane");

        let doc = Document::new()
            .with_content(Node::new(NodeKind::from("document")).children(vec![entry]));
        let output = emit_str(&doc);

        assert!(output.contains("AU  - Smith, John"));
        assert!(output.contains("AU  - Doe, Jane"));
    }
}
