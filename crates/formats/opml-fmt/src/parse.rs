//! OPML parser using `quick-xml`'s pull-based `Reader`.
//!
//! Drives `quick_xml::Reader` directly into [`OpmlDoc`] in a single pass —
//! no intermediate generic-XML tree, no event-layer indirection. This is
//! why [`crate::events()`] (see `events.rs`) can *also* drive the same
//! reader directly and be a genuinely independent implementation rather
//! than a walk over this function's output: both hit `quick_xml::Reader`
//! from scratch, they just build different things from what it yields.
//!
//! Structure recognized: `<opml version="...">`, `<head>` with the fixed
//! OPML 2.0 child-element set (anything else captured in [`Head::extra`]),
//! and `<body>` containing a tree of `<outline>` elements. Content outside
//! that shape (stray top-level comments/PIs, elements before/after
//! `<opml>`) is not modeled by this crate — see the module docs on
//! [`crate::ast`] for what *is* covered, and `opml-fmt`'s README/CLAUDE.md
//! entry for this known scope limit.

use quick_xml::Reader;
use quick_xml::events::Event as XmlEvent;

use crate::ast::*;

/// Parse an OPML document from bytes (assumed UTF-8).
///
/// Never panics: malformed XML is reported via `Diagnostic`s and parsing
/// stops at the point of failure, returning whatever tree was built so far.
pub(crate) fn parse(input: &[u8]) -> (OpmlDoc, Vec<Diagnostic>) {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(true);

    let mut diagnostics = Vec::new();
    let mut xml_decl = None;
    let mut version = String::new();
    let mut head = Head::default();
    let mut body = Body::default();

    #[derive(PartialEq)]
    enum Section {
        None,
        Head,
        Body,
    }
    let mut section = Section::None;
    let mut doc_start = 0usize;
    let mut doc_end = 0usize;

    // Stack of in-progress <outline> frames; top of stack is the innermost
    // currently-open outline. A frame's completed children accumulate in
    // `children` until the matching End tag pops it.
    struct OutlineFrame {
        attrs: Vec<(String, String)>,
        children: Vec<Outline>,
        start: usize,
    }
    let mut outline_stack: Vec<OutlineFrame> = Vec::new();

    // Name of a <head> child element currently being read (accumulating
    // text until its End tag), if any.
    let mut head_field: Option<(String, String, usize)> = None;

    let mut buf = Vec::new();

    macro_rules! push_outline {
        ($o:expr) => {
            if let Some(frame) = outline_stack.last_mut() {
                frame.children.push($o);
            } else {
                body.outlines.push($o);
            }
        };
    }

    loop {
        let pos = reader.buffer_position() as usize;
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Decl(decl)) => {
                let v = decl
                    .version()
                    .map(|v| String::from_utf8_lossy(&v).into_owned())
                    .unwrap_or_else(|_| "1.0".to_string());
                let encoding = decl
                    .encoding()
                    .and_then(|e| e.ok())
                    .map(|e| String::from_utf8_lossy(&e).into_owned());
                let standalone = decl
                    .standalone()
                    .and_then(|s| s.ok())
                    .map(|s| String::from_utf8_lossy(&s).into_owned());
                xml_decl = Some(XmlDecl {
                    version: v,
                    encoding,
                    standalone,
                });
            }
            Ok(XmlEvent::Start(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "opml" => {
                        doc_start = pos;
                        version = attr_value(&e, "version").unwrap_or_else(|| "2.0".to_string());
                    }
                    "head" if section == Section::None => {
                        section = Section::Head;
                        head.span.start = pos;
                    }
                    "body" if section != Section::Body => {
                        section = Section::Body;
                        body.span.start = pos;
                    }
                    "outline" if section == Section::Body => {
                        let attrs = read_attrs(&e, &mut diagnostics, pos);
                        outline_stack.push(OutlineFrame {
                            attrs,
                            children: Vec::new(),
                            start: pos,
                        });
                    }
                    other if section == Section::Head => {
                        head_field = Some((other.to_string(), String::new(), pos));
                    }
                    _ => {
                        diagnostics.push(Diagnostic {
                            message: format!("unexpected element <{name}>"),
                            span: Span {
                                start: pos,
                                end: pos,
                            },
                            severity: rescribe_format_api::Severity::Warning,
                            code: "",
                        });
                    }
                }
            }
            Ok(XmlEvent::Empty(e)) => {
                let name = local_name(&e);
                match name.as_str() {
                    "outline" if section == Section::Body => {
                        let attrs = read_attrs(&e, &mut diagnostics, pos);
                        push_outline!(Outline {
                            attrs,
                            children: Vec::new(),
                            self_closing: true,
                            span: Span {
                                start: pos,
                                end: reader.buffer_position() as usize,
                            },
                        });
                    }
                    other if section == Section::Head => {
                        assign_head_field(&mut head, other, String::new());
                    }
                    _ => {}
                }
            }
            Ok(XmlEvent::Text(t)) => {
                if let Some((_, text, _)) = &mut head_field {
                    let content = t
                        .decode()
                        .map(|c| c.into_owned())
                        .unwrap_or_else(|_| String::from_utf8_lossy(t.as_ref()).into_owned());
                    text.push_str(&content);
                }
            }
            Ok(XmlEvent::End(e)) => {
                let name = end_local_name(&e);
                match name.as_str() {
                    "opml" => {
                        doc_end = reader.buffer_position() as usize;
                    }
                    "head" => {
                        head.span.end = reader.buffer_position() as usize;
                        section = Section::None;
                    }
                    "body" => {
                        body.span.end = reader.buffer_position() as usize;
                    }
                    "outline" => {
                        if let Some(frame) = outline_stack.pop() {
                            push_outline!(Outline {
                                attrs: frame.attrs,
                                children: frame.children,
                                self_closing: false,
                                span: Span {
                                    start: frame.start,
                                    end: reader.buffer_position() as usize,
                                },
                            });
                        } else {
                            diagnostics.push(Diagnostic {
                                message: "unexpected closing tag </outline>".to_string(),
                                span: Span {
                                    start: pos,
                                    end: pos,
                                },
                                severity: rescribe_format_api::Severity::Warning,
                                code: "",
                            });
                        }
                    }
                    _ => {
                        if let Some((field_name, text, _)) = head_field.take()
                            && field_name == name
                        {
                            assign_head_field(&mut head, &field_name, text);
                        }
                    }
                }
            }
            Ok(XmlEvent::Eof) => break,
            Ok(_) => {} // Comments, PIs, CDATA: not modeled at this level; see module docs.
            Err(e) => {
                diagnostics.push(Diagnostic {
                    message: format!("XML parse error: {e}"),
                    span: Span {
                        start: pos,
                        end: pos,
                    },
                    severity: rescribe_format_api::Severity::Warning,
                    code: "",
                });
                break;
            }
        }
        buf.clear();
    }

    // Recover from truncated input: close any still-open outlines.
    while let Some(frame) = outline_stack.pop() {
        diagnostics.push(Diagnostic {
            message: "unclosed element <outline>".to_string(),
            span: Span::NONE,
            severity: rescribe_format_api::Severity::Warning,
            code: "",
        });
        let o = Outline {
            attrs: frame.attrs,
            children: frame.children,
            self_closing: false,
            span: Span::NONE,
        };
        if let Some(parent) = outline_stack.last_mut() {
            parent.children.push(o);
        } else {
            body.outlines.push(o);
        }
    }

    if version.is_empty() {
        version = "2.0".to_string();
    }

    (
        OpmlDoc {
            xml_decl,
            version,
            head,
            body,
            span: Span {
                start: doc_start,
                end: doc_end,
            },
        },
        diagnostics,
    )
}

fn assign_head_field(head: &mut Head, name: &str, value: String) {
    match name {
        "title" => head.title = Some(value),
        "dateCreated" => head.date_created = Some(value),
        "dateModified" => head.date_modified = Some(value),
        "ownerName" => head.owner_name = Some(value),
        "ownerEmail" => head.owner_email = Some(value),
        "ownerId" => head.owner_id = Some(value),
        "docs" => head.docs = Some(value),
        "expansionState" => head.expansion_state = Some(value),
        "vertScrollState" => head.vert_scroll_state = Some(value),
        "windowTop" => head.window_top = Some(value),
        "windowLeft" => head.window_left = Some(value),
        "windowBottom" => head.window_bottom = Some(value),
        "windowRight" => head.window_right = Some(value),
        other => head.extra.push((other.to_string(), value)),
    }
}

fn local_name(e: &quick_xml::events::BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}

fn end_local_name(e: &quick_xml::events::BytesEnd<'_>) -> String {
    String::from_utf8_lossy(e.local_name().as_ref()).into_owned()
}

fn attr_value(e: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        if a.key.local_name().as_ref() == name.as_bytes() {
            Some(
                a.unescape_value()
                    .map(|v| v.into_owned())
                    .unwrap_or_else(|_| String::from_utf8_lossy(&a.value).into_owned()),
            )
        } else {
            None
        }
    })
}

fn read_attrs(
    e: &quick_xml::events::BytesStart<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    pos: usize,
) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    for attr in e.attributes() {
        match attr {
            Ok(attr) => {
                let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                let value = attr
                    .unescape_value()
                    .map(|v| v.into_owned())
                    .unwrap_or_else(|_| String::from_utf8_lossy(&attr.value).into_owned());
                attrs.push((key, value));
            }
            Err(e) => diagnostics.push(Diagnostic {
                message: format!("attribute parse error: {e}"),
                span: Span {
                    start: pos,
                    end: pos,
                },
                severity: rescribe_format_api::Severity::Warning,
                code: "",
            }),
        }
    }
    attrs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_document() {
        let (doc, diags) = parse(
            br#"<?xml version="1.0"?>
<opml version="2.0">
  <head><title>Test</title></head>
  <body>
    <outline text="Item 1"/>
    <outline text="Item 2"/>
  </body>
</opml>"#,
        );
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.version, "2.0");
        assert_eq!(doc.head.title.as_deref(), Some("Test"));
        assert_eq!(doc.body.outlines.len(), 2);
        assert_eq!(doc.body.outlines[0].text(), Some("Item 1"));
    }

    #[test]
    fn parses_nested_outlines() {
        let (doc, diags) = parse(
            br#"<opml version="2.0"><body>
    <outline text="Parent">
      <outline text="Child 1"/>
      <outline text="Child 2"/>
    </outline>
  </body></opml>"#,
        );
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.body.outlines.len(), 1);
        let parent = &doc.body.outlines[0];
        assert_eq!(parent.text(), Some("Parent"));
        assert_eq!(parent.children.len(), 2);
        assert_eq!(parent.children[0].text(), Some("Child 1"));
    }

    #[test]
    fn parses_all_head_fields() {
        let (doc, diags) = parse(
            br#"<opml version="2.0"><head>
    <title>T</title>
    <dateCreated>Mon, 1 Jan 2024 00:00:00 GMT</dateCreated>
    <dateModified>Tue, 2 Jan 2024 00:00:00 GMT</dateModified>
    <ownerName>Alice</ownerName>
    <ownerEmail>alice@example.com</ownerEmail>
    <ownerId>http://example.com/alice</ownerId>
    <docs>http://dev.opml.org/spec2.html</docs>
    <expansionState>1,3</expansionState>
    <vertScrollState>1</vertScrollState>
    <windowTop>1</windowTop>
    <windowLeft>2</windowLeft>
    <windowBottom>3</windowBottom>
    <windowRight>4</windowRight>
  </head><body/></opml>"#,
        );
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert_eq!(doc.head.owner_email.as_deref(), Some("alice@example.com"));
        assert_eq!(doc.head.expansion_state.as_deref(), Some("1,3"));
        assert_eq!(doc.head.window_right.as_deref(), Some("4"));
    }

    #[test]
    fn preserves_unknown_outline_attributes() {
        let (doc, _) = parse(
            br#"<opml version="2.0"><body>
    <outline text="X" isComment="true" myApp:custom="value"/>
  </body></opml>"#,
        );
        let o = &doc.body.outlines[0];
        assert_eq!(o.is_comment(), Some(true));
        assert_eq!(o.attr("myApp:custom"), Some("value"));
    }

    #[test]
    fn preserves_unknown_head_elements() {
        let (doc, _) = parse(
            br#"<opml version="2.0"><head><title>T</title><futureField>abc</futureField></head><body/></opml>"#,
        );
        assert_eq!(
            doc.head.extra,
            vec![("futureField".to_string(), "abc".to_string())]
        );
    }

    #[test]
    fn recovers_from_truncated_input() {
        let (doc, diags) = parse(br#"<opml version="2.0"><body><outline text="X">"#);
        assert!(!diags.is_empty());
        assert_eq!(doc.body.outlines.len(), 1);
    }

    #[test]
    fn no_head_element() {
        let (doc, diags) = parse(br#"<opml version="2.0"><body><outline text="X"/></body></opml>"#);
        assert!(diags.is_empty(), "diagnostics: {diags:?}");
        assert!(doc.head.is_empty());
        assert_eq!(doc.body.outlines.len(), 1);
    }
}
