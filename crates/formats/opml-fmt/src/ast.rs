//! AST types for OPML (Outline Processor Markup Language) documents.
//!
//! Unlike `tei-fmt`/`docbook-fmt`/`jats-fmt` (which model "any well-formed
//! XML document" generically, because those formats have hundreds of
//! element names with document-specific meaning), OPML has a small, fixed
//! grammar defined at <http://dev.opml.org/spec2.html>: a document is
//! exactly `<opml><head>...</head><body><outline>...</outline>...</body></opml>`.
//! That fixed shape is worth modeling directly as a typed AST — `OpmlDoc` /
//! `Head` / `Body` / `Outline` — rather than a generic element tree, so a
//! caller who has never heard of rescribe (a feed-reader import tool, an
//! outliner, a podcast-subscription-list converter) gets a shape that
//! matches the format they already know instead of having to rediscover
//! "which of these XML elements means what" on every use.
//!
//! Losslessness for the one part of OPML that is genuinely open-ended —
//! `<outline>` attributes, which the spec explicitly allows applications to
//! extend with their own (`isComment`, `isBreakpoint`, and any
//! application-specific attribute alongside the well-known `text`, `type`,
//! `xmlUrl`, ...) — is handled by storing *every* attribute verbatim in
//! [`Outline::attrs`] as an ordered `Vec<(String, String)>`, with typed
//! accessor methods (`text()`, `xml_url()`, `is_comment()`, ...) layered on
//! top for convenience. `attrs` is the single source of truth; the
//! accessors never own data the struct doesn't already have, so an unknown
//! or future attribute round-trips through `parse` → `emit` unchanged
//! without the crate needing to know its name.

/// Byte offset span in the source input.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const NONE: Span = Span { start: 0, end: 0 };
}

/// The XML declaration (`<?xml version="1.0" encoding="UTF-8"?>`), if present.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct XmlDecl {
    pub version: String,
    pub encoding: Option<String>,
    pub standalone: Option<String>,
}

/// An OPML document: `<opml version="...">`, `<head>`, `<body>`.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct OpmlDoc {
    /// The `<?xml ... ?>` declaration, if the source had one.
    pub xml_decl: Option<XmlDecl>,
    /// The `<opml>` root's `version` attribute (`"1.0"`, `"1.1"`, or
    /// `"2.0"` per the spec — preserved verbatim, not validated against
    /// that list, since a future/nonstandard value must still round-trip).
    pub version: String,
    pub head: Head,
    pub body: Body,
    pub span: Span,
}

impl OpmlDoc {
    pub fn strip_spans(&self) -> OpmlDoc {
        OpmlDoc {
            xml_decl: self.xml_decl.clone(),
            version: self.version.clone(),
            head: self.head.strip_spans(),
            body: self.body.strip_spans(),
            span: Span::NONE,
        }
    }
}

/// `<head>`: document metadata.
///
/// Every child element the OPML 2.0 spec defines gets a named field. Any
/// other child element (a future spec addition, or an application-specific
/// extension) is preserved verbatim in [`Head::extra`] as `(tag name, text
/// content)` pairs, in source order, rather than silently dropped.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Head {
    pub title: Option<String>,
    pub date_created: Option<String>,
    pub date_modified: Option<String>,
    pub owner_name: Option<String>,
    pub owner_email: Option<String>,
    pub owner_id: Option<String>,
    pub docs: Option<String>,
    pub expansion_state: Option<String>,
    pub vert_scroll_state: Option<String>,
    pub window_top: Option<String>,
    pub window_left: Option<String>,
    pub window_bottom: Option<String>,
    pub window_right: Option<String>,
    /// Any `<head>` child element not in the fixed OPML 2.0 set above,
    /// captured as `(tag name, text content)` in source order.
    pub extra: Vec<(String, String)>,
    pub span: Span,
}

impl Head {
    pub fn strip_spans(&self) -> Head {
        Head {
            span: Span::NONE,
            ..self.clone()
        }
    }

    /// True if every field is unset. Note this is also true for a source
    /// document with **no** `<head>` element at all — an explicit but empty
    /// `<head></head>` and an absent `<head>` both parse to `Head::default()`
    /// and are indistinguishable here (no separate "was present" bit is
    /// tracked). `emit`/`events_from_doc` both treat `is_empty()` as "omit
    /// `<head>` from the output", the same canonical choice in both places.
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.date_created.is_none()
            && self.date_modified.is_none()
            && self.owner_name.is_none()
            && self.owner_email.is_none()
            && self.owner_id.is_none()
            && self.docs.is_none()
            && self.expansion_state.is_none()
            && self.vert_scroll_state.is_none()
            && self.window_top.is_none()
            && self.window_left.is_none()
            && self.window_bottom.is_none()
            && self.window_right.is_none()
            && self.extra.is_empty()
    }
}

/// `<body>`: the top-level list of `<outline>` elements.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Body {
    pub outlines: Vec<Outline>,
    pub span: Span,
}

impl Body {
    pub fn strip_spans(&self) -> Body {
        Body {
            outlines: self.outlines.iter().map(Outline::strip_spans).collect(),
            span: Span::NONE,
        }
    }
}

/// One `<outline>` element: an OPML "unit" — a single item, a feed
/// subscription, a comment, a breakpoint, or a container for nested
/// outlines, depending on which attributes are present.
///
/// `attrs` is the source of truth for every attribute (see the module
/// docs); the accessor methods below read out of it and never carry data
/// `attrs` doesn't already have.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Outline {
    /// All attributes, verbatim, in source order.
    pub attrs: Vec<(String, String)>,
    /// Nested `<outline>` children, in source order.
    pub children: Vec<Outline>,
    /// True if the source wrote this as a self-closing tag (`<outline/>`),
    /// false if it was an explicit open/close pair (`<outline></outline>`
    /// when `children` is empty, or `<outline>...</outline>` otherwise).
    /// Only distinguishable when `children` is empty — both source forms
    /// parse to zero children, but they are not the same bytes, and
    /// losslessness requires preserving which one the input used. Default
    /// (`false`) on a programmatically-constructed `Outline`, matching
    /// `docbook-fmt`'s identical `Node::Element.self_closing` convention.
    pub self_closing: bool,
    pub span: Span,
}

impl Outline {
    pub fn strip_spans(&self) -> Outline {
        Outline {
            attrs: self.attrs.clone(),
            children: self.children.iter().map(Outline::strip_spans).collect(),
            self_closing: self.self_closing,
            span: Span::NONE,
        }
    }

    /// Get an attribute by name (case-sensitive, per the spec).
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// `text` — the required, plain-text (no markup) label for the outline.
    /// Renders as the outline's visible content in every OPML-aware tool.
    pub fn text(&self) -> Option<&str> {
        self.attr("text")
    }

    /// `title` — an alternate label some producers write in addition to (or
    /// instead of) `text`; distinct from it per the spec, not a synonym.
    pub fn title(&self) -> Option<&str> {
        self.attr("title")
    }

    /// `type` — e.g. `"rss"`, `"atom"`, `"link"`, `"include"`.
    pub fn outline_type(&self) -> Option<&str> {
        self.attr("type")
    }

    /// `isComment` — `"true"`/`"false"` string per spec; returns `None` if
    /// absent or not exactly `"true"`/`"false"` (preserved either way via
    /// `attrs`, this accessor just doesn't guess at malformed values).
    pub fn is_comment(&self) -> Option<bool> {
        parse_opml_bool(self.attr("isComment"))
    }

    /// `isBreakpoint` — see [`Outline::is_comment`] for the string format.
    pub fn is_breakpoint(&self) -> Option<bool> {
        parse_opml_bool(self.attr("isBreakpoint"))
    }

    pub fn created(&self) -> Option<&str> {
        self.attr("created")
    }

    pub fn category(&self) -> Option<&str> {
        self.attr("category")
    }

    /// `xmlUrl` — feed URL for a subscription-list outline.
    pub fn xml_url(&self) -> Option<&str> {
        self.attr("xmlUrl")
    }

    /// `htmlUrl` — companion human-readable page URL.
    pub fn html_url(&self) -> Option<&str> {
        self.attr("htmlUrl")
    }
}

fn parse_opml_bool(v: Option<&str>) -> Option<bool> {
    match v {
        Some("true") => Some(true),
        Some("false") => Some(false),
        _ => None,
    }
}

/// Diagnostic message from parsing.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}
