//! HTML emitter: converts an `HtmlDoc` AST back to HTML5.

use crate::ast::*;

/// Options for HTML emission.
#[derive(Clone, Debug, Default)]
pub struct EmitOptions {
    /// If true, add newlines and indentation for block-level elements.
    pub pretty: bool,
}

/// Emit an `HtmlDoc` as HTML bytes.
pub fn emit(doc: &HtmlDoc) -> Vec<u8> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit an `HtmlDoc` as HTML bytes with options.
pub fn emit_with_options(doc: &HtmlDoc, options: &EmitOptions) -> Vec<u8> {
    let mut out = Emitter::new(options.pretty);
    for node in &doc.nodes {
        out.emit_node(node);
    }
    out.finish()
}

struct Emitter {
    buf: String,
    pretty: bool,
    indent: usize,
}

impl Emitter {
    fn new(pretty: bool) -> Self {
        Emitter {
            buf: String::new(),
            pretty,
            indent: 0,
        }
    }

    fn finish(self) -> Vec<u8> {
        self.buf.into_bytes()
    }

    fn newline(&mut self) {
        if self.pretty {
            self.buf.push('\n');
            for _ in 0..self.indent {
                self.buf.push_str("  ");
            }
        }
    }

    fn emit_node(&mut self, node: &Node) {
        match node {
            Node::Doctype {
                name,
                public_id,
                system_id,
                ..
            } => {
                self.buf.push_str("<!DOCTYPE ");
                self.buf.push_str(name);
                if !public_id.is_empty() {
                    self.buf.push_str(" PUBLIC \"");
                    self.buf.push_str(public_id);
                    self.buf.push('"');
                    if !system_id.is_empty() {
                        self.buf.push_str(" \"");
                        self.buf.push_str(system_id);
                        self.buf.push('"');
                    }
                } else if !system_id.is_empty() {
                    self.buf.push_str(" SYSTEM \"");
                    self.buf.push_str(system_id);
                    self.buf.push('"');
                }
                self.buf.push('>');
            }
            Node::Element {
                tag,
                attrs,
                children,
                self_closing,
                ..
            } => {
                let is_block = is_block_element(tag);
                if is_block {
                    self.newline();
                }

                self.buf.push('<');
                self.buf.push_str(tag);
                for (name, value) in attrs {
                    self.buf.push(' ');
                    self.buf.push_str(name);
                    self.buf.push_str("=\"");
                    self.buf.push_str(&escape_attr(value));
                    self.buf.push('"');
                }
                self.buf.push('>');

                if !*self_closing {
                    let has_block_children =
                        children.iter().any(|c| matches!(c, Node::Element { tag, .. } if is_block_element(tag)));

                    if has_block_children {
                        self.indent += 1;
                        for child in children {
                            self.emit_node(child);
                        }
                        self.indent -= 1;
                        if is_block {
                            self.newline();
                        }
                    } else {
                        for child in children {
                            self.emit_node(child);
                        }
                    }

                    self.buf.push_str("</");
                    self.buf.push_str(tag);
                    self.buf.push('>');
                }
            }
            Node::Text { content, .. } => {
                self.buf.push_str(&escape_html(content));
            }
            Node::Comment { content, .. } => {
                self.buf.push_str("<!--");
                self.buf.push_str(content);
                self.buf.push_str("-->");
            }
            Node::Raw { content, .. } => {
                self.buf.push_str(content);
            }
        }
    }
}

/// Check if a tag is a block-level element (for pretty-printing).
fn is_block_element(tag: &str) -> bool {
    matches!(
        tag,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "body"
            | "dd"
            | "details"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "head"
            | "header"
            | "hr"
            | "html"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "summary"
            | "table"
            | "tbody"
            | "td"
            | "tfoot"
            | "th"
            | "thead"
            | "tr"
            | "ul"
    )
}

/// Escape HTML special characters in text content.
pub fn escape_html(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            _ => result.push(c),
        }
    }
    result
}

/// Escape HTML special characters in attribute values.
pub fn escape_attr(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            _ => result.push(c),
        }
    }
    result
}
