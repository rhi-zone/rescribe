//! Native format parser and builder.
//!
//! Standalone crate with no rescribe dependency.
//! Used by `rescribe-read-native` and `rescribe-write-native` as thin adapter layers.

use std::collections::BTreeMap;

// ── Error ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct NativeError(pub String);

impl std::fmt::Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Native error: {}", self.0)
    }
}

impl std::error::Error for NativeError {}

// ── AST ───────────────────────────────────────────────────────────────────────

/// A parsed native format document.
#[derive(Debug, Clone)]
pub struct NativeDoc {
    pub content: NativeNode,
    pub metadata: BTreeMap<String, String>,
    pub resources: Vec<NativeResource>,
}

/// A node in the native format.
#[derive(Debug, Clone)]
pub struct NativeNode {
    pub kind: String,
    pub props: BTreeMap<String, NativeValue>,
    pub children: Vec<NativeNode>,
}

/// A resource in the native format.
#[derive(Debug, Clone)]
pub struct NativeResource {
    pub id: String,
    pub mime_type: String,
    pub size: usize,
}

/// A value in native format properties.
#[derive(Debug, Clone)]
pub enum NativeValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<NativeValue>),
    Map(BTreeMap<String, NativeValue>),
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Parse a native format string into a [`NativeDoc`].
pub fn parse(input: &str) -> Result<NativeDoc, NativeError> {
    let mut p = Parser::new(input);
    p.parse_document()
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn parse_document(&mut self) -> Result<NativeDoc, NativeError> {
        self.skip_whitespace();
        self.expect_str("Document")?;
        self.skip_whitespace();
        self.expect_char('{')?;

        let mut content = NativeNode {
            kind: "document".to_string(),
            props: BTreeMap::new(),
            children: Vec::new(),
        };
        let mut metadata = BTreeMap::new();
        let mut resources = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace();

            if self.peek() == Some('}') {
                self.advance();
                break;
            }

            // Look for content: or metadata: or resources:
            if self.check_str("content:") {
                self.expect_str("content:")?;
                self.skip_whitespace();
                content = self.parse_node()?;
            } else if self.check_str("metadata:") {
                self.expect_str("metadata:")?;
                self.skip_whitespace();
                self.expect_char('{')?;
                while self.peek() != Some('}') && !self.is_at_end() {
                    self.skip_whitespace();
                    if self.peek() == Some('}') {
                        break;
                    }
                    let key = self.parse_identifier()?;
                    self.skip_whitespace();
                    self.expect_char(':')?;
                    self.skip_whitespace();
                    let value = self.parse_value()?;
                    if let NativeValue::String(s) = value {
                        metadata.insert(key, s);
                    }
                    self.skip_whitespace();
                    if self.peek() == Some(',') {
                        self.advance();
                    }
                }
                self.expect_char('}')?;
            } else if self.check_str("resources:") {
                self.expect_str("resources:")?;
                self.skip_whitespace();
                // Accept both {} (fixture format) and [] (legacy format)
                let (close_char, parse_items) = if self.peek() == Some('{') {
                    self.advance();
                    ('}', true)
                } else if self.peek() == Some('[') {
                    self.advance();
                    ('[', true)
                } else {
                    ('\0', false)
                };
                let end_char = if close_char == '[' { ']' } else { close_char };
                if parse_items && end_char != '\0' {
                    while self.peek() != Some(end_char) && !self.is_at_end() {
                        self.skip_whitespace();
                        if self.peek() == Some(end_char) {
                            break;
                        }
                        // Parse Resource { id: "...", mime: "...", size: ... }
                        if self.check_str("Resource") {
                            self.expect_str("Resource")?;
                            self.skip_whitespace();
                            self.expect_char('{')?;
                            let mut res = NativeResource {
                                id: String::new(),
                                mime_type: String::new(),
                                size: 0,
                            };
                            while self.peek() != Some('}') && !self.is_at_end() {
                                self.skip_whitespace();
                                if self.peek() == Some('}') {
                                    break;
                                }
                                let key = self.parse_identifier()?;
                                self.skip_whitespace();
                                self.expect_char(':')?;
                                self.skip_whitespace();
                                let value = self.parse_value()?;
                                match key.as_str() {
                                    "id" => {
                                        if let NativeValue::String(s) = value {
                                            res.id = s;
                                        }
                                    }
                                    "mime" => {
                                        if let NativeValue::String(s) = value {
                                            res.mime_type = s;
                                        }
                                    }
                                    "size" => {
                                        if let NativeValue::Int(n) = value {
                                            res.size = n as usize;
                                        }
                                    }
                                    _ => {}
                                }
                                self.skip_whitespace();
                                if self.peek() == Some(',') {
                                    self.advance();
                                }
                            }
                            self.expect_char('}')?;
                            resources.push(res);
                        } else {
                            break;
                        }
                        self.skip_whitespace();
                    }
                    self.expect_char(end_char)?;
                }
            } else {
                // Try to parse a node directly
                content = self.parse_node()?;
            }
        }

        Ok(NativeDoc {
            content,
            metadata,
            resources,
        })
    }

    fn parse_node(&mut self) -> Result<NativeNode, NativeError> {
        self.skip_whitespace();

        // Parse node kind
        let kind = self.parse_identifier()?;
        self.skip_whitespace();
        self.expect_char('(')?;

        let mut node = NativeNode {
            kind,
            props: BTreeMap::new(),
            children: Vec::new(),
        };

        // Parse optional props
        self.skip_whitespace();
        if self.peek() == Some('{') {
            self.advance();
            while self.peek() != Some('}') && !self.is_at_end() {
                self.skip_whitespace();
                if self.peek() == Some('}') {
                    break;
                }

                let key = self.parse_identifier()?;
                self.skip_whitespace();
                self.expect_char(':')?;
                self.skip_whitespace();
                let value = self.parse_value()?;

                node.props.insert(key, value);

                self.skip_whitespace();
                if self.peek() == Some(',') {
                    self.advance();
                }
            }
            self.expect_char('}')?;
        }

        self.skip_whitespace();
        self.expect_char(')')?;

        // Parse optional children
        self.skip_whitespace();
        if self.peek() == Some('[') {
            self.advance();
            while self.peek() != Some(']') && !self.is_at_end() {
                self.skip_whitespace();
                if self.peek() == Some(']') {
                    break;
                }
                let child = self.parse_node()?;
                node.children.push(child);
                self.skip_whitespace();
            }
            self.expect_char(']')?;
        }

        Ok(node)
    }

    fn parse_identifier(&mut self) -> Result<String, NativeError> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(NativeError("expected identifier".into()));
        }
        Ok(self.input[start..self.pos].to_string())
    }

    fn parse_value(&mut self) -> Result<NativeValue, NativeError> {
        self.skip_whitespace();

        if self.peek() == Some('"') {
            // String value
            self.advance();
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c == '"' && !self.input[self.pos.saturating_sub(1)..self.pos].ends_with('\\') {
                    break;
                }
                self.advance();
            }
            let value = self.input[start..self.pos].replace("\\\"", "\"");
            self.expect_char('"')?;
            Ok(NativeValue::String(value))
        } else if self.peek() == Some('[') {
            // List value
            self.advance();
            let mut items = Vec::new();
            while self.peek() != Some(']') && !self.is_at_end() {
                self.skip_whitespace();
                if self.peek() == Some(']') {
                    break;
                }
                items.push(self.parse_value()?);
                self.skip_whitespace();
                if self.peek() == Some(',') {
                    self.advance();
                }
            }
            self.expect_char(']')?;
            Ok(NativeValue::List(items))
        } else if self.peek() == Some('{') {
            // Map value
            self.advance();
            let mut map = BTreeMap::new();
            while self.peek() != Some('}') && !self.is_at_end() {
                self.skip_whitespace();
                if self.peek() == Some('}') {
                    break;
                }
                let key = self.parse_identifier()?;
                self.skip_whitespace();
                self.expect_char(':')?;
                self.skip_whitespace();
                let value = self.parse_value()?;
                map.insert(key, value);
                self.skip_whitespace();
                if self.peek() == Some(',') {
                    self.advance();
                }
            }
            self.expect_char('}')?;
            Ok(NativeValue::Map(map))
        } else {
            // Number, boolean, or identifier-like value
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
            let text = &self.input[start..self.pos];
            if text.is_empty() {
                return Err(NativeError("expected value".into()));
            }

            if text == "true" {
                Ok(NativeValue::Bool(true))
            } else if text == "false" {
                Ok(NativeValue::Bool(false))
            } else if let Ok(i) = text.parse::<i64>() {
                Ok(NativeValue::Int(i))
            } else if let Ok(f) = text.parse::<f64>() {
                Ok(NativeValue::Float(f))
            } else {
                Err(NativeError(format!("invalid value: {}", text)))
            }
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn expect_char(&mut self, expected: char) -> Result<(), NativeError> {
        if self.peek() == Some(expected) {
            self.advance();
            Ok(())
        } else {
            Err(NativeError(format!(
                "expected '{}', got {:?}",
                expected,
                self.peek()
            )))
        }
    }

    fn expect_str(&mut self, expected: &str) -> Result<(), NativeError> {
        if self.input[self.pos..].starts_with(expected) {
            self.pos += expected.len();
            Ok(())
        } else {
            Err(NativeError(format!("expected '{}'", expected)))
        }
    }

    fn check_str(&self, s: &str) -> bool {
        self.input[self.pos..].starts_with(s)
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Build a native format string from a [`NativeDoc`].
pub fn build(doc: &NativeDoc) -> String {
    let mut ctx = BuildContext::new();

    ctx.write("Document {\n");

    // Metadata
    if !doc.metadata.is_empty() {
        ctx.write("  metadata: {\n");
        for (key, value) in &doc.metadata {
            ctx.write(&format!("    {}: {:?}\n", key, value));
        }
        ctx.write("  }\n");
    }

    // Content
    ctx.write("  content:\n");
    build_node(&doc.content, &mut ctx, 2);

    // Resources
    if !doc.resources.is_empty() {
        ctx.write("  resources: [\n");
        for resource in &doc.resources {
            ctx.write(&format!(
                "    Resource {{ id: {:?}, mime: {:?}, size: {} }}\n",
                resource.id, resource.mime_type, resource.size
            ));
        }
        ctx.write("  ]\n");
    }

    ctx.write("}\n");
    ctx.output
}

struct BuildContext {
    output: String,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            output: String::new(),
        }
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }
}

fn build_node(node: &NativeNode, ctx: &mut BuildContext, indent: usize) {
    let indent_str = "  ".repeat(indent);

    ctx.write(&format!("{}{}(", indent_str, node.kind));

    // Props
    if !node.props.is_empty() {
        ctx.write(" {");
        let props: Vec<String> = node
            .props
            .iter()
            .map(|(k, v)| format!(" {}: {}", k, format_value(v)))
            .collect();
        ctx.write(&props.join(","));
        ctx.write(" }");
    }

    // Children
    if node.children.is_empty() {
        ctx.write(")\n");
    } else {
        ctx.write(") [\n");
        for child in &node.children {
            build_node(child, ctx, indent + 1);
        }
        ctx.write(&format!("{}]\n", indent_str));
    }
}

fn format_value(value: &NativeValue) -> String {
    match value {
        NativeValue::String(s) => format!("{:?}", s),
        NativeValue::Int(i) => format!("{}", i),
        NativeValue::Float(f) => format!("{}", f),
        NativeValue::Bool(b) => format!("{}", b),
        NativeValue::List(items) => {
            let formatted: Vec<String> = items.iter().map(format_value).collect();
            format!("[{}]", formatted.join(", "))
        }
        NativeValue::Map(map) => {
            let formatted: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", k, format_value(v)))
                .collect();
            format!("{{{}}}", formatted.join(", "))
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let input = r#"Document {
  content:
  document() [
    paragraph() [
      text( { content: "Hello" })
    ]
  ]
}"#;
        let result = parse(input).unwrap();
        assert_eq!(result.content.kind, "document");
        assert_eq!(result.content.children.len(), 1);
    }

    #[test]
    fn test_parse_node() {
        let input = "Document { content: heading( { level: 1 }) }";
        let result = parse(input).unwrap();
        assert_eq!(result.content.kind, "heading");
        let level = result.content.props.get("level");
        assert!(level.is_some());
    }

    #[test]
    fn test_parse_with_children() {
        let input = r#"Document {
  content:
  document() [
    paragraph() [
      text()
    ]
  ]
}"#;
        let result = parse(input).unwrap();
        assert_eq!(result.content.children.len(), 1);
        let para = &result.content.children[0];
        assert_eq!(para.kind, "paragraph");
        assert_eq!(para.children.len(), 1);
    }

    #[test]
    fn test_format_value_string() {
        assert_eq!(
            format_value(&NativeValue::String("test".into())),
            "\"test\""
        );
    }

    #[test]
    fn test_format_value_int() {
        assert_eq!(format_value(&NativeValue::Int(42)), "42");
    }

    #[test]
    fn test_format_value_bool() {
        assert_eq!(format_value(&NativeValue::Bool(true)), "true");
    }

    #[test]
    fn test_build_simple() {
        let doc = NativeDoc {
            content: NativeNode {
                kind: "document".to_string(),
                props: BTreeMap::new(),
                children: vec![NativeNode {
                    kind: "paragraph".to_string(),
                    props: BTreeMap::new(),
                    children: vec![NativeNode {
                        kind: "text".to_string(),
                        props: {
                            let mut m = BTreeMap::new();
                            m.insert("content".to_string(), NativeValue::String("Hello".into()));
                            m
                        },
                        children: Vec::new(),
                    }],
                }],
            },
            metadata: BTreeMap::new(),
            resources: Vec::new(),
        };
        let output = build(&doc);
        assert!(output.contains("Document {"));
        assert!(output.contains("document("));
        assert!(output.contains("paragraph("));
    }

    #[test]
    fn test_roundtrip() {
        let input = r#"Document {
  content:
  document() [
    paragraph() [
      text( { content: "Hello world" })
    ]
  ]
}"#;
        let doc = parse(input).unwrap();
        let output = build(&doc);
        let doc2 = parse(&output).unwrap();
        assert_eq!(doc.content.kind, doc2.content.kind);
        assert_eq!(doc.content.children.len(), doc2.content.children.len());
    }
}
