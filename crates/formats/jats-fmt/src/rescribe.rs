//! AST↔`rescribe::Document` translation for JATS.
//!
//! This module only translates between `jats_fmt`'s `JatsDoc`/`Node` AST and
//! rescribe's `Document` IR — no XML tokenizing/parsing/emitting happens
//! here (that all lives in the rest of this crate; see `crate::parse` and
//! `crate::emit`). Enabled by the `rescribe` feature; each direction is
//! additionally gated on the reader/writer mode feature it depends on, so
//! enabling `rescribe` alone (with no mode feature) compiles nothing.
//!
//! Supports JATS 1.0/1.1/1.2/1.3 elements commonly used in scholarly
//! publishing. See the `read` module's doc comments for the detailed
//! element-by-element mapping (sections, titles, lists, tables, math,
//! bibliographic references, footnotes, ...) and its extensive raw-
//! preservation strategy for unmodeled constructs.

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
mod read {
    use std::collections::HashMap;

    use crate::{JatsDoc, Node as JNode};
    use rescribe_core::{
        ConversionResult, Document, FidelityWarning, Node, ParseError, PropValue, Properties,
        Severity, WarningKind,
    };
    use rescribe_format_api::Parse as _;
    use rescribe_std::{node, prop};

    /// Parse JATS XML into a document.
    pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
        let (doc, diagnostics) = JatsDoc::parse(input.as_bytes());

        let mut warnings: Vec<FidelityWarning> = diagnostics
            .into_iter()
            .map(|d| {
                FidelityWarning::new(
                    Severity::Major,
                    WarningKind::FeatureLost("xml-parse-error".to_string()),
                    format!("JATS XML parse error: {}", d.message),
                )
            })
            .collect();

        let mut metadata = Properties::new();
        let mut children = Vec::new();
        for top in &doc.nodes {
            if let JNode::Element {
                name,
                attrs,
                children: kids,
                ..
            } = top
            {
                let converted =
                    convert_children(kids, name, false, false, 0, &mut metadata, &mut warnings);
                match convert_element(name, attrs, converted.clone(), None, 0) {
                    Some(node) => children.push(node),
                    None => {
                        // The root element itself carries no rescribe-level
                        // semantics (shouldn't normally happen for `<article>`),
                        // pass its children through rather than dropping them.
                        children.extend(converted);
                    }
                }
            }
            // Leading/trailing Comment/PI/Doctype/whitespace-Text at the very
            // top level (outside the root element) carry no IR meaning and have
            // no cross-format equivalent to model; JATS documents otherwise
            // consist of exactly one root `<article>` element.
        }

        let document = Document {
            content: Node::new(node::DOCUMENT).children(children),
            resources: Default::default(),
            metadata,
            source: None,
        };

        Ok(ConversionResult::with_warnings(document, warnings))
    }

    /// Convert a slice of JATS child nodes into rescribe IR nodes, discarding
    /// nodes that only exist to be unwrapped (e.g. `<article-meta>`/
    /// `<journal-meta>`, which are consumed for metadata) and passing through
    /// "structural" wrapper elements (like `<title-group>`) as their own
    /// children.
    ///
    /// `in_header` is true when `parent_name` is `<article-meta>`/
    /// `<journal-meta>` itself or a descendant of it (threaded down through the
    /// recursion below) — i.e. whether the *children* of `parent_name` are
    /// front-matter content that will end up consumed by [`extract_metadata`]
    /// rather than surviving as document content nodes.
    ///
    /// `in_biblio` is true when `parent_name` is a bibliographic reference
    /// container (`<ref>`, or a descendant of one that is itself a structural
    /// pass-through such as `<element-citation>`/`<mixed-citation>`/`<date>` —
    /// threaded down the same way `in_header` is) — i.e. whether the *children*
    /// of `parent_name` are citation sub-fields that should be dispatched
    /// through [`convert_biblio_field`] (producing `bibliography_field` nodes)
    /// rather than the generic [`convert_element`]. `<person-group>` is handled
    /// as a special case directly in the loop below (see its own comment)
    /// rather than through this threading, since the author/editor role its
    /// children get depends on `<person-group>`'s own `person-group-type`
    /// attribute, not on the child element's own name.
    fn convert_children(
        children: &[JNode],
        parent_name: &str,
        in_header: bool,
        in_biblio: bool,
        sec_depth: usize,
        metadata: &mut Properties,
        warnings: &mut Vec<FidelityWarning>,
    ) -> Vec<Node> {
        let mut out = Vec::new();
        for child in children {
            match child {
                JNode::Element {
                    name,
                    attrs,
                    children: kids,
                    ..
                } => {
                    // `<person-group>` groups a set of `<name>`/`<collab>`/
                    // `<string-name>` (etc.) contributors whose IR role
                    // (author/editor) is determined by `<person-group>`'s own
                    // `person-group-type` attribute, not by any per-child
                    // element name — so it can't be handled through the
                    // generic `convert_biblio_field` dispatch below (which only
                    // sees one child element name/attrs/children at a time).
                    // Build its field nodes directly from its own raw children
                    // instead of recursing through the normal pipeline.
                    if in_biblio && name == "person-group" {
                        out.extend(convert_person_group(
                            attrs, kids, sec_depth, metadata, warnings,
                        ));
                        continue;
                    }
                    // `<mml:math>` inside `<disp-formula>`/`<inline-formula>` is
                    // real MathML markup (the JATS 1.3 Tag Library documents
                    // both formula elements as containing either `<tex-math>` OR
                    // `<mml:math>`, per
                    // https://jats.nlm.nih.gov/archiving/tag-library/1.3/element/disp-formula.html).
                    // Recursing through the normal pipeline here would flatten
                    // its `<mml:mrow>`/`<mml:mi>`/... structure through the
                    // generic catch-all and then destroy even that via
                    // `extract_text` in the `"disp-formula"`/`"inline-formula"`
                    // arm below — a real, currently-shipping loss bug. Capture
                    // the whole `<mml:math>` subtree verbatim instead (same
                    // `emit_fragment` raw-preservation mechanism used for
                    // unmodeled header children below) as a sentinel `SPAN`
                    // tagged `jats:tag = "mml-math-raw"`, which `split_mathml`
                    // pulls back out in the formula arm.
                    if matches!(parent_name, "disp-formula" | "inline-formula")
                        && name == "mml:math"
                    {
                        let raw =
                            String::from_utf8(crate::emit_fragment(std::slice::from_ref(child)))
                                .unwrap_or_default();
                        out.push(
                            Node::new(node::SPAN)
                                .prop("jats:tag", "mml-math-raw")
                                .prop(prop::CONTENT, raw),
                        );
                        continue;
                    }
                    // `<mml:math>`/`<tex-math>` are very commonly wrapped in an
                    // intervening `<alternatives>` inside `<disp-formula>`/
                    // `<inline-formula>` — the JATS-recommended pattern for
                    // offering both a MathML and a TeX rendering of the same
                    // formula (JATS 1.3 Tag Library, `<alternatives>`'s own
                    // expanded content model: `((object-id)*, (... | tex-math |
                    // mml:math)+)`). Without this, the interception above never
                    // fires (its immediate parent is `<alternatives>`, not
                    // `<disp-formula>`/`<inline-formula>`), so `<mml:math>` fell
                    // through to the generic catch-all and `extract_text`
                    // flattened *both* the TeX and MathML text into one
                    // concatenated, corrupted `math:source` string — a real,
                    // shipping loss bug in the `<mml:math>` raw-capture fix
                    // (`242d7d9ecb`). Treat `<alternatives>` as transparent here:
                    // find its `<mml:math>` child (if any) and raw-capture it the
                    // same way as the direct-child case above, via the same
                    // `mml-math-raw` sentinel `split_mathml` already knows how to
                    // pull back out — so the `"disp-formula"`/`"inline-formula"`
                    // arms need no changes of their own. Every *other* child of
                    // the `<alternatives>` (a sibling `<tex-math>`, or a rarer
                    // third alternative like `<graphic>`) is raw-preserved
                    // verbatim under `jats:alternatives-raw` on that same
                    // sentinel (see `split_mathml`) rather than dropped, per
                    // CLAUDE.md's losslessness rule — `rescribe-write-jats`
                    // re-wraps them back in `<alternatives>` on write (see
                    // `formula_children`). If no `<mml:math>` is present (e.g. a
                    // lone `<tex-math>` still wrapped in `<alternatives>`, or a
                    // future alternative type this reader doesn't specifically
                    // know about), fall back to ordinary conversion of
                    // `<alternatives>`'s children exactly as before this fix —
                    // `<tex-math>`'s own arm already flattens to plain text via
                    // the normal pass-through path, so existing behavior for the
                    // MathML-less case is unchanged.
                    if matches!(parent_name, "disp-formula" | "inline-formula")
                        && name == "alternatives"
                    {
                        let mut mathml_raw: Option<String> = None;
                        let mut other_raw: Vec<String> = Vec::new();
                        for alt_child in kids {
                            let is_mathml = matches!(alt_child, JNode::Element { name: n, .. } if n == "mml:math");
                            if is_mathml && mathml_raw.is_none() {
                                mathml_raw = String::from_utf8(crate::emit_fragment(
                                    std::slice::from_ref(alt_child),
                                ))
                                .ok();
                            } else if matches!(alt_child, JNode::Element { .. })
                                && let Ok(raw) = String::from_utf8(crate::emit_fragment(
                                    std::slice::from_ref(alt_child),
                                ))
                            {
                                other_raw.push(raw);
                            }
                        }
                        if let Some(raw) = mathml_raw {
                            let mut sentinel = Node::new(node::SPAN)
                                .prop("jats:tag", "mml-math-raw")
                                .prop(prop::CONTENT, raw);
                            if !other_raw.is_empty() {
                                sentinel =
                                    sentinel.prop("jats:alternatives-raw", other_raw.join(""));
                                warnings.push(FidelityWarning::new(
                                    Severity::Minor,
                                    WarningKind::FeatureLost("alternatives-representation".to_string()),
                                    format!(
                                        "<alternatives> inside <{parent_name}> offered more than one math representation; kept MathML as the modeled form and raw-preserved the other(s) verbatim"
                                    ),
                                ));
                            }
                            out.push(sentinel);
                            continue;
                        }
                        // No `<mml:math>` found — ordinary conversion (unchanged
                        // behavior for e.g. a lone `<tex-math>` still wrapped in
                        // `<alternatives>`).
                        out.extend(convert_children(
                            kids, name, in_header, in_biblio, sec_depth, metadata, warnings,
                        ));
                        continue;
                    }
                    let child_in_header =
                        in_header || matches!(name.as_str(), "article-meta" | "journal-meta");
                    // Whether *`name`'s own children* should also be dispatched
                    // through `convert_biblio_field` (as opposed to normal
                    // conversion): if we're not in biblio scope yet, entering it
                    // requires `name` itself to be a reference container
                    // (`is_biblio_container`, i.e. `<ref>`); if we're already
                    // inside one, scope only continues through a structural
                    // pass-through wrapper (`is_biblio_field_wrapper` — the
                    // citation-content elements `<element-citation>`/
                    // `<mixed-citation>`, and `<date>`, which itself wraps
                    // `<year>`/`<month>`/`<day>`) — everything else is a *leaf*
                    // field (`article-title`, `source`, `fpage`, ...), and a
                    // leaf field's own children are ordinary markup-capable
                    // inline content, not further sub-fields. Without this
                    // distinction, `<italic>` inside e.g. an `<article-title>`
                    // would itself be mis-dispatched as a raw-preserved "misc"
                    // field instead of a proper `emphasis` node, silently
                    // flattening the markup it exists to preserve.
                    let child_in_biblio = if in_biblio {
                        is_biblio_field_wrapper(name)
                    } else {
                        is_biblio_container(name)
                    };
                    // `<sec>` is the only JATS block-level element that nests
                    // arbitrarily deep inside itself (unlike DocBook's distinct
                    // sect1..sect5 tag names, JATS uses the same `<sec>` tag at
                    // every level) — a real nesting-depth counter, not just the
                    // immediate parent tag name, is needed to give a `<title>`
                    // inside a doubly-nested `<sec>` a deeper heading level than
                    // one inside its outer `<sec>`. See the `"title"` arm in
                    // `convert_element`.
                    let child_sec_depth = sec_depth + usize::from(name == "sec");
                    let converted_kids = convert_children(
                        kids,
                        name,
                        child_in_header,
                        child_in_biblio,
                        child_sec_depth,
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
                    let mut converted = convert_element(
                        name,
                        attrs,
                        converted_kids.clone(),
                        Some(parent_name),
                        sec_depth,
                    );
                    // Any `<article-meta>`/`<journal-meta>` descendant this
                    // reader has no explicit semantic mapping for (i.e.
                    // `convert_element` produced it via its generic catch-all
                    // rather than a dedicated arm — see
                    // `is_modeled_header_field`) is about to be discarded as a
                    // tree node and flattened into metadata by
                    // `extract_metadata`. Rather than lose its internal
                    // structure (`<contrib-group>`'s author entries,
                    // `<pub-date>`'s day/month/year parts, or any other
                    // unmodeled front-matter element), capture the whole
                    // subtree's original XML verbatim (mirroring how
                    // `rescribe-read-docbook`/`rescribe-read-tei` raw-preserve
                    // unmodeled header children via `{fmt}_fmt::emit_fragment`)
                    // so the writer can splice it back byte-for-byte instead of
                    // reconstructing a lossy approximation from flattened text.
                    if in_header
                        && !is_modeled_header_field(name)
                        && let Some(node) = converted.take()
                    {
                        let raw =
                            String::from_utf8(crate::emit_fragment(std::slice::from_ref(child)))
                                .ok();
                        converted = Some(match raw {
                            Some(raw) => node.prop("jats:raw", raw),
                            None => node,
                        });
                    }
                    match converted {
                        Some(node) => out.push(node),
                        None => {
                            if name == "article-meta" || name == "journal-meta" {
                                extract_metadata(&converted_kids, metadata, warnings);
                            } else {
                                // Pass-through wrapper element (e.g.
                                // title-group, def-item, fn-group's own nested
                                // wrappers): splice its already converted
                                // children directly into the parent.
                                out.extend(converted_kids);
                            }
                        }
                    }
                }
                JNode::Text { content, .. } => {
                    if !content.trim().is_empty() {
                        out.push(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()));
                    }
                }
                JNode::Cdata { content, .. } => {
                    out.push(Node::new(node::TEXT).prop(prop::CONTENT, content.clone()));
                }
                JNode::EntityRef { name, .. } => {
                    // Named entity the DTD defines but we cannot resolve
                    // without it — raw-preserve verbatim rather than drop.
                    out.push(
                        Node::new(node::RAW_INLINE)
                            .prop(prop::CONTENT, format!("&{name};"))
                            .prop("jats:entity", name.clone()),
                    );
                }
                JNode::Comment { .. }
                | JNode::ProcessingInstruction { .. }
                | JNode::Doctype { .. } => {
                    // No cross-format meaning and no natural IR raw-block slot
                    // inside inline/block flow content; JATS's own semantic
                    // model has no equivalent for a bare PI/comment here.
                    warnings.push(FidelityWarning::new(
                        Severity::Minor,
                        WarningKind::FeatureLost("comment-or-pi".to_string()),
                        format!("dropped non-content JATS node inside <{parent_name}>"),
                    ));
                }
                JNode::Raw { content, .. } => {
                    // `JNode::Raw` is never produced by `crate::parse` itself
                    // (see its doc comment) — it only exists for downstream
                    // consumers to construct directly. This arm exists purely
                    // so the match stays exhaustive; raw-preserve the content
                    // verbatim rather than drop it if a `JatsDoc` containing one
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

    /// Attach the small set of JATS attributes worth round-tripping generically
    /// (`id`, `xml:lang`) regardless of which element carries them. Applied to
    /// *every* element [`convert_element`] produces (see the `.map()` wrapping
    /// its `match` at the end of that function) — not just the generic-fallback
    /// span/div nodes — so e.g. `id` on a `<sec>` or `xml:lang` on a `<p>`
    /// round-trips the same way it would on an unrecognized element. Mirrors
    /// `rescribe-read-docbook`'s `attach_generic_attrs` for the same two
    /// standard-XML-ish attributes (JATS defines no format-specific analogue of
    /// DocBook's `role`).
    fn attach_generic_attrs(mut node: Node, attrs: &[(String, String)]) -> Node {
        if let Some(id) = get_attr(attrs, "id") {
            node = node.prop(prop::ID, id.to_string());
        }
        if let Some(lang) = get_attr(attrs, "xml:lang") {
            node = node.prop(prop::LANGUAGE, lang.to_string());
        }
        node
    }

    /// Preserve every attribute *other than* `id`/`xml:lang` (already handled by
    /// [`attach_generic_attrs`]) as a `jats:attr:{name}` property. Only used by
    /// [`generic_span`]/[`generic_div`] — the elements this reader has no
    /// dedicated semantic mapping for — since those are exactly the elements
    /// whose attributes (`content-type` on `<named-content>`, `mimetype` on
    /// `<supplementary-material>`, `target-type` on `<target>`, ...) this reader
    /// cannot know the meaning of ahead of time and so must raw-preserve
    /// wholesale rather than silently drop.
    fn attach_all_attrs(mut node: Node, attrs: &[(String, String)]) -> Node {
        for (key, value) in attrs {
            if key == "id" || key == "xml:lang" {
                continue;
            }
            node = node.prop(format!("jats:attr:{key}"), value.clone());
        }
        node
    }

    /// A generic inline "wrapper" element: JATS markup that has no dedicated IR
    /// node kind but must still round-trip losslessly. Represented as a `span`
    /// tagged with the original element name (`jats:tag`) per the
    /// raw-preservation pattern — this is exactly what `span` exists for.
    fn generic_span(name: &str, attrs: &[(String, String)], children: Vec<Node>) -> Node {
        let n = Node::new(node::SPAN)
            .prop("jats:tag", name.to_string())
            .children(children);
        attach_all_attrs(n, attrs)
    }

    /// A generic block-level "wrapper" element: the block-level counterpart to
    /// [`generic_span`]. JATS markup with no dedicated IR node kind, but whose
    /// content model is block-shaped (per [`is_block_element`]) rather than
    /// running inline text — represented as a `div` tagged with the original
    /// element name (`jats:tag`) so the writer can re-emit the exact tag rather
    /// than `<p>`-wrapping a bare span, which would misrepresent an
    /// unrecognized block element as an inline one.
    fn generic_div(name: &str, attrs: &[(String, String)], children: Vec<Node>) -> Node {
        let n = Node::new(node::DIV)
            .prop("jats:tag", name.to_string())
            .children(children);
        attach_all_attrs(n, attrs)
    }

    /// Known JATS block-level elements — used only by the catch-all fallback in
    /// [`convert_element`] to decide whether an element name this reader doesn't
    /// specifically recognize should become a [`generic_div`] (block position)
    /// or a [`generic_span`] (inline position); every element `convert_element`
    /// already gives dedicated handling to never reaches the catch-all, so this
    /// list exists purely to classify the *unrecognized* remainder. It
    /// deliberately includes both this reader's own recognized block vocabulary
    /// (as a cross-reference) and additional JATS elements that are
    /// unambiguously block-shaped but have no dedicated IR mapping yet.
    ///
    /// Schema-verified against the JATS 1.3 (NISO Z39.96-2019) Tag Library
    /// (<https://jats.nlm.nih.gov/archiving/tag-library/1.3/>), per-element pages
    /// under `element/{name}.html`, using each element's expanded content model
    /// and "May be contained in" list as ground truth (same pass previously done
    /// for docbook-fmt in `abd6dd447d` and tei-fmt in `3e3d84bcef`):
    /// - `related-article` was **removed**: its own Usage/Remarks documents two
    ///   uses — front-matter metadata *and* "textual content throughout the
    ///   article (as part of the Journal Article Link Class Elements)" — i.e. it
    ///   is a phrase-level link element like `xref`/`ext-link`, not block-shaped.
    /// - `speech`, `speaker`, `supplementary-material`, `block-alternatives` were
    ///   **added**: `speech` (`(speaker, p+)`) and `supplementary-material`
    ///   (floating object, same class as `fig`) have unambiguous block content
    ///   models; `speaker` is a leading label line with mixed (PCDATA + phrase)
    ///   content, classified block for the same reason as `verse-line`/`sig`
    ///   (occupies its own line in the rendered flow, not running text);
    ///   `block-alternatives` is JATS's explicitly block-only counterpart to
    ///   `alternatives`.
    /// - `alternatives` (plain, non-`block-` form) was deliberately **not**
    ///   added: its own Tag Library page states it "is neither inherently block
    ///   nor inherently inline in nature, because the block or inline quality is
    ///   determined by context and usage" and that it is "typical[ly]... loose
    ///   inside a paragraph" — JATS itself declines to classify it, so it is
    ///   left to the inline default rather than guessed at.
    /// - Every other entry (including the metadata-container group —
    ///   `contrib-group`, `aff`, `pub-date`, `permissions`, `history`,
    ///   `custom-meta-group`, `custom-meta`, `product`) was checked against its
    ///   "May be contained in" list and confirmed to never appear inside `<p>`
    ///   or other running-text contexts, i.e. genuinely block-positioned despite
    ///   some having PCDATA-mixed content models.
    pub(crate) fn is_block_element(tag: &str) -> bool {
        matches!(
            tag,
            // Document structure
            "article"
                | "front"
                | "body"
                | "back"
                | "article-meta"
                | "journal-meta"
                | "sec"
                // Block content
                | "p"
                | "list"
                | "list-item"
                | "def-list"
                | "def-item"
                | "def"
                | "statement"
                | "disp-quote"
                | "boxed-text"
                | "verse-group"
                | "verse-line"
                | "table-wrap"
                | "table-wrap-group"
                // Same navigation-index-only fetch limitation as `glossary`
                // above — classified block by analogy to its sibling
                // `table-wrap` child elements (`caption`, `table-wrap-group`),
                // which the "May be contained in <table-wrap>" family
                // unambiguously is.
                | "table-wrap-foot"
                | "table"
                | "thead"
                | "tbody"
                | "tfoot"
                | "tr"
                | "fig"
                | "fig-group"
                | "caption"
                | "disp-formula"
                | "disp-formula-group"
                | "fn-group"
                | "fn"
                | "ref-list"
                | "ref"
                | "abstract"
                | "kwd-group"
                | "ack"
                | "app-group"
                | "app"
                | "notes"
                // `glossary` was not fetchable from the Tag Library in this
                // session (only its navigation-index entry loaded, not its
                // content-model page) — classified block by analogy to its
                // fellow back-matter sectioning siblings `ack`/`app-group`/
                // `notes` (same "whole back-matter section" shape), not a
                // direct schema citation like the rest of this list.
                | "glossary"
                | "sig-block"
                | "sig"
                | "speech"
                | "speaker"
                | "supplementary-material"
                | "block-alternatives"
                | "colgroup"
                | "col"
                // Front-matter containers
                | "contrib-group"
                | "aff"
                | "pub-date"
                | "permissions"
                | "history"
                | "custom-meta-group"
                | "custom-meta"
                | "product"
        )
    }

    /// Convert one JATS element (with its already-converted children) into a
    /// rescribe node. Returns `None` for elements that either have no IR
    /// representation of their own (pass-through wrappers) or are consumed for
    /// side effects (metadata extraction) — see [`convert_children`] for how
    /// those two cases are told apart.
    fn convert_element(
        name: &str,
        attrs: &[(String, String)],
        children: Vec<Node>,
        parent: Option<&str>,
        sec_depth: usize,
    ) -> Option<Node> {
        let href = get_attr(attrs, "href").or_else(|| get_attr(attrs, "xlink:href"));
        let rid = get_attr(attrs, "rid");
        let ref_type = get_attr(attrs, "ref-type");
        let content_type = get_attr(attrs, "content-type");
        let list_type = get_attr(attrs, "list-type");

        match name {
            // Document structure
            "article" => Some(Node::new(node::DIV).children(children)),
            "front" | "body" | "back" => None, // Pass through
            // Handled by the caller (`convert_children`) via `extract_metadata`.
            "article-meta" | "journal-meta" => None,
            // Structural wrapper (used both under `<article-meta>` for the
            // article title and under `<journal-meta>` for the journal title):
            // no IR node of its own, but its `<article-title>`/`<title>` child
            // is separately mapped to `HEADING` below and must reach
            // `extract_metadata` as a direct sibling rather than being
            // raw-captured wholesale (which would bury the already-modeled
            // title inside a `jats:raw` blob `extract_metadata` never
            // recurses into — see `is_modeled_header_field`).
            "title-group" => None, // Pass through

            // Sections
            "sec" => Some(Node::new(node::DIV).children(children)),

            // Titles. A `<title>` whose parent is `<sec>` gets a level derived
            // from `sec_depth` (how many `<sec>` ancestors enclose it) rather
            // than a hardcoded `2` — JATS nests `<sec>` inside itself
            // arbitrarily deep using the same tag name at every level (unlike
            // DocBook's distinct `sect1`..`sect5`), so telling a doubly-nested
            // section's title apart from its parent's needs an actual depth
            // count, not just the immediate parent tag.
            "title" | "article-title" => {
                let level = match parent {
                    Some("article") | Some("front") | Some("article-meta") => 1,
                    Some("sec") => 1 + sec_depth.max(1),
                    Some("fig") | Some("table-wrap") => 3,
                    _ => 2,
                };
                Some(
                    Node::new(node::HEADING)
                        .prop(prop::LEVEL, level as i64)
                        .children(children),
                )
            }
            // Tagged `jats:subtitle` so `extract_metadata` can tell a `<subtitle>`
            // apart from a `<title>`/`<article-title>` sibling in the same
            // `<title-group>` — both convert to `HEADING`, but they must land in
            // distinct metadata keys (`subtitle` vs `title`) rather than one
            // silently overwriting/merging into the other.
            "subtitle" => Some(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, 2i64)
                    .prop("jats:subtitle", true)
                    .children(children),
            ),

            // Paragraphs
            "p" => Some(Node::new(node::PARAGRAPH).children(children)),

            // Abstract. Also tagged `jats:tag = "abstract"` (on top of the usual
            // `html:class` styling hint) and given every other attribute via
            // `attach_all_attrs` (`abstract-type="structured"`, etc.) — without
            // `jats:tag`, `extract_metadata`'s `<article-meta>` front-matter
            // handling below (which keys off `jats:tag` to recognize an
            // unmodeled header field worth capturing) would silently skip this
            // node entirely: it's a dedicated, non-generic mapping, so it was
            // never being pushed into `Document::metadata` at all, dropping the
            // entire abstract (found via `parse -> emit -> parse` verification
            // while building this fixture, not by inspection alone).
            "abstract" => {
                let n = Node::new(node::DIV)
                    .prop("html:class", "abstract")
                    .prop("jats:tag", "abstract")
                    .children(children);
                Some(attach_all_attrs(n, attrs))
            }

            // Lists. `list-type` has more values than the binary
            // ordered/unordered `prop::ORDERED` can represent (`alpha-lower`,
            // `alpha-upper`, `roman-lower`, `roman-upper`, `simple`, `bullet`,
            // `order`, ...) — the exact original value is additionally
            // raw-preserved as `jats:list-type` so the writer can round-trip the
            // specific numeration style instead of collapsing every ordered
            // variant down to a generic `order`.
            "list" => {
                let ordered = list_type == Some("order")
                    || matches!(
                        list_type,
                        Some("alpha-lower")
                            | Some("alpha-upper")
                            | Some("roman-lower")
                            | Some("roman-upper")
                    );
                let mut node = Node::new(node::LIST).prop(prop::ORDERED, ordered);
                if let Some(lt) = list_type {
                    node = node.prop("jats:list-type", lt.to_string());
                }
                // `continued-from` (an idref to the `<list>` this one continues
                // the numbering of) is raw-preserved rather than resolved to an
                // actual numeric start value — resolving it needs a second pass
                // over the whole document to find and count the referenced
                // list's items, which this single-pass per-element conversion
                // doesn't do. Capturing the idref itself is still lossless.
                if let Some(cf) = get_attr(attrs, "continued-from") {
                    node = node.prop("jats:continued-from", cf.to_string());
                }
                Some(node.children(children))
            }
            "list-item" => Some(Node::new(node::LIST_ITEM).children(children)),

            // Definition lists
            "def-list" => Some(Node::new(node::DEFINITION_LIST).children(children)),
            "def-item" => None, // Pass through
            "term" => Some(Node::new(node::DEFINITION_TERM).children(children)),
            "def" => Some(Node::new(node::DEFINITION_DESC).children(children)),

            // Code. `<code>` (unlike `<preformat>`) is documented as usable
            // either as a standalone block sibling of `<sec>`/`<body>` content
            // or mixed directly into running text — this reader uses the
            // pragmatic signal of "direct child of `<p>`" to tell the two
            // usages apart (the same typical-usage judgment call `is_block_element`'s
            // doc comment already documents making elsewhere; JATS's own tag
            // library page for `<code>` did not yield a fetchable "May be
            // contained in" list to verify this against directly). Both `<code>`
            // and `<preformat>` map to `CODE_BLOCK` in block position but record
            // which original tag was used via `jats:tag` so the writer can
            // re-emit the exact element rather than collapsing both to `<code>`.
            "code" if parent == Some("p") => {
                let text = extract_text(&children);
                Some(
                    Node::new(node::CODE)
                        .prop(prop::CONTENT, text)
                        .prop("jats:tag", "code"),
                )
            }
            "code" | "preformat" => {
                let text = extract_text(&children);
                let mut node = Node::new(node::CODE_BLOCK)
                    .prop(prop::CONTENT, text)
                    .prop("jats:tag", name.to_string());
                if let Some(lang) = content_type {
                    node = node.prop(prop::LANGUAGE, lang.to_string());
                }
                Some(node)
            }
            // `<monospace>` is JATS's dedicated inline-styling element (distinct
            // from `<code>` used inline) — both map to the same `CODE` node kind
            // since rescribe has no separate "styled as monospace" vs "is source
            // code" inline distinction, but `jats:tag` records which one it was
            // so the writer defaults back to `<monospace>` (this arm) unless the
            // `<code>`-inline arm above set `jats:tag` to `"code"`.
            "monospace" => {
                let text = extract_text(&children);
                Some(
                    Node::new(node::CODE)
                        .prop(prop::CONTENT, text)
                        .prop("jats:tag", "monospace"),
                )
            }

            // Block quote. `<boxed-text>` (a callout/sidebar per its own Tag
            // Library page — "highlighting tips, warnings, explanatory notes...
            // visual prominence and topical relevance", explicitly contrasted
            // there with `<disp-quote>`'s "attribution and source") is *not* a
            // quotation and was previously (incorrectly) folded into the same
            // `BLOCKQUOTE` mapping as `<disp-quote>`; it now falls through to the
            // generic block catch-all below (`is_block_element` already lists
            // it), which raw-preserves it as a tagged `div` instead of
            // misrepresenting it as a quote.
            "disp-quote" => Some(Node::new(node::BLOCKQUOTE).children(children)),

            // Inline formatting
            "italic" => Some(Node::new(node::EMPHASIS).children(children)),
            "bold" => Some(Node::new(node::STRONG).children(children)),
            "underline" => {
                let mut node = Node::new(node::UNDERLINE).children(children);
                if let Some(style) = get_attr(attrs, "underline-style") {
                    node = node.prop("jats:underline-style", style.to_string());
                }
                Some(node)
            }
            "strike" => Some(Node::new(node::STRIKEOUT).children(children)),
            "sub" => Some(Node::new(node::SUBSCRIPT).children(children)),
            "sup" => Some(Node::new(node::SUPERSCRIPT).children(children)),
            "sc" => Some(Node::new(node::SMALL_CAPS).children(children)),

            // Links
            "ext-link" => {
                let mut node = Node::new(node::LINK).children(children);
                if let Some(url) = href {
                    node = node.prop(prop::URL, url.to_string());
                }
                // `ext-link-type` defaults to `"uri"` on write when absent (the
                // writer's previous hardcoded behavior, kept as the fallback)
                // but the original value is now round-tripped exactly when
                // present, rather than every `<ext-link>` collapsing to
                // `ext-link-type="uri"` regardless of its source value.
                if let Some(t) = get_attr(attrs, "ext-link-type") {
                    node = node.prop("jats:ext-link-type", t.to_string());
                }
                Some(node)
            }
            // `<xref ref-type="fn">` is a footnote reference (a cross-reference
            // to an `<fn>` by `rid`) — modeled with the standard cross-format
            // `footnote_ref` node kind (the same convention every other
            // rescribe reader with footnote references uses, e.g.
            // `rescribe-read-docbook`'s `footnoteref` -> `FOOTNOTE_REF`) rather
            // than the generic `LINK` every other `<xref ref-type="...">`
            // variant (`fig`, `table`, `bibr`, `sec`, ...) falls to below, which
            // already round-trips those adequately via `jats:ref-type` +
            // `url`/`#{rid}`.
            "xref" if ref_type == Some("fn") => {
                let mut node = Node::new(node::FOOTNOTE_REF).children(children);
                if let Some(r) = rid {
                    node = node.prop(prop::LABEL, r.to_string());
                }
                Some(node)
            }
            "xref" => {
                let mut node = Node::new(node::LINK).children(children);
                if let Some(r) = rid {
                    node = node.prop(prop::URL, format!("#{r}"));
                }
                // Preserved for both the empty (`<xref .../>`) and full
                // (`<xref ...>text</xref>`) element shapes alike — the old
                // hand-rolled reader only attached this for the self-closing
                // case, an inconsistency this rewrite closes (additive fidelity
                // fix, not a construct change).
                if let Some(rt) = ref_type {
                    node = node.prop("jats:ref-type", rt.to_string());
                }
                Some(node)
            }
            "uri" => {
                let url = extract_text(&children);
                Some(
                    Node::new(node::LINK)
                        .prop(prop::URL, url.clone())
                        .child(Node::new(node::TEXT).prop(prop::CONTENT, url)),
                )
            }

            // Figures
            "fig" | "fig-group" => Some(Node::new(node::FIGURE).children(children)),
            "caption" => Some(
                Node::new("figcaption")
                    .prop("html:tag", "figcaption")
                    .children(children),
            ),
            "graphic" | "inline-graphic" => {
                href.map(|url| Node::new(node::IMAGE).prop(prop::URL, url.to_string()))
            }

            // Tables
            // Tagged `jats:tag = "table-wrap"` (distinct from `<fig>`, which gets
            // no tag) so the writer can tell the two `FIGURE`-mapped source
            // elements apart and re-emit the right one — see
            // `rescribe-write-jats`'s `FIGURE` arm, which also needs this
            // distinction to avoid double-wrapping a `TABLE` child in a second,
            // redundant `<table-wrap>` (its own `TABLE` write arm already emits
            // one for a standalone table).
            "table-wrap" => Some(
                Node::new(node::FIGURE)
                    .prop("jats:tag", "table-wrap")
                    .children(children),
            ),
            "table" => Some(Node::new(node::TABLE).children(children)),
            "thead" => Some(Node::new(node::TABLE_HEAD).children(children)),
            "tbody" => Some(Node::new(node::TABLE_BODY).children(children)),
            "tr" => Some(Node::new(node::TABLE_ROW).children(children)),
            "th" => Some(with_cell_span(
                Node::new(node::TABLE_HEADER).children(children),
                attrs,
            )),
            "td" => Some(with_cell_span(
                Node::new(node::TABLE_CELL).children(children),
                attrs,
            )),

            // Math. A `<label>` (e.g. `(1)`) is split out via `split_label`
            // *before* computing `math:source` — without this, since `<label>`
            // has no dedicated mapping and becomes a generic_span spliced
            // directly into `children`, its text was silently folding into
            // `math:source` itself (`"(1)E = mc^2"` instead of `"E = mc^2"`
            // with a separate label), corrupting the math content for any
            // consumer that treats `math:source` as literal TeX. Found via
            // parse -> emit -> parse verification while building the
            // disp-formula-with-label fixture.
            "disp-formula" => {
                let (label, rest) = split_label(children);
                let (mathml, alternatives_raw, rest) = split_mathml(rest);
                let mut node = match mathml {
                    // No explicit `math:format` for the `<tex-math>` case,
                    // matching the existing convention (rescribe-read-html's
                    // `\(...\)` LaTeX case also leaves `math:format` unset —
                    // only the MathML case sets it, to `"mathml"`).
                    Some(source) => Node::new("math_display")
                        .prop("math:source", source)
                        .prop("math:format", "mathml"),
                    None => Node::new("math_display").prop("math:source", extract_text(&rest)),
                };
                if let Some(label) = label {
                    node = node.prop(prop::LABEL, label);
                }
                if let Some(raw) = alternatives_raw {
                    node = node.prop("jats:alternatives-raw", raw);
                }
                Some(node)
            }
            "inline-formula" => {
                let (label, rest) = split_label(children);
                let (mathml, alternatives_raw, rest) = split_mathml(rest);
                let mut node = match mathml {
                    Some(source) => Node::new("math_inline")
                        .prop("math:source", source)
                        .prop("math:format", "mathml"),
                    None => Node::new("math_inline").prop("math:source", extract_text(&rest)),
                };
                if let Some(label) = label {
                    node = node.prop(prop::LABEL, label);
                }
                if let Some(raw) = alternatives_raw {
                    node = node.prop("jats:alternatives-raw", raw);
                }
                Some(node)
            }
            "tex-math" => {
                // Already captured by the parent formula element via
                // `extract_text`.
                None
            }
            "mml:math" => {
                // Normally intercepted and raw-captured by `convert_children`
                // before it ever reaches here (see the `mml-math-raw` sentinel
                // above `convert_element`'s call site). This arm only fires for
                // a stray `<mml:math>` outside a `<disp-formula>`/
                // `<inline-formula>` parent (malformed input, or a `JatsDoc`
                // built directly) — fall back to flattening it to text rather
                // than dropping it silently.
                Some(generic_span("mml:math", attrs, children))
            }

            // Footnotes
            "fn" => Some(Node::new(node::FOOTNOTE_DEF).children(children)),
            "fn-group" => Some(Node::new(node::DIV).children(children)),

            // References. `<ref-list>` is a bibliography container; its own
            // `<title>` (e.g. "References") is handled by the `"title"` arm
            // above like any other block's title, so by the time we get here
            // `children` is already a mix of the resulting `HEADING` (if any)
            // and `bibliography_entry` nodes (each `<ref>` child was converted
            // through `build_bibliography_entry` before reaching here, via
            // `convert_children`'s `in_biblio` threading — see
            // `is_biblio_container`).
            "ref-list" => Some(Node::new(node::BIBLIOGRAPHY).children(children)),
            // A `<ref>` reached here always has `convert_children` having
            // already run over its own children with `in_biblio = true` (per
            // `is_biblio_container`), so `children` is already the fully-built
            // field/date-marker list `build_bibliography_entry` assembles.
            "ref" => Some(build_bibliography_entry(attrs, children)),
            // Defensive fallback: an `<element-citation>`/`<mixed-citation>`
            // encountered *outside* any enclosing `<ref>` (malformed input, or a
            // `JatsDoc` constructed directly rather than parsed) never entered
            // biblio scope (see `is_biblio_container`), so its children were
            // converted as ordinary content rather than citation fields — keep
            // the pre-existing generic `span` mapping for that case rather than
            // silently dropping it.
            "mixed-citation" | "element-citation" => Some(Node::new(node::SPAN).children(children)),

            // `contrib-group`/`contrib`/`name`/`surname`/`given-names`/`aff`/
            // `pub-date`/`volume`/`issue`/`fpage`/`lpage`/`kwd-group`/`kwd`
            // (and any other `<article-meta>`/`<journal-meta>` child with no
            // dedicated semantic mapping) deliberately fall through to the
            // generic catch-all at the bottom of this match — which produces
            // the `generic_span`/`generic_div` node `convert_children`'s
            // `in_header` handling then raw-preserves (see
            // `is_modeled_header_field`) — rather than being special-cased here
            // and dropped.

            // Line break
            "break" => Some(Node::new(node::LINE_BREAK)),

            // Any other element name: this reader has no dedicated semantic
            // mapping for it. Rather than silently dropping the tag and
            // splicing its children straight into the parent (which is what
            // returning `None` here does, via `convert_children`'s pass-through
            // branch), raw-preserve it generically as a tagged div/span keyed
            // by `jats:tag` — block-shaped or inline-shaped depending on
            // `is_block_element` — so `rescribe-write-jats` can re-emit the
            // original tag rather than losing it.
            _ => {
                if is_block_element(name) {
                    Some(generic_div(name, attrs, children))
                } else {
                    Some(generic_span(name, attrs, children))
                }
            }
        }
        // Applied to every branch above (not just the generic fallback) so `id`
        // and `xml:lang` round-trip uniformly regardless of which element
        // carries them — see `attach_generic_attrs`.
        .map(|n| attach_generic_attrs(n, attrs))
    }

    /// Whether `name` is a JATS bibliographic reference container — its children
    /// are citation sub-fields (label, `<element-citation>`/`<mixed-citation>`),
    /// not ordinary document content, so `convert_children` dispatches them
    /// through [`convert_biblio_field`] instead of [`convert_element`]. Only
    /// `<ref>` itself; `<ref-list>` is *not* included here — a `<ref-list>`'s
    /// direct children are `<ref>` elements and its own `<title>`, both of which
    /// are ordinary content from `<ref-list>`'s point of view (the `<ref>`
    /// element is what enters biblio scope, per [`is_biblio_container`]'s use in
    /// `convert_children`).
    fn is_biblio_container(name: &str) -> bool {
        name == "ref"
    }

    /// Whether `name`, encountered *while already inside* biblio scope, is a
    /// structural pass-through wrapper whose own children remain citation
    /// sub-fields (as opposed to a leaf field like `<article-title>`/
    /// `<source>`, whose children are ordinary inline content — see
    /// `convert_children`'s `child_in_biblio` computation for why this
    /// distinction matters). `<element-citation>`/`<mixed-citation>` wrap the
    /// citation's own fields; `<date>` wraps `<year>`/`<month>`/`<day>`.
    /// `<person-group>` is deliberately *not* included — it is intercepted
    /// directly in `convert_children`'s loop (see [`convert_person_group`])
    /// rather than threaded through this generic mechanism, since the role its
    /// children get depends on `<person-group>`'s own attribute.
    fn is_biblio_field_wrapper(name: &str) -> bool {
        matches!(name, "element-citation" | "mixed-citation" | "date")
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
            // Pass-through wrapper: its own children were already converted
            // into `bibliography_field`/marker nodes by the recursive dispatch
            // above — splice them straight into the entry as siblings rather
            // than nesting them one level deeper. Which of `<element-citation>`/
            // `<mixed-citation>` it was (plus its `publication-type`/
            // `publication-format` attributes, if present) would otherwise be
            // lost entirely once its children are spliced flat, so a leading
            // `jats:_citation_tag` marker records it for
            // `build_bibliography_entry` to consume and remove.
            "element-citation" | "mixed-citation" => {
                let mut marker = Node::new("jats:_citation_tag").prop("jats:tag", name.to_string());
                if let Some(pt) = get_attr(attrs, "publication-type") {
                    marker = marker.prop("jats:attr:publication-type", pt.to_string());
                }
                if let Some(pf) = get_attr(attrs, "publication-format") {
                    marker = marker.prop("jats:attr:publication-format", pf.to_string());
                }
                let mut out = vec![marker];
                out.extend(children);
                out
            }

            "article-title" => vec![bib_field("title", "article-title", children, None)],
            "source" => vec![bib_field("container_title", "source", children, None)],
            "publisher-name" => vec![bib_field("publisher", "publisher-name", children, None)],
            "publisher-loc" => vec![bib_field(
                "publisher_location",
                "publisher-loc",
                children,
                None,
            )],
            "edition" => vec![bib_field("edition", "edition", children, None)],
            "volume" => vec![bib_field("volume", "volume", children, None)],
            "issue" => vec![bib_field("issue", "issue", children, None)],
            "fpage" => vec![bib_field("page_first", "fpage", children, None)],
            "lpage" => vec![bib_field("page_last", "lpage", children, None)],

            // `pub-id-type` (doi/isbn/pmid/pmcid/... plus an open vocabulary)
            // names the identifier scheme.
            "pub-id" => vec![bib_field(
                "identifier",
                "pub-id",
                children,
                get_attr(attrs, "pub-id-type"),
            )],

            // A bare person name/collaborative-author name directly inside
            // `<element-citation>`/`<mixed-citation>` — JATS 1.3's own content
            // model permits `<name>`/`<collab>`/`<string-name>` here without a
            // wrapping `<person-group>` (legacy tagging style; the Tag Library's
            // own recommended practice is the `<person-group>`-wrapped form
            // handled by `convert_person_group`). No `jats:person-group-type` is
            // set, so `rescribe-write-jats` re-emits it bare (unwrapped) rather
            // than reconstructing a `<person-group>` that was never there.
            // Defaults to the `author` role since that is this bare form's
            // overwhelmingly common usage (a corporate/collaborative author);
            // a disclosed simplification, not a silent drop — the original tag
            // survives via `jats:tag`.
            "name" | "collab" | "string-name" => vec![bib_field("author", name, children, None)],

            // `<year>`/`<month>`/`<day>`, whether bare (direct children of
            // `<element-citation>`) or nested inside a `<date>` wrapper (see
            // `is_biblio_field_wrapper`), become an internal marker node for
            // `build_bibliography_entry`/`resolve_citation_date` to consume and
            // remove; each may itself carry its own `iso-8601-date` attribute
            // (JATS Tag Library example: `<year iso-8601-date="2001-11">2001
            // </year>`), preserved on the marker so the unambiguous attribute
            // form can be preferred over reconstructing from possibly free-text
            // sub-parts (e.g. `<month>Nov</month>`).
            "year" | "month" | "day" => vec![date_part_marker(name, attrs, children)],

            // `<date>` itself may carry its own `iso-8601-date` attribute
            // (preferred when present) in addition to, or instead of, wrapping
            // `<year>`/`<month>`/`<day>` sub-elements (already converted into
            // `jats:_date_part` markers above, present in `children`).
            "date" => vec![date_wrapper_marker(attrs, children)],

            // `<date-in-citation>` names its own semantics via `content-type`
            // (e.g. `"copyright-year"`) distinct from the citation's primary
            // publication date — rather than guess which of possibly *two*
            // dates on one citation (a primary `<date>`/bare year *and* a
            // `<date-in-citation>`) is "the" `prop::DATE`, this is always kept
            // as its own `misc` field (with `iso-8601-date`/`content-type`
            // preserved as raw attrs) instead of merged into `prop::DATE`.
            "date-in-citation" => {
                let mut node = Node::new(node::BIBLIOGRAPHY_FIELD)
                    .prop(prop::FIELD_ROLE, "misc")
                    .prop("jats:tag", "date-in-citation")
                    .children(children);
                if let Some(iso) = get_attr(attrs, "iso-8601-date") {
                    node = node.prop("jats:attr:iso-8601-date", iso.to_string());
                }
                if let Some(ct) = get_attr(attrs, "content-type") {
                    node = node.prop("jats:attr:content-type", ct.to_string());
                }
                vec![node]
            }

            "label" => vec![bib_field("misc", "label", children, None)],

            // Every other reference-citation element this reader has no
            // dedicated mapping for (`comment`, `season`, `series`, `etal`,
            // `annotation`, `trans-title`, `part-title`, and any other member of
            // JATS's `%citation-elements;` parameter entity): raw-preserve as a
            // `misc` field tagged with the original element name (its own
            // children stay ordinary markup-capable inline nodes), rather than
            // silently dropping it.
            _ => vec![bib_field("misc", name, children, None)],
        }
    }

    /// Build one `bibliography_field` node: `role` is the standard
    /// `prop::FIELD_ROLE` value; `tag` is the original JATS element name
    /// (round-tripped via `jats:tag` so `rescribe-write-jats` can restore the
    /// exact source element); `scheme`, if given, becomes `prop::FIELD_SCHEME`
    /// (used only by `<pub-id>`'s `pub-id-type` attribute).
    fn bib_field(role: &str, tag: &str, children: Vec<Node>, scheme: Option<&str>) -> Node {
        let mut node = Node::new(node::BIBLIOGRAPHY_FIELD)
            .prop(prop::FIELD_ROLE, role.to_string())
            .prop("jats:tag", tag.to_string())
            .children(children);
        if let Some(scheme) = scheme {
            node = node.prop(prop::FIELD_SCHEME, scheme.to_string());
        }
        node
    }

    /// Build an internal `jats:_date_part` marker node for a bare `<year>`/
    /// `<month>`/`<day>` element (see `convert_biblio_field`'s "year"/"month"/
    /// "day" arm) — consumed and removed by `build_bibliography_entry` via
    /// `resolve_citation_date`, never a real IR node kind that could leak into
    /// the final tree.
    fn date_part_marker(part: &str, attrs: &[(String, String)], children: Vec<Node>) -> Node {
        let mut node = Node::new("jats:_date_part")
            .prop("jats:part", part.to_string())
            .children(children);
        if let Some(iso) = get_attr(attrs, "iso-8601-date") {
            node = node.prop("jats:iso", iso.to_string());
        }
        node
    }

    /// Build an internal `jats:_date` marker node for a `<date>` wrapper (see
    /// `convert_biblio_field`'s "date" arm) — consumed and removed by
    /// `build_bibliography_entry` via `resolve_citation_date`. `children` may
    /// contain `jats:_date_part` markers (from a wrapped `<year>`/`<month>`/
    /// `<day>`), any other unmodeled sub-element (raw-preserved by
    /// `convert_biblio_field`'s catch-all), or nothing at all if `<date>` itself
    /// carries `iso-8601-date` with no children.
    fn date_wrapper_marker(attrs: &[(String, String)], children: Vec<Node>) -> Node {
        let mut node = Node::new("jats:_date").children(children);
        if let Some(iso) = get_attr(attrs, "iso-8601-date") {
            node = node.prop("jats:iso", iso.to_string());
        }
        node
    }

    /// Build a `bibliography_entry` node for a `<ref>` element. `children` are
    /// the already-converted `bibliography_field`/date-marker/citation-tag-marker
    /// siblings (see `convert_biblio_field`); this function pulls the internal
    /// `jats:_citation_tag`/`jats:_date`/`jats:_date_part` markers back out —
    /// the citation-tag marker becomes `jats:tag` (+ raw-preserved
    /// `publication-type`/`publication-format`) on the entry itself, and the
    /// date marker(s) become the structured `prop::DATE` property via
    /// `resolve_citation_date`, or — if no unambiguous date could be resolved —
    /// are demoted back to ordinary `misc` field(s) instead of being lost.
    fn build_bibliography_entry(attrs: &[(String, String)], children: Vec<Node>) -> Node {
        let mut citation_tag: Option<Node> = None;
        let mut date_wrapper: Option<Node> = None;
        let mut year_marker: Option<Node> = None;
        let mut month_marker: Option<Node> = None;
        let mut day_marker: Option<Node> = None;
        let mut kids = Vec::with_capacity(children.len());
        for child in children {
            match (child.kind.as_str(), child.props.get_str("jats:part")) {
                ("jats:_citation_tag", _) => citation_tag = Some(child),
                ("jats:_date", _) => date_wrapper = Some(child),
                ("jats:_date_part", Some("year")) => year_marker = Some(child),
                ("jats:_date_part", Some("month")) => month_marker = Some(child),
                ("jats:_date_part", Some("day")) => day_marker = Some(child),
                _ => kids.push(child),
            }
        }
        let date_map = match &date_wrapper {
            Some(wrapper) => resolve_citation_date(
                wrapper.props.get_str("jats:iso"),
                date_wrapper_parts(wrapper, "year"),
                date_wrapper_parts(wrapper, "month"),
                date_wrapper_parts(wrapper, "day"),
            ),
            None => resolve_citation_date(
                None,
                year_marker.as_ref(),
                month_marker.as_ref(),
                day_marker.as_ref(),
            ),
        };
        let mut entry = Node::new(node::BIBLIOGRAPHY_ENTRY);
        if let Some(marker) = &citation_tag {
            if let Some(tag) = marker.props.get_str("jats:tag") {
                entry = entry.prop("jats:tag", tag.to_string());
            }
            if let Some(pt) = marker.props.get_str("jats:attr:publication-type") {
                entry = entry.prop("jats:attr:publication-type", pt.to_string());
            }
            if let Some(pf) = marker.props.get_str("jats:attr:publication-format") {
                entry = entry.prop("jats:attr:publication-format", pf.to_string());
            }
        }
        match date_map {
            Some(map) => entry = entry.prop(prop::DATE, PropValue::Map(map)),
            None => {
                // Couldn't resolve an unambiguous date — demote whatever
                // marker(s) exist back into an ordinary `misc` field (or
                // fields, for the bare year/month/day case, keeping them
                // separate since they weren't wrapped together in the source)
                // instead of losing them.
                if let Some(wrapper) = date_wrapper {
                    kids.push(
                        Node::new(node::BIBLIOGRAPHY_FIELD)
                            .prop(prop::FIELD_ROLE, "misc")
                            .prop("jats:tag", "date")
                            .children(wrapper.children.into_iter().flat_map(|part| part.children)),
                    );
                }
                for marker in [year_marker, month_marker, day_marker]
                    .into_iter()
                    .flatten()
                {
                    let tag = marker
                        .props
                        .get_str("jats:part")
                        .unwrap_or("date")
                        .to_string();
                    kids.push(
                        Node::new(node::BIBLIOGRAPHY_FIELD)
                            .prop(prop::FIELD_ROLE, "misc")
                            .prop("jats:tag", tag)
                            .children(marker.children),
                    );
                }
            }
        }
        attach_generic_attrs(entry.children(kids), attrs)
    }

    /// Find a `jats:_date_part` marker of the given `part` (`"year"`/`"month"`/
    /// `"day"`) among a `jats:_date` wrapper's own children.
    fn date_wrapper_parts<'a>(wrapper: &'a Node, part: &str) -> Option<&'a Node> {
        wrapper.children.iter().find(|c| {
            c.kind.as_str() == "jats:_date_part" && c.props.get_str("jats:part") == Some(part)
        })
    }

    /// Resolve a citation date, preferring an unambiguous `iso-8601-date`
    /// attribute over reconstructing from `<year>`/`<month>`/`<day>` sub-parts —
    /// per the JATS Tag Library's own tagged examples, the attribute may appear
    /// either on the `<date>` wrapper itself (`explicit_iso`) or directly on
    /// `<year>` (`year`'s own `jats:iso`), and either may encode a full
    /// `YYYY[-MM[-DD]]` string on its own (e.g. `<year iso-8601-date="2001-11">
    /// 2001</year>` alongside a separate, redundant `<month>Nov</month>`).
    /// Falls back to parsing `year`/`month`/`day`'s own text content (numeric,
    /// or an English month name/abbreviation for `month`) only when no
    /// attribute is present; returns `None` — rather than guess — when even
    /// that isn't unambiguous (e.g. a `<season>`-only citation with no year).
    fn resolve_citation_date(
        explicit_iso: Option<&str>,
        year: Option<&Node>,
        month: Option<&Node>,
        day: Option<&Node>,
    ) -> Option<HashMap<String, PropValue>> {
        if let Some(iso) = explicit_iso
            && let Some(map) = parse_iso_date_string(iso)
        {
            return Some(map);
        }
        if let Some(y) = year
            && let Some(iso) = y.props.get_str("jats:iso")
            && let Some(map) = parse_iso_date_string(iso)
        {
            return Some(map);
        }
        let y = year.and_then(|n| parse_year_text(&extract_text(&n.children)));
        let m = month.and_then(|n| parse_month_text(&extract_text(&n.children)));
        let d = day.and_then(|n| parse_day_text(&extract_text(&n.children)));
        match (y, m, d) {
            (Some(y), Some(m), Some(d)) => Some(date_map(y, Some(m), Some(d))),
            (Some(y), Some(m), None) => Some(date_map(y, Some(m), None)),
            (Some(y), None, None) => Some(date_map(y, None, None)),
            // Month/day present without a year, or nothing parseable at all —
            // ambiguous or insufficient; don't guess.
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

    /// Parse an `iso-8601-date` attribute value's unambiguous forms (`YYYY`,
    /// `YYYY-MM`, `YYYY-MM-DD`) into `prop::DATE`'s map. Returns `None` for
    /// anything else (JATS does not constrain this attribute's value to a
    /// single format) rather than guess.
    fn parse_iso_date_string(text: &str) -> Option<HashMap<String, PropValue>> {
        let parts: Vec<&str> = text.trim().split('-').collect();
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

    fn parse_day_text(text: &str) -> Option<i64> {
        let t = text.trim();
        let n: i64 = t.parse().ok()?;
        (1..=31).contains(&n).then_some(n)
    }

    /// Parse a `<month>` element's free-text content: either a plain number
    /// (`"11"`) or one of the standard English month names/abbreviations
    /// (`"Nov"`, `"November"`, case-insensitive) — a fixed, unambiguous lookup
    /// table, not a locale guess. Anything else (a non-English month name, a
    /// season, free text) returns `None` rather than being guessed at.
    fn parse_month_text(text: &str) -> Option<i64> {
        let t = text.trim();
        if let Ok(n) = t.parse::<i64>() {
            return (1..=12).contains(&n).then_some(n);
        }
        const MONTHS: [(&str, &str, i64); 12] = [
            ("jan", "january", 1),
            ("feb", "february", 2),
            ("mar", "march", 3),
            ("apr", "april", 4),
            ("may", "may", 5),
            ("jun", "june", 6),
            ("jul", "july", 7),
            ("aug", "august", 8),
            ("sep", "september", 9),
            ("oct", "october", 10),
            ("nov", "november", 11),
            ("dec", "december", 12),
        ];
        let lower = t.to_ascii_lowercase();
        MONTHS
            .iter()
            .find(|(abbr, full, _)| lower == *abbr || lower == *full)
            .map(|(_, _, n)| *n)
    }

    /// Convert a `<person-group>` element's own raw (unconverted) children into
    /// `bibliography_field` nodes: one field per `<name>`/`<collab>`/
    /// `<string-name>`/`<etal>`/`<aff>`/`<role>` (etc.) child, all sharing the
    /// role derived from `<person-group>`'s own `person-group-type` attribute
    /// (`"editor"` maps to the `editor` role; every other value — `"author"`,
    /// `"translator"`, `"allauthors"`, `"guest-editor"`, absent, ...— maps to
    /// `author`, since `prop::FIELD_ROLE`'s vocabulary has no finer-grained
    /// bucket; the *exact* original attribute value is preserved on every
    /// resulting field via `jats:person-group-type` regardless, so this is a
    /// disclosed simplification of the role classification, not a loss of the
    /// underlying data). Each child's own subtree is converted through the
    /// ordinary (non-biblio) pipeline — *not* `convert_biblio_field` — so its
    /// internal markup (`<surname>`/`<given-names>` etc.) is preserved exactly
    /// as any other unrecognized element would be, via `convert_element`'s
    /// generic `generic_span`/`generic_div` catch-all.
    fn convert_person_group(
        attrs: &[(String, String)],
        kids: &[JNode],
        sec_depth: usize,
        metadata: &mut Properties,
        warnings: &mut Vec<FidelityWarning>,
    ) -> Vec<Node> {
        let raw_type = get_attr(attrs, "person-group-type").unwrap_or("author");
        let role = if raw_type == "editor" {
            "editor"
        } else {
            "author"
        };
        let mut out = Vec::new();
        for kid in kids {
            if let JNode::Element {
                name,
                attrs: cattrs,
                children: ckids,
                ..
            } = kid
            {
                // Use `name`'s own already-converted children directly as the
                // field's content — *not* `convert_element(name, ...)`'s result
                // (which, for `<name>`/`<collab>`/`<string-name>` etc., has no
                // dedicated arm and would fall to the generic catch-all,
                // producing a second `span` tagged `jats:tag = name` nested
                // *inside* this field — which already carries that exact same
                // tag on itself). `<surname>`/`<given-names>` (etc.) inside
                // `<name>` are still preserved exactly, via that same generic
                // catch-all, one level down — only the redundant outer
                // wrapper is skipped.
                let field = Node::new(node::BIBLIOGRAPHY_FIELD)
                    .prop(prop::FIELD_ROLE, role)
                    .prop("jats:tag", name.clone())
                    .prop("jats:person-group-type", raw_type.to_string())
                    .children(convert_children(
                        ckids, name, false, false, sec_depth, metadata, warnings,
                    ));
                out.push(attach_generic_attrs(field, cattrs));
            }
            // Whitespace-only text between `<name>` elements carries no
            // meaning here (same treatment as `convert_children`'s own
            // whitespace-text handling); non-`Element` node kinds otherwise
            // don't occur directly inside `<person-group>` per its content
            // model.
        }
        out
    }

    /// Read `colspan`/`rowspan` off a `<th>`/`<td>` element (JATS's table model
    /// uses the same HTML-style attribute names/semantics directly, unlike
    /// DocBook's CALS `morerows`/`namest`/`nameend`) onto the standard
    /// cross-format `colspan`/`rowspan` properties.
    fn with_cell_span(mut node: Node, attrs: &[(String, String)]) -> Node {
        if let Some(n) = get_attr(attrs, "colspan").and_then(|s| s.parse::<i64>().ok()) {
            node = node.prop(prop::COLSPAN, n);
        }
        if let Some(n) = get_attr(attrs, "rowspan").and_then(|s| s.parse::<i64>().ok()) {
            node = node.prop(prop::ROWSPAN, n);
        }
        node
    }

    /// `<article-meta>`/`<journal-meta>` fields `convert_element` gives an
    /// explicit, dedicated semantic mapping to — these are fully modeled in
    /// `Document::metadata` (via `extract_metadata`'s `HEADING` case) and so
    /// must *not* be raw-captured by `convert_children`'s front-matter handling:
    /// their content already round-trips through the semantic property it was
    /// extracted into, and wrapping them in `jats:raw` on top would just
    /// duplicate that content.
    ///
    /// Every other `<article-meta>`/`<journal-meta>` child element name falls to
    /// `convert_element`'s generic catch-all (`generic_span`/`generic_div`) and
    /// gets raw-preserved instead — see `convert_children`.
    fn is_modeled_header_field(name: &str) -> bool {
        matches!(name, "title" | "article-title" | "subtitle")
    }

    /// Extract `<article-meta>`/`<journal-meta>` metadata: title (searched for
    /// as a `HEADING`, matching how `<title>`/`<article-title>` convert anywhere
    /// else) plus every other front-matter field (contrib-group, pub-date,
    /// volume, issue, fpage, lpage, permissions, history, or any other
    /// unrecognized `<article-meta>`/`<journal-meta>` child), each surfaced by
    /// `convert_element` as a `span`/`div` tagged with `jats:tag` so this
    /// function can find them regardless of nesting.
    ///
    /// Every field beyond `title`/`article-title` was raw-captured by
    /// `convert_children` — see `is_modeled_header_field` — and shows up here as
    /// a `span`/`div` carrying a `jats:raw` prop. That subtree's original XML is
    /// stored verbatim as `{tag}_raw` metadata (plus a `{tag}` flattened-text
    /// convenience copy) so `rescribe-write-jats` can splice it back
    /// byte-for-byte; nothing was lost, so descendants aren't recursed into
    /// separately. Only if raw capture itself failed (non-UTF8 content — the
    /// XML source was already UTF8, so this is not expected in practice) does
    /// this fall back to a flatten-to-text-plus-fidelity-warning path.
    ///
    /// Multiple occurrences of a repeatable field (e.g. more than one
    /// `<contrib-group>`) are joined with `"; "` rather than the later one
    /// silently overwriting the earlier — losing all-but-the-last would itself
    /// be a silent drop.
    fn extract_metadata(
        nodes: &[Node],
        metadata: &mut Properties,
        warnings: &mut Vec<FidelityWarning>,
    ) {
        for node in nodes {
            if node.kind.as_str() == node::HEADING {
                let text = extract_text(&node.children);
                if !text.is_empty() {
                    // A `<subtitle>` converts to `HEADING` the same as
                    // `<title>`/`<article-title>` (see `convert_element`) but is
                    // tagged `jats:subtitle` so it lands in its own metadata key
                    // instead of overwriting/merging into `title`.
                    let key = if node.props.get_bool("jats:subtitle") == Some(true) {
                        "subtitle"
                    } else {
                        "title"
                    };
                    metadata.set(key, text);
                }
            } else if matches!(node.kind.as_str(), node::SPAN | node::DIV)
                && let Some(tag) = node.props.get_str("jats:tag")
            {
                let text = extract_text(&node.children);
                match tag {
                    // A `<title>`/`<article-title>` nested somewhere other than
                    // directly under `<title-group>` — `convert_element` still
                    // maps it to `HEADING`, handled above, so there is nothing
                    // to do for a `span`/`div`-shaped "title" here; this arm
                    // only exists so the generic fallback below doesn't clobber
                    // the real title metadata.
                    "title" | "article-title" => {}
                    other if !text.is_empty() || node.props.get_str("jats:raw").is_some() => {
                        if !text.is_empty() {
                            append_metadata(metadata, other, &text);
                        }
                        match node.props.get_str("jats:raw") {
                            // A repeatable field (e.g. more than one
                            // `<contrib-group>`) concatenates its raw XML rather
                            // than the later occurrence silently overwriting the
                            // earlier — valid, since concatenated sibling XML
                            // elements are themselves valid XML content.
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
                                WarningKind::FeatureLost(format!("jats-header-field-{other}")),
                                format!(
                                    "<article-meta>/<journal-meta> <{other}> internal structure is \
                                         not modeled and its raw XML could not be captured; only its \
                                         flattened text was kept in metadata: {text:?}"
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
    /// `<contrib-group>`) silently overwriting an earlier one.
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

    /// Split a leading `<label>` (a `generic_span` tagged `jats:tag = "label"`,
    /// e.g. `<disp-formula>`'s `(1)`) out of a converted-children list, returning
    /// its flattened text separately from the remaining children. Used by the
    /// `disp-formula`/`inline-formula` math arms so a label doesn't fold into
    /// `math:source` — see their doc comments.
    fn split_label(children: Vec<Node>) -> (Option<String>, Vec<Node>) {
        let mut label = None;
        let mut rest = Vec::with_capacity(children.len());
        for child in children {
            if label.is_none()
                && child.kind.as_str() == node::SPAN
                && child.props.get_str("jats:tag") == Some("label")
            {
                label = Some(extract_text(&child.children));
            } else {
                rest.push(child);
            }
        }
        (label, rest)
    }

    /// Pull the raw-captured `<mml:math>` sentinel (see `convert_children`'s
    /// `mml-math-raw` interception) out of a formula's already-`split_label`ed
    /// children, returning its verbatim MathML source, any raw-preserved sibling
    /// alternative(s) captured alongside it (see `convert_children`'s
    /// `<alternatives>`-inside-formula interception — set only when the
    /// `<mml:math>` came from an `<alternatives>` wrapper that also held e.g. a
    /// `<tex-math>` or a third alternative), and the remaining children (which,
    /// in the MathML case, is normally empty — MathML and TeX are alternatives,
    /// not both present as direct siblings per the JATS 1.3 content model).
    fn split_mathml(children: Vec<Node>) -> (Option<String>, Option<String>, Vec<Node>) {
        let mut mathml = None;
        let mut alternatives_raw = None;
        let mut rest = Vec::with_capacity(children.len());
        for child in children {
            if mathml.is_none()
                && child.kind.as_str() == node::SPAN
                && child.props.get_str("jats:tag") == Some("mml-math-raw")
            {
                mathml = child.props.get_str(prop::CONTENT).map(|s| s.to_string());
                alternatives_raw = child
                    .props
                    .get_str("jats:alternatives-raw")
                    .map(|s| s.to_string());
            } else {
                rest.push(child);
            }
        }
        (mathml, alternatives_raw, rest)
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
            let jats = r#"<?xml version="1.0"?>
    <article>
      <front>
        <article-meta>
          <title-group>
            <article-title>Test Article</article-title>
          </title-group>
        </article-meta>
      </front>
      <body>
        <p>Hello, world!</p>
      </body>
    </article>"#;

            let result = parse(jats).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
            assert_eq!(doc.metadata.get_str("title"), Some("Test Article"));
        }

        #[test]
        fn test_parse_sections() {
            let jats = r#"<?xml version="1.0"?>
    <article>
      <body>
        <sec>
          <title>Introduction</title>
          <p>Content here.</p>
        </sec>
      </body>
    </article>"#;

            let result = parse(jats).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }

        #[test]
        fn test_parse_lists() {
            let jats = r#"<?xml version="1.0"?>
    <article>
      <body>
        <list list-type="bullet">
          <list-item><p>Item 1</p></list-item>
          <list-item><p>Item 2</p></list-item>
        </list>
      </body>
    </article>"#;

            let result = parse(jats).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }

        #[test]
        fn test_parse_formatting() {
            let jats = r#"<?xml version="1.0"?>
    <article>
      <body>
        <p><italic>italic</italic> and <bold>bold</bold> text</p>
      </body>
    </article>"#;

            let result = parse(jats).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }

        #[test]
        fn test_parse_table() {
            let jats = r#"<?xml version="1.0"?>
    <article>
      <body>
        <table-wrap>
          <table>
            <thead>
              <tr><th>Header</th></tr>
            </thead>
            <tbody>
              <tr><td>Cell</td></tr>
            </tbody>
          </table>
        </table-wrap>
      </body>
    </article>"#;

            let result = parse(jats).unwrap();
            let doc = result.value;
            assert!(!doc.content.children.is_empty());
        }

        #[test]
        fn test_unresolvable_entity_preserved() {
            let jats = r#"<article><body><p>a &custom; b</p></body></article>"#;
            let result = parse(jats).unwrap();
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
}

#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
mod write {
    use std::collections::HashMap;

    use crate::{JatsDoc, Node as JNode, XmlDecl};
    use rescribe_core::{ConversionResult, Document, EmitError, Node, PropValue};
    use rescribe_format_api::Emit as _;
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
                article_meta_children.push(jats_element(
                    "title-group",
                    vec![],
                    title_group_children,
                ));
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
                    span: crate::Span::NONE,
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
            span: crate::Span::NONE,
        };

        let doc_ast = JatsDoc {
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

    fn jats_element(name: &str, attrs: Vec<(String, String)>, children: Vec<JNode>) -> JNode {
        JNode::Element {
            name: name.to_string(),
            attrs,
            children,
            span: crate::Span::NONE,
        }
    }

    fn jats_text(content: impl Into<String>) -> JNode {
        JNode::Text {
            content: content.into(),
            span: crate::Span::NONE,
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
    ///
    /// When the node also carries `jats:alternatives-raw` (set when
    /// `rescribe-read-jats` found the `<mml:math>` inside a wrapping
    /// `<alternatives>` alongside one or more other alternative representations
    /// — e.g. a sibling `<tex-math>`, or a third `<graphic>` alternative — see
    /// its `convert_children`'s `<alternatives>`-inside-formula interception),
    /// re-wrap the primary representation in `<alternatives>` and splice the raw
    /// sibling(s) back in verbatim, restoring the original wrapper rather than
    /// silently losing it.
    fn formula_children(node: &Node) -> Vec<JNode> {
        let mut children = Vec::new();
        if let Some(label) = node.props.get_str(prop::LABEL) {
            children.push(jats_element("label", vec![], vec![jats_text(label)]));
        }
        if let Some(source) = node.props.get_str("math:source") {
            let primary = if node.props.get_str("math:format") == Some("mathml") {
                JNode::Raw {
                    content: source.to_string(),
                    span: crate::Span::NONE,
                }
            } else {
                jats_element("tex-math", vec![], vec![jats_text(source)])
            };
            match node.props.get_str("jats:alternatives-raw") {
                Some(raw) => children.push(jats_element(
                    "alternatives",
                    vec![],
                    vec![
                        primary,
                        JNode::Raw {
                            content: raw.to_string(),
                            span: crate::Span::NONE,
                        },
                    ],
                )),
                None => children.push(primary),
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
                while let Some(next) = iter.next_if(|next| {
                    next.kind.as_str() == node::BIBLIOGRAPHY_FIELD
                        && next.props.get_str("jats:person-group-type") == Some(pg_type)
                }) {
                    group_kids.push(write_bibliography_field(next));
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
                    span: crate::Span::NONE,
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
            let parsed = crate::rescribe::parse(jats).unwrap();
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
            let parsed = crate::rescribe::parse(jats).unwrap();
            let emitted = emit(&parsed.value).unwrap();
            let xml = String::from_utf8(emitted.value).unwrap();
            assert!(
                xml.contains(r#"<mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>x</mml:mi></mml:math>"#),
                "emitted XML missing raw mml:math: {xml}"
            );

            let reparsed = crate::rescribe::parse(&xml).unwrap();
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
            let parsed = crate::rescribe::parse(jats).unwrap();
            let emitted = emit(&parsed.value).unwrap();
            let xml = String::from_utf8(emitted.value).unwrap();
            assert!(
                xml.contains(r#"<mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>x</mml:mi></mml:math>"#),
                "emitted XML missing raw mml:math: {xml}"
            );

            let reparsed = crate::rescribe::parse(&xml).unwrap();
            let para = &reparsed.value.content.children[0].children[0];
            let formula = &para.children[1];
            assert_eq!(formula.kind.as_str(), "math_inline");
            assert_eq!(formula.props.get_str("math:format"), Some("mathml"));
        }

        /// A `<disp-formula>` whose `<mml:math>`/`<tex-math>` are wrapped in an
        /// `<alternatives>` (the JATS-recommended pattern for offering both a
        /// MathML and a TeX rendering of the same formula — see
        /// `rescribe-read-jats`'s `convert_children`'s `<alternatives>`-inside-
        /// formula interception) must keep *both* representations through
        /// parse -> emit -> reparse, not just the MathML one: the MathML becomes
        /// the modeled `math:source`/`math:format`, and the `<tex-math>` sibling
        /// round-trips verbatim via `jats:alternatives-raw` /
        /// `formula_children`'s `<alternatives>` re-wrap.
        #[test]
        fn test_roundtrip_mathml_tex_alternatives() {
            let jats = r#"<?xml version="1.0" encoding="UTF-8"?>
    <article xmlns:xlink="http://www.w3.org/1999/xlink"><body><disp-formula><alternatives><tex-math>E=mc^2</tex-math><mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>E</mml:mi></mml:math></alternatives></disp-formula></body></article>"#;
            let parsed = crate::rescribe::parse(jats).unwrap();
            let formula = &parsed.value.content.children[0].children[0];
            assert_eq!(formula.kind.as_str(), "math_display");
            assert_eq!(formula.props.get_str("math:format"), Some("mathml"));
            assert!(
                formula
                    .props
                    .get_str("math:source")
                    .unwrap()
                    .contains("mml:mi"),
                "math:source should hold the MathML, not a concatenation with the TeX text"
            );

            let emitted = emit(&parsed.value).unwrap();
            let xml = String::from_utf8(emitted.value).unwrap();
            assert!(
                xml.contains("<alternatives>"),
                "emitted XML should re-wrap in <alternatives>: {xml}"
            );
            assert!(
                xml.contains("<tex-math>E=mc^2</tex-math>"),
                "emitted XML missing raw-preserved tex-math sibling: {xml}"
            );
            assert!(
                xml.contains(r#"<mml:math xmlns:mml="http://www.w3.org/1998/Math/MathML"><mml:mi>E</mml:mi></mml:math>"#),
                "emitted XML missing raw mml:math: {xml}"
            );

            let reparsed = crate::rescribe::parse(&xml).unwrap();
            let formula2 = &reparsed.value.content.children[0].children[0];
            assert_eq!(formula2.kind.as_str(), "math_display");
            assert_eq!(formula2.props.get_str("math:format"), Some("mathml"));
            assert_eq!(
                formula2.props.get_str("math:source"),
                formula.props.get_str("math:source")
            );
        }
    }
}

#[cfg(all(feature = "reader-ast", feature = "rescribe"))]
pub use read::parse;
#[cfg(all(feature = "writer-builder", feature = "rescribe"))]
pub use write::emit;
