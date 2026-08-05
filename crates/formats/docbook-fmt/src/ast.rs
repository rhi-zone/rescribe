//! AST types for DocBook (and generic XML) documents.

// Shared with the rest of the `-fmt` crates via `rescribe-format-api` — see
// that crate's docs for why. The local `Span` shape (`start`/`end`) already
// matched exactly; it just gained `Copy`/`Eq`/`Hash`. The local `Diagnostic`
// (`message`+`span` only) gains `severity`/`code`, populated with
// `Severity::Warning`/`""` at every construction site in `parse.rs`,
// `batch.rs`, and `events.rs`.
pub use rescribe_format_api::{Diagnostic, Severity, Span};

/// The XML declaration (`<?xml version="1.0" encoding="UTF-8"?>`), if present.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct XmlDecl {
    pub version: String,
    pub encoding: Option<String>,
    pub standalone: Option<String>,
}

/// A DocBook (or generic XML) document.
///
/// This AST is deliberately format-agnostic within the "DocBook family" —
/// it represents any well-formed XML document as a tree of elements, text,
/// comments, CDATA sections, and processing instructions. DocBook-specific
/// meaning (which element names mean what) lives in the rescribe adapter
/// layer, not here — the same way `html-fmt`'s `Node::Element` doesn't know
/// which tags are "semantic."
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DocBookDoc {
    /// The `<?xml ... ?>` declaration, if the source had one.
    pub xml_decl: Option<XmlDecl>,
    /// Top-level nodes: usually a single root `Element`, plus any leading or
    /// trailing comments / processing instructions / doctype.
    pub nodes: Vec<Node>,
}

impl DocBookDoc {
    pub fn strip_spans(&self) -> DocBookDoc {
        DocBookDoc {
            xml_decl: self.xml_decl.clone(),
            nodes: self.nodes.iter().map(|n| n.strip_spans()).collect(),
        }
    }

    /// The root element, if the document has one.
    pub fn root(&self) -> Option<&Node> {
        self.nodes
            .iter()
            .find(|n| matches!(n, Node::Element { .. }))
    }
}

/// A node in the XML document tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// An XML element with tag name, attributes, and children.
    Element {
        name: String,
        attrs: Vec<(String, String)>,
        children: Vec<Node>,
        /// True if the source wrote this as a self-closing tag (`<foo/>`),
        /// false if it was an explicit open/close pair (`<foo></foo>` when
        /// `children` is empty, or `<foo>...</foo>` otherwise). Only
        /// distinguishable when `children` is empty — `<foo/>` and
        /// `<foo></foo>` both parse to zero children, but are not the same
        /// source syntax, and losslessness requires preserving which one
        /// the input actually used (see fixtures/docbook/line-break's
        /// `<sbr></sbr>`). Irrelevant (but still tracked, always `false`)
        /// when `children` is non-empty, since only an empty element can be
        /// self-closing in the first place.
        self_closing: bool,
        span: Span,
    },
    /// Text content (entities already decoded).
    Text { content: String, span: Span },
    /// `<![CDATA[ ... ]]>` section.
    Cdata { content: String, span: Span },
    /// `<!-- comment -->`.
    Comment { content: String, span: Span },
    /// `<?target data?>` processing instruction.
    ProcessingInstruction {
        target: String,
        data: String,
        span: Span,
    },
    /// `<!DOCTYPE ...>` declaration, captured verbatim (DocBook 4 documents
    /// commonly declare a public/system DTD here).
    Doctype { content: String, span: Span },
    /// A named entity reference (`&custom;`) that cannot be resolved without
    /// a DTD. The five predefined XML entities (`amp`, `lt`, `gt`, `quot`,
    /// `apos`) and numeric character references (`&#65;`, `&#x41;`) are
    /// resolved automatically by the parser and folded into adjacent `Text`
    /// nodes instead — this variant only appears for entities the parser
    /// cannot resolve, preserved verbatim so the writer can re-emit them.
    EntityRef { name: String, span: Span },
    /// Raw XML content emitted verbatim, with no escaping and no further
    /// interpretation. Never produced by [`crate::parse::parse`] itself —
    /// this variant exists for downstream consumers (e.g. an adapter that
    /// captured a subtree via [`crate::emit::emit_fragment`] and wants to
    /// splice it back in unchanged) to construct directly.
    Raw { content: String, span: Span },
}

/// Resolve one of the five XML-predefined entity names to its character.
/// Returns `None` for any other name (custom/DTD-defined entities, which
/// the parser cannot resolve and instead preserves as [`Node::EntityRef`]).
pub(crate) fn resolve_predefined_entity(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => None,
    }
}

impl Node {
    pub fn strip_spans(&self) -> Node {
        match self {
            Node::Element {
                name,
                attrs,
                children,
                self_closing,
                ..
            } => Node::Element {
                name: name.clone(),
                attrs: attrs.clone(),
                children: children.iter().map(|n| n.strip_spans()).collect(),
                self_closing: *self_closing,
                span: Span::NONE,
            },
            Node::Text { content, .. } => Node::Text {
                content: content.clone(),
                span: Span::NONE,
            },
            Node::Cdata { content, .. } => Node::Cdata {
                content: content.clone(),
                span: Span::NONE,
            },
            Node::Comment { content, .. } => Node::Comment {
                content: content.clone(),
                span: Span::NONE,
            },
            Node::ProcessingInstruction { target, data, .. } => Node::ProcessingInstruction {
                target: target.clone(),
                data: data.clone(),
                span: Span::NONE,
            },
            Node::Doctype { content, .. } => Node::Doctype {
                content: content.clone(),
                span: Span::NONE,
            },
            Node::EntityRef { name, .. } => Node::EntityRef {
                name: name.clone(),
                span: Span::NONE,
            },
            Node::Raw { content, .. } => Node::Raw {
                content: content.clone(),
                span: Span::NONE,
            },
        }
    }

    /// The element name, if this is an `Element`.
    pub fn name(&self) -> Option<&str> {
        match self {
            Node::Element { name, .. } => Some(name),
            _ => None,
        }
    }

    /// The children, if this is an `Element`.
    pub fn children(&self) -> Option<&[Node]> {
        match self {
            Node::Element { children, .. } => Some(children),
            _ => None,
        }
    }

    /// The attributes, if this is an `Element`.
    pub fn attrs(&self) -> Option<&[(String, String)]> {
        match self {
            Node::Element { attrs, .. } => Some(attrs),
            _ => None,
        }
    }

    /// A specific attribute value by name (local name, namespace prefix stripped).
    pub fn get_attr(&self, name: &str) -> Option<&str> {
        self.attrs().and_then(|attrs| {
            attrs
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.as_str())
        })
    }

    /// Concatenation of all descendant text content, depth-first.
    pub fn text_content(&self) -> String {
        let mut out = String::new();
        collect_text(self, &mut out);
        out
    }
}

fn collect_text(node: &Node, out: &mut String) {
    match node {
        Node::Text { content, .. } | Node::Cdata { content, .. } => out.push_str(content),
        Node::Element { children, .. } => {
            for child in children {
                collect_text(child, out);
            }
        }
        _ => {}
    }
}

/// Split a processing instruction's raw content (`target data`) into parts.
pub(crate) fn split_pi(raw: &str) -> (String, String) {
    match raw.find(|c: char| c.is_whitespace()) {
        Some(idx) => (raw[..idx].to_string(), raw[idx..].trim_start().to_string()),
        None => (raw.to_string(), String::new()),
    }
}
