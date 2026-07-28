//! BibTeX writer for rescribe.
//!
//! Emits `bibliography`/`bibliography_entry`/`bibliography_field` IR nodes
//! (see `rescribe_std::node` and ADR 0005 in the rescribe repo) as BibTeX
//! source.

use std::collections::HashMap;

use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, PropValue, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};

/// Legacy flat entry kind, still accepted for backwards compatibility with
/// documents built by hand or by an older reader version (not produced by
/// `rescribe-read-bibtex` any more, which now emits `bibliography_entry`).
const BIBTEX_ENTRY: &str = "bibtex:entry";

/// Emit a document as BibTeX.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as BibTeX with custom options.
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

/// Emit context for tracking state during emission.
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

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }
}

/// Emit a sequence of nodes.
fn emit_nodes(nodes: &[Node], ctx: &mut EmitContext) {
    for node in nodes {
        emit_node(node, ctx);
    }
}

/// Emit a single node.
fn emit_node(node: &Node, ctx: &mut EmitContext) {
    match node.kind.as_str() {
        "document" | node::BIBLIOGRAPHY => emit_nodes(&node.children, ctx),

        node::BIBLIOGRAPHY_ENTRY => emit_bibliography_entry(node, ctx),

        // Legacy shape, kept for backwards compatibility (see `BIBTEX_ENTRY`
        // doc comment above).
        BIBTEX_ENTRY => emit_legacy_entry(node, ctx),
        "citation_entry" => emit_citation_entry(node, ctx),

        _ => {
            if is_bibtex_type(node.kind.as_str()) {
                emit_typed_entry(node, ctx);
            } else {
                ctx.warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
                    format!("Unknown node type for BibTeX: {}", node.kind.as_str()),
                ));
                emit_nodes(&node.children, ctx);
            }
        }
    }
}

/// Check if a string is a known BibTeX entry type.
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

/// Write a `bibliography_entry` node (see `rescribe-read-bibtex`'s
/// `convert_entry`) back to a BibTeX `@type{key, ...}` block. `bibtex:field`
/// on each `bibliography_field` child (set by every field-producing arm of
/// the reader) names the exact source field; `field:role` is the fallback
/// for a field built by a non-BibTeX producer (a cross-format conversion
/// into BibTeX). Repeated `author`/`editor` fields are rejoined with
/// ` and `; a `page_first`/`page_last` pair recombines into one `pages`
/// field; `prop::DATE` becomes `year`/`month`/`day` fields (or a single
/// `date` field with a `?`/`~`/`%` suffix when the source date was
/// marked uncertain/approximate — the BibLaTeX convention, see
/// `rescribe-read-bibtex`'s `convert_entry`).
fn emit_bibliography_entry(node: &Node, ctx: &mut EmitContext) {
    let entry_type = node
        .props
        .get_str("bibtex:entry-type")
        .unwrap_or("misc")
        .to_lowercase();
    let cite_key = node.props.get_str("bibtex:key").unwrap_or("unknown");

    ctx.write("@");
    ctx.write(&entry_type);
    ctx.write("{");
    ctx.write(cite_key);
    ctx.write(",\n");

    if let Some(PropValue::Map(date)) = node.props.get(prop::DATE) {
        emit_date_fields(
            date,
            node.props
                .get_bool("bibtex:date-uncertain")
                .unwrap_or(false),
            node.props
                .get_bool("bibtex:date-approximate")
                .unwrap_or(false),
            ctx,
        );
    }

    let mut authors = Vec::new();
    let mut editors = Vec::new();
    let mut iter = node.children.iter().peekable();
    while let Some(child) = iter.next() {
        if child.kind.as_str() != node::BIBLIOGRAPHY_FIELD {
            continue;
        }
        let role = child.props.get_str(prop::FIELD_ROLE).unwrap_or("misc");
        match role {
            "author" => authors.push(person_field_text(child)),
            "editor" => editors.push(person_field_text(child)),
            "page_first"
                if iter
                    .peek()
                    .and_then(|next| next.props.get_str(prop::FIELD_ROLE))
                    == Some("page_last") =>
            {
                let last = iter.next().unwrap();
                let first_text = flatten_field_text(child);
                let last_text = flatten_field_text(last);
                emit_field("pages", &format!("{first_text}--{last_text}"), ctx);
            }
            _ => {
                let field_name = child.props.get_str("bibtex:field").unwrap_or(role);
                let text = flatten_field_text(child);
                if !text.is_empty() {
                    emit_field(field_name, &text, ctx);
                }
            }
        }
    }
    if !authors.is_empty() {
        emit_field("author", &authors.join(" and "), ctx);
    }
    if !editors.is_empty() {
        emit_field("editor", &editors.join(" and "), ctx);
    }

    ctx.write("}\n\n");
}

/// Reconstruct `prop::DATE`'s `year`/`month`/`day` map (see the property's
/// own doc comment) as BibTeX fields. When the date carries no uncertainty
/// marker, this is the classic `year`/`month`/`day` field trio (month as a
/// three-letter English abbreviation, the form `biblatex`'s own parser
/// recognizes — a bare number wouldn't round-trip through it). When the
/// source date was uncertain or approximate, those flags have no
/// `year`/`month`/`day`-trio equivalent, so a single BibLaTeX-style `date`
/// field with a `?`/`~`/`%` suffix is emitted instead (the inverse of
/// `rescribe-read-bibtex`'s uncertain/approximate handling).
fn emit_date_fields(
    map: &HashMap<String, PropValue>,
    uncertain: bool,
    approximate: bool,
    ctx: &mut EmitContext,
) {
    let as_int = |key: &str| match map.get(key) {
        Some(PropValue::Int(i)) => Some(*i),
        _ => None,
    };
    let Some(year) = as_int("year") else {
        return;
    };
    let month = as_int("month");
    let day = as_int("day");

    if uncertain || approximate {
        let mut text = format!("{year:04}");
        if let Some(m) = month {
            text.push_str(&format!("-{m:02}"));
            if let Some(d) = day {
                text.push_str(&format!("-{d:02}"));
            }
        }
        text.push(if uncertain && approximate {
            '%'
        } else if uncertain {
            '?'
        } else {
            '~'
        });
        emit_field("date", &text, ctx);
        return;
    }

    emit_field("year", &year.to_string(), ctx);
    if let Some(m) = month
        && let Some(abbr) = month_abbr(m)
    {
        emit_field("month", abbr, ctx);
    }
    if let Some(d) = day {
        emit_field("day", &d.to_string(), ctx);
    }
}

fn month_abbr(month: i64) -> Option<&'static str> {
    const ABBR: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    ABBR.get(usize::try_from(month - 1).ok()?).copied()
}

/// Join an `author`/`editor` field's direct `TEXT` children (given name,
/// prefix, family name, suffix — see `rescribe-read-bibtex`'s
/// `person_field`, which emits one `TEXT` node per non-empty `Person` part)
/// with spaces, the inverse of that same split.
fn person_field_text(node: &Node) -> String {
    node.children
        .iter()
        .filter_map(|c| {
            (c.kind.as_str() == node::TEXT)
                .then(|| c.props.get_str(prop::CONTENT))
                .flatten()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Concatenate a field's descendant `TEXT` node content (depth-first).
/// `bibliography_field` children are always ordinary inline nodes (see
/// ADR 0005), but `biblatex` doesn't parse LaTeX markup into structured
/// chunks in the first place (`rescribe-read-bibtex` only ever produces a
/// single `TEXT` child per field), so flattening is lossless for BibTeX
/// specifically even though the IR shape supports richer nesting.
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

/// Emit a legacy flat `bibtex:entry` node (see `BIBTEX_ENTRY` doc comment).
fn emit_legacy_entry(node: &Node, ctx: &mut EmitContext) {
    let entry_type = node
        .props
        .get_str("bibtex:type")
        .unwrap_or("misc")
        .to_lowercase();
    let cite_key = node.props.get_str("bibtex:key").unwrap_or("unknown");

    ctx.write("@");
    ctx.write(&entry_type);
    ctx.write("{");
    ctx.write(cite_key);
    ctx.write(",\n");

    emit_bibtex_fields(node, ctx);

    ctx.write("}\n\n");
}

/// Emit a typed entry (where the node kind is the entry type).
fn emit_typed_entry(node: &Node, ctx: &mut EmitContext) {
    let entry_type = node.kind.as_str().to_lowercase();
    let cite_key = node
        .props
        .get_str("key")
        .or(node.props.get_str(prop::ID))
        .unwrap_or("unknown");

    ctx.write("@");
    ctx.write(&entry_type);
    ctx.write("{");
    ctx.write(cite_key);
    ctx.write(",\n");

    emit_standard_fields(node, ctx);

    ctx.write("}\n\n");
}

/// Emit a citation entry with CSL-like properties.
fn emit_citation_entry(node: &Node, ctx: &mut EmitContext) {
    // Map CSL type to BibTeX type
    let csl_type = node.props.get_str("type").unwrap_or("misc");
    let entry_type = csl_to_bibtex_type(csl_type);
    let cite_key = node.props.get_str(prop::ID).unwrap_or("unknown");

    ctx.write("@");
    ctx.write(entry_type);
    ctx.write("{");
    ctx.write(cite_key);
    ctx.write(",\n");

    emit_csl_fields(node, ctx);

    ctx.write("}\n\n");
}

/// Map CSL types to BibTeX types.
fn csl_to_bibtex_type(csl_type: &str) -> &'static str {
    match csl_type {
        "article-journal" | "article-magazine" | "article-newspaper" => "article",
        "book" => "book",
        "chapter" => "incollection",
        "paper-conference" => "inproceedings",
        "thesis" => "phdthesis",
        "report" => "techreport",
        "webpage" | "post-weblog" => "online",
        "software" => "software",
        "dataset" => "dataset",
        _ => "misc",
    }
}

/// Emit fields from bibtex: prefixed properties.
fn emit_bibtex_fields(node: &Node, ctx: &mut EmitContext) {
    let mut fields: Vec<(&str, String)> = Vec::new();

    // Collect all bibtex: prefixed properties
    for (key, value) in node.props.iter() {
        if let Some(field_name) = key.strip_prefix("bibtex:")
            && field_name != "type"
            && field_name != "key"
            && let rescribe_core::PropValue::String(s) = value
        {
            fields.push((field_name, s.clone()));
        }
    }

    // Sort fields for consistent output
    fields.sort_by(|a, b| a.0.cmp(b.0));

    for (name, value) in fields {
        emit_field(name, &value, ctx);
    }
}

/// Emit standard BibTeX fields from properties.
fn emit_standard_fields(node: &Node, ctx: &mut EmitContext) {
    // Standard BibTeX fields
    let field_mappings = [
        ("author", "author"),
        ("title", "title"),
        ("journal", "journal"),
        ("booktitle", "booktitle"),
        ("year", "year"),
        ("volume", "volume"),
        ("number", "number"),
        ("pages", "pages"),
        ("publisher", "publisher"),
        ("address", "address"),
        ("edition", "edition"),
        ("editor", "editor"),
        ("series", "series"),
        ("month", "month"),
        ("note", "note"),
        ("doi", "doi"),
        ("url", "url"),
        ("isbn", "isbn"),
        ("issn", "issn"),
        ("abstract", "abstract"),
        ("keywords", "keywords"),
        ("institution", "institution"),
        ("school", "school"),
        ("howpublished", "howpublished"),
        ("organization", "organization"),
        ("chapter", "chapter"),
    ];

    for (prop_name, field_name) in field_mappings {
        if let Some(value) = node.props.get_str(prop_name) {
            emit_field(field_name, value, ctx);
        }
    }
}

/// Emit CSL fields mapped to BibTeX fields.
fn emit_csl_fields(node: &Node, ctx: &mut EmitContext) {
    // CSL to BibTeX field mappings
    if let Some(title) = node.props.get_str("title") {
        emit_field("title", title, ctx);
    }

    // Handle authors (could be array or string)
    if let Some(author) = node.props.get_str("author") {
        emit_field("author", author, ctx);
    }

    // Container title maps to journal/booktitle depending on type
    if let Some(container) = node.props.get_str("container-title") {
        let csl_type = node.props.get_str("type").unwrap_or("");
        if csl_type == "article-journal" {
            emit_field("journal", container, ctx);
        } else {
            emit_field("booktitle", container, ctx);
        }
    }

    // Date handling
    if let Some(year) = node.props.get_str("issued") {
        // Try to extract just the year
        let year_str = year.split('-').next().unwrap_or(year);
        emit_field("year", year_str, ctx);
    }

    // Other direct mappings
    let direct_mappings = [
        ("volume", "volume"),
        ("issue", "number"),
        ("page", "pages"),
        ("publisher", "publisher"),
        ("publisher-place", "address"),
        ("DOI", "doi"),
        ("URL", "url"),
        ("ISBN", "isbn"),
        ("ISSN", "issn"),
        ("abstract", "abstract"),
        ("note", "note"),
    ];

    for (csl_name, bibtex_name) in direct_mappings {
        if let Some(value) = node.props.get_str(csl_name) {
            emit_field(bibtex_name, value, ctx);
        }
    }
}

/// Emit a single BibTeX field.
fn emit_field(name: &str, value: &str, ctx: &mut EmitContext) {
    ctx.write("  ");
    ctx.write(name);
    ctx.write(" = {");
    ctx.write(&escape_bibtex(value));
    ctx.write("},\n");
}

/// Escape special BibTeX characters.
fn escape_bibtex(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            // These characters have special meaning in BibTeX
            '#' | '$' | '%' | '&' | '_' => {
                result.push('\\');
                result.push(c);
            }
            // Preserve braces as they're used for grouping
            '{' | '}' => result.push(c),
            _ => result.push(c),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_core::{Document, NodeKind};

    fn emit_str(doc: &Document) -> String {
        let result = emit(doc).unwrap();
        String::from_utf8(result.value).unwrap()
    }

    fn make_field(role: &str, field_name: &str, text: &str) -> Node {
        Node::new(NodeKind::from(node::BIBLIOGRAPHY_FIELD))
            .prop(prop::FIELD_ROLE, role)
            .prop("bibtex:field", field_name)
            .child(Node::new(NodeKind::from(node::TEXT)).prop(prop::CONTENT, text))
    }

    fn make_entry(entry_type: &str, key: &str, fields: Vec<Node>) -> Node {
        Node::new(NodeKind::from(node::BIBLIOGRAPHY_ENTRY))
            .prop("bibtex:entry-type", entry_type)
            .prop("bibtex:key", key)
            .children(fields)
    }

    #[test]
    fn test_emit_article() {
        let entry = make_entry(
            "article",
            "smith2024",
            vec![
                make_field("author", "author", "John Smith"),
                make_field("title", "title", "A Great Paper"),
                make_field("container_title", "journal", "Nature"),
            ],
        );

        let doc = Document::new()
            .with_content(Node::new(NodeKind::from("document")).children(vec![entry]));
        let output = emit_str(&doc);

        assert!(output.contains("@article{smith2024,"));
        assert!(output.contains("author = {John Smith},"));
        assert!(output.contains("title = {A Great Paper},"));
        assert!(output.contains("journal = {Nature},"));
    }

    #[test]
    fn test_emit_multi_author() {
        let entry = make_entry(
            "article",
            "smith2021",
            vec![
                make_field("author", "author", "J. Smith"),
                make_field("author", "author", "A. Jones"),
            ],
        );
        let doc = Document::new()
            .with_content(Node::new(NodeKind::from("document")).children(vec![entry]));
        let output = emit_str(&doc);
        assert!(output.contains("author = {J. Smith and A. Jones},"));
    }

    #[test]
    fn test_emit_date() {
        let mut map = HashMap::new();
        map.insert("year".to_string(), PropValue::Int(2020));
        map.insert("month".to_string(), PropValue::Int(3));
        let entry = Node::new(NodeKind::from(node::BIBLIOGRAPHY_ENTRY))
            .prop("bibtex:entry-type", "article")
            .prop("bibtex:key", "x")
            .prop(prop::DATE, PropValue::Map(map));
        let doc = Document::new()
            .with_content(Node::new(NodeKind::from("document")).children(vec![entry]));
        let output = emit_str(&doc);
        assert!(output.contains("year = {2020},"));
        assert!(output.contains("month = {mar},"));
    }

    #[test]
    fn test_escape_special_chars() {
        let entry = make_entry(
            "misc",
            "test",
            vec![make_field(
                "title",
                "title",
                "100% Pure & Simple: A $10 Solution",
            )],
        );

        let doc = Document::new()
            .with_content(Node::new(NodeKind::from("document")).children(vec![entry]));
        let output = emit_str(&doc);

        assert!(output.contains("100\\% Pure \\& Simple: A \\$10 Solution"));
    }

    #[test]
    fn test_emit_typed_entry() {
        let entry = Node::new(NodeKind::from("article"))
            .prop("key", "test2024")
            .prop("author", "Test Author")
            .prop("title", "Test Title")
            .prop("year", "2024");
        let doc = Document::new()
            .with_content(Node::new(NodeKind::from("document")).children(vec![entry]));
        let output = emit_str(&doc);

        assert!(output.contains("@article{test2024,"));
        assert!(output.contains("author = {Test Author},"));
    }

    #[test]
    fn test_emit_multiple_entries() {
        let entry1 = make_entry(
            "article",
            "first",
            vec![make_field("title", "title", "First")],
        );
        let entry2 = make_entry(
            "book",
            "second",
            vec![make_field("title", "title", "Second")],
        );

        let doc = Document::new()
            .with_content(Node::new(NodeKind::from("document")).children(vec![entry1, entry2]));
        let output = emit_str(&doc);

        assert!(output.contains("@article{first,"));
        assert!(output.contains("@book{second,"));
    }
}
