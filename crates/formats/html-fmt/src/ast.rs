//! AST types for HTML documents.

/// Byte offset span in the source input.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const NONE: Span = Span { start: 0, end: 0 };
}

/// An HTML document.
#[derive(Clone, Debug, PartialEq)]
pub struct HtmlDoc {
    /// Top-level nodes (typically: doctype, then the `<html>` element).
    pub nodes: Vec<Node>,
}

impl HtmlDoc {
    pub fn strip_spans(&self) -> HtmlDoc {
        HtmlDoc {
            nodes: self.nodes.iter().map(|n| n.strip_spans()).collect(),
        }
    }
}

/// A node in the HTML document tree.
#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    /// `<!DOCTYPE html>` or similar.
    Doctype {
        name: String,
        public_id: String,
        system_id: String,
        span: Span,
    },
    /// An HTML element with tag name, attributes, and children.
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
        children: Vec<Node>,
        /// True for void elements (br, hr, img, etc.) that have no closing tag.
        self_closing: bool,
        span: Span,
    },
    /// Text content.
    Text {
        content: String,
        span: Span,
    },
    /// `<!-- comment -->`.
    Comment {
        content: String,
        span: Span,
    },
    /// Raw HTML content to be emitted verbatim without escaping.
    Raw {
        content: String,
        span: Span,
    },
}

impl Node {
    pub fn strip_spans(&self) -> Node {
        match self {
            Node::Doctype {
                name,
                public_id,
                system_id,
                ..
            } => Node::Doctype {
                name: name.clone(),
                public_id: public_id.clone(),
                system_id: system_id.clone(),
                span: Span::NONE,
            },
            Node::Element {
                tag,
                attrs,
                children,
                self_closing,
                ..
            } => Node::Element {
                tag: tag.clone(),
                attrs: attrs.clone(),
                children: children.iter().map(|n| n.strip_spans()).collect(),
                self_closing: *self_closing,
                span: Span::NONE,
            },
            Node::Text { content, .. } => Node::Text {
                content: content.clone(),
                span: Span::NONE,
            },
            Node::Comment { content, .. } => Node::Comment {
                content: content.clone(),
                span: Span::NONE,
            },
            Node::Raw { content, .. } => Node::Raw {
                content: content.clone(),
                span: Span::NONE,
            },
        }
    }

    /// Get the element tag name, if this is an `Element`.
    pub fn tag(&self) -> Option<&str> {
        match self {
            Node::Element { tag, .. } => Some(tag),
            _ => None,
        }
    }

    /// Get the children, if this is an `Element`.
    pub fn children(&self) -> Option<&[Node]> {
        match self {
            Node::Element { children, .. } => Some(children),
            _ => None,
        }
    }

    /// Get the attributes, if this is an `Element`.
    pub fn attrs(&self) -> Option<&[(String, String)]> {
        match self {
            Node::Element { attrs, .. } => Some(attrs),
            _ => None,
        }
    }

    /// Get a specific attribute value by name.
    pub fn get_attr(&self, name: &str) -> Option<&str> {
        self.attrs()
            .and_then(|attrs| attrs.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str()))
    }
}

/// Diagnostic message from parsing.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}

/// Check if a tag name is a void element (no closing tag in HTML5).
pub fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}
