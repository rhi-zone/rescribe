//! DocBook → rescribe reader.
//!
//! Translates `crate::DocBookDoc` (the standalone DocBook/XML AST from this
//! crate) into rescribe's document IR. Supports DocBook 5 and DocBook 4
//! elements.
//!
//! All XML tokenizing/parsing lives in the rest of `docbook-fmt` — this
//! module is a thin AST↔IR translator only (per CLAUDE.md's "adapter layer
//! must never contain parsing or writing logic" rule).
//!
//! # Example
//!
//! ```
//! use docbook_fmt::rescribe::parse;
//!
//! let docbook = r#"<?xml version="1.0"?>
//! <article xmlns="http://docbook.org/ns/docbook">
//!   <title>Example Article</title>
//!   <para>Hello, world!</para>
//! </article>"#;
//!
//! let result = parse(docbook).unwrap();
//! let doc = result.value;
//! ```

use std::collections::HashMap;

use crate::{DocBookDoc, Node as DbNode};
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, PropValue, Properties, Severity,
    WarningKind,
};
use rescribe_format_api::Parse as _;
use rescribe_std::{node, prop};

/// Parse DocBook XML into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, diagnostics) = DocBookDoc::parse(input.as_bytes());

    let mut warnings: Vec<FidelityWarning> = diagnostics
        .into_iter()
        .map(|d| {
            FidelityWarning::new(
                Severity::Major,
                WarningKind::FeatureLost("xml-parse-error".to_string()),
                format!("DocBook XML parse error: {}", d.message),
            )
        })
        .collect();

    let mut metadata = Properties::new();
    let mut children = Vec::new();
    for top in &doc.nodes {
        if let DbNode::Element {
            name,
            attrs,
            children: kids,
            ..
        } = top
        {
            let converted =
                convert_children(kids, name, false, false, &mut metadata, &mut warnings);
            match convert_element(name, attrs, converted.clone(), None) {
                Some(node) => children.push(node),
                None => {
                    // Root element itself carries no rescribe-level
                    // semantics (shouldn't normally happen for
                    // article/book/etc, but pass its children through
                    // rather than dropping them).
                    children.extend(converted);
                }
            }
        }
        // Leading/trailing Comment/PI/Doctype/whitespace-Text at the very
        // top level (outside the root element) carry no IR meaning and
        // have no cross-format equivalent to model; DocBook documents
        // otherwise consist of exactly one root element.
    }

    let document = Document {
        content: Node::new(node::DOCUMENT).children(children),
        resources: Default::default(),
        metadata,
        source: None,
    };

    Ok(ConversionResult::with_warnings(document, warnings))
}

/// Convert a slice of DocBook child nodes into rescribe IR nodes,
/// discarding nodes that only exist to be unwrapped (e.g. `<info>`, which
/// is consumed for metadata) and passing through "structural" wrapper
/// elements (like `<tgroup>`) as their own children.
///
/// `in_header` is true when `parent_name` is `<info>`/`<articleinfo>`/
/// `<bookinfo>` itself or a descendant of it (threaded down through the
/// recursion below) — i.e. whether the *children* of `parent_name` are
/// front-matter content that will end up consumed by [`extract_metadata`]
/// rather than surviving as document content nodes.
///
/// `in_biblio` is true when `parent_name` is a bibliographic citation
/// container (`<biblioentry>`/`<bibliomixed>`/`<biblioset>`/`<bibliomset>`,
/// or a descendant of one — threaded down the same way `in_header` is) —
/// i.e. whether the *children* of `parent_name` are citation sub-fields that
/// should be dispatched through [`convert_biblio_field`] (producing
/// `bibliography_field` nodes) rather than the generic [`convert_element`].
fn convert_children(
    children: &[DbNode],
    parent_name: &str,
    in_header: bool,
    in_biblio: bool,
    metadata: &mut Properties,
    warnings: &mut Vec<FidelityWarning>,
) -> Vec<Node> {
    let mut out = Vec::new();
    for child in children {
        match child {
            DbNode::Element {
                name,
                attrs,
                children: kids,
                ..
            } => {
                // A MathML root element inside `<equation>`/`<informalequation>`/
                // `<inlineequation>` is real MathML markup — the DocBook 5.2
                // reference's content model for all three
                // (https://tdg.docbook.org/tdg/5.2/equation.html /
                // inlineequation.html) is `alt? , (mediaobject+ | mathphrase+ |
                // mml:*+)`, i.e. mutually exclusive alternatives, not a
                // combination. Recursing through the normal pipeline here
                // would flatten the MathML element structure through the
                // generic catch-all and then destroy even that when the
                // `"equation"`/etc. arm below builds `math:source` — the same
                // class of bug fixed for `rescribe-read-jats`'s
                // `disp-formula`/`inline-formula`. Capture the whole MathML
                // subtree verbatim instead (the same `emit_fragment`
                // raw-preservation mechanism used for unmodeled `<info>`
                // children below) as a sentinel `SPAN` tagged
                // `docbook:tag = "mathml-raw"`, which `split_mathml` pulls
                // back out in the equation arms. Namespace prefix is matched
                // by local name (`is_mathml_root`), not hardcoded to `mml:`,
                // since `docbook-fmt`'s AST keeps whatever prefix the source
                // document declared (mirroring how `xlink:href` is matched
                // literally elsewhere in this reader) and DocBook documents
                // are not required to use the `mml:` prefix by convention the
                // way JATS's Tag Library documents do.
                if matches!(
                    parent_name,
                    "equation" | "informalequation" | "inlineequation"
                ) && is_mathml_root(name)
                {
                    let raw = String::from_utf8(crate::emit_fragment(std::slice::from_ref(child)))
                        .unwrap_or_default();
                    out.push(
                        Node::new(node::SPAN)
                            .prop("docbook:tag", "mathml-raw")
                            .prop(prop::CONTENT, raw),
                    );
                    continue;
                }
                let child_in_header =
                    in_header || matches!(name.as_str(), "info" | "articleinfo" | "bookinfo");
                // Whether *`name`'s own children* should also be dispatched
                // through `convert_biblio_field` (as opposed to normal
                // inline conversion): if we're not in biblio scope yet,
                // entering it requires `name` itself to be a citation
                // container (`is_biblio_container`); if we're already
                // inside one, scope only continues through a structural
                // pass-through wrapper (`is_biblio_field_wrapper`) —
                // everything else is a *leaf* field (`title`, `author`,
                // `bibliomisc`, ...), and a leaf field's own children are
                // ordinary markup-capable inline content, not further
                // sub-fields. Without this distinction, `<emphasis>` inside
                // e.g. a `<title>` would itself be mis-dispatched as a
                // raw-preserved "misc" field instead of a proper `emphasis`
                // node, silently flattening the markup it exists to
                // preserve.
                let child_in_biblio = if in_biblio {
                    is_biblio_field_wrapper(name)
                } else {
                    is_biblio_container(name)
                };
                let converted_kids = convert_children(
                    kids,
                    name,
                    child_in_header,
                    child_in_biblio,
                    metadata,
                    warnings,
                );
                if in_biblio {
                    // Inside a citation container, every child is a
                    // bibliographic sub-field (or a nested citation, for
                    // `<biblioset>`) — dispatch through the dedicated
                    // converter instead of the generic element table, and
                    // splice its result(s) straight in (no header-raw-
                    // capture logic applies here; see `convert_biblio_field`).
                    out.extend(convert_biblio_field(name, attrs, converted_kids));
                    continue;
                }
                let mut converted =
                    convert_element(name, attrs, converted_kids.clone(), Some(parent_name));
                // Any `<info>` descendant this reader has no explicit
                // semantic mapping for (i.e. `convert_element` produced it
                // via its generic catch-all rather than the dedicated
                // `title` arm — see `is_modeled_header_field`) is about to
                // be discarded as a tree node and flattened into metadata by
                // `extract_metadata`. Rather than lose its internal
                // structure (`<author>`'s personname parts, `<revhistory>`'s
                // revision entries, or any other unmodeled front-matter
                // element), capture the whole subtree's original XML
                // verbatim (mirroring how `rescribe-read-tei` raw-preserves
                // unmodeled teiHeader children via `tei_fmt::emit_fragment`)
                // so the writer can splice it back byte-for-byte instead of
                // reconstructing a lossy approximation from flattened text.
                if in_header
                    && !is_modeled_header_field(name)
                    && let Some(node) = converted.take()
                {
                    let raw =
                        String::from_utf8(crate::emit_fragment(std::slice::from_ref(child))).ok();
                    converted = Some(match raw {
                        Some(raw) => node.prop("docbook:raw", raw),
                        None => node,
                    });
                }
                match converted {
                    Some(node) => out.push(node),
                    None => {
                        if matches!(name.as_str(), "info" | "articleinfo" | "bookinfo") {
                            extract_metadata(&converted_kids, metadata, warnings);
                        } else {
                            // Pass-through wrapper element (e.g. tgroup,
                            // imageobject, mediaobject): splice its already
                            // converted children directly into the parent.
                            out.extend(converted_kids);
                        }
                    }
                }
            }
            DbNode::Text { content, .. } => {
                if !content.trim().is_empty() {
                    out.push(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()));
                }
            }
            DbNode::Cdata { content, .. } => {
                out.push(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()));
            }
            DbNode::EntityRef { name, .. } => {
                // Named entity DocBook/the DTD defines but we cannot resolve
                // without the DTD — raw-preserve verbatim rather than drop.
                out.push(
                    Node::new(node::RAW_INLINE)
                        .prop(prop::CONTENT, format!("&{name};"))
                        .prop("docbook:entity", name.clone()),
                );
            }
            DbNode::Comment { .. }
            | DbNode::ProcessingInstruction { .. }
            | DbNode::Doctype { .. } => {
                // No cross-format meaning and no natural IR raw-block slot
                // inside inline/block flow content; DocBook's own semantic
                // model has no equivalent for a bare PI/comment here.
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::FeatureLost("comment-or-pi".to_string()),
                    format!("dropped non-content DocBook node inside <{parent_name}>"),
                ));
            }
            DbNode::Raw { content, .. } => {
                // `DbNode::Raw` is never produced by `docbook_fmt::parse`
                // itself (see its doc comment) — it only exists for
                // downstream consumers to construct directly. This arm
                // exists purely so the match stays exhaustive; raw-preserve
                // the content verbatim rather than drop it if a
                // `DocBookDoc` containing one is ever fed through this
                // reader.
                out.push(Node::new(node::RAW_BLOCK).prop(prop::CONTENT, content.clone()));
            }
        }
    }
    out
}

/// Whether `name` (the DocBook AST's literal element name, prefix included —
/// see the `mathml-raw` interception in `convert_children`) is a MathML root
/// element: either bare `math` (default-namespaced) or `{prefix}:math` for
/// any namespace prefix the source document chose.
fn is_mathml_root(name: &str) -> bool {
    name == "math"
        || name
            .rsplit_once(':')
            .is_some_and(|(_, local)| local == "math")
}

/// Pull the raw-captured MathML sentinel (see `convert_children`'s
/// `mathml-raw` interception) out of an equation's already-converted
/// children, returning its verbatim MathML source alongside the remaining
/// children (normally empty in the MathML case — MathML, `<mathphrase>`, and
/// `<mediaobject>` are alternatives per the DocBook 5.2 content model, not
/// combined).
fn split_mathml(children: Vec<Node>) -> (Option<String>, Vec<Node>) {
    let mut mathml = None;
    let mut rest = Vec::with_capacity(children.len());
    for child in children {
        if mathml.is_none()
            && child.kind.as_str() == node::SPAN
            && child.props.get_str("docbook:tag") == Some("mathml-raw")
        {
            mathml = child.props.get_str(prop::CONTENT).map(|s| s.to_string());
        } else {
            rest.push(child);
        }
    }
    (mathml, rest)
}

/// Build a `math_display`/`math_inline` node from an already-converted
/// `<equation>`/`<informalequation>`/`<inlineequation>` child list, per the
/// DocBook 5.2 content model (`alt?, (mediaobject+ | mathphrase+ | mml:*+)`,
/// optionally preceded by `title`/`info` — see `heading_level_for_parent`,
/// which already routes an equation's own `<title>` to `CAPTION` rather than
/// `HEADING`, so it survives here as an ordinary child, same as `<figure>`'s
/// caption).
///
/// Three content alternatives, each handled differently:
/// - MathML (`<mml:math>` or similar): raw-captured verbatim by
///   `convert_children` into the `mathml-raw` sentinel this function pulls
///   back out via `split_mathml` — `math:source` + `math:format = "mathml"`.
/// - `<mathphrase>`: DocBook *phrase-level markup* (DocBook 5.2 reference:
///   `<mathphrase>` holds ordinary inline content like `<superscript>`, e.g.
///   `x<superscript>n</superscript> + y<superscript>n</superscript>` — it is
///   NOT a flattenable-to-text format the way MathML or TeX source is), so
///   its already-converted children (superscript/emphasis/text nodes, from
///   the ordinary `"mathphrase"` catch-all pass-through below) are kept as
///   real child nodes of the math node — matching the repo's existing
///   "nested markup survives as real nodes, not a flat string" convention
///   (see `bibliography_field`'s markup-in-field fixtures) — rather than
///   collapsed into `math:source` the way `extract_text` would.
/// - `<mediaobject>`/`<inlinemediaobject>` (an image standing in for the
///   equation): already converts to a plain `IMAGE` node via the existing
///   `"mediaobject"`/`"inlinemediaobject"` arm; kept as an ordinary child of
///   the math node (same treatment as the `<mathphrase>` case) rather than
///   invented as a special image-only equation representation — it's still
///   equation content per the spec, just a different encoding of it.
fn build_math_node(kind: &str, children: Vec<Node>) -> Node {
    let (mathml, rest) = split_mathml(children);
    match mathml {
        Some(source) => Node::new(kind)
            .prop("math:source", source)
            .prop("math:format", "mathml")
            .children(rest),
        None => Node::new(kind).children(rest),
    }
}

fn get_attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// Attach the small set of DocBook attributes worth round-tripping
/// generically (id, role, xml:lang) regardless of which element carries
/// them. Applied to *every* element `convert_element` produces (see the
/// `.map()` wrapping its `match` at the end of that function) — not just
/// the generic-fallback span/div nodes — so e.g. `xml:id` on a `<section>`
/// or `xml:lang` on a `<para>` round-trips the same way it would on an
/// unrecognized element.
fn attach_generic_attrs(mut node: Node, attrs: &[(String, String)]) -> Node {
    if let Some(id) = get_attr(attrs, "id").or_else(|| get_attr(attrs, "xml:id")) {
        node = node.prop(prop::ID, id.to_string());
    }
    if let Some(role) = get_attr(attrs, "role") {
        node = node.prop("docbook:role", role.to_string());
    }
    // `xml:lang` is the standard XML language attribute (DocBook doesn't
    // define its own) — maps to rescribe's cross-format `language`
    // property, the same convention `rescribe-read-tei` uses for the same
    // attribute.
    if let Some(lang) = get_attr(attrs, "xml:lang") {
        node = node.prop(prop::LANGUAGE, lang.to_string());
    }
    node
}

/// The document-outline heading level a `<title>` should use, if its parent
/// is a genuine DocBook sectioning container (DocBook 5.2 reference:
/// article/book/chapter/part/appendix are the top-level divisions;
/// section/sect1 nest one level deeper than that; sect2-sect5 each nest one
/// level deeper still; simplesect is a non-nesting subsection, kept at the
/// same level `section`/`sect1` used before this function existed to avoid
/// changing existing round-trip behavior). `None` for every other parent
/// (including `table`, `example`, `figure`, admonitions, `qandaset`,
/// `refentry`, or no parent at all) — those containers' `<title>` is a
/// caption, not an outline heading; see the `"title"` arm in
/// `convert_element`.
fn heading_level_for_parent(parent: Option<&str>) -> Option<i64> {
    match parent {
        // A document's own title, inside <info>/<articleinfo>/<bookinfo>,
        // is level 1 — the same level as article/book/chapter/part/appendix
        // — not a caption; `extract_metadata` specifically looks for a
        // `HEADING` child to find it (see `is_modeled_header_field`).
        // `<bibliography>` is a top-level division exactly like `<chapter>`
        // (DocBook 5.2 reference lists it alongside chapter/glossary/index),
        // so its own `<title>` is a heading, not a caption.
        Some("article") | Some("book") | Some("chapter") | Some("part") | Some("appendix")
        | Some("bibliography") | Some("info") | Some("articleinfo") | Some("bookinfo") => Some(1),
        Some("sect1") | Some("section") | Some("simplesect") => Some(2),
        Some("sect2") => Some(3),
        Some("sect3") => Some(4),
        Some("sect4") => Some(5),
        Some("sect5") => Some(6),
        _ => None,
    }
}

/// `spacing="compact"` on `<itemizedlist>`/`<orderedlist>` (DocBook 5.2
/// reference) maps to the standard `tight` property — the same semantic
/// concept CommonMark/GFM readers use for "no paragraph wrapping between
/// items." `spacing="normal"` is the default and needs no property.
fn attach_list_spacing(mut node: Node, attrs: &[(String, String)]) -> Node {
    if let Some(spacing) = get_attr(attrs, "spacing") {
        node = node.prop(prop::TIGHT, spacing == "compact");
    }
    node
}

/// A generic inline "wrapper" element: DocBook markup that has no dedicated
/// IR node kind but must still round-trip losslessly. Represented as a
/// `span` tagged with the original element name (`docbook:tag`) per the
/// raw-preservation pattern — this is exactly what `span` exists for.
fn generic_span(name: &str, _attrs: &[(String, String)], children: Vec<Node>) -> Node {
    Node::new(node::SPAN)
        .prop("docbook:tag", name.to_string())
        .children(children)
}

/// A generic block-level "wrapper" element: the block-level counterpart to
/// [`generic_span`]. DocBook markup with no dedicated IR node kind, but
/// whose content model is block-shaped (per [`is_block_element`]) rather
/// than running inline text — represented as a `div` tagged with the
/// original element name (`docbook:tag`) so the writer can re-emit the exact
/// tag rather than `<para>`-wrapping a bare span, which would misrepresent
/// an unrecognized block element as an inline one.
fn generic_div(name: &str, _attrs: &[(String, String)], children: Vec<Node>) -> Node {
    Node::new(node::DIV)
        .prop("docbook:tag", name.to_string())
        .children(children)
}

/// Collect a `<qandaset>`/`<qandadiv>`'s already-converted children into
/// their final shape (see the `"qandaset"`/`"qandadiv"`/`"qandaentry"` arms
/// in [`convert_element`]): any `DEFINITION_TERM`/`DEFINITION_DESC` nodes —
/// produced by that element's directly-owned `<qandaentry>` children
/// flattening their `question`/`answer`s straight into this list, the same
/// way `<varlistentry>` does for `<variablelist>` — are pulled out (in
/// order) and wrapped in one synthesized `DEFINITION_LIST` tagged
/// `docbook:list-kind = "qanda"`, appended after every other child. Returns
/// `children` unchanged if there are no such nodes (the `qandadiv+` content
/// model alternative, or a `qandaset`/`qandadiv` with only a title/block
/// content and no entries yet).
fn wrap_qanda_entries(children: Vec<Node>) -> Vec<Node> {
    let mut rest = Vec::with_capacity(children.len());
    let mut entries = Vec::new();
    for child in children {
        if matches!(
            child.kind.as_str(),
            node::DEFINITION_TERM | node::DEFINITION_DESC
        ) {
            entries.push(child);
        } else {
            rest.push(child);
        }
    }
    if !entries.is_empty() {
        rest.push(
            Node::new(node::DEFINITION_LIST)
                .prop("docbook:list-kind", "qanda")
                .children(entries),
        );
    }
    rest
}

/// Known DocBook block-level elements — used only by the catch-all fallback
/// in [`convert_element`] to decide whether an element name this reader
/// doesn't specifically recognize should become a [`generic_div`] (block
/// position) or a [`generic_span`] (inline position); every element
/// `convert_element` already gives dedicated handling to never reaches the
/// catch-all, so this list exists purely to classify the *unrecognized*
/// remainder. It deliberately includes both this reader's own recognized
/// block vocabulary (as a cross-reference) and additional DocBook elements
/// that are unambiguously block-shaped but have no dedicated IR mapping yet.
pub(crate) fn is_block_element(tag: &str) -> bool {
    matches!(
        tag,
        // Document / sectioning
        "article"
            | "book"
            | "chapter"
            | "part"
            | "appendix"
            | "preface"
            | "colophon"
            | "dedication"
            | "glossary"
            | "bibliography"
            | "index"
            | "section"
            | "sect1"
            | "sect2"
            | "sect3"
            | "sect4"
            | "sect5"
            | "simplesect"
            | "refentry"
            | "reference"
            | "refsect1"
            | "refsect2"
            | "refsect3"
            | "refsection"
            | "setindex"
            | "partintro"
            // Bibliography/glossary/qanda/index subdivisions and standalone
            // list-shaped elements. All confirmed "Formatted as a displayed
            // block" against the DocBook 5.2 reference (tdg.docbook.org) —
            // found missing by a full audit of every DocBook 5.2 element
            // name against this list (see TODO.md), not just the four
            // element families that audit flagged as confirmed code gaps.
            | "glossentry"
            | "glossdef"
            | "glossdiv"
            | "glosslist"
            | "indexentry"
            | "indexdiv"
            | "refnamediv"
            | "refsynopsisdiv"
            | "refmeta"
            | "entrytbl"
            | "bibliodiv"
            | "bibliolist"
            | "qandadiv"
            | "simplelist"
            | "toc"
            | "tocdiv"
            | "tocentry"
            | "productionset"
            | "production"
            | "productionrecap"
            | "constraintdef"
            | "msgset"
            // Block content
            | "para"
            | "simpara"
            | "formalpara"
            | "blockquote"
            | "epigraph"
            | "itemizedlist"
            | "orderedlist"
            | "listitem"
            | "variablelist"
            | "varlistentry"
            | "segmentedlist"
            | "procedure"
            | "step"
            | "substeps"
            | "programlisting"
            | "screen"
            | "literallayout"
            | "synopsis"
            | "cmdsynopsis"
            | "funcsynopsis"
            | "table"
            | "informaltable"
            | "tgroup"
            | "thead"
            | "tbody"
            | "tfoot"
            | "row"
            | "tr"
            | "figure"
            | "informalfigure"
            | "example"
            | "informalexample"
            | "equation"
            | "informalequation"
            | "mediaobject"
            | "note"
            | "tip"
            | "warning"
            | "caution"
            | "important"
            | "sidebar"
            | "abstract"
            | "qandaset"
            | "qandaentry"
            | "question"
            | "answer"
            | "task"
            | "revhistory"
            | "revision"
            // Front-matter wrappers: never inline in running text, always a
            // displayed-block/title-page structural container (confirmed
            // against the DocBook 5.1 reference: authorgroup wraps
            // author/editor/othercredit and "does not appear inline within
            // paragraph content"; legalnotice holds para/lists/tables and
            // "operates at the block level"; revision documents a single
            // revhistory entry — revnumber/date/authorinitials/revremark —
            // structurally block like a table row).
            | "authorgroup"
            | "legalnotice"
            // Callout listing composition (see the `"programlistingco"`/
            // `"areaspec"`/`"calloutlist"`/`"callout"` arms below) — all
            // block-shaped; `<co>`/`<area>`/`<areaset>` are the inline/
            // EMPTY members of this family and correctly excluded here.
            | "programlistingco"
            | "areaspec"
            | "calloutlist"
            | "callout"
    )
}

/// Convert one DocBook element (with its already-converted children) into a
/// rescribe node. Returns `None` for elements that either have no IR
/// representation of their own (pass-through wrappers) or are consumed for
/// side effects (metadata extraction) — see [`convert_children`] for how
/// those two cases are told apart.
fn convert_element(
    name: &str,
    attrs: &[(String, String)],
    children: Vec<Node>,
    parent: Option<&str>,
) -> Option<Node> {
    let role = get_attr(attrs, "role");
    let url = get_attr(attrs, "url").or_else(|| get_attr(attrs, "xlink:href"));
    let language = get_attr(attrs, "language");

    let result = match name {
        // Document level. Tagged with `docbook:tag` (same convention as
        // `generic_div`) so `rescribe-write-docbook` can re-emit the exact
        // original element instead of losing the distinction between
        // `<book>`/`<chapter>`/`<part>`/`<appendix>`/`<article>` — before
        // this tag was added, the writer's only way to reconstruct a
        // wrapper was inferring a `sectN`/`section` tag from a `HEADING`
        // child's outline level, which can never produce `book`/`chapter`/
        // `part`/`appendix`/`article` and silently corrupted the element
        // identity on every round-trip.
        "article" | "book" | "chapter" | "part" | "appendix" => Some(
            Node::new(node::DIV)
                .prop("docbook:tag", name.to_string())
                .children(children),
        ),

        // Sections. Same `docbook:tag` treatment as above — additionally
        // fixes a second identity-loss case: a `<section>` nested inside
        // another `<section>` (real depth 2+) previously read in at
        // `heading_level_for_parent`'s fixed level-2 and would write back
        // out as `<sect1>` rather than `<section>`, since the writer had no
        // way to know the original tag was `<section>` and not `<sect1>`.
        // Preserving the tag makes reconstruction depth-independent.
        "section" | "sect1" | "sect2" | "sect3" | "sect4" | "sect5" | "simplesect" => Some(
            Node::new(node::DIV)
                .prop("docbook:tag", name.to_string())
                .children(children),
        ),

        // A <title> is a section heading only inside a genuine sectioning
        // container (see `heading_level_for_parent`) — everywhere else
        // (formal <table>, <example>, <figure>, admonitions, <qandaset>,
        // `<refentry>`, ...) it's the container's *caption*, not a heading
        // in the document outline. Mapping every `<title>` to `HEADING`
        // regardless of parent (the pre-fix behavior) round-trips
        // incorrectly for non-sectioning parents: `rescribe-write-docbook`
        // always re-emits a `HEADING` node as a wrapping `<sectN><title>`,
        // so e.g. `<example><title>T</title><para>P</para></example>`
        // would come back as `<example><sect1><title>T</title></sect1>
        // <para>P</para></example>` — a spurious nested section and the
        // title's position corrupted. The `"table"/"informaltable"` arm
        // pulls its `CAPTION` child out into the `title` property; every
        // other container just keeps it as an ordinary child, which
        // `rescribe-write-docbook`'s new `CAPTION` write arm re-emits as
        // `<title>` in its original position.
        "title" => match heading_level_for_parent(parent) {
            Some(level) => Some(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, level)
                    .children(children),
            ),
            None => Some(Node::new(node::CAPTION).children(children)),
        },

        // Paragraphs
        "para" | "simpara" => Some(Node::new(node::PARAGRAPH).children(children)),

        // Block quote
        "blockquote" => Some(Node::new(node::BLOCKQUOTE).children(children)),

        // Lists
        "itemizedlist" => Some(attach_list_spacing(
            Node::new(node::LIST)
                .prop(prop::ORDERED, false)
                .children(children),
            attrs,
        )),
        "orderedlist" => {
            let mut node = Node::new(node::LIST)
                .prop(prop::ORDERED, true)
                .children(children);
            // `numeration` selects the marker style (DocBook 5.2 reference:
            // arabic/upperalpha/loweralpha/upperroman/lowerroman) — mapped
            // to the standard `list_style` property using the same
            // CSS `list-style-type` vocabulary rescribe already uses
            // elsewhere (decimal/upper-alpha/lower-alpha/upper-roman/
            // lower-roman) rather than DocBook's own attribute spelling.
            if let Some(numeration) = get_attr(attrs, "numeration") {
                let style = match numeration {
                    "arabic" => "decimal",
                    "upperalpha" => "upper-alpha",
                    "loweralpha" => "lower-alpha",
                    "upperroman" => "upper-roman",
                    "lowerroman" => "lower-roman",
                    other => other,
                };
                node = node.prop(prop::LIST_STYLE, style.to_string());
            }
            if let Some(start) = get_attr(attrs, "startingnumber")
                && let Ok(n) = start.parse::<i64>()
            {
                node = node.prop(prop::START, n);
            }
            Some(attach_list_spacing(node, attrs))
        }
        // A `<listitem>` whose parent is `<varlistentry>` is a definition's
        // *description*, not an ordinary list item — see the
        // "variablelist"/"varlistentry"/"term" arms below, which build the
        // flat, wrapper-free `definition_list` shape (`DEFINITION_TERM`s
        // followed by `DEFINITION_DESC`s as direct children, matching the
        // convention `rescribe-read-markdown`/`rescribe-read-html` already
        // use for the same node kind) that `rescribe-write-docbook`'s
        // `write_definition_list` groups back into `<varlistentry>` runs.
        "listitem" if parent == Some("varlistentry") => {
            Some(Node::new(node::DEFINITION_DESC).children(children))
        }
        "listitem" => Some(Node::new(node::LIST_ITEM).children(children)),

        // A <procedure> (DocBook 5.2 reference: "encapsulates a task
        // composed of <step>s, and possibly <substeps>") is structurally a
        // numbered list of instructions — the same cross-format shape as
        // an ordered list, so it's modeled as one (tagged `docbook:tag` so
        // the writer re-emits `<procedure>`/`<step>`/`<substeps>` rather
        // than `<orderedlist>`/`<listitem>`, which would lose the
        // "this is a procedure, not just a list" distinction).
        "procedure" => Some(
            Node::new(node::LIST)
                .prop(prop::ORDERED, true)
                .prop("docbook:tag", "procedure")
                .children(children),
        ),
        "substeps" => Some(
            Node::new(node::LIST)
                .prop(prop::ORDERED, true)
                .prop("docbook:tag", "substeps")
                .children(children),
        ),
        "step" => Some(
            Node::new(node::LIST_ITEM)
                .prop("docbook:tag", "step")
                .children(children),
        ),

        // Definition lists. `<varlistentry>` is a structural pass-through
        // wrapper (like `<tgroup>`): its own children (one-or-more `<term>`s
        // then one `<listitem>`, already converted to `DEFINITION_TERM`/
        // `DEFINITION_DESC` above) splice directly into `<variablelist>`'s
        // `DEFINITION_LIST` as a flat run — no `docbook:varlistentry`
        // wrapper node — matching the flat, run-grouped `definition_list`
        // convention `rescribe-read-markdown`/`rescribe-read-html` already
        // use (a group is 1+ consecutive `DEFINITION_TERM` then 0+
        // consecutive `DEFINITION_DESC`, direct children of
        // `DEFINITION_LIST`). `rescribe-write-docbook`'s
        // `write_definition_list` regroups by run to re-emit each
        // `<varlistentry>`.
        "variablelist" => Some(Node::new(node::DEFINITION_LIST).children(children)),
        "varlistentry" => None,
        "term" => Some(Node::new(node::DEFINITION_TERM).children(children)),

        // Q&A lists. DocBook 5.2 reference content model:
        // `qandaset ::= title?, info?, (block content)*,
        // (qandadiv+ | qandaentry+)`; `qandadiv` nests the same shape
        // (recursively, with its own optional title) one level deeper;
        // `qandaentry ::= question, answer*` (exactly one question, zero or
        // more answers — an unanswered question is explicitly permitted).
        // No dedicated node kinds needed: `qandaset`/`qandadiv` map to `DIV`
        // (which already nests arbitrarily, the same way `generic_div`/
        // sectioning containers do) tagged `docbook:tag`, and
        // `qandaentry`/`question`/`answer` reuse the same flat, run-grouped
        // `definition_list` convention `<varlistentry>`/`<term>`/
        // `<listitem>` uses above — `qandaentry` is a pass-through wrapper
        // (like `varlistentry`) whose `question`/`answer` children become
        // `DEFINITION_TERM`/`DEFINITION_DESC` siblings, and
        // `wrap_qanda_entries` collects every such sibling produced by a
        // `qandaset`/`qandadiv`'s directly-owned `qandaentry` children into
        // one synthesized `DEFINITION_LIST` (tagged
        // `docbook:list-kind = "qanda"` so `rescribe-write-docbook`'s
        // `write_definition_list` re-emits `<qandaentry>`/`<question>`/
        // `<answer>` instead of `<varlistentry>`/`<term>`/`<listitem>`),
        // appended after any other content — per the content model,
        // `qandaentry`s always come last and are mutually exclusive with
        // `qandadiv`s, so appending rather than preserving interleaved
        // position is lossless for any conforming document. `defaultlabel`
        // (`none`/`number`/`qanda` — DocBook 5.2 reference: controls
        // whether entries render unlabeled, numbered, or "Q:"/"A:"-prefixed)
        // is only meaningful on `qandaset` itself, not `qandadiv`.
        "qandaset" | "qandadiv" => {
            let mut node = Node::new(node::DIV)
                .prop("docbook:tag", name.to_string())
                .children(wrap_qanda_entries(children));
            if name == "qandaset"
                && let Some(label) = get_attr(attrs, "defaultlabel")
            {
                node = node.prop("docbook:qanda-defaultlabel", label.to_string());
            }
            Some(node)
        }
        // Pass-through wrapper: its `question`/`answer` children (already
        // converted to `DEFINITION_TERM`/`DEFINITION_DESC` by the arms
        // below) splice directly into the enclosing `qandaset`/`qandadiv`'s
        // children, exactly like `varlistentry` above; `wrap_qanda_entries`
        // then collects them into one `DEFINITION_LIST`.
        "qandaentry" => None,
        "question" => Some(Node::new(node::DEFINITION_TERM).children(children)),
        "answer" => Some(Node::new(node::DEFINITION_DESC).children(children)),

        // Code / verbatim blocks. `<synopsis>` and `<address>` (DocBook 5.2
        // reference: both "displayed 'verbatim'; whitespace and line breaks
        // ... are significant", the same as `<screen>`/`<literallayout>`)
        // share the same verbatim-block semantics as `<programlisting>`,
        // just without a programming-language association. The original
        // tag name is kept (`docbook:tag`) so the writer re-emits the exact
        // element instead of always defaulting to `<programlisting>`.
        // `<co/>` callout markers (see the dedicated `"co"` arm below) may be
        // embedded directly inside the mixed text content of any of these
        // verbatim elements (DocBook 5.2 reference:
        // tdg.docbook.org/tdg/5.2/co.html — `<co>` is valid wherever
        // `%co.class;` appears, which includes `programlisting`/`screen`/
        // `literallayout`/`synopsis`). Per ADR 0006, `<co>` itself carries no
        // markup-bearing content (it's EMPTY) so it doesn't need to become a
        // real child node of `code_block` — but its *position* inside the
        // flat text is real information (the whole point of the construct is
        // "this callout annotates this line"), so `extract_verbatim_text`
        // records each marker's character offset into the extracted text as
        // a `docbook:callout-markers` property (a list of `{id, offset,
        // label}` maps) alongside the flat `content` string, rather than
        // extending `code_block`'s content contract to non-flat text.
        "programlisting" | "screen" | "literallayout" | "synopsis" | "address" => {
            let (text, markers) = extract_verbatim_text(&children);
            let mut node = Node::new(node::CODE_BLOCK)
                .prop(prop::CONTENT, text)
                .prop("docbook:tag", name.to_string());
            if let Some(lang) = language {
                node = node.prop(prop::LANGUAGE, lang.to_string());
            }
            if !markers.is_empty() {
                node = node.prop("docbook:callout-markers", PropValue::List(markers));
            }
            Some(node)
        }

        // A callout marker embedded inline in a verbatim element's mixed
        // content (see the `"programlisting"|"screen"|...` arm above, which
        // is where its position actually gets recorded — this arm only
        // builds the sentinel span `extract_verbatim_text` looks for).
        // `xml:id`/`id` is required by the DocBook 5.2 reference and is
        // already captured generically as `prop::ID` by
        // `attach_generic_attrs`, applied uniformly below; `label` has no
        // generic handling, so it's captured explicitly here.
        "co" => {
            let mut node = Node::new(node::SPAN).prop("docbook:tag", "co");
            if let Some(label) = get_attr(attrs, "label") {
                node = node.prop("docbook:label", label.to_string());
            }
            Some(node)
        }

        // The external-coordinates alternative to inline `<co>` markers
        // (DocBook 5.2 reference: `<area>`/`<areaset>` inside `<areaspec>`).
        // `coords` never carries nested markup (per ADR 0006's content-model
        // test — it's plain positional data), so this is a clean Property
        // case: each `<area>`/`<areaset>` becomes a sentinel `SPAN` carrying
        // a `docbook:area` map, which the `"areaspec"`/`"areaset"` arms below
        // collect back out (the same sentinel-then-collect pattern
        // `convert_children`'s MathML interception and `split_mathml` use).
        // `<area>`'s optional `<alt>` child (DocBook 5.2: "text, Graphic
        // inlines (inlinemediaobject)") is flattened to plain text here —
        // the common case is plain text, and the narrow "image nested inside
        // an area's alt text" sub-case degrading to flattened text (rather
        // than a full child-node representation) is a documented residual
        // gap, not a silent drop: the text itself still round-trips.
        "area" => Some(
            Node::new(node::SPAN)
                .prop("docbook:tag", "area")
                .prop("docbook:area", build_area_map(attrs, &children)),
        ),
        "areaset" => {
            let mut map = HashMap::new();
            map.insert("kind".to_string(), PropValue::String("areaset".to_string()));
            if let Some(id) = get_attr(attrs, "id").or_else(|| get_attr(attrs, "xml:id")) {
                map.insert("id".to_string(), PropValue::String(id.to_string()));
            }
            if let Some(units) = get_attr(attrs, "units") {
                map.insert("units".to_string(), PropValue::String(units.to_string()));
            }
            if let Some(otherunits) = get_attr(attrs, "otherunits") {
                map.insert(
                    "otherunits".to_string(),
                    PropValue::String(otherunits.to_string()),
                );
            }
            if let Some(label) = get_attr(attrs, "label") {
                map.insert("label".to_string(), PropValue::String(label.to_string()));
            }
            let areas: Vec<PropValue> = children
                .iter()
                .filter(|c| c.props.get_str("docbook:tag") == Some("area"))
                .filter_map(|c| c.props.get("docbook:area").cloned())
                .collect();
            map.insert("areas".to_string(), PropValue::List(areas));
            Some(
                Node::new(node::SPAN)
                    .prop("docbook:tag", "areaset")
                    .prop("docbook:area", PropValue::Map(map)),
            )
        }
        // `<areaspec>` itself (DocBook 5.2 reference: optional `units`/
        // `otherunits`, no `label` of its own) — collects its `<area>`/
        // `<areaset>` children's sentinel maps into one `docbook:areas` list.
        // This sentinel is consumed by the `"programlistingco"` arm below,
        // which folds it into the paired `<programlisting>`'s `code_block`
        // as `docbook:areaspec` rather than leaving it as an unrelated
        // sibling node — `<areaspec>` only has meaning paired with the
        // listing it annotates.
        "areaspec" => {
            let mut map = HashMap::new();
            if let Some(id) = get_attr(attrs, "id").or_else(|| get_attr(attrs, "xml:id")) {
                map.insert("id".to_string(), PropValue::String(id.to_string()));
            }
            if let Some(units) = get_attr(attrs, "units") {
                map.insert("units".to_string(), PropValue::String(units.to_string()));
            }
            if let Some(otherunits) = get_attr(attrs, "otherunits") {
                map.insert(
                    "otherunits".to_string(),
                    PropValue::String(otherunits.to_string()),
                );
            }
            let areas: Vec<PropValue> = children
                .iter()
                .filter(|c| {
                    matches!(
                        c.props.get_str("docbook:tag"),
                        Some("area") | Some("areaset")
                    )
                })
                .filter_map(|c| c.props.get("docbook:area").cloned())
                .collect();
            map.insert("areas".to_string(), PropValue::List(areas));
            Some(
                Node::new(node::SPAN)
                    .prop("docbook:tag", "areaspec")
                    .prop("docbook:areas", PropValue::Map(map)),
            )
        }
        // `<programlistingco>` (DocBook 5.2 content model: `areaspec?,
        // programlisting`) pairs an optional external coordinate list with
        // the listing it annotates — modeled as a `DIV` tagged
        // `docbook:tag = "programlistingco"` wrapping the (possibly
        // areaspec-augmented) `code_block`, per ADR 0006: neither `<co>` nor
        // `<area>` need `code_block`'s flat-string `content` contract to
        // change, so no new node kind is needed here either.
        "programlistingco" => {
            let mut areaspec_map: Option<HashMap<String, PropValue>> = None;
            let mut code_block = None;
            for child in children {
                if child.props.get_str("docbook:tag") == Some("areaspec") {
                    if let Some(PropValue::Map(inner)) = child.props.get("docbook:areas") {
                        areaspec_map = Some(inner.clone());
                    }
                } else if child.kind.as_str() == node::CODE_BLOCK {
                    code_block = Some(child);
                }
            }
            code_block.map(|mut cb| {
                if let Some(map) = areaspec_map {
                    cb = cb.prop("docbook:areaspec", PropValue::Map(map));
                }
                Node::new(node::DIV)
                    .prop("docbook:tag", "programlistingco")
                    .child(cb)
            })
        }
        // `<calloutlist>`/`<callout>` (DocBook 5.2 reference: `calloutlist`
        // is `title?, info?, (block content)*, callout+`; `callout` is
        // ordinary block content — real prose, per ADR 0006's test, so it
        // stays as real child nodes, not a Property). Modeled as a `LIST`/
        // `LIST_ITEM` pair (matching the existing `procedure`/`step`
        // convention above) tagged `docbook:tag` so the writer re-emits the
        // original elements rather than `<itemizedlist>`/`<listitem>`.
        // `arearefs` (IDREFS — the space-separated `<area>`/`<areaset>`/
        // `<co>` ids this callout annotates) is plain non-markup-bearing
        // attribute data, raw-preserved as a single space-joined string
        // property (the same convention this reader already uses for other
        // IDREFS-shaped attributes).
        "calloutlist" => Some(
            Node::new(node::LIST)
                .prop("docbook:tag", "calloutlist")
                .children(children),
        ),
        "callout" => {
            let mut node = Node::new(node::LIST_ITEM)
                .prop("docbook:tag", "callout")
                .children(children);
            if let Some(arearefs) = get_attr(attrs, "arearefs") {
                node = node.prop("docbook:arearefs", arearefs.to_string());
            }
            Some(node)
        }
        "code" | "literal" | "command" | "filename" | "option" | "computeroutput" | "userinput" => {
            let text = extract_text(&children);
            Some(Node::new(node::CODE).prop(prop::CONTENT, text))
        }

        // Inline formatting
        "emphasis" => {
            if role == Some("strong") || role == Some("bold") {
                Some(Node::new(node::STRONG).children(children))
            } else {
                Some(Node::new(node::EMPHASIS).children(children))
            }
        }
        "subscript" => Some(Node::new(node::SUBSCRIPT).children(children)),
        "superscript" => Some(Node::new(node::SUPERSCRIPT).children(children)),

        // Links
        "link" | "ulink" | "xref" => {
            let mut node = Node::new(node::LINK).children(children);
            if let Some(url) = url {
                node = node.prop(prop::URL, url.to_string());
            } else if let Some(linkend) = get_attr(attrs, "linkend") {
                node = node
                    .prop(prop::URL, format!("#{linkend}"))
                    .child(Node::new(node::TEXT).prop(prop::CONTENT, linkend.to_string()));
            }
            // `xlink:type`/`xlink:role` are XLink attributes DocBook's
            // `<link>` inherits (per the DocBook 5.2 reference); they have
            // no cross-format equivalent, so raw-preserve verbatim under a
            // `docbook:` namespace rather than dropping them.
            if let Some(xtype) = get_attr(attrs, "xlink:type") {
                node = node.prop("docbook:xlink-type", xtype.to_string());
            }
            if let Some(xrole) = get_attr(attrs, "xlink:role") {
                node = node.prop("docbook:xlink-role", xrole.to_string());
            }
            Some(node)
        }

        // Figures and media
        "figure" | "informalfigure" => Some(Node::new(node::FIGURE).children(children)),
        // <imageobject>/<textobject> are pure pass-through wrappers (their
        // own children, `<imagedata>` and alt-text content respectively,
        // are what carry meaning). <mediaobject>/<inlinemediaobject>
        // themselves fold their already-converted `<imageobject>` (an
        // IMAGE node, via the `imagedata` arm) and `<textobject>` (DocBook
        // 5.2 reference: holds a `<phrase>` or `<para>` naming the image's
        // alt text) children into one IMAGE node with the standard `alt`
        // property, rather than passing both through as separate flat
        // siblings — which would leave the alt text as a stray, unrelated
        // node next to the image instead of properly associated with it.
        // See `build_math_node`'s doc comment for the full content-model
        // rationale (MathML raw-captured / mathphrase kept as real child
        // markup / mediaobject kept as a plain child IMAGE node).
        "equation" | "informalequation" => Some(build_math_node("math_display", children)),
        "inlineequation" => Some(build_math_node("math_inline", children)),
        // `<mathphrase>` is DocBook phrase-level markup (see
        // `build_math_node`'s doc comment) — a transparent pass-through so
        // its own children (already-converted `superscript`/`emphasis`/text
        // nodes) splice directly into the enclosing equation node rather
        // than being wrapped in another layer.
        "mathphrase" => None,

        "mediaobject" | "inlinemediaobject" => {
            let mut image = None;
            let mut alt = String::new();
            for child in children {
                if image.is_none() && child.kind.as_str() == node::IMAGE {
                    image = Some(child);
                } else {
                    alt.push_str(&extract_text(std::slice::from_ref(&child)));
                }
            }
            image.map(|img| {
                if alt.is_empty() {
                    img
                } else {
                    img.prop(prop::ALT, alt)
                }
            })
        }
        "imageobject" | "textobject" => None, // Pass through
        "imagedata" | "graphic" => get_attr(attrs, "fileref")
            .map(|url| Node::new(node::IMAGE).prop(prop::URL, url.to_string())),
        "caption" => Some(
            Node::new("figcaption")
                .prop("html:tag", "figcaption")
                .children(children),
        ),

        // Tables. CALS table-model attributes (DocBook 5.2 reference:
        // frame — all/bottom/none/sides/top/topbot — plus colsep/rowsep,
        // confirmed present directly on <table>/<informaltable>) have no
        // cross-format equivalent, so raw-preserve them under `docbook:`.
        // The same attributes can additionally appear on <tgroup> as a
        // finer-grained override; that layer is not separately captured
        // here (`tgroup` stays a pass-through wrapper) — a narrow,
        // disclosed simplification, not a silent drop of the common case.
        "table" | "informaltable" => {
            // A <colspec> (see its own arm below) or the table's <title>
            // (see the `"title"` arm's `heading_level_for_parent` dispatch
            // above, which maps a table's title to CAPTION) must not appear
            // as a table row — pull the title out into the `title` property
            // and keep colspecs as structured leading children rather than
            // TABLE_ROW siblings.
            let mut title = None;
            let mut rest = Vec::with_capacity(children.len());
            for child in children {
                if title.is_none() && child.kind.as_str() == node::CAPTION {
                    title = Some(extract_text(&child.children));
                } else {
                    rest.push(child);
                }
            }
            let mut node = Node::new(node::TABLE).children(rest);
            if let Some(title) = title {
                node = node.prop(prop::TITLE, title);
            }
            if let Some(frame) = get_attr(attrs, "frame") {
                node = node.prop("docbook:frame", frame.to_string());
            }
            if let Some(colsep) = get_attr(attrs, "colsep") {
                node = node.prop("docbook:colsep", colsep.to_string());
            }
            if let Some(rowsep) = get_attr(attrs, "rowsep") {
                node = node.prop("docbook:rowsep", rowsep.to_string());
            }
            Some(node)
        }
        "tgroup" | "thead" | "tbody" | "tfoot" => None, // Pass through
        // <colspec> (DocBook 5.2 reference: colname, colnum, colwidth,
        // colsep, rowsep, align on the entry-alignment vocabulary) has no
        // IR node kind of its own; modeled as a structured child (not a
        // generic_span, so the "table"/"informaltable" arm can keep it out
        // of the row list) carrying its attributes as raw `docbook:`
        // properties since column-width units (`"3*"`, `"1.5in"`) have no
        // cross-format representation.
        "colspec" => {
            let mut node = Node::new("docbook:colspec");
            if let Some(colname) = get_attr(attrs, "colname") {
                node = node.prop("docbook:colname", colname.to_string());
            }
            if let Some(colnum) = get_attr(attrs, "colnum") {
                node = node.prop("docbook:colnum", colnum.to_string());
            }
            if let Some(colwidth) = get_attr(attrs, "colwidth") {
                node = node.prop("docbook:colwidth", colwidth.to_string());
            }
            if let Some(align) = get_attr(attrs, "align") {
                node = node.prop(prop::ALIGN, align.to_string());
            }
            Some(node)
        }
        "row" | "tr" => Some(Node::new(node::TABLE_ROW).children(children)),
        "entry" | "td" => {
            let mut node = Node::new(node::TABLE_CELL).children(children);
            // `morerows` counts *additional* rows an entry spans (DocBook
            // 5.2 reference), whereas the standard cross-format `rowspan`
            // property (mirroring HTML's `rowspan` attribute) counts the
            // *total* rows spanned — hence +1.
            if let Some(morerows) = get_attr(attrs, "morerows")
                && let Ok(n) = morerows.parse::<i64>()
            {
                node = node.prop(prop::ROWSPAN, n + 1);
            }
            // `namest`/`nameend` span an entry across columns *by name*,
            // resolved against the sibling <colspec> list's `colname`s.
            // Resolving that to a plain column count (the standard
            // `colspan` property) would need column-name lookup context
            // this per-entry conversion doesn't have — raw-preserved
            // verbatim rather than guessed at.
            if let Some(namest) = get_attr(attrs, "namest") {
                node = node.prop("docbook:namest", namest.to_string());
            }
            if let Some(nameend) = get_attr(attrs, "nameend") {
                node = node.prop("docbook:nameend", nameend.to_string());
            }
            Some(node)
        }
        "th" => Some(Node::new(node::TABLE_HEADER).children(children)),

        // Footnotes
        "footnote" => Some(Node::new(node::FOOTNOTE_DEF).children(children)),
        // `<footnoteref linkend="…"/>` repeats a `<footnote>` defined
        // elsewhere by reference (DocBook 5.2 reference) — the standard
        // cross-format `footnote_ref` node kind exists for exactly this,
        // with the referenced label under the standard `label` property
        // (the same convention every other rescribe reader with footnote
        // references uses).
        "footnoteref" => get_attr(attrs, "linkend")
            .map(|linkend| Node::new(node::FOOTNOTE_REF).prop(prop::LABEL, linkend.to_string())),

        // Admonitions
        "note" | "tip" | "warning" | "caution" | "important" => Some(
            Node::new(node::BLOCKQUOTE)
                .prop("docbook:type", name)
                .children(children),
        ),

        // <epigraph> (DocBook 5.2 reference: a "Publishing element", block
        // content typically followed by an <attribution> naming its
        // source) is structurally the same shape as a blockquote — reuses
        // the same node kind and `docbook:type` tagging convention as the
        // admonitions above.
        "epigraph" => Some(
            Node::new(node::BLOCKQUOTE)
                .prop("docbook:type", "epigraph")
                .children(children),
        ),
        // <attribution> (names the source of a quotation inside
        // <blockquote>/<epigraph>) has no dedicated IR node kind; kept as
        // an ordinary structured child (not extracted into a property,
        // unlike a table's <title>) since BLOCKQUOTE's writer already
        // re-emits children in their original position.
        "attribution" => Some(Node::new("docbook:attribution").children(children)),

        // Bibliography / citation list. `<bibliography>` is a plain block
        // container (its own `<title>` is handled by the `"title"` arm
        // above, via `heading_level_for_parent`); its `<biblioentry>`/
        // `<bibliomixed>` children were converted through
        // `convert_biblio_field`'s dispatch (see `convert_children`'s
        // `in_biblio` threading) before reaching here, so `children` is
        // already a mix of `bibliography_entry` nodes and any ordinary
        // block content (`<para>`, a nested `<bibliodiv>`, ...).
        "bibliography" => Some(Node::new(node::BIBLIOGRAPHY).children(children)),
        // `<biblioentry>`/`<bibliomixed>` at the top level, i.e. not nested
        // inside another citation container (the normal case — a
        // `<biblioset>` nested *inside* a `<biblioentry>` is instead built
        // by `convert_biblio_field`'s own "biblioset"/"bibliomset" arm,
        // which calls the same `build_bibliography_entry` helper). Also
        // covers the defensive/malformed case of a bare `<biblioset>` with
        // no enclosing `<biblioentry>`.
        "biblioentry" | "bibliomixed" | "biblioset" | "bibliomset" => {
            Some(build_bibliography_entry(name, attrs, children))
        }

        // Abstract and other metadata
        "abstract" => Some(
            Node::new(node::DIV)
                .prop("html:class", "abstract")
                .children(children),
        ),
        // Handled by the caller (`convert_children`) via `extract_metadata`.
        "info" | "articleinfo" | "bookinfo" => None,
        // `author`/`authorgroup`/`date`/`copyright`/`legalnotice`/`pubdate`/
        // `releaseinfo`/`revhistory`/`revision` (and any other `<info>`
        // child with no dedicated semantic mapping) deliberately fall
        // through to the generic catch-all at the bottom of this match —
        // which produces the `generic_span`/`generic_div` node
        // `convert_children`'s `in_header` handling then raw-preserves (see
        // `is_modeled_header_field`) — rather than being special-cased here.
        "personname" | "firstname" | "surname" | "othername" => None, // Just text extraction

        // Line break
        "sbr" => Some(Node::new(node::LINE_BREAK)),

        // Horizontal rule equivalent
        // <bridgehead> (DocBook 5.2 reference: "a heading not tied to the
        // normal sectional hierarchy" — its `renderas` attribute, values
        // sect1-sect5, only controls visual level, not actual nesting)
        // reuses the standard `heading` node kind for its cross-format
        // level meaning, but is tagged `docbook:tag` so the writer
        // re-emits a bare `<bridgehead>` rather than the `<sectN><title>`
        // wrapper a real nested-section HEADING gets — bridgehead is
        // explicitly *not* a section.
        "bridgehead" => {
            let level = get_attr(attrs, "renderas")
                .and_then(|r| r.strip_prefix("sect"))
                .and_then(|n| n.parse::<i64>().ok())
                .map(|n| n + 1) // sect1 nests one level under the top (1) => heading level 2
                .unwrap_or(4);
            Some(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, level)
                    .prop("docbook:tag", "bridgehead")
                    .children(children),
            )
        }

        // Anchors: cross-format equivalent to an empty link target
        "anchor" => {
            let mut node = Node::new(node::LINK);
            if let Some(id) = get_attr(attrs, "id").or_else(|| get_attr(attrs, "xml:id")) {
                node = node.prop("id", id.to_string());
            }
            Some(node)
        }

        // Any other element name: this reader has no dedicated semantic
        // mapping for it. Rather than silently dropping the tag and
        // splicing its children straight into the parent (which is what
        // returning `None` here does, via `convert_children`'s pass-through
        // branch), raw-preserve it generically as a tagged div/span keyed
        // by `docbook:tag` — block-shaped or inline-shaped depending on
        // `is_block_element` — so `rescribe-write-docbook` can re-emit the
        // original tag rather than losing it.
        _ => {
            if is_block_element(name) {
                Some(generic_div(name, attrs, children))
            } else {
                Some(generic_span(name, attrs, children))
            }
        }
    };
    // Applied uniformly to whatever node the match above produced (see
    // `attach_generic_attrs`'s doc comment) rather than each arm attaching
    // its own subset of generic attributes.
    result.map(|n| attach_generic_attrs(n, attrs))
}

/// Whether `name` is a DocBook bibliographic citation container — its
/// children are citation sub-fields (author, title, publisher, ...), not
/// ordinary document content, so `convert_children` dispatches them through
/// [`convert_biblio_field`] instead of [`convert_element`]. `<biblioset>`/
/// `<bibliomset>` group sub-citations (e.g. an article within a book) and
/// are themselves containers for the same reason.
fn is_biblio_container(name: &str) -> bool {
    matches!(
        name,
        "biblioentry" | "bibliomixed" | "biblioset" | "bibliomset"
    )
}

/// Whether `name`, encountered *while already inside* biblio scope, is a
/// structural pass-through wrapper whose own children remain citation
/// sub-fields (as opposed to a leaf field like `<title>`/`<author>`, whose
/// children are ordinary inline content — see `convert_children`'s
/// `child_in_biblio` computation for why this distinction matters).
/// `<publisher>` wraps `<publishername>`/`<address>`; `<address>` may itself
/// wrap `<city>`; `<authorgroup>` wraps individual `<author>`/`<editor>`
/// elements; `<biblioset>`/`<bibliomset>` wrap a nested citation's own
/// fields.
fn is_biblio_field_wrapper(name: &str) -> bool {
    matches!(
        name,
        "authorgroup" | "publisher" | "address" | "biblioset" | "bibliomset"
    )
}

/// Convert one child of a bibliographic citation container (see
/// [`is_biblio_container`]) into zero or more IR nodes. Returns a `Vec`
/// rather than `Option<Node>` because a single DocBook element sometimes
/// expands to more than one IR node (a split `<pagenums>` becomes
/// `page_first` + `page_last` fields) and sometimes to zero (a pass-through
/// wrapper like `<authorgroup>` contributes only its already-converted
/// children).
///
/// `children` are `name`'s own children, already recursively converted
/// (inheriting the same biblio-field dispatch — see `convert_children`'s
/// `in_biblio` threading — so e.g. a `<personname>` inside `<author>` has
/// already been unwrapped to plain inline content by `convert_element`'s
/// existing `personname`/`firstname`/`surname`/`othername` pass-through
/// arm, which is untouched by this function).
fn convert_biblio_field(name: &str, attrs: &[(String, String)], children: Vec<Node>) -> Vec<Node> {
    match name {
        // A `<biblioset>`/`<bibliomset>` nested inside another citation
        // container groups sub-citations (DocBook 5.2 reference: e.g. an
        // article `<biblioset>` inside the containing book's
        // `<biblioentry>`) — modeled as a nested `bibliography_entry`,
        // exactly like a top-level one (see `convert_element`'s own
        // "biblioset"/"bibliomset" arm, which calls the same helper).
        "biblioset" | "bibliomset" => vec![build_bibliography_entry(name, attrs, children)],

        // Pass-through wrappers: their own children (individual
        // `<author>`/`<editor>` elements) were already converted into
        // `bibliography_field` nodes by the recursive dispatch above —
        // splice them straight into the entry as siblings rather than
        // nesting them one level deeper.
        "authorgroup" => children,

        "author" => vec![bib_field("author", "author", children, None)],
        "editor" => vec![bib_field("editor", "editor", children, None)],
        "title" => vec![bib_field("title", "title", children, None)],

        // `<publisher>` normally wraps `<publishername>` (and optionally
        // `<address>`) — both of which, via this same dispatch, already
        // produced their own `publisher`/`publisher_location` fields by the
        // time we get here, so `<publisher>` itself is just a pass-through.
        // A `<publisher>` with bare text content directly (no
        // `<publishername>` child) is the fallback case: treat its own
        // content as the publisher-name field.
        "publisher" => {
            if children
                .iter()
                .any(|c| c.kind.as_str() == node::BIBLIOGRAPHY_FIELD)
            {
                children
            } else {
                vec![bib_field("publisher", "publisher", children, None)]
            }
        }
        "publishername" => vec![bib_field("publisher", "publishername", children, None)],

        // Same pass-through-or-fallback pattern as `<publisher>`/
        // `<publishername>` above, for `<address>`/`<city>`.
        "address" => {
            if children
                .iter()
                .any(|c| c.kind.as_str() == node::BIBLIOGRAPHY_FIELD)
            {
                children
            } else {
                vec![bib_field("publisher_location", "address", children, None)]
            }
        }
        "city" => vec![bib_field("publisher_location", "city", children, None)],

        "edition" => vec![bib_field("edition", "edition", children, None)],
        "volumenum" => vec![bib_field("volume", "volumenum", children, None)],
        "issuenum" => vec![bib_field("issue", "issuenum", children, None)],

        // `class` (DocBook 5.2 reference: doi/isbn/issn/uri/... plus an
        // open `other` value) names the identifier scheme.
        "biblioid" => vec![bib_field(
            "identifier",
            "biblioid",
            children,
            get_attr(attrs, "class"),
        )],

        "bibliomisc" => vec![bib_field("misc", "bibliomisc", children, None)],

        "pagenums" => convert_pagenums(children),

        // A bibliographic date has no `bibliography_field` role of its own
        // — it's captured as the structured `prop::DATE` property on the
        // *entry*, not a child node (see `prop::DATE`'s doc comment for the
        // rationale). Emit an internal marker node for
        // `build_bibliography_entry` to consume and remove; if the date
        // text isn't unambiguously parseable, that function demotes it back
        // to an ordinary `misc` field instead of losing it.
        "date" | "pubdate" => vec![
            Node::new("docbook:_date")
                .prop("docbook:tag", name.to_string())
                .children(children),
        ],

        // Every other bibliographic element this reader has no dedicated
        // mapping for (`subtitle`, `titleabbrev`, `collab`, `isbn`, `issn`,
        // `abbrev`, `corpauthor`, `printhistory`, and any future addition to
        // DocBook's `bibliocomponent.mix`/`bibliomixed.mix` content models):
        // raw-preserve as a `misc` field tagged with the original element
        // name (its own children stay ordinary markup-capable inline
        // nodes), rather than silently dropping it.
        _ => vec![bib_field("misc", name, children, None)],
    }
}

/// Build one `bibliography_field` node: `role` is the standard
/// `prop::FIELD_ROLE` value; `tag` is the original DocBook element name
/// (round-tripped via `docbook:tag` so `rescribe-write-docbook` can restore
/// the exact source element, e.g. `<publishername>` vs. a bare
/// `<publisher>`, or `<biblioid>` vs. a generic misc field); `scheme`, if
/// given, becomes `prop::FIELD_SCHEME` (used only by `<biblioid>`'s `class`
/// attribute).
fn bib_field(role: &str, tag: &str, children: Vec<Node>, scheme: Option<&str>) -> Node {
    let mut node = Node::new(node::BIBLIOGRAPHY_FIELD)
        .prop(prop::FIELD_ROLE, role.to_string())
        .prop("docbook:tag", tag.to_string())
        .children(children);
    if let Some(scheme) = scheme {
        node = node.prop(prop::FIELD_SCHEME, scheme.to_string());
    }
    node
}

/// The result of attempting to split a `<pagenums>` string into a page
/// range.
enum PageSplit {
    /// A single page number (no separator) — unambiguous, no split needed.
    Single,
    /// A clean `first`-`last` numeric range.
    Range(String, String),
    /// Anything else (a discontiguous list like `"12, 34"`, non-numeric
    /// page labels like `"xii-xiv"` or `"42ff"`, multiple separators, ...)
    /// — splitting would be a guess, so the string is kept whole instead.
    Ambiguous,
}

/// Split a `<pagenums>` string on the typical range separator (hyphen, en
/// dash, or em dash) if — and only if — both sides are purely numeric.
/// Anything less clear-cut is reported as [`PageSplit::Ambiguous`] rather
/// than guessed at, per CLAUDE.md's no-guessing rule.
fn split_page_range(text: &str) -> PageSplit {
    let t = text.trim();
    if t.is_empty() {
        return PageSplit::Ambiguous;
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        return PageSplit::Single;
    }
    for sep in ['-', '\u{2013}', '\u{2014}'] {
        if let Some((first, last)) = t.split_once(sep) {
            let first = first.trim();
            let last = last.trim();
            if !first.is_empty()
                && !last.is_empty()
                && first.chars().all(|c| c.is_ascii_digit())
                && last.chars().all(|c| c.is_ascii_digit())
            {
                return PageSplit::Range(first.to_string(), last.to_string());
            }
        }
    }
    PageSplit::Ambiguous
}

/// Convert a `<pagenums>` element's already-converted children into
/// `bibliography_field` node(s): a clean split becomes `page_first` (+
/// `page_last`, for an actual range); an ambiguous/unsplittable string keeps
/// its original inline content whole as a `misc` field, additionally
/// raw-preserving the flattened original string under `docbook:pagenums` —
/// a belt-and-suspenders convenience for a consumer that only understands
/// the `page_first`/`page_last` vocabulary and would otherwise have to
/// re-flatten `children` itself to recover the exact source text.
fn convert_pagenums(children: Vec<Node>) -> Vec<Node> {
    let text = extract_text(&children);
    match split_page_range(&text) {
        // The original children are reused verbatim: no split occurred, so
        // nothing was at risk of being torn apart.
        PageSplit::Single => vec![bib_field("page_first", "pagenums", children, None)],
        PageSplit::Range(first, last) => vec![
            bib_field(
                "page_first",
                "pagenums",
                vec![Node::new(node::TEXT).prop(prop::CONTENT, first)],
                None,
            ),
            bib_field(
                "page_last",
                "pagenums",
                vec![Node::new(node::TEXT).prop(prop::CONTENT, last)],
                None,
            ),
        ],
        PageSplit::Ambiguous => vec![
            Node::new(node::BIBLIOGRAPHY_FIELD)
                .prop(prop::FIELD_ROLE, "misc")
                .prop("docbook:tag", "pagenums")
                .prop("docbook:pagenums", text)
                .children(children),
        ],
    }
}

/// Build a `bibliography_entry` node (used both for a top-level
/// `<biblioentry>`/`<bibliomixed>` — see `convert_element` — and for a
/// nested `<biblioset>`/`<bibliomset>` — see `convert_biblio_field`).
/// `children` are the already-converted `bibliography_field`/nested-entry
/// siblings (see `convert_biblio_field`); this function additionally pulls
/// out the internal `docbook:_date` marker (see `convert_biblio_field`'s
/// "date"/"pubdate" arm) and turns it into the structured `prop::DATE`
/// property, or — if the date text isn't unambiguously parseable — demotes
/// it to an ordinary `misc` field instead of losing it.
fn build_bibliography_entry(name: &str, attrs: &[(String, String)], children: Vec<Node>) -> Node {
    let mut entry = Node::new(node::BIBLIOGRAPHY_ENTRY).prop("docbook:tag", name.to_string());
    if matches!(name, "biblioset" | "bibliomset")
        && let Some(relation) = get_attr(attrs, "relation")
    {
        entry = entry.prop("docbook:biblioset-relation", relation.to_string());
    }
    let mut kids = Vec::with_capacity(children.len());
    for child in children {
        if child.kind.as_str() == "docbook:_date" {
            let text = extract_text(&child.children);
            match parse_bibliographic_date(&text) {
                Some(date) => entry = entry.prop(prop::DATE, PropValue::Map(date)),
                None => {
                    let tag = child
                        .props
                        .get_str("docbook:tag")
                        .unwrap_or("date")
                        .to_string();
                    kids.push(
                        Node::new(node::BIBLIOGRAPHY_FIELD)
                            .prop(prop::FIELD_ROLE, "misc")
                            .prop("docbook:tag", tag)
                            .children(child.children),
                    );
                }
            }
        } else {
            kids.push(child);
        }
    }
    attach_generic_attrs(entry.children(kids), attrs)
}

/// Parse a bibliographic `<date>`/`<pubdate>` string into `prop::DATE`'s
/// `year`/`month`/`day` map, accepting only the unambiguous ISO 8601 forms
/// (`YYYY`, `YYYY-MM`, `YYYY-MM-DD`). DocBook's `<date>` content is
/// otherwise free text (e.g. `"Spring 2021"`, `"March 2020"`) with no
/// single well-defined parse — rather than guess, this returns `None` for
/// anything else, and the caller ([`build_bibliography_entry`]) keeps the
/// text as an ordinary field instead.
fn parse_bibliographic_date(text: &str) -> Option<HashMap<String, PropValue>> {
    let parts: Vec<&str> = text.trim().split('-').collect();
    let is_year = |s: &str| s.len() == 4 && s.chars().all(|c| c.is_ascii_digit());
    let is_two_digit_in = |s: &str, range: std::ops::RangeInclusive<u32>| {
        s.len() == 2 && s.parse::<u32>().is_ok_and(|n| range.contains(&n))
    };
    match parts.as_slice() {
        [y] if is_year(y) => {
            let mut map = HashMap::new();
            map.insert("year".to_string(), PropValue::Int(y.parse().ok()?));
            Some(map)
        }
        [y, m] if is_year(y) && is_two_digit_in(m, 1..=12) => {
            let mut map = HashMap::new();
            map.insert("year".to_string(), PropValue::Int(y.parse().ok()?));
            map.insert("month".to_string(), PropValue::Int(m.parse().ok()?));
            Some(map)
        }
        [y, m, d] if is_year(y) && is_two_digit_in(m, 1..=12) && is_two_digit_in(d, 1..=31) => {
            let mut map = HashMap::new();
            map.insert("year".to_string(), PropValue::Int(y.parse().ok()?));
            map.insert("month".to_string(), PropValue::Int(m.parse().ok()?));
            map.insert("day".to_string(), PropValue::Int(d.parse().ok()?));
            Some(map)
        }
        _ => None,
    }
}

/// `<info>`/`<articleinfo>`/`<bookinfo>` fields `convert_element` gives an
/// explicit, dedicated semantic mapping to — these are fully modeled in
/// `Document::metadata` (via `extract_metadata`'s `HEADING` case) and so
/// must *not* be raw-captured by `convert_children`'s front-matter handling:
/// their content already round-trips through the semantic property it was
/// extracted into, and wrapping them in `docbook:raw` on top would just
/// duplicate that content.
///
/// Every other `<info>` child element name falls to `convert_element`'s
/// generic catch-all (`generic_span`/`generic_div`) and gets raw-preserved
/// instead — see `convert_children`.
fn is_modeled_header_field(name: &str) -> bool {
    matches!(name, "title")
}

/// Extract `<info>`/`<articleinfo>`/`<bookinfo>` metadata: title (searched
/// for as a `HEADING`, matching how `<title>` converts anywhere else) plus
/// every other front-matter field (author, authorgroup, date, copyright,
/// legalnotice, pubdate, releaseinfo, revhistory, or any other unrecognized
/// `<info>` child), each surfaced by `convert_element` as a `span`/`div`
/// tagged with `docbook:tag` so this function can find them regardless of
/// nesting.
///
/// Every field beyond `title` was raw-captured by `convert_children` — see
/// `is_modeled_header_field` — and shows up here as a `span`/`div` carrying
/// a `docbook:raw` prop. That subtree's original XML is stored verbatim as
/// `{tag}_raw` metadata (plus a `{tag}` flattened-text convenience copy) so
/// `rescribe-write-docbook` can splice it back byte-for-byte; nothing was
/// lost, so descendants aren't recursed into separately. Only if raw
/// capture itself failed (non-UTF8 content — the XML source was already
/// UTF8, so this is not expected in practice) does this fall back to a
/// flatten-to-text-plus-fidelity-warning path.
///
/// Multiple occurrences of a repeatable field (e.g. more than one
/// `<author>`) are joined with `"; "` rather than the later one silently
/// overwriting the earlier — losing all-but-the-last author would itself be
/// a silent drop.
fn extract_metadata(
    nodes: &[Node],
    metadata: &mut Properties,
    warnings: &mut Vec<FidelityWarning>,
) {
    for node in nodes {
        if node.kind.as_str() == node::HEADING {
            let title = extract_text(&node.children);
            if !title.is_empty() {
                metadata.set("title", title);
            }
        } else if matches!(node.kind.as_str(), node::SPAN | node::DIV)
            && let Some(tag) = node.props.get_str("docbook:tag")
        {
            let text = extract_text(&node.children);
            match tag {
                // A `<title>` nested somewhere other than directly in
                // `<info>` (e.g. a bibliographic title reference) —
                // `convert_element` still maps it to `HEADING`, handled
                // above, so there is nothing to do for a `span`/`div`-shaped
                // "title" here; this arm only exists so the generic
                // fallback below doesn't clobber the real title metadata.
                "title" => {}
                other if !text.is_empty() || node.props.get_str("docbook:raw").is_some() => {
                    if !text.is_empty() {
                        append_metadata(metadata, other, &text);
                    }
                    match node.props.get_str("docbook:raw") {
                        // A repeatable field (e.g. more than one `<author>`)
                        // concatenates its raw XML rather than the later
                        // occurrence silently overwriting the earlier —
                        // valid, since concatenated sibling XML elements are
                        // themselves valid XML content.
                        Some(raw) => {
                            let key = format!("{other}_raw");
                            match metadata.get_str(&key) {
                                Some(existing) => {
                                    let combined = format!("{existing}{raw}");
                                    metadata.set(key, combined);
                                }
                                None => metadata.set(key, raw.to_string()),
                            }
                            continue;
                        }
                        None => warnings.push(FidelityWarning::new(
                            Severity::Minor,
                            WarningKind::FeatureLost(format!("docbook-info-field-{other}")),
                            format!(
                                "<info> <{other}> internal structure is not modeled and its \
                                     raw XML could not be captured; only its flattened text was \
                                     kept in metadata: {text:?}"
                            ),
                        )),
                    }
                }
                _ => {}
            }
        }
        extract_metadata(&node.children, metadata, warnings);
    }
}

/// Set a metadata field, joining onto any existing value with `"; "` rather
/// than a later occurrence of a repeatable field (e.g. more than one
/// `<author>`) silently overwriting an earlier one.
fn append_metadata(metadata: &mut Properties, key: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    match metadata.get_str(key) {
        Some(existing) => {
            let combined = format!("{existing}; {value}");
            metadata.set(key, combined);
        }
        None => metadata.set(key, value.to_string()),
    }
}

fn extract_text(nodes: &[Node]) -> String {
    let mut text = String::new();
    for node in nodes {
        if node.kind.as_str() == node::TEXT
            && let Some(content) = node.props.get_str(prop::CONTENT)
        {
            text.push_str(content);
        }
        text.push_str(&extract_text(&node.children));
    }
    text
}

/// Like [`extract_text`], but for a verbatim element's (`<programlisting>`/
/// `<screen>`/etc) already-converted children: pulls a `<co/>` marker
/// sentinel (the `"co"` arm in [`convert_element`]) out of the flow instead
/// of recursing into it (it's always empty), recording its character offset
/// into the text built so far — the position information that's the actual
/// point of the construct — alongside its `id`/`label`. Returns the flat
/// text (identical to what `extract_text` would produce, since a `co`
/// sentinel never contributes text) plus the ordered list of markers as
/// `docbook:callout-markers`-shaped `PropValue::Map`s.
fn extract_verbatim_text(nodes: &[Node]) -> (String, Vec<PropValue>) {
    let mut text = String::new();
    let mut markers = Vec::new();
    collect_verbatim(nodes, &mut text, &mut markers);
    (text, markers)
}

fn collect_verbatim(nodes: &[Node], text: &mut String, markers: &mut Vec<PropValue>) {
    for node in nodes {
        if node.kind.as_str() == node::SPAN && node.props.get_str("docbook:tag") == Some("co") {
            let mut map = HashMap::new();
            if let Some(id) = node.props.get_str(prop::ID) {
                map.insert("id".to_string(), PropValue::String(id.to_string()));
            }
            map.insert(
                "offset".to_string(),
                PropValue::Int(text.chars().count() as i64),
            );
            if let Some(label) = node.props.get_str("docbook:label") {
                map.insert("label".to_string(), PropValue::String(label.to_string()));
            }
            markers.push(PropValue::Map(map));
            continue;
        }
        if node.kind.as_str() == node::TEXT
            && let Some(content) = node.props.get_str(prop::CONTENT)
        {
            text.push_str(content);
        }
        collect_verbatim(&node.children, text, markers);
    }
}

/// Build a `docbook:area`-shaped map from an `<area>` element's attributes
/// (see the `"area"` arm in [`convert_element`]) — `kind` distinguishes it
/// from an `<areaset>` entry when both appear together in an `<areaspec>`'s
/// collected `areas` list.
fn build_area_map(attrs: &[(String, String)], children: &[Node]) -> PropValue {
    let mut map = HashMap::new();
    map.insert("kind".to_string(), PropValue::String("area".to_string()));
    if let Some(id) = get_attr(attrs, "id").or_else(|| get_attr(attrs, "xml:id")) {
        map.insert("id".to_string(), PropValue::String(id.to_string()));
    }
    if let Some(coords) = get_attr(attrs, "coords") {
        map.insert("coords".to_string(), PropValue::String(coords.to_string()));
    }
    if let Some(units) = get_attr(attrs, "units") {
        map.insert("units".to_string(), PropValue::String(units.to_string()));
    }
    if let Some(otherunits) = get_attr(attrs, "otherunits") {
        map.insert(
            "otherunits".to_string(),
            PropValue::String(otherunits.to_string()),
        );
    }
    if let Some(label) = get_attr(attrs, "label") {
        map.insert("label".to_string(), PropValue::String(label.to_string()));
    }
    let alt = extract_text(children);
    if !alt.is_empty() {
        map.insert("alt".to_string(), PropValue::String(alt));
    }
    PropValue::Map(map)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_article() {
        let docbook = r#"<?xml version="1.0"?>
<article xmlns="http://docbook.org/ns/docbook">
  <title>Test Article</title>
  <para>Hello, world!</para>
</article>"#;

        let result = parse(docbook).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_sections() {
        let docbook = r#"<?xml version="1.0"?>
<article>
  <section>
    <title>Section 1</title>
    <para>Content</para>
  </section>
</article>"#;

        let result = parse(docbook).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_lists() {
        let docbook = r#"<?xml version="1.0"?>
<article>
  <itemizedlist>
    <listitem><para>Item 1</para></listitem>
    <listitem><para>Item 2</para></listitem>
  </itemizedlist>
</article>"#;

        let result = parse(docbook).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_code() {
        let docbook = r#"<?xml version="1.0"?>
<article>
  <programlisting language="rust">fn main() {}</programlisting>
</article>"#;

        let result = parse(docbook).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_emphasis() {
        let docbook = r#"<?xml version="1.0"?>
<article>
  <para><emphasis>italic</emphasis> and <emphasis role="strong">bold</emphasis></para>
</article>"#;

        let result = parse(docbook).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_unresolvable_entity_preserved() {
        let docbook = r#"<article><para>a &custom; b</para></article>"#;
        let result = parse(docbook).unwrap();
        let doc = result.value;
        let article = &doc.content.children[0];
        let para = &article.children[0];
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::RAW_INLINE)
        );
    }
}
