//! Resource management for embedded content (images, fonts, etc.).

use crate::Properties;
use std::collections::HashMap;

/// Unique identifier for an embedded resource.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct ResourceId(String);

/// Map of resource IDs to resources.
pub type ResourceMap = HashMap<ResourceId, Resource>;

/// An embedded resource (image, font, data file, etc.).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Resource {
    /// Original filename or identifier.
    pub name: Option<String>,
    /// MIME type.
    pub mime_type: String,
    /// Raw data.
    ///
    /// Per ADR 0010, serializes as a base64 string — an acknowledged performance/
    /// memory compromise (see the ADR for the tradeoff and reopening condition).
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_data_base64"))]
    pub data: Vec<u8>,
    /// Resource metadata.
    pub metadata: Properties,
}

#[cfg(feature = "serde")]
fn serialize_data_base64<S: serde::Serializer>(
    data: &[u8],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use base64::Engine as _;
    serializer.serialize_str(&base64::engine::general_purpose::STANDARD.encode(data))
}

impl ResourceId {
    /// Generate a new unique resource ID.
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(format!("res_{id}"))
    }

    /// Create a resource ID from a string.
    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Get the ID as a string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ResourceId {
    fn default() -> Self {
        Self::new()
    }
}

impl Resource {
    /// Create a new resource.
    pub fn new(mime_type: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            name: None,
            mime_type: mime_type.into(),
            data,
            metadata: Properties::new(),
        }
    }

    /// Set the resource name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Create an image resource.
    pub fn image(mime_type: impl Into<String>, data: Vec<u8>) -> Self {
        Self::new(mime_type, data)
    }

    /// Create a PNG image resource.
    pub fn png(data: Vec<u8>) -> Self {
        Self::new("image/png", data)
    }

    /// Create a JPEG image resource.
    pub fn jpeg(data: Vec<u8>) -> Self {
        Self::new("image/jpeg", data)
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;

    #[test]
    fn data_serializes_as_base64_string() {
        let res = Resource::png(vec![0x89, 0x50, 0x4e, 0x47]);
        let json = serde_json::to_value(&res).unwrap();
        // "iVBORw==" is the base64 encoding of [0x89, 0x50, 0x4e, 0x47].
        assert_eq!(json["data"], serde_json::json!("iVBORw=="));
        assert_eq!(json["mime_type"], serde_json::json!("image/png"));
    }

    #[test]
    fn resource_id_serializes_transparently_as_plain_string() {
        let id = ResourceId::from_string("res_0");
        assert_eq!(
            serde_json::to_value(&id).unwrap(),
            serde_json::json!("res_0")
        );
    }

    #[test]
    fn resource_map_uses_resource_id_as_plain_string_key() {
        let mut map: ResourceMap = ResourceMap::new();
        map.insert(
            ResourceId::from_string("res_0"),
            Resource::png(vec![1, 2, 3]),
        );
        let json = serde_json::to_value(&map).unwrap();
        assert!(json.get("res_0").is_some());
    }
}
