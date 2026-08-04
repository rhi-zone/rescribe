//! BibTeX writer for rescribe.
//!
//! Emits `bibliography`/`bibliography_entry`/`bibliography_field` IR nodes
//! (see `rescribe_std::node` and ADR 0005 in the rescribe repo) as BibTeX
//! source.
//!
//! Actual BibTeX syntax (entry headers, field escaping, brace wrapping) is
//! produced by the `biblatex` crate's own `Entry::to_bibtex_string()` /
//! `Bibliography::to_bibtex_string()` (the same crate `rescribe-read-bibtex`
//! uses to parse). This adapter's only job is building a `biblatex::Entry`
//! from the rescribe IR shapes it accepts — it does not hand-roll escaping
//! or field/entry syntax itself.

use biblatex::{Bibliography, Chunk, Date, DateValue, Datetime, Entry, EntryType, Spanned};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, PropValue, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};
use std::collections::HashMap;

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

    collect_nodes(&doc.content.children, &mut ctx);

    let output = ctx.bibliography.to_bibtex_string();

    Ok(ConversionResult::with_warnings(
        output.into_bytes(),
        ctx.warnings,
    ))
}

/// Emit context for tracking state during emission.
struct EmitContext {
    bibliography: Bibliography,
    warnings: Vec<FidelityWarning>,
}

impl EmitContext {
    fn new() -> Self {
        Self {
            bibliography: Bibliography::new(),
            warnings: Vec::new(),
        }
    }

    /// Insert a finished entry, warning (rather than silently dropping) if
    /// its cite key collides with one already present — BibTeX itself
    /// requires unique keys, so a collision here reflects unrepresentable
    /// source data, not an adapter bug.
    fn insert(&mut self, entry: Entry) {
        let key = entry.key.clone();
        if self.bibliography.insert(entry).is_some() {
            self.warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::UnsupportedNode(BIBTEX_ENTRY.to_string()),
                format!(
                    "Duplicate BibTeX cite key '{key}': an earlier entry with the same key was overwritten"
                ),
            ));
        }
    }
}

/// Build an `EntryType` from a source-provided type name, warning when the
/// name isn't part of `biblatex`'s known vocabulary: `Entry::to_bibtex_string`
/// silently collapses any `EntryType::Unknown(_)` to `misc` (there is no
/// public API to opt out of this), so a custom/unrecognized entry type name
/// is lost on emission. This is a `biblatex`-crate limitation, not a design
/// choice made here.
fn entry_type_for(name: &str, warnings: &mut Vec<FidelityWarning>) -> EntryType {
    let ty = EntryType::new(name);
    if matches!(ty, EntryType::Unknown(_)) {
        warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::UnsupportedNode(format!("bibtex:entry-type={name}")),
            format!(
                "Custom BibTeX entry type '{name}' is not recognized by the biblatex crate and \
                 will be emitted as 'misc' (biblatex::EntryType::Unknown collapses to Misc on write)"
            ),
        ));
    }
    ty
}

/// Set a field to a single plain (non-verbatim) chunk if `value` is
/// non-empty. Using `Chunk::Normal` (rather than `Entry::set_as::<String>`,
/// which produces `Chunk::Verbatim` and gets double-braced by `biblatex`'s
/// writer) matches the brace/escaping style `rescribe-read-bibtex` reads
/// back via `format_verbatim()` either way.
fn set_field(entry: &mut Entry, name: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    entry.set(
        name,
        vec![Spanned::detached(Chunk::Normal(value.to_string()))],
    );
}

/// Reconstruct `prop::DATE`'s `year`/`month`/`day` map (see the property's
/// own doc comment) as a `biblatex::Date` and hand it to `Entry::set_as`,
/// letting the crate serialize it. `Entry::to_bibtex_string` re-derives the
/// `year`/`month`/`day` fields from this internally and, in doing so,
/// unconditionally discards the uncertain/approximate markers (BibTeX has no
/// field-trio equivalent for them and the crate provides no way to opt out
/// of that re-derivation) — a warning is emitted in that case. BibLaTeX
/// output is unaffected: `Entry::to_biblatex_string` keeps a single `date`
/// field with the `?`/`~`/`%` suffix intact.
fn set_date(
    entry: &mut Entry,
    map: &HashMap<String, PropValue>,
    uncertain: bool,
    approximate: bool,
    warnings: &mut Vec<FidelityWarning>,
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

    let datetime = Datetime {
        year: year as i32,
        month: month.map(|m| (m - 1) as u8),
        day: day.map(|d| (d - 1) as u8),
        time: None,
    };
    let date = Date {
        value: DateValue::At(datetime),
        uncertain,
        approximate,
    };
    entry.set_as::<Date>("date", &date);

    if uncertain || approximate {
        warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::UnsupportedNode("bibtex:date-uncertain".to_string()),
            "BibTeX has no year/month/day representation for uncertain/approximate dates; \
             the marker is dropped when biblatex::Entry::to_bibtex_string splits the date into \
             separate fields (BibLaTeX output preserves it)"
                .to_string(),
        ));
    }
}

/// Walk a sequence of nodes, converting recognized entry shapes into
/// `biblatex::Entry` values and inserting them into `ctx.bibliography`.
fn collect_nodes(nodes: &[Node], ctx: &mut EmitContext) {
    for node in nodes {
        collect_node(node, ctx);
    }
}

fn collect_node(node: &Node, ctx: &mut EmitContext) {
    match node.kind.as_str() {
        "document" | node::BIBLIOGRAPHY => collect_nodes(&node.children, ctx),

        node::BIBLIOGRAPHY_ENTRY => {
            let entry = build_bibliography_entry(node, &mut ctx.warnings);
            ctx.insert(entry);
        }

        // Legacy shape, kept for backwards compatibility (see `BIBTEX_ENTRY`
        // doc comment above).
        BIBTEX_ENTRY => {
            let entry = build_legacy_entry(node, &mut ctx.warnings);
            ctx.insert(entry);
        }
        "citation_entry" => {
            let entry = build_citation_entry(node, &mut ctx.warnings);
            ctx.insert(entry);
        }

        _ => {
            if is_bibtex_type(node.kind.as_str()) {
                let entry = build_typed_entry(node, &mut ctx.warnings);
                ctx.insert(entry);
            } else {
                ctx.warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
                    format!("Unknown node type for BibTeX: {}", node.kind.as_str()),
                ));
                collect_nodes(&node.children, ctx);
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

/// Build a `biblatex::Entry` from a `bibliography_entry` node (see
/// `rescribe-read-bibtex`'s `convert_entry`). `bibtex:field` on each
/// `bibliography_field` child (set by every field-producing arm of the
/// reader) names the exact source field; `field:role` is the fallback for a
/// field built by a non-BibTeX producer (a cross-format conversion into
/// BibTeX). Repeated `author`/`editor` fields are rejoined with ` and `; a
/// `page_first`/`page_last` pair recombines into one `pages` field;
/// `prop::DATE` is handled by `set_date` above.
fn build_bibliography_entry(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Entry {
    let entry_type_name = node.props.get_str("bibtex:entry-type").unwrap_or("misc");
    let cite_key = node.props.get_str("bibtex:key").unwrap_or("unknown");

    let mut entry = Entry::new(
        cite_key.to_string(),
        entry_type_for(entry_type_name, warnings),
    );

    if let Some(PropValue::Map(date)) = node.props.get(prop::DATE) {
        set_date(
            &mut entry,
            date,
            node.props
                .get_bool("bibtex:date-uncertain")
                .unwrap_or(false),
            node.props
                .get_bool("bibtex:date-approximate")
                .unwrap_or(false),
            warnings,
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
                set_field(&mut entry, "pages", &format!("{first_text}--{last_text}"));
            }
            _ => {
                let field_name = child.props.get_str("bibtex:field").unwrap_or(role);
                let text = flatten_field_text(child);
                set_field(&mut entry, field_name, &text);
            }
        }
    }
    if !authors.is_empty() {
        set_field(&mut entry, "author", &authors.join(" and "));
    }
    if !editors.is_empty() {
        set_field(&mut entry, "editor", &editors.join(" and "));
    }

    entry
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

/// Build a `biblatex::Entry` from a legacy flat `bibtex:entry` node (see
/// `BIBTEX_ENTRY` doc comment).
fn build_legacy_entry(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Entry {
    let entry_type_name = node.props.get_str("bibtex:type").unwrap_or("misc");
    let cite_key = node.props.get_str("bibtex:key").unwrap_or("unknown");

    let mut entry = Entry::new(
        cite_key.to_string(),
        entry_type_for(entry_type_name, warnings),
    );

    let mut fields: Vec<(&str, String)> = Vec::new();
    for (key, value) in node.props.iter() {
        if let Some(field_name) = key.strip_prefix("bibtex:")
            && field_name != "type"
            && field_name != "key"
            && let rescribe_core::PropValue::String(s) = value
        {
            fields.push((field_name, s.clone()));
        }
    }
    fields.sort_by(|a, b| a.0.cmp(b.0));
    for (name, value) in fields {
        set_field(&mut entry, name, &value);
    }

    entry
}

/// Build a `biblatex::Entry` from a typed entry (where the node kind is the
/// entry type).
fn build_typed_entry(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Entry {
    let entry_type_name = node.kind.as_str();
    let cite_key = node
        .props
        .get_str("key")
        .or(node.props.get_str(prop::ID))
        .unwrap_or("unknown");

    let mut entry = Entry::new(
        cite_key.to_string(),
        entry_type_for(entry_type_name, warnings),
    );

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
            set_field(&mut entry, field_name, value);
        }
    }

    entry
}

/// Build a `biblatex::Entry` from a citation entry with CSL-like properties.
fn build_citation_entry(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Entry {
    let csl_type = node.props.get_str("type").unwrap_or("misc");
    let entry_type_name = csl_to_bibtex_type(csl_type);
    let cite_key = node.props.get_str(prop::ID).unwrap_or("unknown");

    let mut entry = Entry::new(
        cite_key.to_string(),
        entry_type_for(entry_type_name, warnings),
    );

    if let Some(title) = node.props.get_str("title") {
        set_field(&mut entry, "title", title);
    }
    if let Some(author) = node.props.get_str("author") {
        set_field(&mut entry, "author", author);
    }
    if let Some(container) = node.props.get_str("container-title") {
        if csl_type == "article-journal" {
            set_field(&mut entry, "journal", container);
        } else {
            set_field(&mut entry, "booktitle", container);
        }
    }
    if let Some(issued) = node.props.get_str("issued") {
        let year_str = issued.split('-').next().unwrap_or(issued);
        set_field(&mut entry, "year", year_str);
    }

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
            set_field(&mut entry, bibtex_name, value);
        }
    }

    entry
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
        assert!(output.contains("month = {03},"));
    }

    #[test]
    fn test_emit_date_uncertain_warns() {
        let mut map = HashMap::new();
        map.insert("year".to_string(), PropValue::Int(2020));
        let entry = Node::new(NodeKind::from(node::BIBLIOGRAPHY_ENTRY))
            .prop("bibtex:entry-type", "article")
            .prop("bibtex:key", "x")
            .prop(prop::DATE, PropValue::Map(map))
            .prop("bibtex:date-uncertain", true);
        let doc = Document::new()
            .with_content(Node::new(NodeKind::from("document")).children(vec![entry]));
        let result = emit(&doc).unwrap();
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.message.contains("uncertain"))
        );
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

    #[test]
    fn test_unknown_entry_type_warns_and_falls_back_to_misc() {
        let entry = make_entry("frobnicate", "x", vec![make_field("title", "title", "T")]);
        let doc = Document::new()
            .with_content(Node::new(NodeKind::from("document")).children(vec![entry]));
        let result = emit(&doc).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("@misc{x,"));
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.message.contains("frobnicate"))
        );
    }
}
