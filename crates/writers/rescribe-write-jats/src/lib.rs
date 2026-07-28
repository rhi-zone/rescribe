//! JATS XML writer for rescribe.
//!
//! Translates rescribe's document IR into `jats_fmt::JatsDoc` (the
//! standalone JATS/XML AST from the `jats-fmt` crate) and serializes it via
//! `jats_fmt::emit`. All XML writing lives in `jats-fmt` — this crate is a
//! thin IR↔AST translator only (per CLAUDE.md's "adapter layer must never
//! contain parsing or writing logic" rule).
//!
//! # Example
//!
//! ```
//! use rescribe_write_jats::emit;
//! use rescribe_core::{Document, Node, Properties};
//!
//! let doc = Document {
//!     content: Node::new("document"),
//!     resources: Default::default(),
//!     metadata: Properties::new(),
//!     source: None,
//! };
//!
//! let result = emit(&doc).unwrap();
//! let xml = String::from_utf8(result.value).unwrap();
//! ```

use std::collections::HashMap;

use jats_fmt::{JatsDoc, Node as JNode, XmlDecl};
use rescribe_core::{ConversionResult, Document, EmitError, Node, PropValue};
use rescribe_std::{node, prop};

/// Emit a document to JATS XML.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let warnings = Vec::new();

    let mut root_children = Vec::new();
    // Any metadata key ending in `_raw` is a whole-subtree verbatim capture
    // of an unmodeled `<article-meta>`/`<journal-meta>` front-matter
    // element (see `rescribe-read-jats`'s `convert_children`/
    // `extract_metadata` — `{tag}_raw`, e.g. `contrib-group_raw` or
    // `pub-date_raw`). Collected once here so both the "do we even need an
    // `<article-meta>`" check and the splice-back loop below share one
    // scan.
    let mut meta_raw_fields: Vec<(&str, &str)> = doc
        .metadata
        .iter()
        .filter_map(|(key, _)| {
            let tag = key.strip_suffix("_raw")?;
            Some((tag, doc.metadata.get_str(key)?))
        })
        .collect();
    meta_raw_fields.sort_unstable_by_key(|(tag, _)| *tag);
    let title = doc.metadata.get_str("title");
    let subtitle = doc.metadata.get_str("subtitle");
    if title.is_some() || subtitle.is_some() || !meta_raw_fields.is_empty() {
        let mut article_meta_children = Vec::new();
        if title.is_some() || subtitle.is_some() {
            let mut title_group_children = Vec::new();
            if let Some(title) = title {
                title_group_children.push(jats_element(
                    "article-title",
                    vec![],
                    vec![jats_text(title)],
                ));
            }
            if let Some(subtitle) = subtitle {
                title_group_children.push(jats_element(
                    "subtitle",
                    vec![],
                    vec![jats_text(subtitle)],
                ));
            }
            article_meta_children.push(jats_element("title-group", vec![], title_group_children));
        }
        // Splice back every raw-captured `<article-meta>`/`<journal-meta>`
        // subtree byte-for-byte (see `rescribe-read-jats`'s
        // `convert_children`/`extract_metadata` — any `{tag}_raw` metadata
        // field, e.g. `contrib-group_raw` or `pub-date_raw`). This is
        // lossless where reconstructing the element from its flattened
        // text would not be; sorted by tag for deterministic output since
        // `Properties` iterates in unspecified order.
        for (_, raw) in &meta_raw_fields {
            article_meta_children.push(JNode::Raw {
                content: (*raw).to_string(),
                span: jats_fmt::Span::NONE,
            });
        }
        root_children.push(jats_element(
            "front",
            vec![],
            vec![jats_element("article-meta", vec![], article_meta_children)],
        ));
    }

    let mut body_children = Vec::new();
    for child in &doc.content.children {
        body_children.extend(write_node(child));
    }
    root_children.push(jats_element("body", vec![], body_children));

    let root = JNode::Element {
        name: "article".to_string(),
        attrs: vec![
            (
                "xmlns:xlink".to_string(),
                "http://www.w3.org/1999/xlink".to_string(),
            ),
            ("article-type".to_string(), "research-article".to_string()),
        ],
        children: root_children,
        span: jats_fmt::Span::NONE,
    };

    let doc_ast = JatsDoc {
        xml_decl: Some(XmlDecl {
            version: "1.0".to_string(),
            encoding: Some("UTF-8".to_string()),
            standalone: None,
        }),
        nodes: vec![root],
    };

    let bytes = jats_fmt::emit(&doc_ast);
    Ok(ConversionResult::with_warnings(bytes, warnings))
}

fn jats_element(name: &str, attrs: Vec<(String, String)>, children: Vec<JNode>) -> JNode {
    JNode::Element {
        name: name.to_string(),
        attrs,
        children,
        span: jats_fmt::Span::NONE,
    }
}

fn jats_text(content: impl Into<String>) -> JNode {
    JNode::Text {
        content: content.into(),
        span: jats_fmt::Span::NONE,
    }
}

/// Build a `<disp-formula>`/`<inline-formula>`'s children from a
/// `math_display`/`math_inline` node: an optional `<label>` (see
/// `rescribe-read-jats`'s `split_label`) followed by either the verbatim
/// `<mml:math>` subtree (when `math:format == "mathml"`, captured by
/// `rescribe-read-jats`'s `mml-math-raw` sentinel — re-emitted byte-for-byte
/// via `JNode::Raw`, the same splicing mechanism used for raw-preserved
/// `<article-meta>`/`<journal-meta>` header content above) or a `<tex-math>`
/// wrapping the plain-text source (the pre-existing behavior, unchanged).
fn formula_children(node: &Node) -> Vec<JNode> {
    let mut children = Vec::new();
    if let Some(label) = node.props.get_str(prop::LABEL) {
        children.push(jats_element("label", vec![], vec![jats_text(label)]));
    }
    if let Some(source) = node.props.get_str("math:source") {
        if node.props.get_str("math:format") == Some("mathml") {
            children.push(JNode::Raw {
                content: source.to_string(),
                span: jats_fmt::Span::NONE,
            });
        } else {
            children.push(jats_element("tex-math", vec![], vec![jats_text(source)]));
        }
    }
    children
}

/// Build the generic `id`/`xml:lang` attributes for a node, if the IR node
/// carries the corresponding raw-preserved property (see
/// `rescribe-read-jats`'s `attach_generic_attrs`, applied to *every*
/// converted element on read — this is its writer-side counterpart, called
/// from every `jats_element(...)` build site below, not just the generic
/// span/div fallback).
fn generic_attrs(node: &Node) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    if let Some(id) = node.props.get_str(prop::ID) {
        attrs.push(("id".to_string(), id.to_string()));
    }
    if let Some(lang) = node.props.get_str(prop::LANGUAGE) {
        attrs.push(("xml:lang".to_string(), lang.to_string()));
    }
    attrs
}

/// Every extra attribute a `generic_span`/`generic_div` (unrecognized
/// element) captured on read via `jats:attr:{name}` properties — the
/// writer-side counterpart of `rescribe-read-jats`'s `attach_all_attrs`.
/// Only meaningful on `SPAN`/`DIV` nodes carrying a `jats:tag` prop; other
/// node kinds simply have no such properties and this returns empty.
fn generic_extra_attrs(node: &Node) -> Vec<(String, String)> {
    node.props
        .iter()
        .filter_map(|(key, value)| {
            let name = key.strip_prefix("jats:attr:")?;
            match value {
                PropValue::String(s) => Some((name.to_string(), s.clone())),
                _ => None,
            }
        })
        .collect()
}

/// Build `colspan`/`rowspan` attributes for a `<th>`/`<td>` from the
/// standard cross-format `colspan`/`rowspan` properties (see
/// `rescribe-read-jats`'s `with_cell_span` for the reader side).
fn cell_span_attrs(node: &Node) -> Vec<(String, String)> {
    let mut attrs = generic_attrs(node);
    if let Some(n) = node.props.get_int(prop::COLSPAN)
        && n > 1
    {
        attrs.push(("colspan".to_string(), n.to_string()));
    }
    if let Some(n) = node.props.get_int(prop::ROWSPAN)
        && n > 1
    {
        attrs.push(("rowspan".to_string(), n.to_string()));
    }
    attrs
}

/// Build a bare `<table>...</table>` element (no wrapping `<table-wrap>`)
/// from a `TABLE` node. Shared by the `TABLE` write arm (which wraps the
/// result in a synthesized `<table-wrap>`) and the `table-wrap`-tagged
/// `FIGURE` arm (which supplies its own, already-present `<table-wrap>` and
/// must not wrap twice).
fn table_element(node: &Node) -> JNode {
    let has_structure = node
        .children
        .iter()
        .any(|c| c.kind.as_str() == node::TABLE_HEAD || c.kind.as_str() == node::TABLE_BODY);
    let table_children: Vec<JNode> = if has_structure {
        node.children.iter().flat_map(write_node).collect()
    } else {
        vec![jats_element(
            "tbody",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )]
    };
    jats_element("table", generic_attrs(node), table_children)
}

/// Convert one rescribe IR (block-level) node into zero or more JATS AST
/// nodes.
fn write_node(node: &Node) -> Vec<JNode> {
    match node.kind.as_str() {
        node::DOCUMENT => node.children.iter().flat_map(write_node).collect(),

        // A `div` tagged `jats:tag` is a `generic_div` (an unrecognized
        // block-level element raw-preserved by the reader's catch-all, see
        // `rescribe-read-jats::generic_div`) — re-emit its original tag
        // name. Any other `div` (article/sec/abstract/etc, which the
        // reader unwraps regardless of the writer re-wrapping them) just
        // flattens into its children, same as `DOCUMENT`.
        node::DIV => match node.props.get_str("jats:tag") {
            Some(tag) => {
                let mut attrs = generic_attrs(node);
                attrs.extend(generic_extra_attrs(node));
                vec![jats_element(
                    tag,
                    attrs,
                    node.children.iter().flat_map(write_node).collect(),
                )]
            }
            None => node.children.iter().flat_map(write_node).collect(),
        },

        // A `bibliography` (`<ref-list>`) whose own title (see
        // `rescribe-read-jats`'s `"title"` arm — a `<ref-list>`'s own
        // `<title>` converts to an ordinary `HEADING` like any other
        // block's title, since `<ref-list>` isn't in `heading_level_for_parent`'s
        // dedicated top-level-division list) must re-emit that title as a
        // bare `<title>`, *not* the generic `HEADING` arm's `<sec><title>`
        // wrapping below — `<ref-list>`'s content model is `(title?,
        // (ref | ref-list)+)`, which has no room for a nested `<sec>`.
        node::BIBLIOGRAPHY => {
            let mut kids = Vec::with_capacity(node.children.len());
            for child in &node.children {
                if child.kind.as_str() == node::HEADING {
                    kids.push(jats_element(
                        "title",
                        vec![],
                        child.children.iter().flat_map(write_inline).collect(),
                    ));
                } else {
                    kids.extend(write_node(child));
                }
            }
            vec![jats_element("ref-list", generic_attrs(node), kids)]
        }

        node::BIBLIOGRAPHY_ENTRY => vec![write_bibliography_entry(node)],

        // A `bibliography_field` shouldn't normally appear outside a
        // `BIBLIOGRAPHY_ENTRY`'s own children (handled directly by
        // `write_bibliography_entry`, not through this dispatch), but
        // delegate to the same field writer defensively rather than falling
        // to the generic "recurse into children" catch-all below, which
        // would drop the field's role/tag entirely.
        node::BIBLIOGRAPHY_FIELD => vec![write_bibliography_field(node)],

        node::HEADING => vec![jats_element(
            "sec",
            vec![],
            vec![jats_element(
                "title",
                generic_attrs(node),
                node.children.iter().flat_map(write_inline).collect(),
            )],
        )],

        node::PARAGRAPH => vec![jats_element(
            "p",
            generic_attrs(node),
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::BLOCKQUOTE => vec![jats_element(
            "disp-quote",
            generic_attrs(node),
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::LIST => {
            // `jats:list-type` (see `rescribe-read-jats`) round-trips the
            // exact original `list-type` value (`alpha-lower`, `roman-upper`,
            // ...); only fall back to the lossy ordered/unordered ->
            // `order`/`bullet` derivation when the source never had one
            // (e.g. a list built programmatically, not read from JATS).
            let list_type = match node.props.get_str("jats:list-type") {
                Some(lt) => lt.to_string(),
                None => {
                    let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                    (if ordered { "order" } else { "bullet" }).to_string()
                }
            };
            let mut attrs = vec![("list-type".to_string(), list_type)];
            if let Some(cf) = node.props.get_str("jats:continued-from") {
                attrs.push(("continued-from".to_string(), cf.to_string()));
            }
            attrs.extend(generic_attrs(node));
            vec![jats_element(
                "list",
                attrs,
                node.children.iter().flat_map(write_node).collect(),
            )]
        }

        node::LIST_ITEM => vec![jats_element(
            "list-item",
            generic_attrs(node),
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::DEFINITION_LIST => {
            let mut entries = Vec::new();
            let mut i = 0;
            while i < node.children.len() {
                let mut entry_children = write_node(&node.children[i]);
                if i + 1 < node.children.len() {
                    entry_children.extend(write_node(&node.children[i + 1]));
                }
                entries.push(jats_element("def-item", vec![], entry_children));
                i += 2;
            }
            vec![jats_element("def-list", generic_attrs(node), entries)]
        }

        node::DEFINITION_TERM => vec![jats_element(
            "term",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::DEFINITION_DESC => vec![jats_element(
            "def",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::CODE_BLOCK => {
            // `jats:tag` (see `rescribe-read-jats`) round-trips whether the
            // source was `<code>` or `<preformat>` — default to `<code>` for
            // IR trees not built from a JATS `<code>`/`<preformat>` read.
            let tag = node.props.get_str("jats:tag").unwrap_or("code");
            let mut attrs = Vec::new();
            // NOTE: `prop::LANGUAGE` is deliberately *not* routed through
            // `generic_attrs` (-> `xml:lang`) here — on `CODE_BLOCK` it
            // means the code's programming language (`content-type`), the
            // same property reused for a different cross-format meaning
            // than the natural-language `xml:lang` `generic_attrs` assumes
            // everywhere else. Only `id` is pulled in generically.
            if let Some(lang) = node.props.get_str(prop::LANGUAGE) {
                attrs.push(("content-type".to_string(), lang.to_string()));
            }
            if let Some(id) = node.props.get_str(prop::ID) {
                attrs.push(("id".to_string(), id.to_string()));
            }
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            vec![jats_element(tag, attrs, vec![jats_text(content)])]
        }

        // A bare `TABLE` (not already nested inside a `FIGURE` tagged
        // `table-wrap` — see that arm above) is standalone IR content with
        // no wrapping `<table-wrap>` of its own; JATS requires `<table>` to
        // sit inside a `<table-wrap>`, so one is synthesized here. A `TABLE`
        // that *is* a `table-wrap`-tagged `FIGURE`'s child is written via
        // `table_element` directly instead (by that arm), bypassing this one
        // entirely, so the synthesized wrapper here is never redundant with
        // an already-present source `<table-wrap>`.
        node::TABLE => vec![jats_element(
            "table-wrap",
            vec![],
            vec![table_element(node)],
        )],

        node::TABLE_HEAD => vec![jats_element(
            "thead",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::TABLE_BODY => vec![jats_element(
            "tbody",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::TABLE_ROW => vec![jats_element(
            "tr",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::TABLE_CELL => vec![jats_element(
            "td",
            cell_span_attrs(node),
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::TABLE_HEADER => vec![jats_element(
            "th",
            cell_span_attrs(node),
            node.children.iter().flat_map(write_inline).collect(),
        )],

        // `jats:tag = "table-wrap"` (see `rescribe-read-jats`'s `"table-wrap"`
        // arm) distinguishes a `<table-wrap>`-sourced `FIGURE` from a plain
        // `<fig>` one. A direct `TABLE` child is written via `table_element`
        // (a bare `<table>...</table>`, no wrapper) rather than
        // `write_node` (whose own `TABLE` arm would synthesize a *second*,
        // redundant `<table-wrap>` nested inside this one).
        node::FIGURE if node.props.get_str("jats:tag") == Some("table-wrap") => {
            let children = node
                .children
                .iter()
                .flat_map(|c| {
                    if c.kind.as_str() == node::TABLE {
                        vec![table_element(c)]
                    } else {
                        write_node(c)
                    }
                })
                .collect();
            vec![jats_element("table-wrap", generic_attrs(node), children)]
        }

        node::FIGURE => vec![jats_element(
            "fig",
            generic_attrs(node),
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::IMAGE => vec![jats_element(
            "graphic",
            node.props
                .get_str(prop::URL)
                .map(|url| vec![("xlink:href".to_string(), url.to_string())])
                .unwrap_or_default(),
            vec![],
        )],

        node::HORIZONTAL_RULE => Vec::new(), // JATS doesn't have HR

        node::FOOTNOTE_DEF => vec![jats_element(
            "fn",
            generic_attrs(node),
            node.children.iter().flat_map(write_node).collect(),
        )],

        "math_display" => vec![jats_element(
            "disp-formula",
            generic_attrs(node),
            formula_children(node),
        )],

        // `figcaption` (a custom node kind — see `rescribe-read-jats`'s
        // `"caption"` arm) round-trips back to `<caption>`. Previously had
        // no arm here at all, so it fell to the catch-all below and its
        // `<caption>` wrapper was silently dropped, leaving a bare `<p>` on
        // round-trip — the same class of bug found in `docbook-fmt`'s
        // `<caption>`/`figcaption` handling in an earlier session.
        "figcaption" => vec![jats_element(
            "caption",
            generic_attrs(node),
            node.children.iter().flat_map(write_node).collect(),
        )],

        // A `span` tagged `jats:tag` (a `generic_span`, e.g. `<label>`
        // landing directly among a `<fig>`/`<table-wrap>`'s block-level
        // children) must re-emit as itself via `write_inline`'s `SPAN` arm,
        // not fall to the "unknown block - recurse into children" catch-all
        // below — that catch-all would descend straight to the span's
        // `TEXT` child, which the next arm's block-position `TEXT` case then
        // wraps in a spurious `<p>`, silently losing the original tag
        // entirely (e.g. `<label>Figure 1</label>` round-tripping as a bare
        // `<p>Figure 1</p>`).
        node::SPAN if node.props.get_str("jats:tag").is_some() => write_inline(node),

        // Inline nodes that appear at block level: wrap in a <p>.
        node::TEXT | node::EMPHASIS | node::STRONG | node::CODE | node::LINK => {
            vec![jats_element("p", vec![], write_inline(node))]
        }

        _ => {
            // Unknown block - recurse into children
            node.children.iter().flat_map(write_node).collect()
        }
    }
}

/// Write a `bibliography_entry` node back to `<ref>` (see
/// rescribe-read-jats's `build_bibliography_entry`). A `label` field (see
/// rescribe-read-jats's `convert_biblio_field`'s `"label"` arm) becomes a
/// direct `<ref>` child *before* the citation wrapper — `<ref>`'s own
/// content model is `(label?, (element-citation | mixed-citation | ...)+)`,
/// so a `<label>` must not end up nested *inside* the citation element.
/// `jats:tag` (set by rescribe-read-jats's `"element-citation"`/
/// `"mixed-citation"` marker handling) picks which of the two wrapper
/// elements to re-emit, defaulting to `<element-citation>` for an entry
/// built by a non-JATS producer. `prop::DATE` becomes leading `<year>`/
/// `<month>`/`<day>` children (see `write_citation_date`); consecutive
/// `author`/`editor` fields sharing the same `jats:person-group-type` (see
/// rescribe-read-jats's `convert_person_group`) are regrouped back into one
/// `<person-group person-group-type="...">` wrapper — a field with no such
/// prop (the bare `<name>`/`<collab>`/`<string-name>` case) is instead
/// re-emitted unwrapped, exactly as read.
fn write_bibliography_entry(node: &Node) -> JNode {
    let attrs = generic_attrs(node);
    let citation_tag = node.props.get_str("jats:tag").unwrap_or("element-citation");
    let mut citation_attrs = Vec::new();
    if let Some(pt) = node.props.get_str("jats:attr:publication-type") {
        citation_attrs.push(("publication-type".to_string(), pt.to_string()));
    }
    if let Some(pf) = node.props.get_str("jats:attr:publication-format") {
        citation_attrs.push(("publication-format".to_string(), pf.to_string()));
    }

    let mut ref_kids = Vec::new();
    let mut citation_kids = Vec::with_capacity(node.children.len() + 3);
    if let Some(PropValue::Map(date)) = node.props.get(prop::DATE) {
        citation_kids.extend(write_citation_date(date));
    }

    let mut iter = node.children.iter().peekable();
    while let Some(child) = iter.next() {
        if child.kind.as_str() == node::BIBLIOGRAPHY_ENTRY {
            // Not produced by rescribe-read-jats itself (JATS `<ref>` has no
            // nested-reference construct), but a cross-format conversion
            // into JATS could build one — recurse rather than drop it.
            ref_kids.push(write_bibliography_entry(child));
            continue;
        }
        if child.kind.as_str() != node::BIBLIOGRAPHY_FIELD {
            citation_kids.extend(write_inline(child));
            continue;
        }
        if child.props.get_str("jats:tag") == Some("label") {
            ref_kids.push(write_bibliography_field(child));
            continue;
        }
        if let Some(pg_type) = child.props.get_str("jats:person-group-type") {
            let mut group_kids = vec![write_bibliography_field(child)];
            while let Some(next) = iter.peek() {
                if next.kind.as_str() == node::BIBLIOGRAPHY_FIELD
                    && next.props.get_str("jats:person-group-type") == Some(pg_type)
                {
                    group_kids.push(write_bibliography_field(iter.next().unwrap()));
                } else {
                    break;
                }
            }
            citation_kids.push(jats_element(
                "person-group",
                vec![("person-group-type".to_string(), pg_type.to_string())],
                group_kids,
            ));
            continue;
        }
        citation_kids.push(write_bibliography_field(child));
    }
    ref_kids.push(jats_element(citation_tag, citation_attrs, citation_kids));
    jats_element("ref", attrs, ref_kids)
}

/// Write one `bibliography_field` node back to its originating JATS
/// element. `jats:tag` (set by every arm of rescribe-read-jats's
/// `convert_biblio_field`/`convert_person_group`) takes priority when
/// present, since it names the exact source element (a person-group
/// member's own tag — `name`/`collab`/`string-name`/`etal`/`aff`/`role` —
/// or the original tag behind a raw-preserved `misc` field); `prop::
/// FIELD_ROLE` is the fallback for a field built by a non-JATS producer (a
/// cross-format conversion into JATS).
fn write_bibliography_field(node: &Node) -> JNode {
    let inline_children: Vec<JNode> = node.children.iter().flat_map(write_inline).collect();
    let role = node.props.get_str(prop::FIELD_ROLE).unwrap_or("misc");
    let tag = node
        .props
        .get_str("jats:tag")
        .unwrap_or_else(|| default_tag_for_role(role));
    let mut attrs = Vec::new();
    if tag == "pub-id"
        && let Some(scheme) = node.props.get_str(prop::FIELD_SCHEME)
    {
        attrs.push(("pub-id-type".to_string(), scheme.to_string()));
    }
    if tag == "date-in-citation" {
        if let Some(iso) = node.props.get_str("jats:attr:iso-8601-date") {
            attrs.push(("iso-8601-date".to_string(), iso.to_string()));
        }
        if let Some(ct) = node.props.get_str("jats:attr:content-type") {
            attrs.push(("content-type".to_string(), ct.to_string()));
        }
    }
    jats_element(tag, attrs, inline_children)
}

/// `prop::FIELD_ROLE`'s standard vocabulary, mapped to the JATS element a
/// field with no `jats:tag` (i.e. built by a non-JATS producer) should
/// re-emit as.
fn default_tag_for_role(role: &str) -> &'static str {
    match role {
        "author" | "editor" => "name",
        "title" => "article-title",
        "container_title" => "source",
        "publisher" => "publisher-name",
        "publisher_location" => "publisher-loc",
        "edition" => "edition",
        "volume" => "volume",
        "issue" => "issue",
        "page_first" => "fpage",
        "page_last" => "lpage",
        "identifier" => "pub-id",
        _ => "comment",
    }
}

/// Format `prop::DATE`'s `year`/`month`/`day` map (see the property's own
/// doc comment) into `<year iso-8601-date="...">`/`<month>`/`<day>`
/// elements — the inverse of rescribe-read-jats's `resolve_citation_date`.
/// The `iso-8601-date` attribute is always attached to `<year>` (the JATS
/// Tag Library's own convention — see its tagged `element-citation`
/// examples) so a reader that prefers the unambiguous attribute form
/// recovers exactly the same date without needing the separate `<month>`/
/// `<day>` elements at all.
fn write_citation_date(map: &HashMap<String, PropValue>) -> Vec<JNode> {
    let as_int = |key: &str| match map.get(key) {
        Some(PropValue::Int(i)) => Some(i),
        _ => None,
    };
    let year = as_int("year");
    let month = as_int("month");
    let day = as_int("day");
    let mut out = Vec::new();
    if let Some(y) = year {
        let iso = match (month, day) {
            (Some(m), Some(d)) => format!("{y:04}-{m:02}-{d:02}"),
            (Some(m), None) => format!("{y:04}-{m:02}"),
            _ => format!("{y:04}"),
        };
        out.push(jats_element(
            "year",
            vec![("iso-8601-date".to_string(), iso)],
            vec![jats_text(y.to_string())],
        ));
    }
    if let Some(m) = month {
        out.push(jats_element(
            "month",
            vec![],
            vec![jats_text(format!("{m:02}"))],
        ));
    }
    if let Some(d) = day {
        out.push(jats_element(
            "day",
            vec![],
            vec![jats_text(format!("{d:02}"))],
        ));
    }
    out
}

/// Convert one rescribe IR (inline-level) node into zero or more JATS AST
/// nodes.
fn write_inline(node: &Node) -> Vec<JNode> {
    match node.kind.as_str() {
        node::TEXT => match node.props.get_str(prop::CONTENT) {
            Some(content) => vec![jats_text(content)],
            None => Vec::new(),
        },

        node::EMPHASIS => vec![jats_element(
            "italic",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::STRONG => vec![jats_element(
            "bold",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::UNDERLINE => vec![jats_element(
            "underline",
            node.props
                .get_str("jats:underline-style")
                .map(|s| vec![("underline-style".to_string(), s.to_string())])
                .unwrap_or_default(),
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::STRIKEOUT => vec![jats_element(
            "strike",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::CODE => {
            // `jats:tag` (see `rescribe-read-jats`) distinguishes an inline
            // `<code>` from `<monospace>` — default to `<monospace>` (the
            // pre-existing behavior) for IR trees not built from a JATS
            // read.
            let tag = node.props.get_str("jats:tag").unwrap_or("monospace");
            let mut children: Vec<JNode> = node
                .props
                .get_str(prop::CONTENT)
                .map(jats_text)
                .into_iter()
                .collect();
            children.extend(node.children.iter().flat_map(write_inline));
            vec![jats_element(tag, generic_attrs(node), children)]
        }

        node::LINK => {
            let mut attrs = Vec::new();
            if let Some(url) = node.props.get_str(prop::URL) {
                attrs.push(("xlink:href".to_string(), url.to_string()));
                // `jats:ext-link-type` (see `rescribe-read-jats`) round-trips
                // the original `ext-link-type` value; `"uri"` is JATS's own
                // default and remains the fallback for IR trees not built
                // from a JATS read.
                let link_type = node
                    .props
                    .get_str("jats:ext-link-type")
                    .unwrap_or("uri")
                    .to_string();
                attrs.push(("ext-link-type".to_string(), link_type));
            }
            attrs.extend(generic_attrs(node));
            vec![jats_element(
                "ext-link",
                attrs,
                node.children.iter().flat_map(write_inline).collect(),
            )]
        }

        node::FOOTNOTE_REF => {
            let mut attrs = vec![("ref-type".to_string(), "fn".to_string())];
            if let Some(label) = node.props.get_str(prop::LABEL) {
                attrs.push(("rid".to_string(), label.to_string()));
            }
            vec![jats_element(
                "xref",
                attrs,
                node.children.iter().flat_map(write_inline).collect(),
            )]
        }

        node::SUBSCRIPT => vec![jats_element(
            "sub",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::SUPERSCRIPT => vec![jats_element(
            "sup",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::SMALL_CAPS => vec![jats_element(
            "sc",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::LINE_BREAK => vec![jats_element("break", vec![], vec![])],

        node::SOFT_BREAK => vec![jats_text(" ")],

        node::IMAGE => vec![jats_element(
            "inline-graphic",
            node.props
                .get_str(prop::URL)
                .map(|url| vec![("xlink:href".to_string(), url.to_string())])
                .unwrap_or_default(),
            vec![],
        )],

        "math_inline" => vec![jats_element(
            "inline-formula",
            vec![],
            formula_children(node),
        )],

        // A raw entity reference preserved by the reader: re-emit verbatim.
        node::RAW_INLINE => match node.props.get_str("jats:entity") {
            Some(name) => vec![JNode::EntityRef {
                name: name.to_string(),
                span: jats_fmt::Span::NONE,
            }],
            None => node.children.iter().flat_map(write_inline).collect(),
        },

        // A `span` tagged `jats:tag` is a `generic_span` (an unrecognized
        // inline-level element raw-preserved by the reader's catch-all, see
        // `rescribe-read-jats::generic_span`) — re-emit its original tag
        // name.
        node::SPAN => match node.props.get_str("jats:tag") {
            Some(tag) => {
                let mut attrs = generic_attrs(node);
                attrs.extend(generic_extra_attrs(node));
                vec![jats_element(
                    tag,
                    attrs,
                    node.children.iter().flat_map(write_inline).collect(),
                )]
            }
            None => node.children.iter().flat_map(write_inline).collect(),
        },

        _ => {
            // Unknown inline - recurse
            node.children.iter().flat_map(write_inline).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_core::Properties;

    #[test]
    fn test_emit_empty() {
        let doc = Document {
            content: Node::new(node::DOCUMENT),
            resources: Default::default(),
            metadata: Properties::new(),
            source: None,
        };

        let result = emit(&doc).unwrap();
        let xml = String::from_utf8(result.value).unwrap();
        assert!(xml.contains("<article"));
        assert!(xml.contains("</article>"));
    }

    #[test]
    fn test_emit_paragraph() {
        let doc = Document {
            content: Node::new(node::DOCUMENT).child(
                Node::new(node::PARAGRAPH)
                    .child(Node::new(node::TEXT).prop(prop::CONTENT, "Hello, world!")),
            ),
            resources: Default::default(),
            metadata: Properties::new(),
            source: None,
        };

        let result = emit(&doc).unwrap();
        let xml = String::from_utf8(result.value).unwrap();
        assert!(xml.contains("<p>Hello, world!</p>"));
    }

    #[test]
    fn test_emit_with_title() {
        let mut metadata = Properties::new();
        metadata.set("title", "Test Article".to_string());

        let doc = Document {
            content: Node::new(node::DOCUMENT),
            resources: Default::default(),
            metadata,
            source: None,
        };

        let result = emit(&doc).unwrap();
        let xml = String::from_utf8(result.value).unwrap();
        assert!(xml.contains("<article-title>Test Article</article-title>"));
    }

    #[test]
    fn test_emit_formatting() {
        let doc = Document {
            content: Node::new(node::DOCUMENT).child(
                Node::new(node::PARAGRAPH)
                    .child(
                        Node::new(node::EMPHASIS)
                            .child(Node::new(node::TEXT).prop(prop::CONTENT, "italic")),
                    )
                    .child(Node::new(node::TEXT).prop(prop::CONTENT, " and "))
                    .child(
                        Node::new(node::STRONG)
                            .child(Node::new(node::TEXT).prop(prop::CONTENT, "bold")),
                    ),
            ),
            resources: Default::default(),
            metadata: Properties::new(),
            source: None,
        };

        let result = emit(&doc).unwrap();
        let xml = String::from_utf8(result.value).unwrap();
        assert!(xml.contains("<italic>italic</italic>"));
        assert!(xml.contains("<bold>bold</bold>"));
    }

    #[test]
    fn test_roundtrip_through_reader() {
        let jats = r#"<?xml version="1.0"?>
<article><body><p>Hello <italic>world</italic></p></body></article>"#;
        let parsed = rescribe_read_jats::parse(jats).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(xml.contains("<p>Hello <italic>world</italic></p>"));
    }

    /// A `<disp-formula>` with embedded `<mml:math>` MathML must round-trip
    /// byte-for-byte through parse -> emit -> reparse: the `mml:math`
    /// subtree is raw-preserved (see `rescribe-read-jats`'s `mml-math-raw`
    /// sentinel / `split_mathml`) and re-spliced verbatim on write (see
    /// `formula_children`'s `JNode::Raw` branch), so a second parse must
    /// recover the exact same `math:source`.
    #[test]
    fn test_roundtrip_mathml_disp_formula() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article xmlns:xlink="http://www.w3.org/1999/xlink"><body><disp-formula><mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>x</mml:mi></mml:math></disp-formula></body></article>"#;
        let parsed = rescribe_read_jats::parse(jats).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(
            xml.contains(r#"<mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>x</mml:mi></mml:math>"#),
            "emitted XML missing raw mml:math: {xml}"
        );

        let reparsed = rescribe_read_jats::parse(&xml).unwrap();
        let formula = &reparsed.value.content.children[0].children[0];
        assert_eq!(formula.kind.as_str(), "math_display");
        assert_eq!(formula.props.get_str("math:format"), Some("mathml"));
        assert_eq!(
            formula.props.get_str("math:source"),
            parsed.value.content.children[0].children[0]
                .props
                .get_str("math:source")
        );
    }

    /// Same round-trip guarantee for `<inline-formula>`/`math_inline`.
    #[test]
    fn test_roundtrip_mathml_inline_formula() {
        let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
<article xmlns:xlink="http://www.w3.org/1999/xlink"><body><p>x is <inline-formula><mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>x</mml:mi></mml:math></inline-formula>.</p></body></article>"#;
        let parsed = rescribe_read_jats::parse(jats).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(
            xml.contains(r#"<mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>x</mml:mi></mml:math>"#),
            "emitted XML missing raw mml:math: {xml}"
        );

        let reparsed = rescribe_read_jats::parse(&xml).unwrap();
        let para = &reparsed.value.content.children[0].children[0];
        let formula = &para.children[1];
        assert_eq!(formula.kind.as_str(), "math_inline");
        assert_eq!(formula.props.get_str("math:format"), Some("mathml"));
    }
}
