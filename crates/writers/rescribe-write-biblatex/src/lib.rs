//! BibLaTeX writer for rescribe.
//!
//! Emits documents as BibLaTeX source with BibLaTeX-specific fields
//! (date, journaltitle, subtitle, etc.).
//!
//! Actual BibLaTeX syntax (entry headers, field escaping, brace wrapping) is
//! produced by the `biblatex` crate's own `Entry::to_biblatex_string()` /
//! `Bibliography::to_biblatex_string()` (the same crate `rescribe-read-biblatex`
//! and `rescribe-read-bibtex` use to parse). This adapter's only job is
//! building a `biblatex::Entry` from the rescribe IR shapes it accepts — it
//! does not hand-roll escaping or field/entry syntax itself.

use biblatex::{Bibliography, Chunk, Entry, EntryType, Spanned};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, FidelityWarning, Node, Severity,
    WarningKind,
};
use rescribe_std::prop;

/// BibLaTeX entry node type.
const BIBLATEX_ENTRY: &str = "biblatex:entry";
const BIBTEX_ENTRY: &str = "bibtex:entry";

/// Emit a document as BibLaTeX.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document as BibLaTeX with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let mut ctx = EmitContext::new();
    collect_nodes(&doc.content.children, &mut ctx);
    let output = ctx.bibliography.to_biblatex_string();
    Ok(ConversionResult::with_warnings(
        output.into_bytes(),
        ctx.warnings,
    ))
}

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
    /// its cite key collides with one already present — BibLaTeX itself
    /// requires unique keys, so a collision here reflects unrepresentable
    /// source data, not an adapter bug.
    fn insert(&mut self, entry: Entry) {
        let key = entry.key.clone();
        if self.bibliography.insert(entry).is_some() {
            self.warnings.push(FidelityWarning::new(
                Severity::Minor,
                WarningKind::UnsupportedNode(BIBLATEX_ENTRY.to_string()),
                format!(
                    "Duplicate BibLaTeX cite key '{key}': an earlier entry with the same key was overwritten"
                ),
            ));
        }
    }
}

/// Build an `EntryType` from a source-provided type name, warning when the
/// name isn't part of `biblatex`'s known vocabulary: both
/// `Entry::to_biblatex_string` and `Entry::to_bibtex_string` silently
/// collapse any `EntryType::Unknown(_)` to `misc` (there is no public API to
/// opt out of this), so a custom/unrecognized entry type name is lost on
/// emission. This is a `biblatex`-crate limitation, not a design choice made
/// here.
fn entry_type_for(name: &str, warnings: &mut Vec<FidelityWarning>) -> EntryType {
    let ty = EntryType::new(name);
    if matches!(ty, EntryType::Unknown(_)) {
        warnings.push(FidelityWarning::new(
            Severity::Minor,
            WarningKind::UnsupportedNode(format!("biblatex:type={name}")),
            format!(
                "Custom BibLaTeX entry type '{name}' is not recognized by the biblatex crate and \
                 will be emitted as 'misc' (biblatex::EntryType::Unknown collapses to Misc on write)"
            ),
        ));
    }
    ty
}

/// Set a field to a single plain (non-verbatim) chunk if `value` is
/// non-empty. Using `Chunk::Normal` (rather than `Entry::set_as::<String>`,
/// which produces `Chunk::Verbatim` and gets double-braced by `biblatex`'s
/// writer) matches the single-brace style both readers read back via
/// `format_verbatim()` either way.
fn set_field(entry: &mut Entry, name: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    entry.set(
        name,
        vec![Spanned::detached(Chunk::Normal(value.to_string()))],
    );
}

fn collect_nodes(nodes: &[Node], ctx: &mut EmitContext) {
    for node in nodes {
        collect_node(node, ctx);
    }
}

fn collect_node(node: &Node, ctx: &mut EmitContext) {
    match node.kind.as_str() {
        "document" | "definition_list" => collect_nodes(&node.children, ctx),
        BIBLATEX_ENTRY => {
            let entry = build_biblatex_entry(node, &mut ctx.warnings);
            ctx.insert(entry);
        }
        BIBTEX_ENTRY => {
            let entry = build_bibtex_entry(node, &mut ctx.warnings);
            ctx.insert(entry);
        }
        "citation_entry" => {
            let entry = build_citation_entry(node, &mut ctx.warnings);
            ctx.insert(entry);
        }
        _ => {
            if is_biblatex_type(node.kind.as_str()) {
                let entry = build_typed_entry(node, &mut ctx.warnings);
                ctx.insert(entry);
            } else {
                ctx.warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::UnsupportedNode(node.kind.as_str().to_string()),
                    format!("Unknown node type for BibLaTeX: {}", node.kind.as_str()),
                ));
                collect_nodes(&node.children, ctx);
            }
        }
    }
}

fn is_biblatex_type(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "article"
            | "book"
            | "mvbook"
            | "inbook"
            | "bookinbook"
            | "suppbook"
            | "booklet"
            | "collection"
            | "mvcollection"
            | "incollection"
            | "suppcollection"
            | "manual"
            | "misc"
            | "online"
            | "patent"
            | "periodical"
            | "suppperiodical"
            | "proceedings"
            | "mvproceedings"
            | "inproceedings"
            | "reference"
            | "mvreference"
            | "inreference"
            | "report"
            | "set"
            | "software"
            | "thesis"
            | "unpublished"
            | "xdata"
            | "dataset"
    )
}

fn build_biblatex_entry(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Entry {
    let entry_type_name = node.props.get_str("biblatex:type").unwrap_or("misc");
    let cite_key = node.props.get_str("biblatex:key").unwrap_or("unknown");

    let mut entry = Entry::new(
        cite_key.to_string(),
        entry_type_for(entry_type_name, warnings),
    );

    let mut fields: Vec<(&str, String)> = Vec::new();
    for (key, value) in node.props.iter() {
        if let Some(field_name) = key.strip_prefix("biblatex:")
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

fn build_bibtex_entry(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Entry {
    let entry_type_name = node.props.get_str("bibtex:type").unwrap_or("misc");
    let cite_key = node.props.get_str("bibtex:key").unwrap_or("unknown");

    let mut entry = Entry::new(
        cite_key.to_string(),
        entry_type_for(entry_type_name, warnings),
    );

    // BibLaTeX field mappings from BibTeX.
    let field_mappings = [
        ("bibtex:journal", "journaltitle"),
        ("bibtex:year", "date"),
        ("bibtex:address", "location"),
    ];
    for (bibtex_field, biblatex_field) in field_mappings {
        if let Some(value) = node.props.get_str(bibtex_field) {
            set_field(&mut entry, biblatex_field, value);
        }
    }

    // Direct mappings (same field name in both).
    for (key, value) in node.props.iter() {
        if let Some(field_name) = key.strip_prefix("bibtex:")
            && field_name != "type"
            && field_name != "key"
            && field_name != "journal"
            && field_name != "year"
            && field_name != "address"
            && let rescribe_core::PropValue::String(s) = value
        {
            set_field(&mut entry, field_name, s);
        }
    }

    entry
}

fn build_citation_entry(node: &Node, warnings: &mut Vec<FidelityWarning>) -> Entry {
    let csl_type = node.props.get_str("type").unwrap_or("misc");
    let entry_type_name = csl_to_biblatex_type(csl_type);
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
            set_field(&mut entry, "journaltitle", container);
        } else {
            set_field(&mut entry, "booktitle", container);
        }
    }
    if let Some(issued) = node.props.get_str("issued") {
        set_field(&mut entry, "date", issued);
    }

    let direct_mappings = [
        ("volume", "volume"),
        ("issue", "number"),
        ("page", "pages"),
        ("publisher", "publisher"),
        ("publisher-place", "location"),
        ("DOI", "doi"),
        ("URL", "url"),
        ("ISBN", "isbn"),
        ("ISSN", "issn"),
        ("abstract", "abstract"),
        ("note", "note"),
    ];
    for (csl_name, biblatex_name) in direct_mappings {
        if let Some(value) = node.props.get_str(csl_name) {
            set_field(&mut entry, biblatex_name, value);
        }
    }

    entry
}

fn csl_to_biblatex_type(csl: &str) -> &'static str {
    match csl {
        "article-journal" | "article-magazine" | "article-newspaper" => "article",
        "book" => "book",
        "chapter" => "incollection",
        "paper-conference" => "inproceedings",
        "thesis" => "thesis",
        "report" => "report",
        "webpage" | "post-weblog" => "online",
        "software" => "software",
        "dataset" => "dataset",
        "patent" => "patent",
        _ => "misc",
    }
}

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

    // BibLaTeX standard fields.
    let field_mappings = [
        ("author", "author"),
        ("title", "title"),
        ("subtitle", "subtitle"),
        ("journaltitle", "journaltitle"),
        ("journal", "journaltitle"), // Map BibTeX journal to BibLaTeX journaltitle
        ("booktitle", "booktitle"),
        ("maintitle", "maintitle"),
        ("date", "date"),
        ("year", "date"), // Map year to date
        ("volume", "volume"),
        ("number", "number"),
        ("pages", "pages"),
        ("publisher", "publisher"),
        ("location", "location"),
        ("address", "location"), // Map BibTeX address to BibLaTeX location
        ("edition", "edition"),
        ("editor", "editor"),
        ("series", "series"),
        ("note", "note"),
        ("doi", "doi"),
        ("eprint", "eprint"),
        ("eprinttype", "eprinttype"),
        ("url", "url"),
        ("urldate", "urldate"),
        ("isbn", "isbn"),
        ("issn", "issn"),
        ("abstract", "abstract"),
        ("keywords", "keywords"),
        ("institution", "institution"),
    ];

    let mut emitted = std::collections::HashSet::new();
    for (prop_name, field_name) in field_mappings {
        if !emitted.contains(field_name)
            && let Some(value) = node.props.get_str(prop_name)
        {
            set_field(&mut entry, field_name, value);
            emitted.insert(field_name);
        }
    }

    entry
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_core::{Document, NodeKind};

    fn emit_str(doc: &Document) -> String {
        String::from_utf8(emit(doc).unwrap().value).unwrap()
    }

    #[test]
    fn test_emit_article() {
        let entry = Node::new(NodeKind::from("article"))
            .prop("key", "smith2024")
            .prop("author", "John Smith")
            .prop("title", "A Great Paper")
            .prop("journaltitle", "Nature")
            .prop("date", "2024-05-15");

        let doc = Document::new().with_content(Node::new(NodeKind::from("document")).child(entry));
        let output = emit_str(&doc);

        assert!(output.contains("@article{smith2024,"));
        assert!(output.contains("author = {John Smith},"));
        assert!(output.contains("journaltitle = {Nature},"));
        assert!(output.contains("date = {2024-05-15},"));
    }

    #[test]
    fn test_emit_online() {
        let entry = Node::new(NodeKind::from("online"))
            .prop("key", "website2024")
            .prop("author", "Jane Doe")
            .prop("title", "A Great Website")
            .prop("url", "https://example.com")
            .prop("urldate", "2024-01-15");

        let doc = Document::new().with_content(Node::new(NodeKind::from("document")).child(entry));
        let output = emit_str(&doc);

        assert!(output.contains("@online{website2024,"));
        assert!(output.contains("url = {https://example.com},"));
    }

    #[test]
    fn test_emit_with_subtitle() {
        let entry = Node::new(NodeKind::from("book"))
            .prop("key", "knuth1984")
            .prop("author", "Donald E. Knuth")
            .prop("title", "The TeXbook")
            .prop("subtitle", "A Complete Guide to TeX")
            .prop("publisher", "Addison-Wesley")
            .prop("date", "1984");

        let doc = Document::new().with_content(Node::new(NodeKind::from("document")).child(entry));
        let output = emit_str(&doc);

        assert!(output.contains("@book{knuth1984,"));
        assert!(output.contains("subtitle = {A Complete Guide to TeX},"));
    }

    #[test]
    fn test_year_to_date() {
        let entry = Node::new(NodeKind::from("article"))
            .prop("key", "test")
            .prop("year", "2024");

        let doc = Document::new().with_content(Node::new(NodeKind::from("document")).child(entry));
        let output = emit_str(&doc);

        // BibLaTeX should use date field
        assert!(output.contains("date = {2024},"));
    }

    #[test]
    fn test_unknown_entry_type_warns_and_falls_back_to_misc() {
        let entry = Node::new(NodeKind::from(BIBLATEX_ENTRY))
            .prop("biblatex:type", "frobnicate")
            .prop("biblatex:key", "x")
            .prop("biblatex:title", "T");

        let doc = Document::new().with_content(Node::new(NodeKind::from("document")).child(entry));
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
