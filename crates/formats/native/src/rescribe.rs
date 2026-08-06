//! AST<->rescribe::Document translation for the native format.
//!
//! Thin adapter layer that translates between [`crate::NativeDoc`] and
//! rescribe's `Document` IR. This module only exists when the `rescribe`
//! feature is enabled; the rest of the crate has no rescribe dependency.

use crate::{NativeDoc, NativeNode, NativeResource, NativeValue};
use rescribe_core::{
    ConversionResult, Document, EmitError, EmitOptions, ParseError, ParseOptions, PropValue,
    Properties, Resource, ResourceId,
};
use rescribe_std::Node;
use std::collections::{BTreeMap, HashMap};

/// Parse native format input into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse native format input into a document with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let native_doc = crate::parse(input)
        .map_err(|e| ParseError::Invalid(format!("Failed to parse native format: {}", e)))?;

    let content = convert_node(&native_doc.content);
    let mut resources = HashMap::new();
    for res in &native_doc.resources {
        let resource_id = ResourceId::from_string(res.id.clone());
        resources.insert(
            resource_id,
            Resource::new(res.mime_type.clone(), Vec::new()),
        );
    }

    let mut metadata = Properties::new();
    for (key, value) in &native_doc.metadata {
        metadata.set(key.clone(), value.clone());
    }

    let doc = Document {
        content,
        resources,
        metadata,
        source: None,
    };

    Ok(ConversionResult::ok(doc))
}

/// Convert a native node to a rescribe node.
fn convert_node(native_node: &NativeNode) -> Node {
    let mut node = Node::new(native_node.kind.as_str());

    // Convert properties
    for (key, value) in native_node.props.iter() {
        let prop_value = convert_value(value);
        node = node.prop(key, prop_value);
    }

    // Convert children
    for child in &native_node.children {
        node = node.child(convert_node(child));
    }

    node
}

/// Convert a native value to a rescribe PropValue.
/// Native format stores everything as strings when round-tripping.
fn convert_value(value: &NativeValue) -> PropValue {
    match value {
        NativeValue::String(s) => PropValue::String(s.clone()),
        NativeValue::Int(i) => PropValue::String(i.to_string()),
        NativeValue::Float(f) => PropValue::String(f.to_string()),
        NativeValue::Bool(b) => PropValue::String(b.to_string()),
        NativeValue::List(items) => PropValue::List(items.iter().map(convert_value).collect()),
        NativeValue::Map(map) => {
            let converted: HashMap<String, PropValue> = map
                .iter()
                .map(|(k, v)| (k.clone(), convert_value(v)))
                .collect();
            PropValue::Map(converted)
        }
    }
}

/// Emit a document to native format.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document to native format with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let content = convert_node_back(&doc.content);
    let mut resources = Vec::new();
    for (id, resource) in &doc.resources {
        resources.push(NativeResource {
            id: id.as_str().to_string(),
            mime_type: resource.mime_type.clone(),
            size: resource.data.len(),
        });
    }

    let mut metadata = BTreeMap::new();
    for (key, value) in doc.metadata.iter() {
        // Convert PropValue to string representation for native metadata
        let value_str = match value {
            PropValue::String(s) => s.clone(),
            PropValue::Int(i) => i.to_string(),
            PropValue::Float(f) => f.to_string(),
            PropValue::Bool(b) => b.to_string(),
            _ => format!("{:?}", value),
        };
        metadata.insert(key.clone(), value_str);
    }

    let native_doc = NativeDoc {
        content,
        metadata,
        resources,
    };

    let output = crate::build(&native_doc);
    Ok(ConversionResult::ok(output.into_bytes()))
}

/// Convert a rescribe node to a native node.
fn convert_node_back(node: &rescribe_core::Node) -> NativeNode {
    let mut native_node = NativeNode {
        kind: node.kind.0.clone(),
        props: BTreeMap::new(),
        children: Vec::new(),
    };

    // Convert properties
    for (key, value) in node.props.iter() {
        let native_value = convert_value_back(value);
        native_node.props.insert(key.clone(), native_value);
    }

    // Convert children
    for child in &node.children {
        native_node.children.push(convert_node_back(child));
    }

    native_node
}

/// Convert a rescribe PropValue to a native value.
fn convert_value_back(value: &PropValue) -> NativeValue {
    match value {
        PropValue::String(s) => NativeValue::String(s.clone()),
        PropValue::Int(i) => NativeValue::Int(*i),
        PropValue::Float(f) => NativeValue::Float(*f),
        PropValue::Bool(b) => NativeValue::Bool(*b),
        PropValue::List(items) => NativeValue::List(items.iter().map(convert_value_back).collect()),
        PropValue::Map(map) => {
            let converted: BTreeMap<String, NativeValue> = map
                .iter()
                .map(|(k, v)| (k.clone(), convert_value_back(v)))
                .collect();
            NativeValue::Map(converted)
        }
    }
}

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
        assert_eq!(result.value.content.kind.as_str(), "document");
        assert_eq!(result.value.content.children.len(), 1);
    }

    #[test]
    fn test_parse_with_properties() {
        let input = r#"Document {
  content:
  heading( { level: 2 })
}"#;
        let result = parse(input).unwrap();
        assert_eq!(result.value.content.kind.0.as_str(), "heading");
        assert!(result.value.content.props.contains("level"));
    }

    #[test]
    fn test_emit_basic() {
        use rescribe_std::builder::*;

        let doc = doc(|d| {
            d.heading(1, |h| h.text("Title"))
                .para(|p| p.text("Hello world"))
        });
        let output = String::from_utf8(emit(&doc).unwrap().value).unwrap();
        assert!(output.contains("Document {"));
        assert!(output.contains("heading("));
        assert!(output.contains("paragraph("));
        assert!(output.contains("text("));
    }

    #[test]
    fn test_emit_props() {
        use rescribe_std::builder::*;

        let doc = doc(|d| d.heading(2, |h| h.text("Level 2")));
        let output = String::from_utf8(emit(&doc).unwrap().value).unwrap();
        assert!(output.contains("level: 2"));
    }
}
