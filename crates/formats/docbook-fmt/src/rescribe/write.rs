//! rescribe → DocBook writer.
//!
//! Translates rescribe's document IR into `crate::DocBookDoc` (the
//! standalone DocBook/XML AST from this crate) and serializes it via
//! `crate::emit`. All XML writing lives in the rest of `docbook-fmt` —
//! this module is a thin IR↔AST translator only (per CLAUDE.md's "adapter
//! layer must never contain parsing or writing logic" rule).
//!
//! # Example
//!
//! ```
//! use docbook_fmt::rescribe::emit;
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

use crate::{DocBookDoc, Node as DbNode, XmlDecl};
use rescribe_core::{ConversionResult, Document, EmitError, Node, PropValue};
use rescribe_format_api::Emit as _;
use rescribe_std::{node, prop};

/// Emit a document to DocBook XML.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let warnings = Vec::new();

    let mut root_children = Vec::new();
    // Any metadata key ending in `_raw` is a whole-subtree verbatim capture
    // of an unmodeled `<info>` front-matter element (see the `read`
    // sibling module's `convert_children`/`extract_metadata` —
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
    let mut info: Option<DbNode> = None;
    if title.is_some() || !info_raw_fields.is_empty() {
        let mut info_children = Vec::new();
        if let Some(title) = title {
            info_children.push(db_element("title", vec![], vec![db_text(title)]));
        }
        // Splice back every raw-captured `<info>` subtree byte-for-byte
        // (see the `read` sibling module's `convert_children`/
        // `extract_metadata` — any `{tag}_raw` metadata field, e.g.
        // `author_raw` or `revhistory_raw`). This is lossless where
        // reconstructing the element from its flattened text would not be;
        // sorted by tag for deterministic output since `Properties`
        // iterates in unspecified order.
        for (_, raw) in &info_raw_fields {
            info_children.push(DbNode::Raw {
                content: (*raw).to_string(),
                span: crate::Span::NONE,
            });
        }
        info = Some(db_element("info", vec![], info_children));
    }

    let xmlns_attrs = vec![
        (
            "xmlns".to_string(),
            "http://docbook.org/ns/docbook".to_string(),
        ),
        ("version".to_string(), "5.0".to_string()),
    ];

    // If the document's content is exactly one structural sectioning
    // container (see `is_structural_sectioning_tag`) — the overwhelmingly
    // common case, since the `read` sibling module's `parse` wraps a whole
    // parsed file in exactly one such `DIV` — reuse *its* tag as the XML
    // root instead of always hardcoding `<article>`. Before `docbook:tag`
    // was added to these DIVs, hardcoding `<article>` was the only
    // information available here at all, but it silently turned every
    // `<book>`/`<chapter>`/`<part>`/`<appendix>` root back into `<article>`
    // on round-trip — the exact bug this reader/writer change fixes. Any
    // other shape of `doc.content` (multiple top-level children, or a
    // top-level child this writer doesn't recognize as a root — e.g. a
    // document built programmatically rather than round-tripped) falls
    // back to the pre-existing synthesized-`<article>`-wrapper behavior.
    let root = match doc.content.children.as_slice() {
        [only] if only.kind.as_str() == node::DIV => match only.props.get_str("docbook:tag") {
            Some(tag) if is_structural_sectioning_tag(tag) => {
                let tag = tag.to_string();
                write_sectioning_container(&tag, only, xmlns_attrs, info.into_iter().collect())
            }
            _ => {
                root_children.extend(info);
                for child in &doc.content.children {
                    root_children.extend(write_node(child));
                }
                db_element("article", xmlns_attrs, root_children)
            }
        },
        _ => {
            root_children.extend(info);
            for child in &doc.content.children {
                root_children.extend(write_node(child));
            }
            db_element("article", xmlns_attrs, root_children)
        }
    };

    let doc_ast = DocBookDoc {
        xml_decl: Some(XmlDecl {
            version: "1.0".to_string(),
            encoding: Some("UTF-8".to_string()),
            standalone: None,
        }),
        nodes: vec![root],
    };

    let bytes = doc_ast.emit();
    Ok(ConversionResult::with_warnings(bytes, warnings))
}

fn db_element(name: &str, attrs: Vec<(String, String)>, children: Vec<DbNode>) -> DbNode {
    DbNode::Element {
        name: name.to_string(),
        attrs,
        children,
        // No real source syntax to preserve here — this element is
        // synthesized from the rescribe IR, not parsed from DocBook bytes —
        // so prefer the self-closing form for an empty element, matching
        // this adapter's pre-existing output exactly (emit() previously
        // always wrote `<foo/>` for an empty element regardless of this
        // field, which didn't exist yet).
        self_closing: true,
        span: crate::Span::NONE,
    }
}

fn db_text(content: impl Into<String>) -> DbNode {
    DbNode::Text {
        content: content.into(),
        span: crate::Span::NONE,
    }
}

/// Build the generic `id`/`role` attributes for a node, if the IR node
/// carries the corresponding raw-preserved properties (see the `read`
/// sibling module's `attach_generic_attrs` for the reader side of this
/// round trip).
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

/// Build a `<equation>`/`<informalequation>`/`<inlineequation>`'s children
/// from a `math_display`/`math_inline` node, per the DocBook 5.2 content
/// model (`alt?, (mediaobject+ | mathphrase+ | mml:*+)`, optionally preceded
/// by a title — see the `read` sibling module's `build_math_node`'s doc
/// comment for the full reader-side rationale this mirrors):
/// - A `CAPTION` child (the equation's own `<title>`) re-emits via
///   `write_node` regardless of position — a title is always block-shaped
///   even inside an inline equation (which in practice won't have one).
/// - An `IMAGE` child re-emits via `write_child` — `write_node` for
///   `math_display` (producing `<mediaobject>`) or `write_inline` for
///   `math_inline` (producing `<inlinemediaobject>`), matching each
///   content model's own alternative.
/// - `math:format == "mathml"` re-splices the raw-captured `math:source`
///   verbatim via `DbNode::Raw` (same mechanism as the raw-preserved
///   `<info>` header fields above).
/// - Any other remaining children (the `<mathphrase>` case: real inline
///   markup, not a flat string — see `build_math_node`) are collected and
///   re-wrapped in a single `<mathphrase>`.
fn equation_children(node: &Node, write_child: fn(&Node) -> Vec<DbNode>) -> Vec<DbNode> {
    let mut out = Vec::new();
    let mut mathphrase_kids = Vec::new();
    for child in &node.children {
        match child.kind.as_str() {
            node::CAPTION => out.extend(write_node(child)),
            node::IMAGE => out.extend(write_child(child)),
            _ => mathphrase_kids.push(child),
        }
    }
    if node.props.get_str("math:format") == Some("mathml")
        && let Some(source) = node.props.get_str("math:source")
    {
        out.push(DbNode::Raw {
            content: source.to_string(),
            span: crate::Span::NONE,
        });
    }
    if !mathphrase_kids.is_empty() {
        let phrase_children = mathphrase_kids
            .iter()
            .flat_map(|c| write_inline(c))
            .collect();
        out.push(db_element("mathphrase", vec![], phrase_children));
    }
    out
}

/// Splice a `code_block`'s `docbook:callout-markers` (see the `read`
/// sibling module's `extract_verbatim_text`) back into its flat `content`
/// string as alternating text/`<co/>` DocBook AST nodes, ordered by each
/// marker's recorded character offset. Falls back to a single text node
/// (the previous, marker-less behavior) when the property is absent or
/// malformed.
fn write_verbatim_children(content: &str, markers_prop: Option<&PropValue>) -> Vec<DbNode> {
    let Some(PropValue::List(markers)) = markers_prop else {
        return vec![db_text(content)];
    };
    let mut entries: Vec<(usize, Option<String>, Option<String>)> = markers
        .iter()
        .filter_map(|m| {
            let PropValue::Map(map) = m else {
                return None;
            };
            let offset = match map.get("offset") {
                Some(PropValue::Int(i)) if *i >= 0 => *i as usize,
                _ => return None,
            };
            let id = match map.get("id") {
                Some(PropValue::String(s)) => Some(s.clone()),
                _ => None,
            };
            let label = match map.get("label") {
                Some(PropValue::String(s)) => Some(s.clone()),
                _ => None,
            };
            Some((offset, id, label))
        })
        .collect();
    if entries.is_empty() {
        return vec![db_text(content)];
    }
    entries.sort_by_key(|(offset, ..)| *offset);

    let chars: Vec<char> = content.chars().collect();
    let mut out = Vec::new();
    let mut cursor = 0usize;
    for (offset, id, label) in entries {
        let offset = offset.min(chars.len());
        if offset > cursor {
            out.push(db_text(chars[cursor..offset].iter().collect::<String>()));
        }
        let mut attrs = Vec::new();
        if let Some(id) = id {
            attrs.push(("id".to_string(), id));
        }
        if let Some(label) = label {
            attrs.push(("label".to_string(), label));
        }
        out.push(db_element("co", attrs, vec![]));
        cursor = offset;
    }
    if cursor < chars.len() {
        out.push(db_text(chars[cursor..].iter().collect::<String>()));
    }
    out
}

/// Re-emit a `<programlistingco>`'s `<areaspec>` sibling from a
/// `code_block`'s `docbook:areaspec` map (see the `read` sibling module's
/// "areaspec" arm) — `id`/`units`/`otherunits` are `<areaspec>`'s own
/// attributes (DocBook 5.2 reference: it has no `label` of its own), and
/// `areas` is the ordered list of `<area>`/`<areaset>` children.
fn write_areaspec(map: &HashMap<String, PropValue>) -> DbNode {
    let mut attrs = Vec::new();
    for key in ["id", "units", "otherunits"] {
        if let Some(PropValue::String(v)) = map.get(key) {
            attrs.push((key.to_string(), v.clone()));
        }
    }
    let children = match map.get("areas") {
        Some(PropValue::List(items)) => items.iter().filter_map(write_area_entry).collect(),
        _ => vec![],
    };
    db_element("areaspec", attrs, children)
}

/// Re-emit one `<area>` or `<areaset>` from its `docbook:area`-shaped map
/// (see the `read` sibling module's "area"/"areaset" arms) — `kind` selects
/// which; an `<areaset>` recurses into its own nested `areas` list of plain
/// `<area>` entries.
fn write_area_entry(entry: &PropValue) -> Option<DbNode> {
    let PropValue::Map(map) = entry else {
        return None;
    };
    if map.get("kind") == Some(&PropValue::String("areaset".to_string())) {
        let mut attrs = Vec::new();
        for key in ["id", "units", "otherunits", "label"] {
            if let Some(PropValue::String(v)) = map.get(key) {
                attrs.push((key.to_string(), v.clone()));
            }
        }
        let nested = match map.get("areas") {
            Some(PropValue::List(items)) => items.iter().filter_map(write_area_entry).collect(),
            _ => vec![],
        };
        return Some(db_element("areaset", attrs, nested));
    }
    Some(write_area(map))
}

/// Re-emit a single `<area>` (DocBook 5.2 reference: EMPTY plus an optional
/// `<alt>` child) from its `docbook:area` map.
fn write_area(map: &HashMap<String, PropValue>) -> DbNode {
    let mut attrs = Vec::new();
    for key in ["id", "coords", "units", "otherunits", "label"] {
        if let Some(PropValue::String(v)) = map.get(key) {
            attrs.push((key.to_string(), v.clone()));
        }
    }
    let children = match map.get("alt") {
        Some(PropValue::String(alt)) => vec![db_element("alt", vec![], vec![db_text(alt.clone())])],
        _ => vec![],
    };
    db_element("area", attrs, children)
}

/// Convert one rescribe IR (block-level) node into zero or more DocBook AST
/// nodes.
/// Whether `tag` is one of the DocBook structural sectioning containers
/// that the `read` sibling module tags a `DIV` with via `docbook:tag`
/// (article/book/chapter/part/appendix/section/sect1-5/simplesect — see
/// that reader's "Document level"/"Sections" `convert_element` arms).
/// These need [`write_sectioning_container`]'s title-extraction treatment
/// rather than the generic `Some(tag) =>` handling the rest of `docbook:tag`
/// uses, because their own `<title>` reads in as a `HEADING` child (per
/// `heading_level_for_parent`) — letting that `HEADING` reach the generic
/// `node::HEADING` write arm would wrap it in a synthesized `<sectN>`,
/// double-nesting a spurious section inside the real one.
fn is_structural_sectioning_tag(tag: &str) -> bool {
    matches!(
        tag,
        "article"
            | "book"
            | "chapter"
            | "part"
            | "appendix"
            | "section"
            | "sect1"
            | "sect2"
            | "sect3"
            | "sect4"
            | "sect5"
            | "simplesect"
    )
}

/// Re-emit a DocBook structural sectioning container (`<book>`, `<chapter>`,
/// `<sect1>`, `<bibliography>`, ...) as `tag` wrapping its children, with
/// its own leading `HEADING` child (its `<title>`, per
/// `heading_level_for_parent`) converted directly to a `<title>` in natural
/// position instead of falling through to the generic `node::HEADING` write
/// arm — which always synthesizes its own `<sectN>` wrapper and would
/// double-wrap the title in a spurious nested section, and (for `<book>`/
/// `<chapter>`/etc, which the generic arm can't produce at all) would
/// silently discard the container's real body content that follows the
/// title, since that content is a sibling of the DIV/BIBLIOGRAPHY node the
/// generic arm never sees.
///
/// Only the first `HEADING` child is treated as the title — later
/// `HEADING`s belong to nested sections and go through the ordinary
/// `write_node` recursion (a nested section is its own separately-tagged
/// `DIV`, so its `HEADING` is a child of *that* `DIV`, never a direct
/// sibling here — but the position-based flag is kept as defense in depth).
///
/// `extra_attrs`/`leading_children` let [`emit`] fold in the document-root
/// concerns (the `xmlns`/`version` attributes, and a synthesized `<info>`
/// front-matter block from `doc.metadata`) when this container is also the
/// XML document root, without duplicating the title-extraction loop.
fn write_sectioning_container(
    tag: &str,
    node: &Node,
    extra_attrs: Vec<(String, String)>,
    leading_children: Vec<DbNode>,
) -> DbNode {
    let mut attrs = extra_attrs;
    attrs.extend(generic_attrs(node));
    let mut db_children = leading_children;
    let mut title_written = false;
    for child in &node.children {
        if !title_written && child.kind.as_str() == node::HEADING {
            db_children.push(db_element(
                "title",
                vec![],
                child.children.iter().flat_map(write_inline).collect(),
            ));
            title_written = true;
        } else {
            db_children.extend(write_node(child));
        }
    }
    db_element(tag, attrs, db_children)
}

fn write_node(node: &Node) -> Vec<DbNode> {
    match node.kind.as_str() {
        node::DOCUMENT => node.children.iter().flat_map(write_node).collect(),

        // A `div` tagged `docbook:tag` is either a structural sectioning
        // container (article/book/chapter/part/appendix/section/sect1-5/
        // simplesect — see the `read` sibling module's `convert_element`'s
        // "Document level"/"Sections" arms) or a `generic_div` (an
        // unrecognized block-level element raw-preserved by the reader's
        // catch-all, see that module's `generic_div`) — either way
        // re-emit its original tag name. A `div` with no `docbook:tag` at
        // all (pre-existing content from before every DIV carried one, or a
        // synthetic wrapper) just flattens into its children, same as
        // `DOCUMENT`.
        node::DIV => match node.props.get_str("docbook:tag") {
            // `<programlistingco>` (see the `read` sibling module's
            // "programlistingco" arm): content model `areaspec?,
            // programlisting`. Its (possibly areaspec-augmented) `code_block`
            // child carries `docbook:areaspec` if the source had an
            // `<areaspec>` — pulled out here and re-emitted as the `
            // <areaspec>` sibling *before* the listing, matching the content
            // model's order; `write_node` on the code_block itself
            // separately handles splicing any `<co/>` markers back into the
            // listing's text (see the `CODE_BLOCK` arm below), independent
            // of whether an `<areaspec>` is also present.
            Some("programlistingco") => {
                let areaspec = node.children.iter().find_map(|c| {
                    if c.kind.as_str() != node::CODE_BLOCK {
                        return None;
                    }
                    match c.props.get("docbook:areaspec") {
                        Some(PropValue::Map(map)) => Some(write_areaspec(map)),
                        _ => None,
                    }
                });
                let mut children: Vec<DbNode> = areaspec.into_iter().collect();
                children.extend(node.children.iter().flat_map(write_node));
                vec![db_element(
                    "programlistingco",
                    generic_attrs(node),
                    children,
                )]
            }
            Some(tag) if is_structural_sectioning_tag(tag) => {
                vec![write_sectioning_container(tag, node, vec![], vec![])]
            }
            Some(tag) => {
                let mut attrs = generic_attrs(node);
                // `docbook:qanda-defaultlabel` (see the `read` sibling
                // module's "qandaset" arm) is only meaningful on
                // `<qandaset>` itself.
                if tag == "qandaset"
                    && let Some(label) = node.props.get_str("docbook:qanda-defaultlabel")
                {
                    attrs.push(("defaultlabel".to_string(), label.to_string()));
                }
                vec![db_element(
                    tag,
                    attrs,
                    node.children.iter().flat_map(write_node).collect(),
                )]
            }
            // `html:class == "abstract"` (see the `read` sibling module's
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

        // A container's caption (see the `read` sibling module's
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

        // `docbook:tag == "bridgehead"` (see the `read` sibling module's
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

        // <attribution> (see the `read` sibling module's "attribution" arm)
        // — phrase-level content, so re-emitted via write_inline like
        // CAPTION, not write_node.
        "docbook:attribution" => vec![db_element(
            "attribution",
            vec![],
            node.children.iter().flat_map(write_inline).collect(),
        )],

        node::LIST => {
            // `docbook:tag` = "procedure"/"substeps"/"calloutlist" (see the
            // `read` sibling module's "procedure"|"substeps"|"calloutlist"
            // arms) re-emits the original element instead of
            // `<orderedlist>`.
            let tag = match node.props.get_str("docbook:tag") {
                Some(tag @ ("procedure" | "substeps" | "calloutlist")) => tag,
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

        node::LIST_ITEM => {
            let tag = node.props.get_str("docbook:tag").unwrap_or("listitem");
            let mut attrs = Vec::new();
            // `arearefs` (see the `read` sibling module's "callout" arm)
            // only applies to `<callout>`, the `calloutlist`-flavored
            // `LIST_ITEM`.
            if tag == "callout"
                && let Some(arearefs) = node.props.get_str("docbook:arearefs")
            {
                attrs.push(("arearefs".to_string(), arearefs.to_string()));
            }
            vec![db_element(
                tag,
                attrs,
                node.children.iter().flat_map(write_node).collect(),
            )]
        }

        node::DEFINITION_LIST => write_definition_list(node),

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
            // (see the `read` sibling module's "programlisting"|"screen"|
            // "literallayout"|"synopsis"|"address" arm) — defaults to
            // `programlisting` for CODE_BLOCK nodes built directly by
            // non-DocBook producers.
            let tag = node
                .props
                .get_str("docbook:tag")
                .unwrap_or("programlisting");
            // `docbook:callout-markers` (see the `read` sibling module's
            // "programlisting"|... arm / `extract_verbatim_text`) splices
            // `<co/>` markers back into the text at their recorded offsets;
            // absent it, this is exactly the previous single-text-node
            // behavior.
            let children =
                write_verbatim_children(content, node.props.get("docbook:callout-markers"));
            vec![db_element(tag, attrs, children)]
        }

        node::TABLE => {
            // Split children back into <colspec> siblings (see the `read`
            // sibling module's dedicated "colspec" arm — a structured
            // "docbook:colspec"-kind child, not a table row) and actual
            // row children.
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
            // one) — see the `read` sibling module's `"title" if parent ==
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
            // (additional rows spanned) — see the `read` sibling module's
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

        // See the `read` sibling module's `build_math_node`'s doc comment
        // for the full DocBook 5.2 content-model rationale this mirrors on
        // write. `<equation>` (has a `CAPTION` child, from an original
        // `<title>`) vs `<informalequation>` (doesn't) — same
        // title-presence tag selection convention as the
        // `TABLE`/`informaltable` arm above.
        "math_display" => {
            let has_caption = node
                .children
                .iter()
                .any(|c| c.kind.as_str() == node::CAPTION);
            let tag = if has_caption {
                "equation"
            } else {
                "informalequation"
            };
            vec![db_element(tag, vec![], equation_children(node, write_node))]
        }

        node::HORIZONTAL_RULE => Vec::new(), // DocBook doesn't have HR

        node::FOOTNOTE_DEF => vec![db_element(
            "footnote",
            vec![],
            node.children.iter().flat_map(write_node).collect(),
        )],

        // Same title-double-wrap risk as the structural `DIV`s handled by
        // `write_sectioning_container` above: `<bibliography>`'s own
        // `<title>` reads in as a `HEADING` too (`heading_level_for_parent`
        // gives it level 1, same as `<chapter>`) but `BIBLIOGRAPHY` is a
        // dedicated node kind, not a `docbook:tag`-carrying `DIV`, so it
        // needs the same treatment applied directly rather than via the
        // `DIV` match arm. Found while fixing the `DIV` case (same root
        // cause), not part of the audit's named findings.
        node::BIBLIOGRAPHY => vec![write_sectioning_container(
            "bibliography",
            node,
            vec![],
            vec![],
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

        // A `generic_span` (see the `read` sibling module's `generic_span`
        // — an unrecognized inline element, e.g. `<arg>` inside
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

/// Write a `definition_list`'s flat, run-grouped children back to their
/// DocBook shape. Per the `read` sibling module's convention (see its
/// `"variablelist"`/`"varlistentry"`/`"qandaset"`/`"qandaentry"` doc
/// comments — the same flat shape `rescribe-read-markdown`/
/// `rescribe-read-html` already use for `definition_list`), a group is
/// one-or-more consecutive `DEFINITION_TERM` followed by zero-or-more
/// consecutive `DEFINITION_DESC`, both direct children of `DEFINITION_LIST`
/// with no wrapper node in between; each group is one logical entry.
///
/// Two flavors, selected by `docbook:list-kind` (set by the reader only for
/// a list synthesized from a `<qandaset>`/`<qandadiv>`'s `<qandaentry>`
/// children — see `wrap_qanda_entries`):
/// - Default (from `<variablelist>`): each group becomes one
///   `<varlistentry>` with N `<term>`s and M `<listitem>`s, all wrapped in
///   one `<variablelist>`.
/// - `"qanda"`: each group becomes one `<qandaentry>` with N `<question>`s
///   and M `<answer>`s (per the content model N is always 1 for a
///   `qandaentry` this reader produced, but multiple is handled the same
///   way rather than special-cased), spliced directly into the return value
///   with no extra wrapper element — `wrap_qanda_entries` only introduces
///   the `DEFINITION_LIST` as an IR-side convenience; it has no DocBook
///   element of its own, unlike `<variablelist>`.
fn write_definition_list(node: &Node) -> Vec<DbNode> {
    let qanda = node.props.get_str("docbook:list-kind") == Some("qanda");
    let (term_tag, desc_tag, entry_tag) = if qanda {
        ("question", "answer", "qandaentry")
    } else {
        ("term", "listitem", "varlistentry")
    };
    let mut entries = Vec::new();
    let mut i = 0;
    let n = node.children.len();
    while i < n {
        if node.children[i].kind.as_str() != node::DEFINITION_TERM {
            // Defensive: a non-term child shouldn't appear directly in a
            // definition_list per the flat run-grouped convention above,
            // but don't silently drop it if one somehow does.
            entries.extend(write_node(&node.children[i]));
            i += 1;
            continue;
        }
        let mut kids = Vec::new();
        while i < n && node.children[i].kind.as_str() == node::DEFINITION_TERM {
            // `<term>` (variablelist) is phrase/inline content, but
            // `<question>` (qandaset) is block content (DocBook 5.2
            // reference: `question ::= label?, (block content)+`, the same
            // shape as `<answer>`) — its `para` children must round-trip as
            // real `<para>` elements, not be flattened through
            // `write_inline`, which would silently drop the wrapper.
            let term_children: Vec<DbNode> = if qanda {
                node.children[i]
                    .children
                    .iter()
                    .flat_map(write_node)
                    .collect()
            } else {
                node.children[i]
                    .children
                    .iter()
                    .flat_map(write_inline)
                    .collect()
            };
            kids.push(db_element(term_tag, vec![], term_children));
            i += 1;
        }
        while i < n && node.children[i].kind.as_str() == node::DEFINITION_DESC {
            kids.push(db_element(
                desc_tag,
                vec![],
                node.children[i]
                    .children
                    .iter()
                    .flat_map(write_node)
                    .collect(),
            ));
            i += 1;
        }
        entries.push(db_element(entry_tag, vec![], kids));
    }
    if qanda {
        entries
    } else {
        vec![db_element("variablelist", vec![], entries)]
    }
}

/// Write a `bibliography_entry` node back to `<biblioentry>`/`<bibliomixed>`/
/// `<biblioset>`/`<bibliomset>` (see the `read` sibling module's
/// `build_bibliography_entry` — `docbook:tag` remembers which one the
/// original element was; defaults to `<biblioentry>` for an entry built by a
/// non-DocBook producer). `prop::DATE` (a structured year/month/day map, see
/// its own doc comment) becomes a leading `<date>` child; a `page_first` +
/// `page_last` pair of sibling fields recombines into one `<pagenums>`
/// (see the `read` sibling module's `convert_pagenums` for the reader-side
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
        // directly between fields (see the `read` sibling module's
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
            && let Some(last) =
                iter.next_if(|next| next.props.get_str(prop::FIELD_ROLE) == Some("page_last"))
        {
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
/// element. `docbook:tag` (set by every arm of the `read` sibling module's
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
/// doc comment) back into an ISO 8601 string — the inverse of the `read`
/// sibling module's `parse_bibliographic_date`. Zero-padded to two digits
/// for month/day per ISO 8601 (`2020-03-05`, not `2020-3-5`).
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

        // DocBook has no dedicated strikethrough element; `role="strikethrough"`
        // on `<emphasis>` is the standard convention (mirroring the
        // `role="strong"` convention just above for STRONG) — without this
        // arm, a `strikeout` node fed from a non-DocBook reader (e.g.
        // pandoc-json) fell through to the generic "unknown inline -
        // recurse" catch-all below, which silently dropped the wrapper
        // entirely and emitted only the bare text, losing the strikeout
        // markup altogether.
        node::STRIKEOUT => vec![db_element(
            "emphasis",
            vec![("role".to_string(), "strikethrough".to_string())],
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

        // Same content-model handling as the block `"math_display"` arm
        // above (see the `read` sibling module's `build_math_node`'s doc
        // comment), using `write_inline` throughout so an `IMAGE` child
        // becomes `<inlinemediaobject>` (not `<mediaobject>`) per
        // `<inlineequation>`'s content model.
        "math_inline" => vec![db_element(
            "inlineequation",
            vec![],
            equation_children(node, write_inline),
        )],

        // A raw entity reference preserved by the reader: re-emit verbatim.
        node::RAW_INLINE => match node.props.get_str("docbook:entity") {
            Some(name) => vec![DbNode::EntityRef {
                name: name.to_string(),
                span: crate::Span::NONE,
            }],
            None => node.children.iter().flat_map(write_inline).collect(),
        },

        // A `span` tagged `docbook:tag` is a `generic_span` (an
        // unrecognized inline-level element raw-preserved by the reader's
        // catch-all, see the `read` sibling module's `generic_span`) —
        // re-emit its original tag name.
        node::SPAN => match node.props.get_str("docbook:tag") {
            Some(tag) => vec![db_element(
                tag,
                generic_attrs(node),
                node.children.iter().flat_map(write_inline).collect(),
            )],
            None => node.children.iter().flat_map(write_inline).collect(),
        },

        // A `div` landing in an inline-content position — e.g. a
        // raw-preserved block-shaped element like `<entrytbl>` (a table
        // nested inside a table cell; see `is_block_element`'s `entrytbl`
        // entry) that ends up as a `TABLE_CELL`/`TABLE_HEADER` child, which
        // writes its children via `write_inline`, not `write_node` — must
        // not fall through to the generic "unknown inline - recurse"
        // catch-all below: that catch-all also recurses via `write_inline`
        // for every descendant, which has no arm for further block node
        // kinds either (`TABLE_ROW`, `TABLE_CELL`, `PARAGRAPH`, ...),
        // silently flattening the whole subtree down to bare text and
        // losing every intermediate tag — found while fixture-testing the
        // `entrytbl` block-classification fix, not part of the audit's
        // named findings. `write_node`'s `DIV` arm already has the correct
        // tag/title/children dispatch for arbitrarily-nested block content
        // regardless of which context invoked it, so delegate to it
        // directly rather than duplicating that dispatch here.
        node::DIV => write_node(node),

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
        let parsed = super::super::read::parse(docbook).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(xml.contains("<para>Hello <emphasis>world</emphasis></para>"));
    }

    /// An `<equation>` with embedded `<mml:math>` MathML must round-trip
    /// byte-for-byte through parse -> emit -> reparse: the `mml:math`
    /// subtree is raw-preserved (see the `read` sibling module's
    /// `mathml-raw` sentinel / `split_mathml`) and re-spliced verbatim on
    /// write (see `equation_children`'s `DbNode::Raw` branch), so a second
    /// parse must recover the exact same `math:source`.
    #[test]
    fn test_roundtrip_mathml_equation() {
        let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<article xmlns="http://docbook.org/ns/docbook"><equation><title>T</title><mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>x</mml:mi></mml:math></equation></article>"#;
        let parsed = super::super::read::parse(docbook).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(
            xml.contains(r#"<mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>x</mml:mi></mml:math>"#),
            "emitted XML missing raw mml:math: {xml}"
        );
        assert!(
            xml.contains("<equation>"),
            "expected <equation> (has title): {xml}"
        );

        let reparsed = super::super::read::parse(&xml).unwrap();
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

    /// An `<informalequation>` with a `<mathphrase>` must round-trip its
    /// nested inline markup (e.g. `<superscript>`) as real child nodes, not
    /// flattened text — see the `read` sibling module's `build_math_node`'s
    /// doc comment.
    #[test]
    fn test_roundtrip_mathphrase() {
        let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<article xmlns="http://docbook.org/ns/docbook"><informalequation><mathphrase>x<superscript>n</superscript></mathphrase></informalequation></article>"#;
        let parsed = super::super::read::parse(docbook).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(
            xml.contains("<informalequation><mathphrase>x<superscript>n</superscript></mathphrase></informalequation>"),
            "unexpected emitted XML: {xml}"
        );
    }

    /// `<inlineequation>` with `<mml:math>` round-trips via
    /// `<inlinemediaobject>`-style inline write path, producing
    /// `math_inline` with `math:format = "mathml"` on reparse.
    #[test]
    fn test_roundtrip_mathml_inlineequation() {
        let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<article xmlns="http://docbook.org/ns/docbook"><para>x is <inlineequation><mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>x</mml:mi></mml:math></inlineequation>.</para></article>"#;
        let parsed = super::super::read::parse(docbook).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(
            xml.contains(r#"<mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>x</mml:mi></mml:math>"#),
            "emitted XML missing raw mml:math: {xml}"
        );

        let reparsed = super::super::read::parse(&xml).unwrap();
        let para = &reparsed.value.content.children[0].children[0];
        let formula = &para.children[1];
        assert_eq!(formula.kind.as_str(), "math_inline");
        assert_eq!(formula.props.get_str("math:format"), Some("mathml"));
    }

    /// Regression test for a pre-existing bug: `<variablelist>` with more
    /// than one `<varlistentry>` used to write back corrupted (all entries'
    /// `<term>`s/`<listitem>`s bled into a single `<varlistentry>`), because
    /// the old `DEFINITION_LIST` write arm assumed direct children were a
    /// flat alternating `[term, desc, term, desc, ...]` sequence while the
    /// reader actually nested each entry inside a `docbook:varlistentry`
    /// wrapper node the writer had no arm for (silently dropped by the
    /// generic catch-all). Both sides now agree on the flat, run-grouped
    /// shape (see the `read` sibling module's `"varlistentry"` doc comment
    /// and `write_definition_list`) — this asserts a full
    /// parse -> emit -> reparse round trip is lossless for 2+ entries.
    #[test]
    fn test_roundtrip_multi_entry_variablelist() {
        let docbook = r#"<?xml version="1.0"?>
<article xmlns="http://docbook.org/ns/docbook">
  <variablelist>
    <varlistentry>
      <term>Term1</term>
      <listitem><para>Def1.</para></listitem>
    </varlistentry>
    <varlistentry>
      <term>Term2</term>
      <listitem><para>Def2.</para></listitem>
    </varlistentry>
  </variablelist>
</article>"#;
        let parsed = super::super::read::parse(docbook).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(
            xml.contains(
                "<variablelist><varlistentry><term>Term1</term><listitem><para>Def1.</para></listitem></varlistentry><varlistentry><term>Term2</term><listitem><para>Def2.</para></listitem></varlistentry></variablelist>"
            ),
            "entries bled together or were dropped: {xml}"
        );
        let reparsed = super::super::read::parse(&xml).unwrap();
        assert_eq!(parsed.value.content, reparsed.value.content);
    }

    /// A `<varlistentry>` with multiple `<term>`s sharing one `<listitem>`
    /// (DocBook 5.2 reference permits this — multiple terms for one
    /// definition) must round-trip as one run: 2 `DEFINITION_TERM` siblings
    /// followed by 1 `DEFINITION_DESC`, grouped back into a single
    /// `<varlistentry>`.
    #[test]
    fn test_roundtrip_variablelist_multiple_terms_one_desc() {
        let docbook = r#"<?xml version="1.0"?>
<article xmlns="http://docbook.org/ns/docbook">
  <variablelist>
    <varlistentry>
      <term>TermA</term>
      <term>TermB</term>
      <listitem><para>Shared definition.</para></listitem>
    </varlistentry>
  </variablelist>
</article>"#;
        let parsed = super::super::read::parse(docbook).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(
            xml.contains(
                "<variablelist><varlistentry><term>TermA</term><term>TermB</term><listitem><para>Shared definition.</para></listitem></varlistentry></variablelist>"
            ),
            "unexpected emitted XML: {xml}"
        );
        let reparsed = super::super::read::parse(&xml).unwrap();
        assert_eq!(parsed.value.content, reparsed.value.content);
    }

    /// `<qandaset defaultlabel="...">` with multiple `<qandaentry>`s —
    /// including one with two `<answer>`s and one with zero (DocBook 5.2
    /// reference explicitly permits an unanswered question) — must
    /// round-trip losslessly: `defaultlabel`, the question/answer pairing,
    /// and the per-question answer count all survive.
    #[test]
    fn test_roundtrip_qandaset_multi_entry() {
        let docbook = r#"<?xml version="1.0"?>
<article xmlns="http://docbook.org/ns/docbook">
  <qandaset defaultlabel="qanda">
    <qandaentry>
      <question><para>Q1?</para></question>
      <answer><para>A1.</para></answer>
      <answer><para>A1b.</para></answer>
    </qandaentry>
    <qandaentry>
      <question><para>Q2, unanswered?</para></question>
    </qandaentry>
  </qandaset>
</article>"#;
        let parsed = super::super::read::parse(docbook).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(
            xml.contains(r#"<qandaset defaultlabel="qanda">"#),
            "defaultlabel dropped: {xml}"
        );
        assert!(
            xml.contains(
                "<qandaentry><question><para>Q1?</para></question><answer><para>A1.</para></answer><answer><para>A1b.</para></answer></qandaentry>"
            ),
            "multi-answer entry corrupted: {xml}"
        );
        assert!(
            xml.contains(
                "<qandaentry><question><para>Q2, unanswered?</para></question></qandaentry>"
            ),
            "unanswered question not preserved: {xml}"
        );
        let reparsed = super::super::read::parse(&xml).unwrap();
        assert_eq!(parsed.value.content, reparsed.value.content);
    }

    /// `<qandaset>` containing nested `<qandadiv>`s (each with its own
    /// title and `<qandaentry>`s) must round-trip: `DIV` nests naturally
    /// (the same pattern `generic_div`/sectioning containers already use),
    /// so no dedicated node kind is required — see `wrap_qanda_entries`.
    #[test]
    fn test_roundtrip_qandaset_nested_qandadiv() {
        let docbook = r#"<?xml version="1.0"?>
<article xmlns="http://docbook.org/ns/docbook">
  <qandaset>
    <qandadiv>
      <title>Section One</title>
      <qandaentry>
        <question><para>Q1?</para></question>
        <answer><para>A1.</para></answer>
      </qandaentry>
    </qandadiv>
    <qandadiv>
      <title>Section Two</title>
      <qandaentry>
        <question><para>Q2?</para></question>
        <answer><para>A2.</para></answer>
      </qandaentry>
    </qandadiv>
  </qandaset>
</article>"#;
        let parsed = super::super::read::parse(docbook).unwrap();
        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(
            xml.contains(
                "<qandaset><qandadiv><title>Section One</title><qandaentry><question><para>Q1?</para></question><answer><para>A1.</para></answer></qandaentry></qandadiv><qandadiv><title>Section Two</title><qandaentry><question><para>Q2?</para></question><answer><para>A2.</para></answer></qandaentry></qandadiv></qandaset>"
            ),
            "unexpected emitted XML: {xml}"
        );
        let reparsed = super::super::read::parse(&xml).unwrap();
        assert_eq!(parsed.value.content, reparsed.value.content);
    }

    /// Inline-marker flavor of the callout composition (see the `read`
    /// sibling module's `"co"`/`extract_verbatim_text` doc comments):
    /// `<co/>` markers embedded directly in a bare `<programlisting>`
    /// (no `<programlistingco>`/`<areaspec>` wrapper needed — DocBook 5.2
    /// permits `<co>` wherever `%co.class;` appears in the verbatim
    /// elements' own content model), paired with a sibling `<calloutlist>`
    /// that references the markers' ids directly via `arearefs`.
    #[test]
    fn test_roundtrip_inline_co_markers() {
        let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<article xmlns="http://docbook.org/ns/docbook"><example><programlisting>a<co xml:id="co.1"/>b<co xml:id="co.2" label="2"/>c</programlisting><calloutlist><callout arearefs="co.1"><para>one</para></callout><callout arearefs="co.2"><para>two</para></callout></calloutlist></example></article>"#;
        let parsed = super::super::read::parse(docbook).unwrap();

        let example = &parsed.value.content.children[0].children[0];
        let code_block = &example.children[0];
        assert_eq!(code_block.kind.as_str(), node::CODE_BLOCK);
        assert_eq!(code_block.props.get_str(prop::CONTENT), Some("abc"));
        let markers = match code_block.props.get("docbook:callout-markers") {
            Some(PropValue::List(m)) => m,
            other => panic!("expected callout-markers list, got {other:?}"),
        };
        assert_eq!(markers.len(), 2);
        let PropValue::Map(m0) = &markers[0] else {
            panic!("marker 0 not a map")
        };
        assert_eq!(m0.get("id"), Some(&PropValue::String("co.1".to_string())));
        assert_eq!(m0.get("offset"), Some(&PropValue::Int(1)));
        let PropValue::Map(m1) = &markers[1] else {
            panic!("marker 1 not a map")
        };
        assert_eq!(m1.get("id"), Some(&PropValue::String("co.2".to_string())));
        assert_eq!(m1.get("offset"), Some(&PropValue::Int(2)));
        assert_eq!(m1.get("label"), Some(&PropValue::String("2".to_string())));

        let calloutlist = &example.children[1];
        assert_eq!(calloutlist.kind.as_str(), node::LIST);
        assert_eq!(
            calloutlist.props.get_str("docbook:tag"),
            Some("calloutlist")
        );
        let callout0 = &calloutlist.children[0];
        assert_eq!(callout0.kind.as_str(), node::LIST_ITEM);
        assert_eq!(callout0.props.get_str("docbook:tag"), Some("callout"));
        assert_eq!(callout0.props.get_str("docbook:arearefs"), Some("co.1"));

        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(
            xml.contains(
                r#"<programlisting>a<co id="co.1"/>b<co id="co.2" label="2"/>c</programlisting>"#
            ),
            "unexpected emitted programlisting: {xml}"
        );
        assert!(
            xml.contains(
                r#"<calloutlist><callout arearefs="co.1"><para>one</para></callout><callout arearefs="co.2"><para>two</para></callout></calloutlist>"#
            ),
            "unexpected emitted calloutlist: {xml}"
        );

        let reparsed = super::super::read::parse(&xml).unwrap();
        assert_eq!(parsed.value.content, reparsed.value.content);
    }

    /// External-coordinates flavor (see the `read` sibling module's
    /// `"programlistingco"`/`"areaspec"`/`"area"` doc comments):
    /// `<programlistingco>` wraps an `<areaspec>` (holding `<area>`
    /// coordinate records, no inline `<co/>` markers in the listing itself)
    /// and the `<programlisting>` it annotates, with a sibling
    /// `<calloutlist>` whose `<callout>`s reference the `<area>` ids.
    #[test]
    fn test_roundtrip_areaspec_external_coordinates() {
        let docbook = r#"<?xml version="1.0" encoding="UTF-8"?>
<article xmlns="http://docbook.org/ns/docbook"><example><programlistingco><areaspec units="linecolumn"><area xml:id="area.1" coords="1 1"/><area xml:id="area.2" coords="2 1" label="2"/></areaspec><programlisting>fn main() {
    print();
}</programlisting></programlistingco><calloutlist><callout arearefs="area.1"><para>Function definition.</para></callout><callout arearefs="area.2"><para>The call.</para></callout></calloutlist></example></article>"#;
        let parsed = super::super::read::parse(docbook).unwrap();

        let example = &parsed.value.content.children[0].children[0];
        let wrapper = &example.children[0];
        assert_eq!(wrapper.kind.as_str(), node::DIV);
        assert_eq!(
            wrapper.props.get_str("docbook:tag"),
            Some("programlistingco")
        );
        let code_block = &wrapper.children[0];
        assert_eq!(code_block.kind.as_str(), node::CODE_BLOCK);
        assert!(
            code_block
                .props
                .get_str(prop::CONTENT)
                .unwrap()
                .contains("fn main")
        );
        let areaspec = match code_block.props.get("docbook:areaspec") {
            Some(PropValue::Map(m)) => m,
            other => panic!("expected areaspec map, got {other:?}"),
        };
        assert_eq!(
            areaspec.get("units"),
            Some(&PropValue::String("linecolumn".to_string()))
        );
        let areas = match areaspec.get("areas") {
            Some(PropValue::List(a)) => a,
            other => panic!("expected areas list, got {other:?}"),
        };
        assert_eq!(areas.len(), 2);
        let PropValue::Map(a0) = &areas[0] else {
            panic!("area 0 not a map")
        };
        assert_eq!(a0.get("id"), Some(&PropValue::String("area.1".to_string())));
        assert_eq!(
            a0.get("coords"),
            Some(&PropValue::String("1 1".to_string()))
        );
        let PropValue::Map(a1) = &areas[1] else {
            panic!("area 1 not a map")
        };
        assert_eq!(a1.get("label"), Some(&PropValue::String("2".to_string())));

        let emitted = emit(&parsed.value).unwrap();
        let xml = String::from_utf8(emitted.value).unwrap();
        assert!(
            xml.contains(r#"<areaspec units="linecolumn"><area id="area.1" coords="1 1"/><area id="area.2" coords="2 1" label="2"/></areaspec>"#),
            "unexpected emitted areaspec: {xml}"
        );

        let reparsed = super::super::read::parse(&xml).unwrap();
        assert_eq!(parsed.value.content, reparsed.value.content);
    }
}
