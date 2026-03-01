//! Native format reader for rescribe.
//!
//! Thin adapter layer that uses the `native` crate to parse
//! native format strings and convert them to rescribe Documents.

use native::NativeValue;
use rescribe_core::{
    ConversionResult, Document, ParseError, ParseOptions, PropValue, Resource, ResourceId,
};
use rescribe_std::Node;
use std::collections::HashMap;

/// Parse native format input into a document.
pub fn parse(input: &str) -> Result<ConversionResult<Document>, ParseError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parse native format input into a document with options.
pub fn parse_with_options(
    input: &str,
    _options: &ParseOptions,
) -> Result<ConversionResult<Document>, ParseError> {
    let native_doc = native::parse(input)
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

    let mut metadata = rescribe_core::Properties::new();
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
fn convert_node(native_node: &native::NativeNode) -> Node {
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
}
