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
use std::collections::HashMap;

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
    let output = render_bibliography(&ctx);
    Ok(ConversionResult::with_warnings(output, ctx.warnings))
}

/// Per-entry raw-preservation splice: the entry's original type name, when
/// `EntryType::new()` didn't recognize it (`EntryType::Unknown(_)`).
/// `Entry::to_biblatex_string()` collapses `Unknown` to `misc` internally,
/// with no public API to opt out — see `render_bibliography`, which splices
/// the original name back into the entry's own rendered output.
#[derive(Default)]
struct EntrySplice {
    raw_entry_type: Option<String>,
}

struct EmitContext {
    bibliography: Bibliography,
    /// Splice instructions keyed by cite key, applied in `render_bibliography`.
    splices: HashMap<String, EntrySplice>,
    warnings: Vec<FidelityWarning>,
}

impl EmitContext {
    fn new() -> Self {
        Self {
            bibliography: Bibliography::new(),
            splices: HashMap::new(),
            warnings: Vec::new(),
        }
    }

    /// Insert a finished entry (with its raw-preservation splice info),
    /// warning (rather than silently dropping) if its cite key collides with
    /// one already present — BibLaTeX itself requires unique keys, so a
    /// collision here reflects unrepresentable source data, not an adapter
    /// bug.
    fn insert(&mut self, entry: Entry, splice: EntrySplice) {
        let key = entry.key.clone();
        self.splices.insert(key.clone(), splice);
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

/// Render every entry in `ctx.bibliography` to BibLaTeX and apply each
/// entry's raw-preservation splice, joining with the exact same
/// blank-line-between, trailing-newline convention
/// `biblatex::Bibliography::write_biblatex` uses (verified against
/// `biblatex` 0.11's source: no separator before the first entry, `"\n\n"`
/// between entries, one trailing `"\n"`) — so output is byte-identical to
/// the un-spliced path when no splice applies. Per-entry serialization
/// itself is still entirely `Entry::to_biblatex_string()`; this function
/// only joins strings and substitutes the entry-type token in the entry's
/// own output, it does not build BibLaTeX syntax.
fn render_bibliography(ctx: &EmitContext) -> Vec<u8> {
    let mut parts = Vec::new();
    for entry in ctx.bibliography.iter() {
        let mut text = entry.to_biblatex_string();
        if let Some(splice) = ctx.splices.get(&entry.key)
            && let Some(raw_type) = &splice.raw_entry_type
        {
            text = splice_entry_type(&text, raw_type);
        }
        parts.push(text);
    }
    let mut out = parts.join("\n\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out.into_bytes()
}

/// Replace a `@misc{` entry header with `@{raw_type}{`. Only ever called when
/// `raw_type` was the entry's real source type and `EntryType::new(raw_type)`
/// produced `Unknown` (so `Entry::to_biblatex_string()` is guaranteed to have
/// emitted `@misc{` — see `EntryType::to_biblatex`), and `raw_type` itself
/// can never literally be `"misc"` (that string parses to the known `Misc`
/// variant, not `Unknown`), so this never fires as a spurious no-op rename.
fn splice_entry_type(entry_text: &str, raw_type: &str) -> String {
    match entry_text.strip_prefix("@misc{") {
        Some(rest) => format!("@{raw_type}{{{rest}"),
        None => entry_text.to_string(),
    }
}

/// Build an `EntryType` from a source-provided type name. When the name
/// isn't part of `biblatex`'s known vocabulary (`EntryType::Unknown`),
/// `Entry::to_biblatex_string` would otherwise silently collapse it to
/// `misc` (there is no public API to opt out) — the caller splices the
/// original name back into the rendered output instead (see
/// `splice_entry_type`), so this no longer needs to warn.
fn entry_type_for(name: &str) -> EntryType {
    EntryType::new(name)
}

/// The original entry type name, if `ty` came back `Unknown` — i.e. the
/// value `splice_entry_type` needs, or `None` if `ty` is one `biblatex`
/// recognizes and will round-trip on its own.
fn raw_entry_type_if_unknown(ty: &EntryType, name: &str) -> Option<String> {
    matches!(ty, EntryType::Unknown(_)).then(|| name.to_string())
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
            let (entry, splice) = build_biblatex_entry(node);
            ctx.insert(entry, splice);
        }
        BIBTEX_ENTRY => {
            let (entry, splice) = build_bibtex_entry(node);
            ctx.insert(entry, splice);
        }
        "citation_entry" => {
            let (entry, splice) = build_citation_entry(node);
            ctx.insert(entry, splice);
        }
        _ => {
            if is_biblatex_type(node.kind.as_str()) {
                let (entry, splice) = build_typed_entry(node);
                ctx.insert(entry, splice);
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

fn build_biblatex_entry(node: &Node) -> (Entry, EntrySplice) {
    let entry_type_name = node.props.get_str("biblatex:type").unwrap_or("misc");
    let cite_key = node.props.get_str("biblatex:key").unwrap_or("unknown");

    let ty = entry_type_for(entry_type_name);
    let splice = EntrySplice {
        raw_entry_type: raw_entry_type_if_unknown(&ty, entry_type_name),
    };
    let mut entry = Entry::new(cite_key.to_string(), ty);

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

    (entry, splice)
}

fn build_bibtex_entry(node: &Node) -> (Entry, EntrySplice) {
    let entry_type_name = node.props.get_str("bibtex:type").unwrap_or("misc");
    let cite_key = node.props.get_str("bibtex:key").unwrap_or("unknown");

    let ty = entry_type_for(entry_type_name);
    let splice = EntrySplice {
        raw_entry_type: raw_entry_type_if_unknown(&ty, entry_type_name),
    };
    let mut entry = Entry::new(cite_key.to_string(), ty);

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

    (entry, splice)
}

fn build_citation_entry(node: &Node) -> (Entry, EntrySplice) {
    let csl_type = node.props.get_str("type").unwrap_or("misc");
    let entry_type_name = csl_to_biblatex_type(csl_type);
    let cite_key = node.props.get_str(prop::ID).unwrap_or("unknown");

    let ty = entry_type_for(entry_type_name);
    let splice = EntrySplice {
        raw_entry_type: raw_entry_type_if_unknown(&ty, entry_type_name),
    };
    let mut entry = Entry::new(cite_key.to_string(), ty);

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

    (entry, splice)
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

fn build_typed_entry(node: &Node) -> (Entry, EntrySplice) {
    let entry_type_name = node.kind.as_str();
    let cite_key = node
        .props
        .get_str("key")
        .or(node.props.get_str(prop::ID))
        .unwrap_or("unknown");

    let ty = entry_type_for(entry_type_name);
    let splice = EntrySplice {
        raw_entry_type: raw_entry_type_if_unknown(&ty, entry_type_name),
    };
    let mut entry = Entry::new(cite_key.to_string(), ty);

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

    (entry, splice)
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

    /// Custom/unrecognized entry types now round-trip: `biblatex::EntryType::new`
    /// falls back to `Unknown(name)`, which `Entry::to_biblatex_string()`
    /// would otherwise collapse to `misc`; `render_bibliography` splices the
    /// original name back over the emitted `@misc{` header, so no fidelity
    /// warning fires.
    #[test]
    fn test_unknown_entry_type_round_trips_no_warning() {
        let entry = Node::new(NodeKind::from(BIBLATEX_ENTRY))
            .prop("biblatex:type", "frobnicate")
            .prop("biblatex:key", "x")
            .prop("biblatex:title", "T");

        let doc = Document::new().with_content(Node::new(NodeKind::from("document")).child(entry));
        let result = emit(&doc).unwrap();
        let output = String::from_utf8(result.value).unwrap();
        assert!(output.contains("@frobnicate{x,"), "output was:\n{output}");
        assert!(!output.contains("@misc{x,"), "output was:\n{output}");
        assert!(
            result.warnings.is_empty(),
            "warnings: {:?}",
            result.warnings
        );
    }

    /// A recognized standard type (`misc` itself) never gets spliced.
    #[test]
    fn test_misc_entry_type_not_spliced() {
        let entry = Node::new(NodeKind::from(BIBLATEX_ENTRY))
            .prop("biblatex:type", "misc")
            .prop("biblatex:key", "x")
            .prop("biblatex:title", "T");
        let doc = Document::new().with_content(Node::new(NodeKind::from("document")).child(entry));
        let output = emit_str(&doc);
        assert!(output.contains("@misc{x,"), "output was:\n{output}");
    }

    /// Two entries, only one with a custom type: proves the entry-type
    /// splice is keyed per cite key and doesn't touch other entries'
    /// `@misc{` headers.
    #[test]
    fn test_unknown_entry_type_does_not_bleed_into_other_entries() {
        let entry1 = Node::new(NodeKind::from(BIBLATEX_ENTRY))
            .prop("biblatex:type", "frobnicate")
            .prop("biblatex:key", "custom_one");
        let entry2 = Node::new(NodeKind::from(BIBLATEX_ENTRY))
            .prop("biblatex:type", "misc")
            .prop("biblatex:key", "real_misc_two");
        let doc = Document::new()
            .with_content(Node::new(NodeKind::from("document")).children(vec![entry1, entry2]));
        let output = emit_str(&doc);
        assert!(
            output.contains("@frobnicate{custom_one,"),
            "output was:\n{output}"
        );
        assert!(
            output.contains("@misc{real_misc_two,"),
            "output was:\n{output}"
        );
    }
}
