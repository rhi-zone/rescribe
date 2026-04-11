//! HTML parser using html5ever's tree builder.
//!
//! Produces a faithful representation of the HTML5 parse tree. html5ever
//! implements the full HTML5 parsing algorithm including error recovery,
//! implied element insertion, and foster parenting.

use html5ever::tendril::TendrilSink;
use html5ever::parse_document;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

use crate::ast::*;

/// Parse an HTML document from bytes (assumed UTF-8).
///
/// Returns the parsed document and any diagnostics (parse errors reported
/// by html5ever). html5ever never fails — malformed input is handled per
/// the HTML5 error-recovery rules.
pub fn parse(input: &[u8]) -> (HtmlDoc, Vec<Diagnostic>) {
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut &*input)
        .expect("reading from &[u8] cannot fail");

    let mut diagnostics = Vec::new();
    for err in dom.errors.borrow().iter() {
        diagnostics.push(Diagnostic {
            message: err.to_string(),
            span: Span::NONE,
        });
    }

    let nodes = convert_children(&dom.document);
    (HtmlDoc { nodes }, diagnostics)
}

/// Convert children of an html5ever handle to our AST nodes.
fn convert_children(handle: &Handle) -> Vec<Node> {
    handle
        .children
        .borrow()
        .iter()
        .map(convert_node)
        .collect()
}

/// Convert a single html5ever node to our AST.
fn convert_node(handle: &Handle) -> Node {
    match &handle.data {
        NodeData::Document => {
            // Document node appearing as a child — wrap children in a synthetic element.
            let children = convert_children(handle);
            Node::Element {
                tag: String::new(),
                attrs: Vec::new(),
                children,
                self_closing: false,
                span: Span::NONE,
            }
        }
        NodeData::Doctype {
            name,
            public_id,
            system_id,
        } => Node::Doctype {
            name: name.to_string(),
            public_id: public_id.to_string(),
            system_id: system_id.to_string(),
            span: Span::NONE,
        },
        NodeData::Element { name, attrs, .. } => {
            let tag = name.local.to_string();
            let attrs_vec: Vec<(String, String)> = attrs
                .borrow()
                .iter()
                .map(|a| (a.name.local.to_string(), a.value.to_string()))
                .collect();
            let children = convert_children(handle);
            let self_closing = is_void_element(&tag) && children.is_empty();
            Node::Element {
                tag,
                attrs: attrs_vec,
                children,
                self_closing,
                span: Span::NONE,
            }
        }
        NodeData::Text { contents } => Node::Text {
            content: contents.borrow().to_string(),
            span: Span::NONE,
        },
        NodeData::Comment { contents } => Node::Comment {
            content: contents.to_string(),
            span: Span::NONE,
        },
        NodeData::ProcessingInstruction { .. } => Node::Comment {
            content: String::new(),
            span: Span::NONE,
        },
    }
}
