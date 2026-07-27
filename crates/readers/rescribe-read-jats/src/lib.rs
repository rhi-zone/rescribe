//! JATS XML reader for rescribe.
//!
//! Translates `jats_fmt::JatsDoc` (the standalone JATS/XML AST from the
//! `jats-fmt` crate) into rescribe's document IR. Supports JATS 1.0/1.1/1.2/
//! 1.3 elements commonly used in scholarly publishing.
//!
//! All XML tokenizing/parsing lives in `jats-fmt` — this crate is a thin
//! AST↔IR translator only (per CLAUDE.md's "adapter layer must never
//! contain parsing or writing logic" rule).
//!
//! # Example
//!
//! ```
//! use rescribe_read_jats::parse;
//!
//! let jats = r#"<?xml version="1.0"?>
//! <article>
//!   <front>
//!     <article-meta>
//!       <title-group>
//!         <article-title>Test Article</article-title>
//!       </title-group>
//!     </article-meta>
//!   </front>
//!   <body>
//!     <p>Hello, world!</p>
//!   </body>
//! </article>"#;
//!
//! let result = parse(jats).unwrap();
//! let doc = result.value;
//! ```

use jats_fmt::Node as JNode;
use rescribe_core::{
    ConversionResult, Document, FidelityWarning, Node, ParseError, Properties, Severity,
    WarningKind,
};
use rescribe_std::{node, prop};

/// Parse JATS XML into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    let (doc, diagnostics) = jats_fmt::parse(input.as_bytes());

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
            let converted = convert_children(kids, name, false, &mut metadata, &mut warnings);
            match convert_element(name, attrs, converted.clone(), None) {
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
fn convert_children(
    children: &[JNode],
    parent_name: &str,
    in_header: bool,
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
                let child_in_header =
                    in_header || matches!(name.as_str(), "article-meta" | "journal-meta");
                let converted_kids =
                    convert_children(kids, name, child_in_header, metadata, warnings);
                let mut converted =
                    convert_element(name, attrs, converted_kids.clone(), Some(parent_name));
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
                        String::from_utf8(jats_fmt::emit_fragment(std::slice::from_ref(child)))
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
            JNode::Comment { .. } | JNode::ProcessingInstruction { .. } | JNode::Doctype { .. } => {
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
                // `JNode::Raw` is never produced by `jats_fmt::parse` itself
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
/// (id) regardless of which element carries them.
fn attach_generic_attrs(mut node: Node, attrs: &[(String, String)]) -> Node {
    if let Some(id) = get_attr(attrs, "id") {
        node = node.prop(prop::ID, id.to_string());
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
    attach_generic_attrs(n, attrs)
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
    attach_generic_attrs(n, attrs)
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
            | "sig-block"
            | "sig"
            | "speech"
            | "speaker"
            | "supplementary-material"
            | "block-alternatives"
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

        // Titles
        "title" | "article-title" => {
            let level = match parent {
                Some("article") | Some("front") | Some("article-meta") => 1,
                Some("sec") => 2,
                Some("fig") | Some("table-wrap") => 3,
                _ => 2,
            };
            Some(
                Node::new(node::HEADING)
                    .prop(prop::LEVEL, level as i64)
                    .children(children),
            )
        }
        "subtitle" => Some(
            Node::new(node::HEADING)
                .prop(prop::LEVEL, 2i64)
                .children(children),
        ),

        // Paragraphs
        "p" => Some(Node::new(node::PARAGRAPH).children(children)),

        // Abstract
        "abstract" => Some(
            Node::new(node::DIV)
                .prop("html:class", "abstract")
                .children(children),
        ),

        // Lists
        "list" => {
            let ordered = list_type == Some("order");
            Some(
                Node::new(node::LIST)
                    .prop(prop::ORDERED, ordered)
                    .children(children),
            )
        }
        "list-item" => Some(Node::new(node::LIST_ITEM).children(children)),

        // Definition lists
        "def-list" => Some(Node::new(node::DEFINITION_LIST).children(children)),
        "def-item" => None, // Pass through
        "term" => Some(Node::new(node::DEFINITION_TERM).children(children)),
        "def" => Some(Node::new(node::DEFINITION_DESC).children(children)),

        // Code
        "code" | "preformat" => {
            let text = extract_text(&children);
            let mut node = Node::new(node::CODE_BLOCK).prop(prop::CONTENT, text);
            if let Some(lang) = content_type {
                node = node.prop(prop::LANGUAGE, lang.to_string());
            }
            Some(node)
        }
        "monospace" => {
            let text = extract_text(&children);
            Some(Node::new(node::CODE).prop(prop::CONTENT, text))
        }

        // Block quote
        "disp-quote" | "boxed-text" => Some(Node::new(node::BLOCKQUOTE).children(children)),

        // Inline formatting
        "italic" => Some(Node::new(node::EMPHASIS).children(children)),
        "bold" => Some(Node::new(node::STRONG).children(children)),
        "underline" => Some(Node::new(node::UNDERLINE).children(children)),
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
        "table-wrap" => Some(Node::new(node::FIGURE).children(children)),
        "table" => Some(Node::new(node::TABLE).children(children)),
        "thead" => Some(Node::new(node::TABLE_HEAD).children(children)),
        "tbody" => Some(Node::new(node::TABLE_BODY).children(children)),
        "tr" => Some(Node::new(node::TABLE_ROW).children(children)),
        "th" => Some(Node::new(node::TABLE_HEADER).children(children)),
        "td" => Some(Node::new(node::TABLE_CELL).children(children)),

        // Math
        "disp-formula" => {
            let text = extract_text(&children);
            Some(Node::new("math_display").prop("math:source", text))
        }
        "inline-formula" => {
            let text = extract_text(&children);
            Some(Node::new("math_inline").prop("math:source", text))
        }
        "tex-math" | "mml:math" => {
            // Already captured by the parent formula element.
            None
        }

        // Footnotes
        "fn" => Some(Node::new(node::FOOTNOTE_DEF).children(children)),
        "fn-group" => Some(Node::new(node::DIV).children(children)),

        // References
        "ref-list" => Some(
            Node::new(node::DIV)
                .prop("html:class", "references")
                .children(children),
        ),
        "ref" => Some(
            Node::new(node::PARAGRAPH)
                .prop("jats:ref", true)
                .children(children),
        ),
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
    matches!(name, "title" | "article-title")
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
            let title = extract_text(&node.children);
            if !title.is_empty() {
                metadata.set("title", title);
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
