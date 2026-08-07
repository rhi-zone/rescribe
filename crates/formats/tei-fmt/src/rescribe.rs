//! AST↔`rescribe::Document` translation, gated behind the `rescribe` feature.
//!
//! Translates `tei_fmt::TeiDoc` (this crate's own standalone TEI/XML AST)
//! to and from rescribe's `Document` IR. Supports common TEI P5 elements
//! used in digital humanities. This module only ever calls into
//! `crate::parse`/`crate::emit`/`crate::emit_fragment` — it never tokenizes,
//! parses, or emits TEI/XML bytes itself (per CLAUDE.md's "The `rescribe`
//! feature module must never contain parsing or writing logic" rule).

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub mod read {
    use std::collections::HashMap;

    use crate::{Node as TNode, TeiDoc};

    use rescribe_core::{
        ConversionResult, Document, FidelityWarning, Node, ParseError, PropValue, Properties,
        Severity, WarningKind,
    };
    use rescribe_format_api::Parse as _;
    use rescribe_std::{node, prop};

    /// Parse TEI XML into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        let (doc, diagnostics) = TeiDoc::parse(input.as_bytes());

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
                let converted =
                    convert_children(kids, name, false, false, &mut metadata, &mut warnings);
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
    ///
    /// `in_biblio` is true when `parent_name` is a bibliographic reference
    /// container (`<biblStruct>`/`<bibl>` — see [`is_biblio_container`] — or a
    /// descendant of one that is itself a structural pass-through wrapper, see
    /// [`is_biblio_field_wrapper`]) — i.e. whether the *children* of
    /// `parent_name` are citation sub-fields that should be dispatched through
    /// [`convert_biblio_field`] (producing `bibliography_field` nodes, or a
    /// nested `bibliography_entry` for `<monogr>`/`<series>`) rather than the
    /// generic [`convert_element`]. Mirrors the `in_header` threading above and
    /// the identical pattern used for `rescribe-read-jats`'s `in_biblio`
    /// (`060c0858d5`).
    fn convert_children(
        children: &[TNode],
        parent_name: &str,
        in_header: bool,
        in_biblio: bool,
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
                    // Whether `name`'s own children should also be dispatched
                    // through `convert_biblio_field`: if we're not in biblio
                    // scope yet, entering it requires `name` itself to be a
                    // reference container (`is_biblio_container` — `<bibl>`
                    // only counts when directly inside `<listBibl>`, since a
                    // bare `<bibl>` elsewhere in running prose — e.g. attributing
                    // a `<cit>`'s quote — is a lightweight inline citation, not a
                    // full bibliography entry; see the `int-cit-bibl` fixture);
                    // if we're already inside one, scope only continues through
                    // a structural pass-through wrapper (`is_biblio_field_wrapper`
                    // — `<analytic>`/`<imprint>` splice their fields straight
                    // into the entry, `<monogr>`/`<series>` become their own
                    // nested entry) — everything else is a *leaf* field
                    // (`<author>`, `<title>`, `<date>`, ...), whose own children
                    // are ordinary markup-capable inline content, not further
                    // sub-fields. Without this distinction, `<hi>` inside e.g.
                    // a `<title>` would itself be mis-dispatched as a
                    // raw-preserved "misc" field instead of a proper `emphasis`
                    // node, silently flattening the markup it exists to
                    // preserve.
                    let child_in_biblio = if in_biblio {
                        is_biblio_field_wrapper(name)
                    } else {
                        is_biblio_container(name, parent_name)
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
                        // Inside a reference container, every child is a
                        // bibliographic sub-field — dispatch through the
                        // dedicated converter instead of the generic element
                        // table, and splice its result(s) straight in (no
                        // header-raw-capture logic applies here; see
                        // `convert_biblio_field`).
                        out.extend(convert_biblio_field(name, attrs, converted_kids));
                        continue;
                    }
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
                            String::from_utf8(crate::emit_fragment(std::slice::from_ref(child)))
                                .ok();
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
                TNode::Comment { .. }
                | TNode::ProcessingInstruction { .. }
                | TNode::Doctype { .. } => {
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
    ///
    /// Verified 2026-07-27 against the TEI P5 Guidelines reference
    /// (tei-c.org/release/doc/tei-p5-doc/en/html/ref-*.html and the
    /// `model.*` content-model-class pages), mirroring the docbook-fmt pass
    /// (see `abd6dd447d`). Every element already listed checked out against
    /// its TEI model class (`model.pLike`, `model.inter`, `model.titlepagePart`,
    /// `model.physDescPart`, `macro.specialPara`, etc. — all block-level);
    /// `l` in particular was double-checked since it's also a legal `<p>`
    /// child, but that just puts it in the same `model.inter`/`model.paraPart`
    /// family as `list`/`table`/`quote`, not phrase-level. Three block-level
    /// elements were missing and are added here: `objectDesc`/`supportDesc`
    /// (`model.physDescPart`, siblings of the already-listed `additions`/
    /// `accMat`) and `titlePart` (`model.titlepagePart`, sibling of the
    /// already-listed `docTitle`/`docAuthor`/etc.). No fixtures exercised the
    /// old (missing) shape for these three, so none needed updating.
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
                | "analytic"
                | "monogr"
                | "series"
                | "imprint"
                // Manuscript description / title page apparatus
                | "msContents"
                | "msItem"
                | "msIdentifier"
                | "physDesc"
                | "objectDesc"
                | "supportDesc"
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
                | "titlePart"
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
            // Bibliography. `<listBibl>` is a bibliography container; its own
            // `<head>` (e.g. "References") is handled by the `"head"` arm above
            // like any other block's title, so by the time we get here
            // `children` is already a mix of the resulting `HEADING` (if any)
            // and `bibliography_entry` nodes (each `<bibl>`/`<biblStruct>` child
            // was converted through `build_bibliography_entry` before reaching
            // here, via `convert_children`'s `in_biblio` threading — see
            // `is_biblio_container`).
            "listBibl" => Some(Node::new(node::BIBLIOGRAPHY).children(children)),
            // A `<biblStruct>` reached here always has `convert_children` having
            // already run over its own children with `in_biblio = true` (per
            // `is_biblio_container`, which admits `<biblStruct>` unconditionally
            // — unlike `<bibl>`, it has no other common use), so `children` is
            // already the fully-built field/nested-entry/date-marker list
            // `build_bibliography_entry` assembles.
            "biblStruct" => Some(build_bibliography_entry("biblStruct", attrs, children)),
            // `<bibl>` is TEI's loose, mixed-content citation form — but it is
            // also legitimately used as a lightweight inline citation/
            // attribution *outside* a bibliography list (e.g. `<cit><quote>…
            // </quote><bibl>…</bibl></cit>` — see the `int-cit-bibl` fixture).
            // Only elevate it to a full `bibliography_entry` when it is a direct
            // child of `<listBibl>` (see `is_biblio_container`, which this arm
            // must agree with exactly); anywhere else, keep the pre-existing
            // raw-preserving `span` mapping.
            "bibl" => {
                if parent == Some("listBibl") {
                    Some(build_bibliography_entry("bibl", attrs, children))
                } else {
                    Some(generic_span("bibl", attrs, children))
                }
            }

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
                Some("underline") | Some("u") => {
                    Some(Node::new(node::UNDERLINE).children(children))
                }
                Some("strike") | Some("strikethrough") => {
                    Some(Node::new(node::STRIKEOUT).children(children))
                }
                Some("sup") | Some("superscript") => {
                    Some(Node::new(node::SUPERSCRIPT).children(children))
                }
                Some("sub") | Some("subscript") => {
                    Some(Node::new(node::SUBSCRIPT).children(children))
                }
                Some("sc") | Some("smallcaps") => {
                    Some(Node::new(node::SMALL_CAPS).children(children))
                }
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
            "persName" | "placeName" | "orgName" | "name" | "seg" | "w" | "pc" | "choice"
            | "orig" | "reg" | "sic" | "corr" | "add" | "del" | "supplied" | "unclear" | "abbr"
            | "expan" => Some(generic_span(name, attrs, children)),
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

    /// Whether `name` is a TEI bibliographic reference container — its children
    /// are citation sub-fields, not ordinary document content, so
    /// `convert_children` dispatches them through [`convert_biblio_field`]
    /// instead of [`convert_element`]. `<biblStruct>` always counts (it has no
    /// other use in TEI); `<bibl>` only counts when it is a direct child of
    /// `<listBibl>` — see the `"bibl"` arm of [`convert_element`], which this
    /// must agree with exactly, for why a bare `<bibl>` elsewhere is left as a
    /// lightweight inline citation instead.
    fn is_biblio_container(name: &str, parent_name: &str) -> bool {
        name == "biblStruct" || (name == "bibl" && parent_name == "listBibl")
    }

    /// Whether `name`, encountered *while already inside* biblio scope, is a
    /// structural pass-through wrapper whose own children remain citation
    /// sub-fields (as opposed to a leaf field like `<author>`/`<title>`, whose
    /// children are ordinary inline content — see `convert_children`'s
    /// `child_in_biblio` computation for why this distinction matters).
    ///
    /// `<analytic>` groups the fields describing the citation's own entity (an
    /// article's title/authors); `<imprint>` groups the publication facts
    /// (`<publisher>`/`<pubPlace>`/`<date>`) inside a `<monogr>`. Both are
    /// presentational groupings of *this* entry's own fields, so their children
    /// splice straight into the entry (see `convert_biblio_field`'s
    /// `"analytic" | "imprint"` arm) rather than becoming a nested entity.
    ///
    /// `<monogr>`/`<series>` are different: they describe the *containing*
    /// publication (the journal/book an article appeared in, or the series a
    /// book belongs to) as its own citable unit — modeled as a nested
    /// `bibliography_entry` (see `convert_biblio_field`'s `"monogr" | "series"`
    /// arm), mirroring DocBook's `<biblioset>` nesting. This is an explicit
    /// human-approved fork resolution from the original design session for the
    /// TEI vertical, not a fresh design choice made here: the analytic level's
    /// fields flatten into the entry's direct children, while monogr/series
    /// become nested `bibliography_entry` children.
    fn is_biblio_field_wrapper(name: &str) -> bool {
        matches!(name, "analytic" | "monogr" | "series" | "imprint")
    }

    /// Convert one child of a bibliographic reference container (see
    /// [`is_biblio_container`]/[`is_biblio_field_wrapper`]) into zero or more IR
    /// nodes.
    ///
    /// `children` are `name`'s own children, already recursively converted
    /// (inheriting the same biblio-field dispatch — see `convert_children`'s
    /// `in_biblio` threading).
    fn convert_biblio_field(
        name: &str,
        attrs: &[(String, String)],
        children: Vec<Node>,
    ) -> Vec<Node> {
        match name {
            // Transparent structural wrappers: splice their own
            // (already-converted) children straight into the entry as siblings
            // rather than nesting one level deeper — see `is_biblio_field_wrapper`.
            "analytic" | "imprint" => children,

            // The containing publication (journal/book/series), modeled as its
            // own nested `bibliography_entry` — see `is_biblio_field_wrapper`.
            "monogr" | "series" => vec![build_bibliography_entry(name, attrs, children)],

            "author" => vec![bib_field("author", "author", children, None)],
            "editor" => vec![bib_field("editor", "editor", children, None)],

            // `<title>`'s `@level` (`a`/`m`/`s`/`u`) names which bibliographic
            // level it describes, but the analytic/monogr/series structural
            // nesting already tells `rescribe-write-tei` which level a title
            // belongs to positionally — `@level` is preserved verbatim as
            // `tei:attr:level` purely so an unusual original value (or a
            // `<title>` with no clear structural home) round-trips exactly,
            // without the writer needing to guess it back from context alone.
            "title" => {
                let mut node = bib_field("title", "title", children, None);
                if let Some(level) = get_attr(attrs, "level") {
                    node = node.prop("tei:attr:level", level.to_string());
                }
                vec![node]
            }

            "publisher" => vec![bib_field("publisher", "publisher", children, None)],
            "pubPlace" => vec![bib_field("publisher_location", "pubPlace", children, None)],
            "edition" => vec![bib_field("edition", "edition", children, None)],

            // `@unit` names what `<biblScope>` measures: `volume`/`vol` and
            // `issue`/`number` map directly onto the standard roles; a
            // `page`/`pp` scope with explicit `@from`/`@to` bounds splits
            // unambiguously into `page_first`/`page_last` (each a plain-text
            // field, since a page number carries no markup); anything else
            // (an unbounded page range as free text, or any other unit value)
            // is kept as a `misc` field with the original `@unit` preserved
            // raw, rather than guessing at a split.
            "biblScope" => convert_bibl_scope(attrs, children),

            // `@type` (`doi`/`isbn`/`issn`/`url`/... plus an open vocabulary)
            // names the identifier scheme.
            "idno" => vec![bib_field(
                "identifier",
                "idno",
                children,
                get_attr(attrs, "type"),
            )],

            // `att.datable`'s dating attributes (`@when`/`@notBefore`/
            // `@notAfter`/`@from`/`@to`, or their `-iso`-suffixed siblings) —
            // see `date_marker`/`resolve_tei_date` for how these become the
            // structured `prop::DATE` (or are demoted to a raw-preserved `misc`
            // field when they can't be).
            "date" => vec![date_marker(attrs, children)],

            // Every other citation-scope element this reader has no dedicated
            // mapping for (`respStmt`, `extent`, `note`, `textLang`,
            // `availability`, `distributor`, `authority`, `ptr`/`ref` used as a
            // citation link, ...): raw-preserve as a `misc` field tagged with
            // the original element name (its own children stay ordinary
            // markup-capable inline nodes), rather than silently dropping it.
            _ => vec![bib_field("misc", name, children, None)],
        }
    }

    /// Convert a `<biblScope>` element into one or more `bibliography_field`
    /// nodes — see `convert_biblio_field`'s `"biblScope"` arm for the role
    /// mapping this implements.
    fn convert_bibl_scope(attrs: &[(String, String)], children: Vec<Node>) -> Vec<Node> {
        let unit = get_attr(attrs, "unit");
        if let Some(unit_str @ ("page" | "pp")) = unit
            && let (Some(from), Some(to)) = (get_attr(attrs, "from"), get_attr(attrs, "to"))
        {
            let page_field = |role: &str, text: &str| {
                Node::new(node::BIBLIOGRAPHY_FIELD)
                    .prop(prop::FIELD_ROLE, role.to_string())
                    .prop("tei:tag", "biblScope")
                    .prop("tei:attr:unit", unit_str.to_string())
                    .child(Node::new(node::TEXT).prop(prop::CONTENT, text.to_string()))
            };
            return vec![page_field("page_first", from), page_field("page_last", to)];
        }
        let role = match unit {
            Some("volume") | Some("vol") => "volume",
            Some("issue") | Some("number") => "issue",
            _ => "misc",
        };
        let mut node = bib_field(role, "biblScope", children, None);
        if let Some(u) = unit {
            node = node.prop("tei:attr:unit", u.to_string());
        }
        vec![node]
    }

    /// Build one `bibliography_field` node: `role` is the standard
    /// `prop::FIELD_ROLE` value; `tag` is the original TEI element name
    /// (round-tripped via `tei:tag` so `rescribe-write-tei` can restore the
    /// exact source element); `scheme`, if given, becomes `prop::FIELD_SCHEME`
    /// (used only by `<idno>`'s `@type`).
    fn bib_field(role: &str, tag: &str, children: Vec<Node>, scheme: Option<&str>) -> Node {
        let mut node = Node::new(node::BIBLIOGRAPHY_FIELD)
            .prop(prop::FIELD_ROLE, role.to_string())
            .prop("tei:tag", tag.to_string())
            .children(children);
        if let Some(scheme) = scheme {
            node = node.prop(prop::FIELD_SCHEME, scheme.to_string());
        }
        node
    }

    /// The five TEI `att.datable` dating-value attribute names this reader
    /// understands. `when` marks a single point in time; `notBefore`/
    /// `notAfter`/`from`/`to` each mark a one-sided *bound*, not a point. See
    /// [`resolve_tei_date`] for how each is resolved, and its doc comment for
    /// the documented fork around `notBefore`+`notAfter`/`from`+`to` pairs.
    const DATE_ATTRS: [&str; 5] = ["when", "notBefore", "notAfter", "from", "to"];

    /// Look up a TEI dating attribute's effective value: the `-iso`-suffixed
    /// sibling (e.g. `when-iso`) is preferred when present, per the TEI
    /// Guidelines' `att.datable.iso` class — it supplies an ISO-normalized value
    /// alongside a base attribute that may use a non-ISO dating scheme.
    fn tei_date_value<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
        get_attr(attrs, &format!("{name}-iso")).or_else(|| get_attr(attrs, name))
    }

    /// Resolve a `<date>` element's structured date from its `att.datable`
    /// attributes. Returns `(map, attr_name)` when exactly one dating attribute
    /// resolves unambiguously — `attr_name` is the base attribute name used
    /// (`when`/`notBefore`/`notAfter`/`from`/`to`, without the `-iso` suffix
    /// even if the `-iso` sibling supplied the actual value), recorded by the
    /// caller as `tei:date-attr` so a reader can tell a point-in-time (`when`)
    /// apart from a one-sided bound (the other four) without that distinction
    /// being lost by flattening both into the same bare Map.
    ///
    /// **Documented design fork**: returns `None` — rather than invent a range
    /// representation — when *both* `notBefore`+`notAfter` or *both* `from`+`to`
    /// are present together. Those pairs jointly express a genuine two-point
    /// RANGE (a lower bound and an upper bound), which does not fit
    /// `prop::DATE`'s single year/month/day Map at all — there is no single
    /// "the" point to store, unlike a lone `notBefore` or `notAfter` (which is
    /// adequately captured as one bound, tagged via `tei:date-attr`). This is
    /// exactly the fork the original task brief flagged as a possible
    /// structural mismatch; per CLAUDE.md's no-guessing rule this is not
    /// resolved here — see TODO.md. The caller falls back to raw-preserving the
    /// original attributes on a `misc` field instead of populating `prop::DATE`
    /// for this case, so nothing is silently dropped; only the *modeling* of a
    /// two-point range remains an open question for a human to decide.
    fn resolve_tei_date(
        attrs: &[(String, String)],
    ) -> Option<(HashMap<String, PropValue>, &'static str)> {
        let has = |n: &str| {
            get_attr(attrs, n).is_some() || get_attr(attrs, &format!("{n}-iso")).is_some()
        };
        let range_pair = (has("notBefore") && has("notAfter")) || (has("from") && has("to"));
        if range_pair {
            return None;
        }
        for name in DATE_ATTRS {
            if let Some(v) = tei_date_value(attrs, name)
                && let Some(map) = parse_tei_date_string(v)
            {
                return Some((map, name));
            }
        }
        None
    }

    /// Parse a dating attribute's unambiguous forms (`YYYY`, `YYYY-MM`,
    /// `YYYY-MM-DD`, optionally followed by a `T`-separated time component,
    /// which is ignored — `prop::DATE` has no time-of-day slot) into
    /// `prop::DATE`'s map. Returns `None` for anything else (TEI does not
    /// constrain these attributes to a single format — e.g. a non-Gregorian
    /// calendar date with no `-iso` sibling) rather than guess.
    fn parse_tei_date_string(text: &str) -> Option<HashMap<String, PropValue>> {
        let date_part = text.split('T').next().unwrap_or(text).trim();
        let parts: Vec<&str> = date_part.split('-').collect();
        match parts.as_slice() {
            [y] => Some(date_map(parse_year_text(y)?, None, None)),
            [y, m] => Some(date_map(
                parse_year_text(y)?,
                Some(parse_two_digit(m, 1..=12)?),
                None,
            )),
            [y, m, d] => Some(date_map(
                parse_year_text(y)?,
                Some(parse_two_digit(m, 1..=12)?),
                Some(parse_two_digit(d, 1..=31)?),
            )),
            _ => None,
        }
    }

    fn date_map(year: i64, month: Option<i64>, day: Option<i64>) -> HashMap<String, PropValue> {
        let mut map = HashMap::new();
        map.insert("year".to_string(), PropValue::Int(year));
        if let Some(month) = month {
            map.insert("month".to_string(), PropValue::Int(month));
        }
        if let Some(day) = day {
            map.insert("day".to_string(), PropValue::Int(day));
        }
        map
    }

    fn parse_year_text(text: &str) -> Option<i64> {
        let t = text.trim();
        if t.len() == 4 && t.chars().all(|c| c.is_ascii_digit()) {
            t.parse().ok()
        } else {
            None
        }
    }

    fn parse_two_digit(text: &str, range: std::ops::RangeInclusive<i64>) -> Option<i64> {
        if text.len() == 2 && text.chars().all(|c| c.is_ascii_digit()) {
            text.parse::<i64>().ok().filter(|n| range.contains(n))
        } else {
            None
        }
    }

    /// Build an internal `tei:_date` marker node for a `<date>` element (see
    /// `convert_biblio_field`'s `"date"` arm) — consumed and removed by
    /// `build_bibliography_entry`, never a real IR node kind that could leak
    /// into the final tree. Carries the resolved `prop::DATE` Map + `tei:date-
    /// attr` when `resolve_tei_date` succeeds, and *always* raw-preserves every
    /// dating attribute present (`tei:attr:{name}`) — needed both for the
    /// misc-field fallback (unresolved/range case) and so a second date on the
    /// same entry level (see `build_bibliography_entry`) isn't lost even when a
    /// first one was already promoted to `prop::DATE`. `children` (the date's
    /// own already-converted markup-capable content, e.g. `<date>1 May
    /// <num value="2020">2020</num></date>`) is kept on the marker so its
    /// display text can be recovered as `tei:date-text` even when the date
    /// resolves via an attribute.
    fn date_marker(attrs: &[(String, String)], children: Vec<Node>) -> Node {
        let mut marker = Node::new("tei:_date").children(children);
        if let Some((map, attr_name)) = resolve_tei_date(attrs) {
            marker = marker.prop(prop::DATE, PropValue::Map(map));
            marker = marker.prop("tei:date-attr", attr_name.to_string());
        }
        for name in DATE_ATTRS {
            if let Some(v) = get_attr(attrs, name) {
                marker = marker.prop(format!("tei:attr:{name}"), v.to_string());
            }
            let iso_name = format!("{name}-iso");
            if let Some(v) = get_attr(attrs, &iso_name) {
                marker = marker.prop(format!("tei:attr:{iso_name}"), v.to_string());
            }
        }
        marker
    }

    /// Build a `bibliography_entry` node for a `<biblStruct>`/`<bibl>`/
    /// `<monogr>`/`<series>` element. `tag` is the original element name
    /// (round-tripped via `tei:tag` so `rescribe-write-tei` knows which wrapper
    /// to re-emit, and whether `<analytic>`/`<imprint>` wrapping applies).
    /// `children` are the already-converted `bibliography_field`/nested-entry/
    /// date-marker siblings (see `convert_biblio_field`); this function pulls
    /// the internal `tei:_date` marker(s) back out. The *first* marker with a
    /// resolved date is promoted onto the entry as `prop::DATE` (+ `tei:date-
    /// attr`, + `tei:date-text` if the source `<date>` had its own display
    /// text); every other marker on this same level (an unresolved/range date,
    /// or a second date alongside the primary one — TEI permits e.g. both a
    /// publication date and a copyright date) is demoted to an ordinary `misc`
    /// field instead of being lost, preserving whichever raw `tei:attr:*`
    /// attributes it carried.
    fn build_bibliography_entry(
        tag: &str,
        attrs: &[(String, String)],
        children: Vec<Node>,
    ) -> Node {
        let mut date_markers = Vec::new();
        let mut kids = Vec::with_capacity(children.len());
        for child in children {
            if child.kind.as_str() == "tei:_date" {
                date_markers.push(child);
            } else {
                kids.push(child);
            }
        }
        let mut entry = Node::new(node::BIBLIOGRAPHY_ENTRY).prop("tei:tag", tag.to_string());
        let mut promoted = false;
        for marker in date_markers {
            if !promoted && let Some(PropValue::Map(map)) = marker.props.get(prop::DATE) {
                entry = entry.prop(prop::DATE, PropValue::Map(map.clone()));
                if let Some(attr_name) = marker.props.get_str("tei:date-attr") {
                    entry = entry.prop("tei:date-attr", attr_name.to_string());
                }
                let text = extract_text(&marker.children);
                if !text.is_empty() {
                    entry = entry.prop("tei:date-text", text);
                }
                promoted = true;
                continue;
            }
            let mut field = Node::new(node::BIBLIOGRAPHY_FIELD)
                .prop(prop::FIELD_ROLE, "misc")
                .prop("tei:tag", "date")
                .children(marker.children);
            for (key, value) in marker.props.iter() {
                if key.starts_with("tei:attr:") {
                    field = field.prop(key.clone(), value.clone());
                }
            }
            kids.push(field);
        }
        attach_generic_attrs(entry.children(kids), attrs)
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
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub mod write {
    use std::collections::HashMap;

    use crate::{Node as TNode, Span as TSpan, TeiDoc, XmlDecl};

    use rescribe_core::{ConversionResult, Document, EmitError, Node, PropValue};
    use rescribe_format_api::Emit as _;
    use rescribe_std::{node, prop};

    /// Emit a document to TEI XML.
    pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
        let warnings = Vec::new();

        let mut tei_children = Vec::new();

        // Write teiHeader if we have any header-shaped metadata.
        let title = doc.metadata.get_str("title");
        // Any metadata key ending in `_raw` is a whole-subtree verbatim capture
        // of an unmodeled teiHeader element (see the reader module's
        // `convert_children`/`extract_metadata` — `{tag}_raw`, e.g.
        // `msDesc_raw` or `particDesc_raw`). Collected once here so both the
        // "do we even need a header" check and the splice-back loop below
        // share one scan.
        let mut header_raw_fields: Vec<(&str, &str)> = doc
            .metadata
            .iter()
            .filter_map(|(key, _)| {
                let tag = key.strip_suffix("_raw")?;
                Some((tag, doc.metadata.get_str(key)?))
            })
            .collect();
        header_raw_fields.sort_unstable_by_key(|(tag, _)| *tag);
        let has_header_metadata = title.is_some()
            || doc.metadata.get_str("author").is_some()
            || doc.metadata.get_str("editor").is_some()
            || doc.metadata.get_str("publisher").is_some()
            || doc.metadata.get_str("idno").is_some()
            || doc.metadata.get_str("language").is_some()
            || doc.metadata.get_str("abstract").is_some()
            || doc.metadata.get_str("keywords").is_some()
            || doc.metadata.get_str("revisions").is_some()
            || !header_raw_fields.is_empty();
        if has_header_metadata {
            let mut title_stmt_children = Vec::new();
            title_stmt_children.push(tei_element(
                "title",
                vec![],
                vec![tei_text(title.unwrap_or("Untitled"))],
            ));
            for author in doc
                .metadata
                .get_str("author")
                .into_iter()
                .flat_map(split_joined)
            {
                title_stmt_children.push(tei_element("author", vec![], vec![tei_text(author)]));
            }
            for editor in doc
                .metadata
                .get_str("editor")
                .into_iter()
                .flat_map(split_joined)
            {
                title_stmt_children.push(tei_element("editor", vec![], vec![tei_text(editor)]));
            }

            let mut pub_stmt_children = vec![tei_element(
                "p",
                vec![],
                vec![tei_text("Generated by rescribe")],
            )];
            if let Some(publisher) = doc.metadata.get_str("publisher") {
                pub_stmt_children.push(tei_element("publisher", vec![], vec![tei_text(publisher)]));
            }
            if let Some(idno) = doc.metadata.get_str("idno") {
                pub_stmt_children.push(tei_element("idno", vec![], vec![tei_text(idno)]));
            }

            let file_desc_children = vec![
                tei_element("titleStmt", vec![], title_stmt_children),
                tei_element("publicationStmt", vec![], pub_stmt_children),
                tei_element(
                    "sourceDesc",
                    vec![],
                    vec![tei_element(
                        "p",
                        vec![],
                        vec![tei_text("Converted document")],
                    )],
                ),
            ];

            let mut profile_desc_children = Vec::new();
            if let Some(language) = doc.metadata.get_str("language") {
                profile_desc_children.push(tei_element(
                    "langUsage",
                    vec![],
                    vec![tei_element(
                        "language",
                        vec![("ident".to_string(), language.to_string())],
                        vec![],
                    )],
                ));
            }
            if let Some(abstract_text) = doc.metadata.get_str("abstract") {
                profile_desc_children.push(tei_element(
                    "abstract",
                    vec![],
                    vec![tei_text(abstract_text)],
                ));
            }
            if let Some(keywords) = doc.metadata.get_str("keywords") {
                let terms = keywords
                    .split(", ")
                    .map(|k| tei_element("term", vec![], vec![tei_text(k)]))
                    .collect();
                profile_desc_children.push(tei_element(
                    "textClass",
                    vec![],
                    vec![tei_element("keywords", vec![], terms)],
                ));
            }
            // `profileDesc`, `encodingDesc`, and `revisionDesc` are siblings of
            // `fileDesc` under `teiHeader` per the TEI schema (not nested
            // inside it) — the reader unwraps all of these wrappers into a
            // flat scan regardless, but matching real TEI element structure
            // keeps emitted documents schema-valid.
            let mut header_children = vec![tei_element("fileDesc", vec![], file_desc_children)];
            // Splice back every raw-captured teiHeader subtree byte-for-byte
            // (see the reader module's `convert_children`/`extract_metadata`
            // — any `{tag}_raw` metadata field, e.g. `msDesc_raw` or
            // `particDesc_raw`, not just the two historically hardcoded
            // `<msDesc>`/`<encodingDesc>` names). This is lossless where
            // reconstructing the element from its flattened text would not be;
            // sorted by tag for deterministic output since `Properties` iterates
            // in unspecified order.
            for (_, raw) in &header_raw_fields {
                header_children.push(TNode::Raw {
                    content: (*raw).to_string(),
                    span: TSpan::NONE,
                });
            }
            if !profile_desc_children.is_empty() {
                header_children.push(tei_element("profileDesc", vec![], profile_desc_children));
            }
            if let Some(revisions) = doc.metadata.get_str("revisions") {
                let changes = revisions
                    .split("; ")
                    .map(|entry| match entry.split_once(": ") {
                        Some((when, text)) => tei_element(
                            "change",
                            vec![("when".to_string(), when.to_string())],
                            vec![tei_text(text)],
                        ),
                        None => tei_element("change", vec![], vec![tei_text(entry)]),
                    })
                    .collect();
                header_children.push(tei_element("revisionDesc", vec![], changes));
            }

            tei_children.push(tei_element("teiHeader", vec![], header_children));
        }

        let mut body_children = Vec::new();
        for child in &doc.content.children {
            body_children.extend(write_node(child));
        }
        tei_children.push(tei_element(
            "text",
            vec![],
            vec![tei_element("body", vec![], body_children)],
        ));

        let root = TNode::Element {
            name: "TEI".to_string(),
            attrs: vec![(
                "xmlns".to_string(),
                "http://www.tei-c.org/ns/1.0".to_string(),
            )],
            children: tei_children,
            span: TSpan::NONE,
        };

        let doc_ast = TeiDoc {
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

    fn tei_element(name: &str, attrs: Vec<(String, String)>, children: Vec<TNode>) -> TNode {
        TNode::Element {
            name: name.to_string(),
            attrs,
            children,
            span: TSpan::NONE,
        }
    }

    /// Split a `"; "`-joined repeatable metadata field (see the reader
    /// module's `append_metadata`) back into its individual values.
    fn split_joined(value: &str) -> Vec<&str> {
        value.split("; ").collect()
    }

    fn tei_text(content: impl Into<String>) -> TNode {
        TNode::Text {
            content: content.into(),
            span: TSpan::NONE,
        }
    }

    /// Build the generic `xml:id`/`n`/`xml:lang`/`corresp`/`sameAs` attributes
    /// for a node, if the IR node carries the corresponding raw-preserved
    /// properties (see the reader module's `attach_generic_attrs` for the
    /// reader side of this round trip).
    fn generic_attrs(node: &Node) -> Vec<(String, String)> {
        let mut attrs = Vec::new();
        if let Some(id) = node.props.get_str(prop::ID) {
            attrs.push(("xml:id".to_string(), id.to_string()));
        }
        if let Some(n) = node.props.get_str("tei:n") {
            attrs.push(("n".to_string(), n.to_string()));
        }
        if let Some(lang) = node.props.get_str(prop::LANGUAGE) {
            attrs.push(("xml:lang".to_string(), lang.to_string()));
        }
        if let Some(corresp) = node.props.get_str("tei:corresp") {
            attrs.push(("corresp".to_string(), corresp.to_string()));
        }
        if let Some(same_as) = node.props.get_str("tei:same-as") {
            attrs.push(("sameAs".to_string(), same_as.to_string()));
        }
        attrs
    }

    /// Re-emit a `style:align` value as the matching TEI `rend` value, if
    /// present (the inverse of `align_from_rend` on the reader side).
    fn rend_from_align(node: &Node) -> Option<(String, String)> {
        node.props
            .get_str(prop::STYLE_ALIGN)
            .map(|align| ("rend".to_string(), align.to_string()))
    }

    /// Re-emit a generic `span` node tagged with `tei:tag` (see the reader
    /// module's `generic_span`) back to its original TEI element name, with
    /// any element-specific attributes it carried restored.
    fn write_generic_span(node: &Node, mut attrs: Vec<(String, String)>) -> Vec<TNode> {
        let tag = match node.props.get_str("tei:tag") {
            Some(tag) => tag,
            None => return node.children.iter().flat_map(write_inline).collect(),
        };
        match tag {
            "date" => {
                if let Some(when) = node.props.get_str("tei:when") {
                    attrs.push(("when".to_string(), when.to_string()));
                }
            }
            "num" => {
                if let Some(value) = node.props.get_str("tei:value") {
                    attrs.push(("value".to_string(), value.to_string()));
                }
            }
            "measure" => {
                if let Some(unit) = node.props.get_str("tei:unit") {
                    attrs.push(("unit".to_string(), unit.to_string()));
                }
                if let Some(quantity) = node.props.get_str("tei:quantity") {
                    attrs.push(("quantity".to_string(), quantity.to_string()));
                }
            }
            "milestone" => {
                if let Some(unit) = node.props.get_str("tei:unit") {
                    attrs.push(("unit".to_string(), unit.to_string()));
                }
            }
            "gap" => {
                if let Some(reason) = node.props.get_str("tei:reason") {
                    attrs.push(("reason".to_string(), reason.to_string()));
                }
                if let Some(extent) = node.props.get_str("tei:extent") {
                    attrs.push(("extent".to_string(), extent.to_string()));
                }
            }
            "space" => {
                if let Some(extent) = node.props.get_str("tei:extent") {
                    attrs.push(("extent".to_string(), extent.to_string()));
                }
            }
            "title" => {
                if let Some(level) = node.props.get_str("tei:level") {
                    attrs.push(("level".to_string(), level.to_string()));
                }
            }
            "language" => {
                if let Some(ident) = node.props.get_str("tei:ident") {
                    attrs.push(("ident".to_string(), ident.to_string()));
                }
            }
            "change" => {
                if let Some(when) = node.props.get_str("tei:when") {
                    attrs.push(("when".to_string(), when.to_string()));
                }
            }
            _ => {}
        }
        vec![tei_element(
            tag,
            attrs,
            node.children.iter().flat_map(write_inline).collect(),
        )]
    }

    /// Convert one rescribe IR (block-level) node into zero or more TEI AST
    /// nodes.
    fn write_node(node: &Node) -> Vec<TNode> {
        let attrs = generic_attrs(node);
        match node.kind.as_str() {
            node::DOCUMENT => node.children.iter().flat_map(write_node).collect(),

            node::DIV => {
                let mut div_attrs = attrs;
                if let Some(align) = rend_from_align(node) {
                    div_attrs.push(align);
                }
                let name = match node.props.get_str("tei:type") {
                    Some("verse") => "lg",
                    Some("sp") => "sp",
                    Some("epigraph") => "epigraph",
                    Some("argument") => "argument",
                    // No dedicated `tei:type` — this may be a `generic_div`
                    // (an unrecognized block-level element raw-preserved by
                    // the reader's catch-all, see the reader module's
                    // `generic_div`); re-emit its original tag name if so,
                    // falling back to plain `div`.
                    _ => node.props.get_str("tei:tag").unwrap_or("div"),
                };
                vec![tei_element(
                    name,
                    div_attrs,
                    node.children.iter().flat_map(write_node).collect(),
                )]
            }

            // `<listBibl>`'s content model is `(head?, (bibl|biblStruct|
            // biblFull|listBibl)+)`, which already has room for a bare `<head>`
            // — unlike JATS's `<ref-list>`, no special title-unwrapping is
            // needed here; the ordinary `node::HEADING` arm below already
            // produces exactly that.
            node::BIBLIOGRAPHY => vec![tei_element(
                "listBibl",
                attrs,
                node.children.iter().flat_map(write_node).collect(),
            )],

            node::BIBLIOGRAPHY_ENTRY => vec![write_bibliography_entry(node)],

            // A `bibliography_field` shouldn't normally appear outside a
            // `BIBLIOGRAPHY_ENTRY`'s own children (handled directly by
            // `write_bibliography_entry`/`write_entry_children`, not through
            // this dispatch), but delegate to the same field writer defensively
            // rather than falling to the generic "recurse into children"
            // catch-all below, which would drop the field's role/tag entirely.
            node::BIBLIOGRAPHY_FIELD => vec![write_bibliography_field(node, None)],

            node::HEADING => vec![tei_element(
                "head",
                attrs,
                node.children.iter().flat_map(write_inline).collect(),
            )],

            node::PARAGRAPH => {
                let mut p_attrs = attrs;
                let name = match node.props.get_str("tei:type") {
                    Some("line") => "l",
                    Some("speaker") => "speaker",
                    Some("stage") => "stage",
                    Some("byline") => "byline",
                    Some("dateline") => "dateline",
                    Some("salute") => "salute",
                    Some("signed") => "signed",
                    Some("trailer") => "trailer",
                    _ => match node.props.get_str("tei:tag") {
                        Some("ab") => "ab",
                        _ => "p",
                    },
                };
                if node.props.get_str("tei:tag") != Some("ab")
                    && let Some(align) = rend_from_align(node)
                {
                    p_attrs.push(align);
                }
                vec![tei_element(
                    name,
                    p_attrs,
                    node.children.iter().flat_map(write_inline).collect(),
                )]
            }

            node::BLOCKQUOTE => vec![tei_element(
                "quote",
                attrs,
                node.children.iter().flat_map(write_node).collect(),
            )],

            node::LIST => {
                if node.props.get_str("tei:type") == Some("castList") {
                    return vec![tei_element(
                        "castList",
                        attrs,
                        node.children.iter().flat_map(write_node).collect(),
                    )];
                }
                let ordered = node.props.get_bool(prop::ORDERED).unwrap_or(false);
                let mut list_attrs = attrs;
                if let Some(t) = node.props.get_str("tei:type") {
                    list_attrs.push(("type".to_string(), t.to_string()));
                }
                if ordered && node.props.get_str("tei:type") != Some("ordered") {
                    list_attrs.push(("rend".to_string(), "numbered".to_string()));
                }
                vec![tei_element(
                    "list",
                    list_attrs,
                    node.children.iter().flat_map(write_node).collect(),
                )]
            }

            node::LIST_ITEM => {
                let name = match node.props.get_str("tei:tag") {
                    Some("label") => "label",
                    Some("castItem") => "castItem",
                    _ => "item",
                };
                vec![tei_element(
                    name,
                    attrs,
                    node.children.iter().flat_map(write_node).collect(),
                )]
            }

            node::DEFINITION_LIST => vec![tei_element(
                "gloss",
                attrs,
                node.children.iter().flat_map(write_node).collect(),
            )],

            node::DEFINITION_TERM => vec![tei_element(
                "term",
                attrs,
                node.children.iter().flat_map(write_inline).collect(),
            )],

            node::DEFINITION_DESC => vec![tei_element(
                "def",
                attrs,
                node.children.iter().flat_map(write_node).collect(),
            )],

            node::CODE_BLOCK => {
                let content = node.props.get_str(prop::CONTENT).unwrap_or("");
                vec![tei_element("eg", attrs, vec![tei_text(content)])]
            }

            node::TABLE => vec![tei_element(
                "table",
                attrs,
                node.children.iter().flat_map(write_node).collect(),
            )],

            node::TABLE_HEAD | node::TABLE_BODY | node::TABLE_FOOT => {
                // TEI doesn't have thead/tbody, pass through.
                node.children.iter().flat_map(write_node).collect()
            }

            node::TABLE_ROW => vec![tei_element(
                "row",
                attrs,
                node.children.iter().flat_map(write_node).collect(),
            )],

            node::TABLE_CELL => {
                let mut cell_attrs = attrs;
                table_cell_dims(node, &mut cell_attrs);
                vec![tei_element(
                    "cell",
                    cell_attrs,
                    node.children.iter().flat_map(write_inline).collect(),
                )]
            }

            node::TABLE_HEADER => {
                let mut cell_attrs = attrs;
                cell_attrs.push(("rend".to_string(), "header".to_string()));
                table_cell_dims(node, &mut cell_attrs);
                vec![tei_element(
                    "cell",
                    cell_attrs,
                    node.children.iter().flat_map(write_inline).collect(),
                )]
            }

            node::FIGURE => vec![tei_element(
                "figure",
                attrs,
                node.children.iter().flat_map(write_node).collect(),
            )],

            "figcaption" => vec![tei_element(
                "figDesc",
                attrs,
                node.children.iter().flat_map(write_inline).collect(),
            )],

            node::IMAGE => {
                let mut graphic_attrs = attrs;
                graphic_url_attrs(node, &mut graphic_attrs);
                vec![tei_element("graphic", graphic_attrs, vec![])]
            }

            node::HORIZONTAL_RULE => vec![tei_element("pb", attrs, vec![])],

            node::FOOTNOTE_DEF => {
                let mut note_attrs = attrs;
                if let Some(place) = node.props.get_str("tei:place") {
                    note_attrs.push(("place".to_string(), place.to_string()));
                }
                if let Some(t) = node.props.get_str("tei:type") {
                    note_attrs.push(("type".to_string(), t.to_string()));
                }
                vec![tei_element(
                    "note",
                    note_attrs,
                    node.children.iter().flat_map(write_node).collect(),
                )]
            }

            "math_display" => {
                let mut children = Vec::new();
                if let Some(source) = node.props.get_str("math:source") {
                    children.push(tei_text(source));
                }
                vec![tei_element("formula", attrs, children)]
            }

            // Inline nodes that appear at block level: wrap in a <p>.
            node::TEXT | node::EMPHASIS | node::STRONG | node::CODE | node::LINK | node::SPAN => {
                vec![tei_element("p", vec![], write_inline(node))]
            }

            _ => node.children.iter().flat_map(write_node).collect(),
        }
    }

    /// Restore `cols`/`rows` attributes onto a `<cell>` from their
    /// raw-preserved `tei:cols`/`tei:rows` properties.
    fn table_cell_dims(node: &Node, attrs: &mut Vec<(String, String)>) {
        if let Some(cols) = node.props.get_str("tei:cols") {
            attrs.push(("cols".to_string(), cols.to_string()));
        }
        if let Some(rows) = node.props.get_str("tei:rows") {
            attrs.push(("rows".to_string(), rows.to_string()));
        }
    }

    /// Restore `url`/`width`/`height` attributes onto a `<graphic>` element.
    fn graphic_url_attrs(node: &Node, attrs: &mut Vec<(String, String)>) {
        if let Some(url) = node.props.get_str(prop::URL) {
            attrs.push(("url".to_string(), url.to_string()));
        }
        if let Some(width) = node.props.get_str("tei:width") {
            attrs.push(("width".to_string(), width.to_string()));
        }
        if let Some(height) = node.props.get_str("tei:height") {
            attrs.push(("height".to_string(), height.to_string()));
        }
    }

    /// Convert one rescribe IR (inline-level) node into zero or more TEI AST
    /// nodes.
    fn write_inline(node: &Node) -> Vec<TNode> {
        let attrs = generic_attrs(node);
        match node.kind.as_str() {
            node::TEXT => match node.props.get_str(prop::CONTENT) {
                Some(content) => vec![tei_text(content)],
                None => Vec::new(),
            },

            node::EMPHASIS => {
                // A `tei:rend` prop (attached by the reader when it saw a `rend`
                // value on `<hi>` it didn't otherwise recognize) takes priority
                // over the default "italic" mapping, so an unrecognized rend
                // value round-trips rather than being silently coerced.
                let rend = node.props.get_str("tei:rend").unwrap_or("italic");
                hi_element(rend, attrs, node)
            }
            node::STRONG => hi_element("bold", attrs, node),
            node::UNDERLINE => hi_element("underline", attrs, node),
            node::STRIKEOUT => hi_element("strike", attrs, node),
            node::SUBSCRIPT => hi_element("sub", attrs, node),
            node::SUPERSCRIPT => hi_element("sup", attrs, node),
            node::SMALL_CAPS => hi_element("sc", attrs, node),

            node::CODE => {
                let mut children: Vec<TNode> = node
                    .props
                    .get_str(prop::CONTENT)
                    .map(tei_text)
                    .into_iter()
                    .collect();
                children.extend(node.children.iter().flat_map(write_inline));
                vec![tei_element("code", attrs, children)]
            }

            node::SPAN => write_generic_span(node, attrs),

            node::LINK => {
                let mut link_attrs = attrs;
                if let Some(url) = node.props.get_str(prop::URL) {
                    link_attrs.push(("target".to_string(), url.to_string()));
                }
                vec![tei_element(
                    "ref",
                    link_attrs,
                    node.children.iter().flat_map(write_inline).collect(),
                )]
            }

            node::LINE_BREAK => vec![tei_element("lb", attrs, vec![])],

            node::SOFT_BREAK => vec![tei_text(" ")],

            node::IMAGE => {
                let mut graphic_attrs = attrs;
                graphic_url_attrs(node, &mut graphic_attrs);
                vec![tei_element("graphic", graphic_attrs, vec![])]
            }

            "math_inline" => {
                let mut children = Vec::new();
                if let Some(source) = node.props.get_str("math:source") {
                    children.push(tei_text(source));
                }
                vec![tei_element("formula", attrs, children)]
            }

            // A raw entity reference preserved by the reader: re-emit verbatim.
            node::RAW_INLINE => match node.props.get_str("tei:entity") {
                Some(name) => vec![TNode::EntityRef {
                    name: name.to_string(),
                    span: TSpan::NONE,
                }],
                None => node.children.iter().flat_map(write_inline).collect(),
            },

            _ => node.children.iter().flat_map(write_inline).collect(),
        }
    }

    fn hi_element(rend: &str, mut attrs: Vec<(String, String)>, node: &Node) -> Vec<TNode> {
        attrs.push(("rend".to_string(), rend.to_string()));
        vec![tei_element(
            "hi",
            attrs,
            node.children.iter().flat_map(write_inline).collect(),
        )]
    }

    /// The five TEI `att.datable` dating-value attribute names — see the
    /// reader module's constant of the same name.
    const DATE_ATTRS: [&str; 5] = ["when", "notBefore", "notAfter", "from", "to"];

    /// Write a `bibliography_entry` node back to `<biblStruct>`/`<bibl>`/
    /// `<monogr>`/`<series>` (see the reader module's `build_bibliography_entry`
    /// /`convert_biblio_field`'s `"monogr" | "series"` arm). `tei:tag` (set by
    /// every entry-producing arm of the reader) picks which element to
    /// re-emit, defaulting to `<biblStruct>` for an entry built by a non-TEI
    /// producer.
    ///
    /// - `"bibl"`: the loose, mixed-content form — fields and free text/nested
    ///   entries are written flat, in original document order, with no
    ///   `<analytic>`/`<imprint>` wrapping.
    /// - `"monogr"`/`"series"`: direct children of the entry, in original
    ///   order; for `"monogr"` specifically, `publisher`/`publisher_location`
    ///   fields (plus the entry's own resolved date, if any) are pulled into a
    ///   required `<imprint>` wrapper — TEI's own `monogr` content model groups
    ///   publication facts there — rather than left as direct `monogr` children.
    /// - anything else (`"biblStruct"`, or an entry from a non-TEI producer):
    ///   direct field children are the analytic level's own fields, wrapped in
    ///   `<analytic>` only when the entry also has nested `monogr`/`series`
    ///   children (see the reader module's `is_biblio_field_wrapper`'s doc
    ///   comment for why this pairing determines the wrap) — a "flat" entry
    ///   with no nested structure at all emits its fields directly instead of
    ///   inventing an `<analytic>` wrapper with no monogr/series counterpart.
    ///
    /// `prop::DATE` becomes a leading `<date>` child (see `write_citation_date`)
    /// — for `"monogr"` this lands inside the `<imprint>` wrapper alongside
    /// `publisher`/`pubPlace`; for every other tag it's the first child of
    /// whichever direct-field group it belongs to. Mirrors the "dates move to a
    /// canonical position rather than an exact original slot" precedent already
    /// established by `rescribe-write-jats::write_bibliography_entry`.
    fn write_bibliography_entry(node: &Node) -> TNode {
        let attrs = generic_attrs(node);
        let tag = node.props.get_str("tei:tag").unwrap_or("biblStruct");
        let date_kid = match node.props.get(prop::DATE) {
            Some(PropValue::Map(map)) => Some(write_citation_date(
                map,
                node.props.get_str("tei:date-attr").unwrap_or("when"),
                node.props.get_str("tei:date-text"),
            )),
            _ => None,
        };

        match tag {
            "bibl" => {
                let mut kids: Vec<TNode> = date_kid.into_iter().collect();
                write_entry_children(&node.children, None, None, None, &mut kids);
                tei_element("bibl", attrs, kids)
            }
            "series" => {
                let mut kids: Vec<TNode> = date_kid.into_iter().collect();
                write_entry_children(&node.children, Some("s"), None, None, &mut kids);
                tei_element("series", attrs, kids)
            }
            "monogr" => {
                let mut imprint_kids: Vec<TNode> = date_kid.into_iter().collect();
                let mut kids = Vec::new();
                write_entry_children(
                    &node.children,
                    Some("m"),
                    Some(&mut imprint_kids),
                    None,
                    &mut kids,
                );
                if !imprint_kids.is_empty() {
                    kids.push(tei_element("imprint", vec![], imprint_kids));
                }
                tei_element("monogr", attrs, kids)
            }
            _ => {
                let mut direct: Vec<TNode> = date_kid.into_iter().collect();
                let mut nested = Vec::new();
                write_entry_children(
                    &node.children,
                    Some("a"),
                    None,
                    Some(&mut nested),
                    &mut direct,
                );
                let mut kids = Vec::new();
                if !direct.is_empty() {
                    if !nested.is_empty() {
                        kids.push(tei_element("analytic", vec![], direct));
                    } else {
                        kids.extend(direct);
                    }
                }
                kids.extend(nested);
                tei_element(tag, attrs, kids)
            }
        }
    }

    /// Write an entry/monogr/series/bibl's own direct children — fields, nested
    /// entries, and free inline content (the loose `<bibl>` mixed-content case)
    /// — in original document order into `out`.
    ///
    /// `imprint_out`, when given, receives `publisher`/`publisher_location`
    /// fields instead of `out` (the `<monogr>` case — see
    /// `write_bibliography_entry`). `nested_out`, when given, receives nested
    /// `bibliography_entry` children (`<monogr>`/`<series>`) instead of `out`,
    /// so the caller can decide separately whether to wrap the remaining direct
    /// fields in `<analytic>` (the `"biblStruct"`-shaped default case).
    ///
    /// A `biblScope` `page_first` field immediately followed by its `page_last`
    /// sibling (see the reader module's `convert_bibl_scope`) is merged back
    /// into one `<biblScope unit="page" from="…" to="…">` rather than emitting
    /// two separate elements.
    fn write_entry_children(
        children: &[Node],
        default_title_level: Option<&str>,
        mut imprint_out: Option<&mut Vec<TNode>>,
        mut nested_out: Option<&mut Vec<TNode>>,
        out: &mut Vec<TNode>,
    ) {
        let mut iter = children.iter().peekable();
        while let Some(child) = iter.next() {
            if child.kind.as_str() == node::BIBLIOGRAPHY_ENTRY {
                let written = write_bibliography_entry(child);
                match nested_out.as_deref_mut() {
                    Some(nested) => nested.push(written),
                    None => out.push(written),
                }
                continue;
            }
            if child.kind.as_str() != node::BIBLIOGRAPHY_FIELD {
                out.extend(write_inline(child));
                continue;
            }
            let role = child.props.get_str(prop::FIELD_ROLE);
            if let Some(imprint) = imprint_out.as_deref_mut()
                && matches!(role, Some("publisher") | Some("publisher_location"))
            {
                imprint.push(write_bibliography_field(child, default_title_level));
                continue;
            }
            if child.props.get_str("tei:tag") == Some("biblScope")
                && role == Some("page_first")
                && let Some(next) = iter.next_if(|next| {
                    next.kind.as_str() == node::BIBLIOGRAPHY_FIELD
                        && next.props.get_str("tei:tag") == Some("biblScope")
                        && next.props.get_str(prop::FIELD_ROLE) == Some("page_last")
                })
            {
                let from = field_plain_text(child);
                let to = field_plain_text(next);
                out.push(tei_element(
                    "biblScope",
                    vec![
                        ("unit".to_string(), "page".to_string()),
                        ("from".to_string(), from),
                        ("to".to_string(), to),
                    ],
                    vec![],
                ));
                continue;
            }
            out.push(write_bibliography_field(child, default_title_level));
        }
    }

    /// Concatenate a `bibliography_field`'s plain-text `TEXT` children (used
    /// only for the synthetic `page_first`/`page_last` fields the reader
    /// module's `convert_bibl_scope` builds, which carry exactly one such
    /// child).
    fn field_plain_text(node: &Node) -> String {
        node.children
            .iter()
            .filter_map(|c| {
                if c.kind.as_str() == node::TEXT {
                    c.props.get_str(prop::CONTENT)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Write one `bibliography_field` node back to its originating TEI element.
    /// `tei:tag` (set by every arm of the reader module's
    /// `convert_biblio_field`) takes priority when present, since it names the
    /// exact source element; `prop::FIELD_ROLE` is the fallback for a field
    /// built by a non-TEI producer (a cross-format conversion into TEI).
    ///
    /// `default_title_level` supplies `<title>`'s `@level` when the field
    /// itself carries no raw-preserved `tei:attr:level` — see
    /// `write_bibliography_entry`'s per-tag calls for how the structural
    /// position (analytic/monogr/series) determines this default.
    fn write_bibliography_field(node: &Node, default_title_level: Option<&str>) -> TNode {
        let inline_children: Vec<TNode> = node.children.iter().flat_map(write_inline).collect();
        let role = node.props.get_str(prop::FIELD_ROLE).unwrap_or("misc");
        let tag = node
            .props
            .get_str("tei:tag")
            .unwrap_or_else(|| default_tag_for_role(role));
        let mut attrs = Vec::new();
        match tag {
            "title" => {
                let level = node.props.get_str("tei:attr:level").or(default_title_level);
                if let Some(level) = level {
                    attrs.push(("level".to_string(), level.to_string()));
                }
            }
            "idno" => {
                if let Some(scheme) = node.props.get_str(prop::FIELD_SCHEME) {
                    attrs.push(("type".to_string(), scheme.to_string()));
                }
            }
            "biblScope" => {
                if let Some(unit) = node.props.get_str("tei:attr:unit") {
                    attrs.push(("unit".to_string(), unit.to_string()));
                } else if let Some(unit) = match role {
                    "volume" => Some("volume"),
                    "issue" => Some("issue"),
                    _ => None,
                } {
                    attrs.push(("unit".to_string(), unit.to_string()));
                }
            }
            // A demoted secondary/unresolved date (see the reader module's
            // `build_bibliography_entry`) — restore whichever raw dating
            // attributes it carried.
            "date" => {
                for name in DATE_ATTRS {
                    if let Some(v) = node.props.get_str(&format!("tei:attr:{name}")) {
                        attrs.push((name.to_string(), v.to_string()));
                    }
                    let iso_name = format!("{name}-iso");
                    if let Some(v) = node.props.get_str(&format!("tei:attr:{iso_name}")) {
                        attrs.push((iso_name, v.to_string()));
                    }
                }
            }
            _ => {}
        }
        tei_element(tag, attrs, inline_children)
    }

    /// `prop::FIELD_ROLE`'s standard vocabulary, mapped to the TEI element a
    /// field with no `tei:tag` (i.e. built by a non-TEI producer) should
    /// re-emit as.
    fn default_tag_for_role(role: &str) -> &'static str {
        match role {
            "author" | "editor" => "author",
            "title" => "title",
            "container_title" => "title",
            "publisher" => "publisher",
            "publisher_location" => "pubPlace",
            "edition" => "edition",
            "volume" | "issue" | "page_first" | "page_last" => "biblScope",
            "identifier" => "idno",
            _ => "note",
        }
    }

    /// Format `prop::DATE`'s `year`/`month`/`day` map (see the property's own
    /// doc comment) into a `<date>` element using whichever `att.datable`
    /// attribute `tei:date-attr` (set by the reader module's
    /// `resolve_tei_date`) says was originally used — the inverse of that
    /// resolution. `text`, if the source `<date>` had its own display text
    /// (`tei:date-text`), becomes the element's content; otherwise the
    /// formatted ISO string doubles as the display text, since `<date>`
    /// requires *some* content to be schema-valid prose.
    fn write_citation_date(
        map: &HashMap<String, PropValue>,
        attr_name: &str,
        text: Option<&str>,
    ) -> TNode {
        let as_int = |key: &str| match map.get(key) {
            Some(PropValue::Int(i)) => Some(*i),
            _ => None,
        };
        let year = as_int("year");
        let month = as_int("month");
        let day = as_int("day");
        let iso = match (year, month, day) {
            (Some(y), Some(m), Some(d)) => format!("{y:04}-{m:02}-{d:02}"),
            (Some(y), Some(m), None) => format!("{y:04}-{m:02}"),
            _ => format!("{:04}", year.unwrap_or(0)),
        };
        let display = text.map(str::to_string).unwrap_or_else(|| iso.clone());
        tei_element(
            "date",
            vec![(attr_name.to_string(), iso)],
            vec![tei_text(display)],
        )
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
            assert!(xml.contains("<TEI"));
            assert!(xml.contains("</TEI>"));
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
            assert!(xml.contains("<hi rend=\"italic\">italic</hi>"));
            assert!(xml.contains("<hi rend=\"bold\">bold</hi>"));
        }

        #[test]
        fn test_emit_xml_id_and_n() {
            let doc = Document {
                content: Node::new(node::DOCUMENT).child(
                    Node::new(node::DIV)
                        .prop(prop::ID, "d1")
                        .prop("tei:n", "1")
                        .child(Node::new(node::PARAGRAPH)),
                ),
                resources: Default::default(),
                metadata: Properties::new(),
                source: None,
            };

            let result = emit(&doc).unwrap();
            let xml = String::from_utf8(result.value).unwrap();
            assert!(xml.contains("xml:id=\"d1\""));
            assert!(xml.contains("n=\"1\""));
        }

        #[test]
        fn test_roundtrip_through_reader() {
            let tei = r#"<?xml version="1.0"?>
<TEI><text><body><p>Hello <hi rend="italic">world</hi></p></body></text></TEI>"#;
            let parsed = super::super::read::parse(tei).unwrap();
            let emitted = emit(&parsed.value).unwrap();
            let xml = String::from_utf8(emitted.value).unwrap();
            assert!(xml.contains("<p>Hello <hi rend=\"italic\">world</hi></p>"));
        }

        #[test]
        fn test_roundtrip_xml_id_n_and_entity() {
            let tei = r#"<TEI><text><body><div xml:id="d1" n="2"><p>a &custom; b</p></div></body></text></TEI>"#;
            let parsed = super::super::read::parse(tei).unwrap();
            let emitted = emit(&parsed.value).unwrap();
            let xml = String::from_utf8(emitted.value).unwrap();
            assert!(xml.contains("xml:id=\"d1\""));
            assert!(xml.contains("n=\"2\""));
            assert!(xml.contains("&custom;"));
        }

        #[test]
        fn test_roundtrip_biblstruct_markup_survives() {
            // Regression test for the TEI citation/bibliography vertical: a
            // `<hi>` nested inside an `<analytic>` `<title>` must survive
            // parse -> emit -> reparse -> emit byte-for-byte, and the second
            // generation's output must be stable (proving `bibliography_field`
            // children are ordinary markup-capable inline nodes, not a flat
            // string property).
            let tei = r#"<?xml version="1.0" encoding="UTF-8"?>
<TEI xmlns="http://www.tei-c.org/ns/1.0"><text><body><listBibl><biblStruct><analytic><title level="a">Foo <hi rend="italic">Bar</hi></title></analytic><monogr><title level="m">Some Journal</title></monogr></biblStruct></listBibl></body></text></TEI>"#;
            let parsed = super::super::read::parse(tei).unwrap();
            let emitted = emit(&parsed.value).unwrap();
            let xml1 = String::from_utf8(emitted.value).unwrap();
            assert!(xml1.contains("<hi rend=\"italic\">Bar</hi>"));

            let reparsed = super::super::read::parse(&xml1).unwrap();
            let emitted2 = emit(&reparsed.value).unwrap();
            let xml2 = String::from_utf8(emitted2.value).unwrap();
            assert_eq!(
                xml1, xml2,
                "output must be stable across a second round trip"
            );
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::parse;

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::emit;
