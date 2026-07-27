//! DocBook writer for rescribe.
//!
//! Translates rescribe's document IR into `docbook_fmt::DocBookDoc` (the
//! standalone DocBook/XML AST from the `docbook-fmt` crate) and serializes
//! it via `docbook_fmt::emit`. All XML writing lives in `docbook-fmt` —
//! this crate is a thin IR↔AST translator only (per CLAUDE.md's "adapter
//! layer must never contain parsing or writing logic" rule).
//!
//! # Example
//!
//! ```
//! use rescribe_write_docbook::emit;
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

use docbook_fmt::{DocBookDoc, Node as DbNode, XmlDecl};
use rescribe_core::{ConversionResult, Document, EmitError, Node, PropValue};
use rescribe_std::{node, prop};

/// Emit a document to DocBook XML.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let warnings = Vec::new();

    let mut root_children = Vec::new();
    // Any metadata key ending in `_raw` is a whole-subtree verbatim capture
    // of an unmodeled `<info>` front-matter element (see
    // `rescribe-read-docbook`'s `convert_children`/`extract_metadata` —
    // `{tag}_raw`, e.g. `author_raw` or `revhistory_raw`). Collected once
    // here so both the "do we even need an `<info>`" check and the
    // splice-back loop below share one scan.
    let mut info_raw_fields: Vec<(&str, &str)> = doc
        .metadata
        .iter()
        .filter_map(|(key, _)| {
            let tag = key.strip_suffix("_raw")?;
            Some((tag, doc.metadata.get_str(key)?))
        })
        .collect();
    info_raw_fields.sort_unstable_by_key(|(tag, _)| *tag);
    let title = doc.metadata.get_str("title");
    if title.is_some() || !info_raw_fields.is_empty() {
        let mut info_children = Vec::new();
        if let Some(title) = title {
            info_children.push(db_element("title", vec![], vec![db_text(title)]));
        }
        // Splice back every raw-captured `<info>` subtree byte-for-byte
        // (see `rescribe-read-docbook`'s `convert_children`/
        // `extract_metadata` — any `{tag}_raw` metadata field, e.g.
        // `author_raw` or `revhistory_raw`). This is lossless where
        // reconstructing the element from its flattened text would not be;
        // sorted by tag for deterministic output since `Properties`
        // iterates in unspecified order.
        for (_, raw) in &info_raw_fields {
            info_children.push(DbNode::Raw {
                content: (*raw).to_string(),
                span: docbook_fmt::Span::NONE,
            });
        }
        root_children.push(db_element("info", vec![], info_children));
    }
    for child in &doc.content.children {
        root_children.extend(write_node(child));
    }

    let root = DbNode::Element {
        name: "article".to_string(),
        attrs: vec![
            (
                "xmlns".to_string(),
                "http://docbook.org/ns/docbook".to_string(),
            ),
            ("version".to_string(), "5.0".to_string()),
        ],
        children: root_children,
        span: docbook_fmt::Span::NONE,
    };

    let doc_ast = DocBookDoc {
        xml_decl: Some(XmlDecl {
            version: "1.0".to_string(),
            encoding: Some("UTF-8".to_string()),
            standalone: None,
        }),
        nodes: vec![root],
    };

    let bytes = docbook_fmt::emit(&doc_ast);
    Ok(ConversionResult::with_warnings(bytes, warnings))
}

fn db_element(name: &str, attrs: Vec<(String, String)>, children: Vec<DbNode>) -> DbNode {
    DbNode::Element {
        name: name.to_string(),
        attrs,
        children,
        span: docbook_fmt::Span::NONE,
    }
}

fn db_text(content: impl Into<String>) -> DbNode {
    DbNode::Text {
        content: content.into(),
        span: docbook_fmt::Span::NONE,
    }
}

/// Build the generic `id`/`role` attributes for a node, if the IR node
/// carries the corresponding raw-preserved properties (see
/// `rescribe-read-docbook`'s `attach_generic_attrs` for the reader side of
/// this round trip).
fn generic_attrs(node: &Node) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    if let Some(id) = node.props.get_str(prop::ID) {
        attrs.push(("id".to_string(), id.to_string()));
    }
    if let Some(role) = node.props.get_str("docbook:role") {
        attrs.push(("role".to_string(), role.to_string()));
    }
    attrs
}

/// Convert one rescribe IR (block-level) node into zero or more DocBook AST
/// nodes.
fn write_node(node: &Node) -> Vec<DbNode> {
    match node.kind.as_str() {
        node::DOCUMENT => node.children.iter().flat_map(write_node).collect(),

        // A `div` tagged `docbook:tag` is a `generic_div` (an unrecognized
        // block-level element raw-preserved by the reader's catch-all, see
        // `rescribe-read-docbook::generic_div`) — re-emit its original tag
        // name. Any other `div` (article/book/chapter/section/etc, which
        // the reader unwraps regardless of the writer re-wrapping them)
        // just flattens into its children, same as `DOCUMENT`.
        node::DIV => match node.props.get_str("docbook:tag") {
            Some(tag) => vec![db_element(
                tag,
                generic_attrs(node),
                node.children.iter().flat_map(write_node).collect(),
            )],
            // `html:class == "abstract"` (see rescribe-read-docbook's
            // "abstract" arm, the one dedicated DIV mapping that doesn't
            // use `docbook:tag`) still needs to round-trip back to
            // `<abstract>` — falling through to the untagged case below
            // would silently flatten it into its children, losing the
            // fact it was ever an `<abstract>` at all.
            None if node.props.get_str("html:class") == Some("abstract") => vec![db_element(
                "abstract",
                vec![],
                node.children.iter().flat_map(write_node).collect(),
            )],
            None => node.children.iter().flat_map(write_node).collect(),
        },

        // A container's caption (see rescribe-read-docbook's
        // `heading_level_for_parent` — any `<title>` whose parent isn't a
        // genuine sectioning container maps here instead of to `HEADING`)
        // — re-emit as `<title>` in place, not wrapped in a spurious
        // `<sectN>` the way a `HEADING` would be. The `TABLE` arm above
        // handles its own title specially (via the `title` property, since
        // a formal table's title needs to come before `<tgroup>`); every
        // other container (example, figure, admonitions, qandaset,
        // refentry, ...) just keeps its `CAPTION` child in natural
        // position, which lands here.
        node::CAPTION => vec![db_element(
            "title",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        // `docbook:tag == "bridgehead"` (see rescribe-read-docbook's
        // "bridgehead" arm): a bridgehead is explicitly *not* tied to the
        // section hierarchy, so it must not get the `<sectN><title>`
        // wrapper below — re-emit as a bare `<bridgehead renderas="sectN">`
        // instead, with its level round-tripped through `renderas`.
        node::HEADING if node.props.get_str("docbook:tag") == Some("bridgehead") => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(4);
            let renderas = format!("sect{}", (level - 1).clamp(1, 5));
            vec![db_element(
                "bridgehead",
                vec![("renderas".to_string(), renderas)],
                node.children.iter().flat_map(write_inline).collect(),
            )]
        }

        node::HEADING => {
            let level = node.props.get_int(prop::LEVEL).unwrap_or(1);
            let section_tag = match level {
                1 => "section",
                2 => "sect1",
                3 => "sect2",
                4 => "sect3",
                5 => "sect4",
                _ => "sect5",
            };
            vec![db_element(
                section_tag,
                vec![],
                vec![db_element(
                    "title",
                    vec![],
                    node.children.iter().flat_map(write_inline).collect(),
                )],
            )]
        }

        node::PARAGRAPH => vec![db_element(
            "para",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::BLOCKQUOTE => {
            let tag = node
                .props
                .get_str("docbook:type")
                .filter(|t| {
                    matches!(
                        *t,
                        "note" | "tip" | "warning" | "caution" | "important" | "epigraph"
                    )
                })
                .unwrap_or("blockquote");
            vec![db_element(
                tag,
                vec![],
                node.children.iter().flat_map(write_node).collect(),
            )]
        }

        // <attribution> (see rescribe-read-docbook's "attribution" arm) —
        // phrase-level content, so re-emitted via write_inline like
        // CAPTION, not write_node.
        "docbook:attribution" => vec![db_element(
            "attribution",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::LIST => {
            // `docbook:tag` = "procedure"/"substeps" (see
            // rescribe-read-docbook's "procedure"|"substeps" arm) re-emits
            // the original element instead of `<orderedlist>`.
            let tag = match node.props.get_str("docbook:tag") {
                Some(tag @ ("procedure" | "substeps")) => tag,
                _ => {
                    if node.props.get_bool(prop::ORDERED).unwrap_or(false) {
                        "orderedlist"
                    } else {
                        "itemizedlist"
                    }
                }
            };
            vec![db_element(
                tag,
                vec![],
                node.children.iter().flat_map(write_node).collect(),
            )]
        }

        node::LIST_ITEM => vec![db_element(
            node.props.get_str("docbook:tag").unwrap_or("listitem"),
            vec![],
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
                entries.push(db_element("varlistentry", vec![], entry_children));
                i += 2;
            }
            vec![db_element("variablelist", vec![], entries)]
        }

        node::DEFINITION_TERM => vec![db_element(
            "term",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::DEFINITION_DESC => vec![db_element(
            "listitem",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::CODE_BLOCK => {
            let mut attrs = Vec::new();
            if let Some(lang) = node.props.get_str(prop::LANGUAGE) {
                attrs.push(("language".to_string(), lang.to_string()));
            }
            let content = node.props.get_str(prop::CONTENT).unwrap_or("");
            // `docbook:tag` remembers which verbatim element this came from
            // (see rescribe-read-docbook's "programlisting"|"screen"|
            // "literallayout"|"synopsis"|"address" arm) — defaults to
            // `programlisting` for CODE_BLOCK nodes built directly by
            // non-DocBook producers.
            let tag = node
                .props
                .get_str("docbook:tag")
                .unwrap_or("programlisting");
            vec![db_element(tag, attrs, vec![db_text(content)])]
        }

        node::TABLE => {
            // Split children back into <colspec> siblings (see
            // rescribe-read-docbook's dedicated "colspec" arm — a
            // structured "docbook:colspec"-kind child, not a table row)
            // and actual row children.
            let mut colspecs = Vec::new();
            let mut rows = Vec::new();
            for child in &node.children {
                if child.kind.as_str() == "docbook:colspec" {
                    let mut attrs = Vec::new();
                    if let Some(colname) = child.props.get_str("docbook:colname") {
                        attrs.push(("colname".to_string(), colname.to_string()));
                    }
                    if let Some(colnum) = child.props.get_str("docbook:colnum") {
                        attrs.push(("colnum".to_string(), colnum.to_string()));
                    }
                    if let Some(colwidth) = child.props.get_str("docbook:colwidth") {
                        attrs.push(("colwidth".to_string(), colwidth.to_string()));
                    }
                    if let Some(align) = child.props.get_str(prop::ALIGN) {
                        attrs.push(("align".to_string(), align.to_string()));
                    }
                    colspecs.push(db_element("colspec", attrs, vec![]));
                } else {
                    rows.extend(write_node(child));
                }
            }
            let mut tgroup_children = colspecs;
            tgroup_children.push(db_element("tbody", vec![], rows));

            // A `title` property means this was a formal <table> (DocBook
            // 5.2: table requires a title, informaltable must not have
            // one) — see rescribe-read-docbook's `"title" if parent ==
            // Some("table")` arm.
            let tag = if node.props.get_str(prop::TITLE).is_some() {
                "table"
            } else {
                "informaltable"
            };
            let mut table_children = Vec::new();
            if let Some(title) = node.props.get_str(prop::TITLE) {
                table_children.push(db_element("title", vec![], vec![db_text(title)]));
            }
            table_children.push(db_element("tgroup", vec![], tgroup_children));

            let mut attrs = generic_attrs(node);
            if let Some(frame) = node.props.get_str("docbook:frame") {
                attrs.push(("frame".to_string(), frame.to_string()));
            }
            if let Some(colsep) = node.props.get_str("docbook:colsep") {
                attrs.push(("colsep".to_string(), colsep.to_string()));
            }
            if let Some(rowsep) = node.props.get_str("docbook:rowsep") {
                attrs.push(("rowsep".to_string(), rowsep.to_string()));
            }
            vec![db_element(tag, attrs, table_children)]
        }

        node::TABLE_ROW => vec![db_element(
            "row",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::TABLE_CELL | node::TABLE_HEADER => {
            let mut attrs = Vec::new();
            // `rowspan` (total rows spanned) round-trips back to `morerows`
            // (additional rows spanned) — see rescribe-read-docbook's
            // "entry"|"td" arm for the inverse `+1` conversion.
            if let Some(rowspan) = node.props.get_int(prop::ROWSPAN)
                && rowspan > 1
            {
                attrs.push(("morerows".to_string(), (rowspan - 1).to_string()));
            }
            if let Some(namest) = node.props.get_str("docbook:namest") {
                attrs.push(("namest".to_string(), namest.to_string()));
            }
            if let Some(nameend) = node.props.get_str("docbook:nameend") {
                attrs.push(("nameend".to_string(), nameend.to_string()));
            }
            vec![db_element(
                "entry",
                attrs,
                node.children.iter().flat_map(write_inline).collect(),
            )]
        }

        node::FIGURE => vec![db_element(
            "figure",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::IMAGE => vec![db_element(
            "mediaobject",
            vec![],
            vec![db_element(
                "imageobject",
                vec![],
                vec![db_element(
                    "imagedata",
                    node.props
                        .get_str(prop::URL)
                        .map(|url| vec![("fileref".to_string(), url.to_string())])
                        .unwrap_or_default(),
                    vec![],
                )],
            )],
        )],

        node::HORIZONTAL_RULE => Vec::new(), // DocBook doesn't have HR

        node::FOOTNOTE_DEF => vec![db_element(
            "footnote",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::BIBLIOGRAPHY => vec![db_element(
            "bibliography",
            generic_attrs(node),
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::BIBLIOGRAPHY_ENTRY => vec![write_bibliography_entry(node)],

        // A `bibliography_field` shouldn't normally appear outside a
        // `BIBLIOGRAPHY_ENTRY`'s own children (handled directly by
        // `write_bibliography_entry`, not through this dispatch), but
        // delegate to the same field writer defensively rather than falling
        // to the generic "recurse into children" catch-all below, which
        // would drop the field's role/tag entirely.
        node::BIBLIOGRAPHY_FIELD => vec![write_bibliography_field(node)],

        // Inline nodes that appear at block level: wrap in a <para>.
        node::TEXT | node::EMPHASIS | node::STRONG | node::CODE | node::LINK => {
            vec![db_element("para", vec![], write_inline(node))]
        }

        // A `generic_span` (see rescribe-read-docbook's `generic_span` — an
        // unrecognized inline element, e.g. `<arg>` inside
        // `<cmdsynopsis>`, raw-preserved with its tag under `docbook:tag`)
        // that ends up as a direct child of a raw-preserved block
        // container (e.g. `<cmdsynopsis>`'s mixed inline content model, not
        // `<para>`-based like most block containers) is re-emitted as
        // itself, not wrapped in a synthetic `<para>` the original never
        // had — unlike TEXT/EMPHASIS/etc above, which need a `<para>`
        // wrapper to be valid content at all, a bare element is already
        // valid without one. Without this arm it would fall to the
        // catch-all `_` below, which only recurses into children and would
        // silently drop the tag itself.
        node::SPAN => write_inline(node),

        _ => {
            // Unknown block - recurse into children
            node.children.iter().flat_map(write_node).collect()
        }
    }
}

/// Write a `bibliography_entry` node back to `<biblioentry>`/`<bibliomixed>`/
/// `<biblioset>`/`<bibliomset>` (see rescribe-read-docbook's
/// `build_bibliography_entry` — `docbook:tag` remembers which one the
/// original element was; defaults to `<biblioentry>` for an entry built by a
/// non-DocBook producer). `prop::DATE` (a structured year/month/day map, see
/// its own doc comment) becomes a leading `<date>` child; a `page_first` +
/// `page_last` pair of sibling fields recombines into one `<pagenums>`
/// (see rescribe-read-docbook's `convert_pagenums` for the reader-side
/// split); a nested `BIBLIOGRAPHY_ENTRY` child (from `<biblioset>` nesting)
/// recurses through this same function.
fn write_bibliography_entry(node: &Node) -> DbNode {
    let tag = node.props.get_str("docbook:tag").unwrap_or("biblioentry");
    let mut attrs = generic_attrs(node);
    if let Some(relation) = node.props.get_str("docbook:biblioset-relation") {
        attrs.push(("relation".to_string(), relation.to_string()));
    }
    let mut kids = Vec::with_capacity(node.children.len() + 1);
    if let Some(PropValue::Map(date)) = node.props.get(prop::DATE) {
        let text = format_bibliographic_date(date);
        if !text.is_empty() {
            kids.push(db_element("date", vec![], vec![db_text(text)]));
        }
    }
    let mut iter = node.children.iter().peekable();
    while let Some(child) = iter.next() {
        if child.kind.as_str() == node::BIBLIOGRAPHY_ENTRY {
            kids.push(write_bibliography_entry(child));
            continue;
        }
        // `<bibliomixed>`'s mixed content model interleaves free text
        // directly between fields (see rescribe-read-docbook's
        // `convert_children` — a `bibliomixed` entry's non-element
        // children, e.g. plain running text between citation parts, are
        // ordinary inline nodes, not `bibliography_field`s). Re-emit them
        // as plain inline content rather than routing through
        // `write_bibliography_field`, which would wrap them in a spurious
        // `<bibliomisc>`.
        if child.kind.as_str() != node::BIBLIOGRAPHY_FIELD {
            kids.extend(write_inline(child));
            continue;
        }
        if child.props.get_str(prop::FIELD_ROLE) == Some("page_first")
            && iter
                .peek()
                .and_then(|next| next.props.get_str(prop::FIELD_ROLE))
                == Some("page_last")
        {
            let last = iter.next().unwrap();
            let first_text = flatten_field_text(child);
            let last_text = flatten_field_text(last);
            kids.push(db_element(
                "pagenums",
                vec![],
                vec![db_text(format!("{first_text}-{last_text}"))],
            ));
            continue;
        }
        kids.push(write_bibliography_field(child));
    }
    db_element(tag, attrs, kids)
}

/// Write one `bibliography_field` node back to its originating DocBook
/// element. `docbook:tag` (set by every arm of rescribe-read-docbook's
/// `convert_biblio_field`) takes priority when present, since it names the
/// exact source element (e.g. `<publishername>` vs. a bare `<publisher>`,
/// or the original tag behind a raw-preserved `misc` field); `prop::
/// FIELD_ROLE` is the fallback for a field built by a non-DocBook producer
/// (a cross-format conversion into DocBook).
fn write_bibliography_field(node: &Node) -> DbNode {
    let inline_children: Vec<DbNode> = node.children.iter().flat_map(write_inline).collect();
    if let Some(tag) = node.props.get_str("docbook:tag") {
        return match tag {
            "author" => db_element(
                "author",
                vec![],
                vec![db_element("personname", vec![], inline_children)],
            ),
            "editor" => db_element(
                "editor",
                vec![],
                vec![db_element("personname", vec![], inline_children)],
            ),
            "publishername" => db_element(
                "publisher",
                vec![],
                vec![db_element("publishername", vec![], inline_children)],
            ),
            "city" => db_element(
                "address",
                vec![],
                vec![db_element("city", vec![], inline_children)],
            ),
            "biblioid" => {
                let mut attrs = Vec::new();
                if let Some(scheme) = node.props.get_str(prop::FIELD_SCHEME) {
                    attrs.push(("class".to_string(), scheme.to_string()));
                }
                db_element("biblioid", attrs, inline_children)
            }
            // `title`, `edition`, `volumenum`, `issuenum`, `publisher`
            // (bare-text case), `address` (bare-text case), `bibliomisc`,
            // `pagenums` (ambiguous-split misc fallback), `date`/`pubdate`
            // (unparseable-date misc fallback), and any other raw-preserved
            // tag (`subtitle`, `titleabbrev`, `isbn`, ...): re-emit the
            // original element name directly with the field's content.
            other => db_element(other, vec![], inline_children),
        };
    }
    // No `docbook:tag` — this field was built by a non-DocBook reader (a
    // cross-format conversion into DocBook). Fall back to the standard
    // `FIELD_ROLE` vocabulary.
    match node.props.get_str(prop::FIELD_ROLE).unwrap_or("misc") {
        "author" => db_element(
            "author",
            vec![],
            vec![db_element("personname", vec![], inline_children)],
        ),
        "editor" => db_element(
            "editor",
            vec![],
            vec![db_element("personname", vec![], inline_children)],
        ),
        // `container_title` has no distinct DocBook element of its own —
        // DocBook instead expresses a citation's container via `<biblioset>`
        // nesting (see `write_bibliography_entry`) — so a flat
        // `container_title` field (which can only arrive from a non-DocBook
        // producer that didn't use nesting) falls back to `<title>` rather
        // than being dropped; a disclosed simplification, not a design fork.
        "title" | "container_title" => db_element("title", vec![], inline_children),
        "publisher" => db_element(
            "publisher",
            vec![],
            vec![db_element("publishername", vec![], inline_children)],
        ),
        "publisher_location" => db_element("address", vec![], inline_children),
        "edition" => db_element("edition", vec![], inline_children),
        "volume" => db_element("volumenum", vec![], inline_children),
        "issue" => db_element("issuenum", vec![], inline_children),
        "page_first" | "page_last" => db_element("pagenums", vec![], inline_children),
        "identifier" => {
            let mut attrs = Vec::new();
            if let Some(scheme) = node.props.get_str(prop::FIELD_SCHEME) {
                attrs.push(("class".to_string(), scheme.to_string()));
            }
            db_element("biblioid", attrs, inline_children)
        }
        _ => db_element("bibliomisc", vec![], inline_children),
    }
}

/// Concatenate a field's descendant `TEXT` node content (depth-first) — used
/// only for re-combining a `page_first`/`page_last` pair back into one
/// `<pagenums>first-last</pagenums>` string, where page numbers are never
/// expected to carry nested markup.
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

/// Format `prop::DATE`'s `year`/`month`/`day` map (see the property's own
/// doc comment) back into an ISO 8601 string — the inverse of
/// rescribe-read-docbook's `parse_bibliographic_date`. Zero-padded to two
/// digits for month/day per ISO 8601 (`2020-03-05`, not `2020-3-5`).
fn format_bibliographic_date(map: &HashMap<String, PropValue>) -> String {
    let as_int = |key: &str| match map.get(key) {
        Some(PropValue::Int(i)) => Some(*i),
        _ => None,
    };
    match (as_int("year"), as_int("month"), as_int("day")) {
        (Some(y), Some(m), Some(d)) => format!("{y:04}-{m:02}-{d:02}"),
        (Some(y), Some(m), None) => format!("{y:04}-{m:02}"),
        (Some(y), None, None) => format!("{y:04}"),
        _ => String::new(),
    }
}

/// Convert one rescribe IR (inline-level) node into zero or more DocBook AST
/// nodes.
fn write_inline(node: &Node) -> Vec<DbNode> {
    match node.kind.as_str() {
        node::TEXT => match node.props.get_str(prop::CONTENT) {
            Some(content) => vec![db_text(content)],
            None => Vec::new(),
        },

        // A footnote embedded at the point of reference (e.g. inside a
        // table cell, which writes its content via write_inline, not
        // write_node) needs its own arm here — without it, the catch-all
        // below would recurse straight into the footnote's block content
        // (its <para>), silently losing the <footnote> wrapper and
        // splicing the note's text directly into the surrounding flow.
        node::FOOTNOTE_DEF => vec![db_element(
            "footnote",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        node::EMPHASIS => vec![db_element(
            "emphasis",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::STRONG => vec![db_element(
            "emphasis",
            vec![("role".to_string(), "strong".to_string())],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::CODE => {
            let mut children: Vec<DbNode> = node
                .props
                .get_str(prop::CONTENT)
                .map(db_text)
                .into_iter()
                .collect();
            children.extend(node.children.iter().flat_map(write_inline));
            vec![db_element("code", vec![], children)]
        }

        node::LINK => {
            let mut attrs = Vec::new();
            if let Some(url) = node.props.get_str(prop::URL) {
                attrs.push(("xlink:href".to_string(), url.to_string()));
            }
            vec![db_element(
                "link",
                attrs,
                node.children.iter().flat_map(write_inline).collect(),
            )]
        }

        node::SUBSCRIPT => vec![db_element(
            "subscript",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::SUPERSCRIPT => vec![db_element(
            "superscript",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::LINE_BREAK => vec![db_element("sbr", vec![], vec![])],

        node::FOOTNOTE_REF => match node.props.get_str(prop::LABEL) {
            Some(label) => vec![db_element(
                "footnoteref",
                vec![("linkend".to_string(), label.to_string())],
                vec![],
            )],
            None => Vec::new(),
        },

        node::SOFT_BREAK => vec![db_text(" ")],

        node::IMAGE => vec![db_element(
            "inlinemediaobject",
            vec![],
            vec![db_element(
                "imageobject",
                vec![],
                vec![db_element(
                    "imagedata",
                    node.props
                        .get_str(prop::URL)
                        .map(|url| vec![("fileref".to_string(), url.to_string())])
                        .unwrap_or_default(),
                    vec![],
                )],
            )],
        )],

        // A raw entity reference preserved by the reader: re-emit verbatim.
        node::RAW_INLINE => match node.props.get_str("docbook:entity") {
            Some(name) => vec![DbNode::EntityRef {
                name: name.to_string(),
                span: docbook_fmt::Span::NONE,
            }],
            None => node.children.iter().flat_map(write_inline).collect(),
        },

        // A `span` tagged `docbook:tag` is a `generic_span` (an
        // unrecognized inline-level element raw-preserved by the reader's
        // catch-all, see `rescribe-read-docbook::generic_span`) — re-emit
        // its original tag name.
        node::SPAN => match node.props.get_str("docbook:tag") {
            Some(tag) => vec![db_element(
                tag,
                generic_attrs(node),
                node.children.iter().flat_map(write_inline).collect(),
            )],
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
        // An empty root element is emitted self-closing (`<article .../>`) —
        // valid XML equivalent to `<article ...></article>`.
        assert!(xml.contains("/>"));
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
        assert!(xml.contains("<para>Hello, world!</para>"));
    }

    #[test]
    fn test_emit_with_title() {
        let mut metadata = Properties::new();
        metadata.set("title", "Test Document".to_string());

        let doc = Document {
            content: Node::new(node::DOCUMENT),
            resources: Default::default(),
            metadata,
            source: None,
        };

        let result = emit(&doc).unwrap();
        let xml = String::from_utf8(result.value).unwrap();
        assert!(xml.contains("<title>Test Document</title>"));
    }

    #[test]
    fn test_roundtrip_through_reader() {
        let docbook = r#"<?xml version="1.0"?>
<article><title>T</title><para>Hello <emphasis>world</emphasis></para></article>"#;
        let parsed = rescribe_read_docbook::parse(docbook).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(xml.contains("<para>Hello <emphasis>world</emphasis></para>"));
    }
}
