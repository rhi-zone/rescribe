//! DocBook reader for rescribe.
//!
//! Translates `docbook_fmt::DocBookDoc` (the standalone DocBook/XML AST from
//! the `docbook-fmt` crate) into rescribe's document IR. Supports DocBook 5
//! and DocBook 4 elements.
//!
//! All XML tokenizing/parsing lives in `docbook-fmt` — this crate is a thin
//! AST↔IR translator only (per CLAUDE.md's "adapter layer must never
//! contain parsing or writing logic" rule).
//!
//! # Example
//!
//! ```
//! use rescribe_read_docbook::parse;
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

use docbook_fmt::Node as DbNode;
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, Properties, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};

/// Parse DocBook XML into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, diagnostics) = docbook_fmt::parse(input.as_bytes());

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
            let converted = convert_children(kids, name, false, &mut metadata, &mut warnings);
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
fn convert_children(
    children: &[DbNode],
    parent_name: &str,
    in_header: bool,
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
                let child_in_header =
                    in_header || matches!(name.as_str(), "info" | "articleinfo" | "bookinfo");
                let converted_kids =
                    convert_children(kids, name, child_in_header, metadata, warnings);
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
                        String::from_utf8(docbook_fmt::emit_fragment(std::slice::from_ref(child)))
                            .ok();
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
        // Document level
        "article" | "book" | "chapter" | "part" | "appendix" => {
            Some(Node::new(node::DIV).children(children))
        }

        // Sections
        "section" | "sect1" | "sect2" | "sect3" | "sect4" | "sect5" | "simplesect" => {
            Some(Node::new(node::DIV).children(children))
        }

        // A formal <table>'s <title> (DocBook 5.2: table requires a title,
        // unlike informaltable) is the table's caption, not a heading in
        // the document outline — mapped to the standard `caption` node kind
        // so the "table"/"informaltable" arm below can pull it out of the
        // row children and fold it into the table's `title` property,
        // rather than it appearing as a stray HEADING mixed in with the
        // table's rows.
        "title" if parent == Some("table") => Some(Node::new(node::CAPTION).children(children)),

        // Titles - convert to heading
        "title" => {
            let level = match parent {
                Some("article") | Some("book") | Some("chapter") | Some("part") => 1,
                Some("sect1") | Some("section") => 2,
                Some("sect2") => 3,
                Some("sect3") => 4,
                Some("sect4") => 5,
                Some("sect5") => 6,
                _ => 2,
            };
            Some(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, level as i64)
                    .children(children),
            )
        }

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
        "listitem" => Some(Node::new(node::LIST_ITEM).children(children)),

        // Definition lists
        "variablelist" => Some(Node::new(node::DEFINITION_LIST).children(children)),
        "varlistentry" => Some(Node::new("docbook:varlistentry").children(children)),
        "term" => Some(Node::new(node::DEFINITION_TERM).children(children)),

        // Code
        "programlisting" | "screen" | "literallayout" => {
            let text = extract_text(&children);
            let mut node = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, text);
            if let Some(lang) = language {
                node = node.prop(prop::LANGUAGE, lang.to_string());
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
        "mediaobject" | "inlinemediaobject" | "imageobject" | "textobject" => None, // Pass through
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
            // (see the `"title" if parent == Some("table")` arm above,
            // mapped to CAPTION) must not appear as a table row — pull the
            // title out into the `title` property and keep colspecs as
            // structured leading children rather than TABLE_ROW siblings.
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

        // Admonitions
        "note" | "tip" | "warning" | "caution" | "important" => Some(
            Node::new(node::BLOCKQUOTE)
                .prop("docbook:type", name)
                .children(children),
        ),

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
        "bridgehead" => Some(
            Node::new(node::HEADING)
                .prop(prop::LEVEL, 4i64)
                .children(children),
        ),

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
