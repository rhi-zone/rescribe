//! TEI XML reader for rescribe.
//!
//! Translates `tei_fmt::TeiDoc` (the standalone TEI/XML AST from the
//! `tei-fmt` crate) into rescribe's document IR. Supports common TEI P5
//! elements used in digital humanities.
//!
//! All XML tokenizing/parsing lives in `tei-fmt` — this crate is a thin
//! AST↔IR translator only (per CLAUDE.md's "adapter layer must never
//! contain parsing or writing logic" rule).

use tei_fmt::Node as TNode;

use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, Properties, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};

/// Parse TEI XML into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, diagnostics) = tei_fmt::parse(input.as_bytes());

    let mut warnings: Vec<FidelityWarning> = diagnostics
        .into_iter()
        .map(|d| {
            FidelityWarning::new(
                Severity::Major,
                WarningKind::FeatureLost("xml-parse-error".to_string()),
                format!("TEI XML parse error: {}", d.message),
            )
        })
        .collect();

    let mut metadata = Properties::new();
    let mut children = Vec::new();
    for top in &doc.nodes {
        if let TNode::Element {
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
                    // The root `<TEI>` element itself carries no rescribe-
                    // level semantics; pass its children through rather than
                    // dropping them.
                    children.extend(converted);
                }
            }
        }
        // Leading/trailing Comment/PI/Doctype/whitespace-Text at the very
        // top level (outside the root element) carry no IR meaning and have
        // no cross-format equivalent to model; TEI documents otherwise
        // consist of exactly one root `<TEI>` element.
    }

    let document = Document {
        content: Node::new(node::DOCUMENT).children(children),
        resources: Default::default(),
        metadata,
        source: None,
    };

    Ok(ConversionResult::with_warnings(document, warnings))
}

/// Convert a slice of TEI child nodes into rescribe IR nodes, discarding
/// nodes that only exist to be unwrapped (e.g. `<teiHeader>`/`<fileDesc>`,
/// which are consumed for metadata) and passing through "structural"
/// wrapper elements (like `<text>`/`<body>`) as their own children.
///
/// `in_header` is true when `parent_name` is `<teiHeader>` itself or a
/// descendant of it (threaded down through the recursion below) — i.e.
/// whether the *children* of `parent_name` are teiHeader content that will
/// end up consumed by [`extract_metadata`] rather than surviving as document
/// content nodes.
fn convert_children(
    children: &[TNode],
    parent_name: &str,
    in_header: bool,
    metadata: &mut Properties,
    warnings: &mut Vec<FidelityWarning>,
) -> Vec<Node> {
    let mut out = Vec::new();
    for child in children {
        match child {
            TNode::Element {
                name,
                attrs,
                children: kids,
                ..
            } => {
                let child_in_header = in_header || name == "teiHeader";
                let converted_kids =
                    convert_children(kids, name, child_in_header, metadata, warnings);
                let mut converted =
                    convert_element(name, attrs, converted_kids.clone(), Some(parent_name));
                // Any teiHeader descendant this reader has no explicit
                // semantic mapping for (i.e. `convert_element` produced it
                // via its generic catch-all rather than a dedicated
                // author/editor/publisher/... arm — see
                // `is_modeled_header_field`) is about to be discarded as a
                // tree node and flattened into metadata by
                // `extract_metadata`. Rather than lose its internal
                // structure (`<msDesc>`'s msIdentifier/physDesc/etc.,
                // `<particDesc>`, `<projectDesc>`, or any other unmodeled
                // TEI header element), capture the whole subtree's original
                // XML verbatim (mirroring how `rescribe-read-html`
                // raw-preserves `<math>` via `html_fmt::emit_fragment`) so
                // the writer can splice it back byte-for-byte instead of
                // reconstructing a lossy approximation from flattened text.
                if in_header
                    && !is_modeled_header_field(name)
                    && let Some(node) = converted.take()
                {
                    let raw =
                        String::from_utf8(tei_fmt::emit_fragment(std::slice::from_ref(child))).ok();
                    converted = Some(match raw {
                        Some(raw) => node.prop("tei:raw", raw),
                        None => node,
                    });
                }
                match converted {
                    Some(node) => out.push(node),
                    None => {
                        if name == "teiHeader" {
                            extract_metadata(&converted_kids, metadata, warnings);
                        } else {
                            // Pass-through wrapper element (e.g. `text`,
                            // `body`, `front`, `back`, `fileDesc`,
                            // `titleStmt`): splice its already converted
                            // children directly into the parent.
                            out.extend(converted_kids);
                        }
                    }
                }
            }
            TNode::Text { content, .. } => {
                if !content.trim().is_empty() {
                    out.push(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()));
                }
            }
            TNode::Cdata { content, .. } => {
                out.push(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()));
            }
            TNode::EntityRef { name, .. } => {
                // Named entity the DTD defines but we cannot resolve
                // without it — raw-preserve verbatim rather than drop.
                out.push(
                    Node::new(node::RAW_INLINE)
                        .prop(prop::CONTENT, format!("&{name};"))
                        .prop("tei:entity", name.clone()),
                );
            }
            TNode::Comment { .. } | TNode::ProcessingInstruction { .. } | TNode::Doctype { .. } => {
                // No cross-format meaning and no natural IR raw-block slot
                // inside inline/block flow content; TEI's own semantic
                // model has no equivalent for a bare PI/comment here.
                warnings.push(FidelityWarning::new(
                    Severity::Minor,
                    WarningKind::FeatureLost("comment-or-pi".to_string()),
                    format!("dropped non-content TEI node inside <{parent_name}>"),
                ));
            }
            TNode::Raw { content, .. } => {
                // `TNode::Raw` is never produced by `tei_fmt::parse` itself
                // (see its doc comment) — it only exists for downstream
                // consumers to construct directly. This arm exists purely
                // so the match stays exhaustive; raw-preserve the content
                // verbatim rather than drop it if a `TeiDoc` containing one
                // is ever fed through this reader.
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

/// Attach the generic TEI attributes that apply to (almost) any element —
/// `xml:id`, `n`, `xml:lang`, `corresp`, and `sameAs` — as properties, if
/// present.
///
/// The pre-split reader captured `xml:id`/`n` into a `FrameAttrs` struct but
/// never actually read them back out when building IR nodes, so they were
/// parsed and then silently discarded on every element that carried them.
/// This closes that gap: `xml:id` becomes the standard `id` property (it is
/// rescribe's own identity-attribute prop, and TEI's `xml:id` is exactly
/// that construct), `xml:lang` becomes the standard `language` property,
/// and `n`/`corresp`/`sameAs` become `tei:n`/`tei:corresp`/`tei:same-as`
/// (TEI-specific linking/numbering attributes with no standard
/// cross-format equivalent).
fn attach_generic_attrs(mut node: Node, attrs: &[(String, String)]) -> Node {
    if let Some(id) = get_attr(attrs, "xml:id") {
        node = node.prop(prop::ID, id.to_string());
    }
    if let Some(n) = get_attr(attrs, "n") {
        node = node.prop("tei:n", n.to_string());
    }
    if let Some(lang) = get_attr(attrs, "xml:lang") {
        node = node.prop(prop::LANGUAGE, lang.to_string());
    }
    if let Some(corresp) = get_attr(attrs, "corresp") {
        node = node.prop("tei:corresp", corresp.to_string());
    }
    if let Some(same_as) = get_attr(attrs, "sameAs") {
        node = node.prop("tei:same-as", same_as.to_string());
    }
    node
}

/// Map a `rend` value that denotes paragraph/heading alignment (as opposed
/// to character-level formatting handled by `<hi>`) to the standard
/// `style:align` property value. Returns `None` for anything else so the
/// caller can fall back to raw-preserving the literal `rend` string.
fn align_from_rend(rend: &str) -> Option<&'static str> {
    match rend {
        "center" | "centre" => Some("center"),
        "right" => Some("right"),
        "left" => Some("left"),
        "justify" | "justified" => Some("justify"),
        _ => None,
    }
}

/// A generic inline "wrapper" element: TEI markup that has no dedicated IR
/// node kind but must still round-trip losslessly. Represented as a `span`
/// tagged with the original element name (`tei:tag`) per the raw-
/// preservation pattern — this is exactly what `span` exists for.
fn generic_span(name: &str, attrs: &[(String, String)], children: Vec<Node>) -> Node {
    let mut n = Node::new(node::SPAN)
        .prop("tei:tag", name.to_string())
        .children(children);
    n = attach_generic_attrs(n, attrs);
    n
}

/// A generic block-level "wrapper" element: the block-level counterpart to
/// [`generic_span`]. TEI markup with no dedicated IR node kind, but whose
/// content model is block-shaped in TEI (per [`is_block_element`]) rather
/// than running inline text — represented as a `div` tagged with the
/// original element name (`tei:tag`) so the writer can re-emit the exact
/// tag rather than `<p>`-wrapping a bare span, which would misrepresent an
/// unrecognized block element as an inline one.
fn generic_div(name: &str, attrs: &[(String, String)], children: Vec<Node>) -> Node {
    let mut n = Node::new(node::DIV)
        .prop("tei:tag", name.to_string())
        .children(children);
    n = attach_generic_attrs(n, attrs);
    n
}

/// Known TEI block-level elements — the block-level counterpart to
/// `rescribe-read-html`'s `is_block_element` allow-list. Used only by the
/// catch-all fallback in [`convert_element`] to decide whether an element
/// name this reader doesn't specifically recognize should become a
/// [`generic_div`] (block position) or a [`generic_span`] (inline
/// position); every element `convert_element` already gives dedicated
/// handling to never reaches the catch-all, so this list exists purely to
/// classify the *unrecognized* remainder — it deliberately includes both
/// this reader's own recognized block vocabulary (as a cross-reference) and
/// additional TEI P5 elements that are unambiguously block-shaped but have
/// no dedicated IR mapping yet (front-matter and manuscript-description
/// apparatus in particular).
pub(crate) fn is_block_element(tag: &str) -> bool {
    matches!(
        tag,
        // Divisions / structural grouping
        "div" | "div1"
            | "div2"
            | "div3"
            | "div4"
            | "div5"
            | "div6"
            | "div7"
            | "group"
            | "floatingText"
            // Paragraph-shaped block content
            | "p"
            | "ab"
            | "head"
            | "speaker"
            | "stage"
            | "byline"
            | "dateline"
            | "salute"
            | "signed"
            | "trailer"
            | "opener"
            | "closer"
            | "postscript"
            | "argument"
            | "epigraph"
            | "l"
            // Lists / tables / drama
            | "list"
            | "item"
            | "gloss"
            | "table"
            | "row"
            | "cell"
            | "castList"
            | "castItem"
            // Quotes / verse
            | "quote"
            | "cit"
            | "lg"
            // Figures / code listings
            | "figure"
            | "figDesc"
            | "eg"
            // Notes (block-shaped at block-dispatch position)
            | "note"
            // Bibliography
            | "listBibl"
            | "biblStruct"
            | "biblFull"
            // Manuscript description / title page apparatus
            | "msContents"
            | "msItem"
            | "msIdentifier"
            | "physDesc"
            | "handDesc"
            | "handNote"
            | "typeDesc"
            | "layoutDesc"
            | "layout"
            | "scriptDesc"
            | "decoDesc"
            | "decoNote"
            | "additions"
            | "bindingDesc"
            | "binding"
            | "sealDesc"
            | "seal"
            | "accMat"
            | "provenance"
            | "acquisition"
            | "condition"
            | "recordHist"
            | "source"
            | "sourceDoc"
            | "history"
            | "titlePage"
            | "docTitle"
            | "docAuthor"
            | "docDate"
            | "docEdition"
            | "docImprint"
            | "imprimatur"
            | "performance"
    )
}

/// TEI header fields `convert_element` gives an explicit, dedicated
/// semantic mapping to — these are fully modeled in `Document::metadata`
/// (via `extract_metadata`'s per-tag match arms) and so must *not* be
/// raw-captured by `convert_children`'s teiHeader handling: their content
/// already round-trips through the semantic property it was extracted into,
/// and wrapping them in `tei:raw` on top would just duplicate that content.
///
/// Every other teiHeader child element name falls to `convert_element`'s
/// generic catch-all (`generic_span`/`generic_div`) and gets raw-preserved
/// instead — see `convert_children`.
fn is_modeled_header_field(name: &str) -> bool {
    matches!(
        name,
        "author"
            | "editor"
            | "publisher"
            | "idno"
            | "language"
            | "abstract"
            | "keywords"
            | "change"
            | "title"
    )
}

/// Convert one TEI element (with its already-converted children) into a
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
    let rend = get_attr(attrs, "rend");
    let target = get_attr(attrs, "target");
    let url = get_attr(attrs, "url");
    let type_attr = get_attr(attrs, "type");

    let node = match name {
        // Document structure
        "TEI" | "text" | "body" | "front" | "back" => None, // Pass through
        // Handled by the caller (`convert_children`) via `extract_metadata`.
        "teiHeader" => None,
        "fileDesc" | "titleStmt" | "publicationStmt" | "sourceDesc" | "profileDesc"
        | "revisionDesc" | "textClass" | "langUsage" => None, // Pass through into teiHeader extraction

        // teiHeader metadata leaves: tagged so `extract_metadata` can find
        // them regardless of which wrapper they were nested under.
        "author" => Some(generic_span("author", attrs, children)),
        "editor" => Some(generic_span("editor", attrs, children)),
        "publisher" => Some(generic_span("publisher", attrs, children)),
        "idno" => Some(generic_span("idno", attrs, children)),
        "language" => {
            let mut n = generic_span("language", attrs, children);
            if let Some(ident) = get_attr(attrs, "ident") {
                n = n.prop("tei:ident", ident.to_string());
            }
            Some(n)
        }
        "abstract" => Some(generic_span("abstract", attrs, children)),
        "keywords" => Some(generic_span("keywords", attrs, children)),
        "change" => {
            let mut n = generic_span("change", attrs, children);
            if let Some(when) = get_attr(attrs, "when") {
                n = n.prop("tei:when", when.to_string());
            }
            Some(n)
        }
        // `encodingDesc`/`msDesc` (deeply nested TEI-specific
        // sub-structures: classification declarations, manuscript
        // description apparatus) have no dedicated semantic modeling here.
        // They deliberately fall through to the generic catch-all at the
        // bottom of this match — same as any other unrecognized teiHeader
        // element — which produces the identical `generic_span`/
        // `generic_div` node `convert_children` then raw-preserves (see
        // `is_modeled_header_field`).

        // Divisions
        "div" | "div1" | "div2" | "div3" | "div4" | "div5" | "div6" => {
            let mut n = Node::new(node::DIV).children(children);
            if let Some(align) = rend.and_then(align_from_rend) {
                n = n.prop(prop::STYLE_ALIGN, align);
            }
            Some(n)
        }

        // Headings
        "head" => {
            let level = match parent {
                Some("div1") | Some("div") => 1,
                Some("div2") => 2,
                Some("div3") => 3,
                Some("div4") => 4,
                Some("div5") => 5,
                Some("div6") => 6,
                _ => 2,
            };
            Some(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, level as i64)
                    .children(children),
            )
        }

        // Paragraphs
        "p" => {
            let mut n = Node::new(node::PARAGRAPH).children(children);
            if let Some(align) = rend.and_then(align_from_rend) {
                n = n.prop(prop::STYLE_ALIGN, align);
            }
            Some(n)
        }

        // Anonymous block
        "ab" => Some(
            Node::new(node::PARAGRAPH)
                .prop("tei:tag", "ab")
                .children(children),
        ),

        // Drama / speech
        "sp" => Some(
            Node::new(node::DIV)
                .prop("tei:type", "sp")
                .children(children),
        ),
        "speaker" => Some(
            Node::new(node::PARAGRAPH)
                .prop("tei:type", "speaker")
                .children(children),
        ),
        "stage" => Some(
            Node::new(node::PARAGRAPH)
                .prop("tei:type", "stage")
                .children(children),
        ),
        "castList" => Some(
            Node::new(node::LIST)
                .prop("tei:type", "castList")
                .children(children),
        ),
        "castItem" => Some(
            Node::new(node::LIST_ITEM)
                .prop("tei:tag", "castItem")
                .children(children),
        ),

        // Prefatory / documentary structure blocks
        "epigraph" => Some(
            Node::new(node::DIV)
                .prop("tei:type", "epigraph")
                .children(children),
        ),
        "argument" => Some(
            Node::new(node::DIV)
                .prop("tei:type", "argument")
                .children(children),
        ),
        "byline" => Some(
            Node::new(node::PARAGRAPH)
                .prop("tei:type", "byline")
                .children(children),
        ),
        "dateline" => Some(
            Node::new(node::PARAGRAPH)
                .prop("tei:type", "dateline")
                .children(children),
        ),
        "salute" => Some(
            Node::new(node::PARAGRAPH)
                .prop("tei:type", "salute")
                .children(children),
        ),
        "signed" => Some(
            Node::new(node::PARAGRAPH)
                .prop("tei:type", "signed")
                .children(children),
        ),
        "trailer" => Some(
            Node::new(node::PARAGRAPH)
                .prop("tei:type", "trailer")
                .children(children),
        ),
        "bibl" => Some(generic_span("bibl", attrs, children)),

        // Editorial intervention: empty (or near-empty) constructs.
        "gap" => {
            let mut n = generic_span("gap", attrs, vec![]);
            if let Some(reason) = get_attr(attrs, "reason") {
                n = n.prop("tei:reason", reason.to_string());
            }
            if let Some(extent) = get_attr(attrs, "extent") {
                n = n.prop("tei:extent", extent.to_string());
            }
            Some(n)
        }
        "space" => {
            let mut n = generic_span("space", attrs, vec![]);
            if let Some(extent) = get_attr(attrs, "extent") {
                n = n.prop("tei:extent", extent.to_string());
            }
            Some(n)
        }

        // Lists
        "list" => {
            let ordered = rend == Some("numbered") || type_attr == Some("ordered");
            let mut n = Node::new(node::LIST)
                .prop(prop::ORDERED, ordered)
                .children(children);
            if let Some(t) = type_attr {
                n = n.prop("tei:type", t.to_string());
            }
            Some(n)
        }
        "item" => Some(Node::new(node::LIST_ITEM).children(children)),
        "label" if parent == Some("list") => Some(
            Node::new(node::LIST_ITEM)
                .prop("tei:tag", "label")
                .children(children),
        ),

        // Glossary/definition lists (block form) vs. inline gloss span.
        "gloss" => {
            if matches!(
                parent,
                Some("body")
                    | Some("div")
                    | Some("div1")
                    | Some("div2")
                    | Some("div3")
                    | Some("div4")
                    | Some("div5")
                    | Some("div6")
                    | Some("front")
                    | Some("back")
                    | None
            ) {
                Some(Node::new(node::DEFINITION_LIST).children(children))
            } else {
                Some(generic_span("gloss", attrs, children))
            }
        }
        "term" => Some(Node::new(node::DEFINITION_TERM).children(children)),
        "def" | "desc" => Some(Node::new(node::DEFINITION_DESC).children(children)),

        // Block quote / cited quotation
        "quote" | "cit" => Some(Node::new(node::BLOCKQUOTE).children(children)),

        // Poetry/verse
        "lg" => Some(
            Node::new(node::DIV)
                .prop("tei:type", "verse")
                .children(children),
        ),
        "l" => Some(
            Node::new(node::PARAGRAPH)
                .prop("tei:type", "line")
                .children(children),
        ),

        // Code: `<eg>` is a block-level example listing; `<code>` is an
        // inline (or short block) fragment of computer code.
        "eg" => {
            let text = extract_text(&children);
            Some(Node::new(node::CODE_BLOCK).prop(prop::CONTENT, text))
        }
        "code" => {
            let text = extract_text(&children);
            Some(Node::new(node::CODE).prop(prop::CONTENT, text))
        }

        // Highlighting (inline formatting)
        "hi" => match rend {
            Some("bold") | Some("b") => Some(Node::new(node::STRONG).children(children)),
            Some("italic") | Some("i") | Some("it") => {
                Some(Node::new(node::EMPHASIS).children(children))
            }
            Some("underline") | Some("u") => Some(Node::new(node::UNDERLINE).children(children)),
            Some("strike") | Some("strikethrough") => {
                Some(Node::new(node::STRIKEOUT).children(children))
            }
            Some("sup") | Some("superscript") => {
                Some(Node::new(node::SUPERSCRIPT).children(children))
            }
            Some("sub") | Some("subscript") => Some(Node::new(node::SUBSCRIPT).children(children)),
            Some("sc") | Some("smallcaps") => Some(Node::new(node::SMALL_CAPS).children(children)),
            other => {
                let mut n = Node::new(node::EMPHASIS).children(children);
                // Preserve an unrecognized `rend` value raw rather than
                // silently coercing it to plain emphasis.
                if let Some(r) = other {
                    n = n.prop("tei:rend", r.to_string());
                }
                Some(n)
            }
        },

        // Semantic highlighting
        "emph" => Some(Node::new(node::EMPHASIS).children(children)),
        "foreign" => Some(generic_span("foreign", attrs, children)),
        "title" => {
            // Could be in metadata or inline, depending on ancestry.
            if matches!(parent, Some("titleStmt") | Some("teiHeader")) {
                let title = extract_text(&children);
                if title.is_empty() {
                    None
                } else {
                    // Extraction happens via `extract_metadata` walking the
                    // converted `teiHeader` subtree, so just surface the
                    // heading-shaped node here; `extract_metadata` looks
                    // for `HEADING` nodes.
                    Some(
                        Node::new(node::HEADING)
                            .prop(prop::LEVEL, 1i64)
                            .children(children),
                    )
                }
            } else {
                // Inline bibliographic title reference — not emphasis.
                let mut n = generic_span("title", attrs, children);
                if let Some(lvl) = get_attr(attrs, "level") {
                    n = n.prop("tei:level", lvl.to_string());
                }
                Some(n)
            }
        }

        // Named entities and editorial apparatus: generic inline spans that
        // round-trip via `tei:tag`, with a handful of format-specific
        // attributes worth preserving explicitly.
        "persName" | "placeName" | "orgName" | "name" | "seg" | "w" | "pc" | "choice" | "orig"
        | "reg" | "sic" | "corr" | "add" | "del" | "supplied" | "unclear" | "abbr" | "expan" => {
            Some(generic_span(name, attrs, children))
        }
        "date" => {
            let mut n = generic_span("date", attrs, children);
            if let Some(when) = get_attr(attrs, "when") {
                n = n.prop("tei:when", when.to_string());
            }
            Some(n)
        }
        "num" => {
            let mut n = generic_span("num", attrs, children);
            if let Some(value) = get_attr(attrs, "value") {
                n = n.prop("tei:value", value.to_string());
            }
            Some(n)
        }
        "measure" => {
            let mut n = generic_span("measure", attrs, children);
            if let Some(unit) = get_attr(attrs, "unit") {
                n = n.prop("tei:unit", unit.to_string());
            }
            if let Some(quantity) = get_attr(attrs, "quantity") {
                n = n.prop("tei:quantity", quantity.to_string());
            }
            Some(n)
        }
        "anchor" => Some(generic_span("anchor", attrs, vec![])),
        "milestone" => {
            let mut n = generic_span("milestone", attrs, vec![]);
            if let Some(unit) = get_attr(attrs, "unit") {
                n = n.prop("tei:unit", unit.to_string());
            }
            Some(n)
        }

        // Links
        "ref" | "ptr" => {
            let mut n = Node::new(node::LINK).children(children);
            if let Some(t) = target {
                n = n.prop(prop::URL, t.to_string());
            }
            Some(n)
        }

        // Figures
        "figure" => Some(Node::new(node::FIGURE).children(children)),
        "figDesc" => Some(
            Node::new("figcaption")
                .prop("html:tag", "figcaption")
                .children(children),
        ),
        "graphic" => url.map(|u| {
            let mut n = Node::new(node::IMAGE).prop(prop::URL, u.to_string());
            if let Some(width) = get_attr(attrs, "width") {
                n = n.prop("tei:width", width.to_string());
            }
            if let Some(height) = get_attr(attrs, "height") {
                n = n.prop("tei:height", height.to_string());
            }
            n
        }),

        // Tables
        "table" => Some(Node::new(node::TABLE).children(children)),
        "row" => Some(Node::new(node::TABLE_ROW).children(children)),
        "cell" => {
            let role = rend;
            let mut n = if role == Some("header") || role == Some("label") {
                Node::new(node::TABLE_HEADER).children(children)
            } else {
                Node::new(node::TABLE_CELL).children(children)
            };
            if let Some(cols) = get_attr(attrs, "cols") {
                n = n.prop("tei:cols", cols.to_string());
            }
            if let Some(rows) = get_attr(attrs, "rows") {
                n = n.prop("tei:rows", rows.to_string());
            }
            Some(n)
        }

        // Notes/footnotes
        "note" => {
            let mut n = Node::new(node::FOOTNOTE_DEF).children(children);
            if let Some(place) = get_attr(attrs, "place") {
                n = n.prop("tei:place", place.to_string());
            }
            if let Some(t) = type_attr {
                n = n.prop("tei:type", t.to_string());
            }
            Some(n)
        }

        // Formula: display by default; `type="inline"` marks an inline
        // formula embedded in running text.
        "formula" => {
            let text = extract_text(&children);
            if type_attr == Some("inline") {
                Some(Node::new("math_inline").prop("math:source", text))
            } else {
                Some(Node::new("math_display").prop("math:source", text))
            }
        }

        // Line/page breaks
        "lb" => Some(Node::new(node::LINE_BREAK)),
        "pb" => Some(Node::new(node::HORIZONTAL_RULE)),

        // Default: an element name this reader doesn't specifically
        // recognize. Raw-preserve it as a generic tagged span/div rather
        // than silently unwrapping it into its parent — losing the fact
        // that `<foo>` ever existed (as opposed to its text just being
        // loose in the parent) is exactly the silent-drop CLAUDE.md
        // forbids. Known structural wrapper elements (`TEI`, `text`,
        // `body`, `teiHeader`, etc.) are matched explicitly above and
        // return `None` on purpose — this arm only catches names genuinely
        // outside the vocabulary this reader models. Branching on
        // `is_block_element` (mirroring `rescribe-read-html`'s
        // `is_block_element` + bare-span/div split) keeps an unrecognized
        // *block*-level element block-shaped instead of producing a bare
        // `span` that the writer would then `<p>`-wrap, which would
        // misrepresent it as inline content.
        _ => {
            if is_block_element(name) {
                Some(generic_div(name, attrs, children))
            } else {
                Some(generic_span(name, attrs, children))
            }
        }
    };

    node.map(|n| attach_generic_attrs(n, attrs))
}

/// Extract `<teiHeader>` metadata: title (searched for as a `HEADING`, per
/// the original reader's approach) plus author/editor/publisher/idno/
/// language/abstract/keywords/revision-history, each surfaced by
/// `convert_element` as a `span` tagged with `tei:tag` so this function can
/// find them regardless of which `fileDesc`/`publicationStmt`/`profileDesc`/
/// etc. wrapper they were nested under.
///
/// Every other teiHeader child (e.g. `<msDesc>`, `<encodingDesc>`,
/// `<particDesc>`, `<projectDesc>`, or any other TEI header element this
/// reader has no dedicated mapping for) was raw-captured by
/// `convert_children` — see `is_modeled_header_field` — and shows up here as
/// a `span`/`div` carrying a `tei:raw` prop. That subtree's original XML is
/// stored verbatim as `{tag}_raw` metadata (plus a `{tag}` flattened-text
/// convenience copy) so `rescribe-write-tei` can splice it back
/// byte-for-byte; nothing was lost, so descendants aren't recursed into
/// separately. Only if raw capture itself failed (non-UTF8 content — the
/// XML source was already UTF8, so this is not expected in practice) does
/// this fall back to the old flatten-to-text-plus-fidelity-warning path.
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
            && let Some(tag) = node.props.get_str("tei:tag")
        {
            let text = extract_text(&node.children);
            match tag {
                "author" => append_metadata(metadata, "author", &text),
                "editor" => append_metadata(metadata, "editor", &text),
                "publisher" if !text.is_empty() => metadata.set("publisher", text),
                "idno" if !text.is_empty() => metadata.set("idno", text),
                "language" => {
                    let lang = node
                        .props
                        .get_str("tei:ident")
                        .map(str::to_string)
                        .unwrap_or(text);
                    if !lang.is_empty() {
                        metadata.set("language", lang);
                    }
                }
                "abstract" if !text.is_empty() => metadata.set("abstract", text),
                "keywords" => {
                    let terms = collect_terms(&node.children);
                    if !terms.is_empty() {
                        metadata.set("keywords", terms.join(", "));
                    } else if !text.is_empty() {
                        metadata.set("keywords", text);
                    }
                }
                "change" if !text.is_empty() => {
                    let entry = match node.props.get_str("tei:when") {
                        Some(when) => format!("{when}: {text}"),
                        None => text,
                    };
                    append_metadata(metadata, "revisions", &entry);
                }
                // A bibliographic/cited-work `<title>` nested somewhere
                // other than `titleStmt`/`teiHeader` directly (e.g. inside
                // a `sourceDesc` `<bibl>`) — `convert_element` tags it
                // `tei:tag = "title"` but it is not the document title, and
                // `is_modeled_header_field` excludes it from raw-capture
                // (see that function's doc comment) precisely because it's
                // already-modeled ground for the real title. Drop it here
                // rather than let the generic fallback below overwrite the
                // document's actual `title` metadata key with it.
                "title" => {}
                // Any other teiHeader field: `convert_children` already
                // attempted to raw-capture its whole subtree (unless it was
                // one of the explicit arms above). If that succeeded, splice
                // it back losslessly via metadata; the whole subtree is
                // preserved, so there's nothing more to gain from recursing
                // into its (already-flattened) children.
                other if !text.is_empty() || node.props.get_str("tei:raw").is_some() => {
                    if !text.is_empty() {
                        metadata.set(other.to_string(), text.clone());
                    }
                    match node.props.get_str("tei:raw") {
                        Some(raw) => {
                            metadata.set(format!("{other}_raw"), raw.to_string());
                            continue;
                        }
                        None => warnings.push(FidelityWarning::new(
                            Severity::Minor,
                            WarningKind::FeatureLost(format!("tei-header-field-{other}")),
                            format!(
                                "teiHeader <{other}> internal structure is not modeled and its \
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

/// Collect the text of every `definition_term` descendant (used for
/// `<keywords><term>…</term>…</keywords>`).
fn collect_terms(nodes: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    for node in nodes {
        if node.kind.as_str() == node::DEFINITION_TERM {
            let text = extract_text(&node.children);
            if !text.is_empty() {
                out.push(text);
            }
        }
        out.extend(collect_terms(&node.children));
    }
    out
}

/// Set a metadata field, joining onto an existing value with `"; "` rather
/// than overwriting it — used for repeatable teiHeader fields.
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
    fn test_parse_simple_document() {
        let tei = r#"<?xml version="1.0"?>
<TEI xmlns="http://www.tei-c.org/ns/1.0">
  <teiHeader>
    <fileDesc>
      <titleStmt>
        <title>Test Document</title>
      </titleStmt>
    </fileDesc>
  </teiHeader>
  <text>
    <body>
      <p>Hello, world!</p>
    </body>
  </text>
</TEI>"#;

        let result = parse(tei).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
        assert_eq!(doc.metadata.get_str("title"), Some("Test Document"));
    }

    #[test]
    fn test_parse_divisions() {
        let tei = r#"<?xml version="1.0"?>
<TEI>
  <text>
    <body>
      <div>
        <head>Introduction</head>
        <p>Content here.</p>
      </div>
    </body>
  </text>
</TEI>"#;

        let result = parse(tei).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_lists() {
        let tei = r#"<?xml version="1.0"?>
<TEI>
  <text>
    <body>
      <list>
        <item>Item 1</item>
        <item>Item 2</item>
      </list>
    </body>
  </text>
</TEI>"#;

        let result = parse(tei).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_formatting() {
        let tei = r#"<?xml version="1.0"?>
<TEI>
  <text>
    <body>
      <p><hi rend="italic">italic</hi> and <hi rend="bold">bold</hi> text</p>
    </body>
  </text>
</TEI>"#;

        let result = parse(tei).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_parse_table() {
        let tei = r#"<?xml version="1.0"?>
<TEI>
  <text>
    <body>
      <table>
        <row>
          <cell rend="header">Header</cell>
        </row>
        <row>
          <cell>Cell</cell>
        </row>
      </table>
    </body>
  </text>
</TEI>"#;

        let result = parse(tei).unwrap();
        let doc = result.value;
        assert!(!doc.content.children.is_empty());
    }

    #[test]
    fn test_xml_id_and_n_preserved() {
        // Regression test for the fidelity bug found while extracting
        // tei-fmt: the old hand-rolled reader captured `xml:id`/`n` into a
        // FrameAttrs struct but never read them back out, so they were
        // parsed and then silently discarded on every element.
        let tei =
            r#"<TEI><text><body><div xml:id="d1" n="1"><p>Text</p></div></body></text></TEI>"#;
        let result = parse(tei).unwrap();
        let doc = result.value;
        let div = doc
            .content
            .children
            .iter()
            .find(|n| n.kind.as_str() == node::DIV)
            .expect("div node");
        assert_eq!(div.props.get_str(prop::ID), Some("d1"));
        assert_eq!(div.props.get_str("tei:n"), Some("1"));
    }

    #[test]
    fn test_unresolvable_entity_preserved() {
        let tei = r#"<TEI><text><body><p>a &custom; b</p></body></text></TEI>"#;
        let result = parse(tei).unwrap();
        let doc = result.value;
        let para = doc
            .content
            .children
            .iter()
            .find(|n| n.kind.as_str() == node::PARAGRAPH)
            .expect("paragraph node");
        assert!(
            para.children
                .iter()
                .any(|n| n.kind.as_str() == node::RAW_INLINE)
        );
    }
}
