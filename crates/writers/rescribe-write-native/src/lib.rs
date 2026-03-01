//! Native format writer for rescribe.
//!
//! Thin adapter layer that uses the `native` crate to build
//! native format strings from rescribe Documents.

use native::{NativeNode, NativeResource, NativeValue};
use rescribe_core::{ConversionResult, Document, EmitError, EmitOptions, PropValue};
use std::collections::BTreeMap;

/// Emit a document to native format.
pub fn emit(doc: &Document) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    emit_with_options(doc, &EmitOptions::default())
}

/// Emit a document to native format with options.
pub fn emit_with_options(
    doc: &Document,
    _options: &EmitOptions,
) -> Result<ConversionResult<Vec<u8>>, EmitError> {
    let content = convert_node(&doc.content);
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

    let native_doc = native::NativeDoc {
        content,
        metadata,
        resources,
    };

    let output = native::build(&native_doc);
    Ok(ConversionResult::ok(output.into_bytes()))
}

/// Convert a rescribe node to a native node.
fn convert_node(node: &rescribe_core::Node) -> NativeNode {
    let mut native_node = NativeNode {
        kind: node.kind.0.clone(),
        props: BTreeMap::new(),
        children: Vec::new(),
    };

    // Convert properties
    for (key, value) in node.props.iter() {
        let native_value = convert_value(value);
        native_node.props.insert(key.clone(), native_value);
    }

    // Convert children
    for child in &node.children {
        native_node.children.push(convert_node(child));
    }

    native_node
}

/// Convert a rescribe PropValue to a native value.
fn convert_value(value: &PropValue) -> NativeValue {
    match value {
        PropValue::String(s) => NativeValue::String(s.clone()),
        PropValue::Int(i) => NativeValue::Int(*i),
        PropValue::Float(f) => NativeValue::Float(*f),
        PropValue::Bool(b) => NativeValue::Bool(*b),
        PropValue::List(items) => NativeValue::List(items.iter().map(convert_value).collect()),
        PropValue::Map(map) => {
            let converted: BTreeMap<String, NativeValue> = map
                .iter()
                .map(|(k, v)| (k.clone(), convert_value(v)))
                .collect();
            NativeValue::Map(converted)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rescribe_std::builder::*;

    fn emit_str(doc: &Document) -> String {
        String::from_utf8(emit(doc).unwrap().value).unwrap()
    }

    #[test]
    fn test_emit_basic() {
        let doc = doc(|d| {
            d.heading(1, |h| h.text("Title"))
                .para(|p| p.text("Hello world"))
        });
        let output = emit_str(&doc);
        assert!(output.contains("Document {"));
        assert!(output.contains("heading("));
        assert!(output.contains("paragraph("));
        assert!(output.contains("text("));
    }

    #[test]
    fn test_emit_props() {
        let doc = doc(|d| d.heading(2, |h| h.text("Level 2")));
        let output = emit_str(&doc);
        assert!(output.contains("level: 2"));
    }
}
