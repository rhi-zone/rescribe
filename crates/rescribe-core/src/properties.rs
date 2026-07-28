//! Property system for extensible node attributes.

use std::collections::HashMap;

/// A collection of properties (key-value pairs).
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Properties(HashMap<String, PropValue>);

/// A property value.
#[derive(Debug, Clone, PartialEq)]
pub enum PropValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    List(Vec<PropValue>),
    Map(HashMap<String, PropValue>),
}

// `PropValue::Float` needs a hand-written `Serialize` impl rather than a derive:
// `f64::NAN`/`INFINITY`/`NEG_INFINITY` have no JSON representation, and serde_json's
// default float serialization errors (or silently emits `null`, depending on version/
// features) for non-finite values. Per ADR 0009, non-finite floats serialize as a
// string sentinel (`"NaN"`, `"Infinity"`, `"-Infinity"`) — an acknowledged compromise,
// not a lossless representation; see the ADR for the tradeoff and reopening condition.
#[cfg(feature = "serde")]
impl serde::Serialize for PropValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            PropValue::String(s) => serializer.serialize_str(s),
            PropValue::Int(i) => serializer.serialize_i64(*i),
            PropValue::Float(f) => {
                if f.is_finite() {
                    serializer.serialize_f64(*f)
                } else if f.is_nan() {
                    serializer.serialize_str("NaN")
                } else if f.is_sign_positive() {
                    serializer.serialize_str("Infinity")
                } else {
                    serializer.serialize_str("-Infinity")
                }
            }
            PropValue::Bool(b) => serializer.serialize_bool(*b),
            PropValue::List(items) => items.serialize(serializer),
            PropValue::Map(map) => map.serialize(serializer),
        }
    }
}

impl Properties {
    /// Create an empty property set.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Set a property.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<PropValue>) {
        self.0.insert(key.into(), value.into());
    }

    /// Get a property.
    pub fn get(&self, key: &str) -> Option<&PropValue> {
        self.0.get(key)
    }

    /// Get a string property.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        match self.0.get(key) {
            Some(PropValue::String(s)) => Some(s),
            _ => None,
        }
    }

    /// Get an integer property.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.0.get(key) {
            Some(PropValue::Int(i)) => Some(*i),
            _ => None,
        }
    }

    /// Get a boolean property.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.0.get(key) {
            Some(PropValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    /// Check if a property exists.
    pub fn contains(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Remove a property and return its value.
    pub fn remove(&mut self, key: &str) -> Option<PropValue> {
        self.0.remove(key)
    }

    /// Iterate over properties.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &PropValue)> {
        self.0.iter()
    }

    /// Check if the property set is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the number of properties.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

// Conversions
impl From<String> for PropValue {
    fn from(s: String) -> Self {
        PropValue::String(s)
    }
}

impl From<&str> for PropValue {
    fn from(s: &str) -> Self {
        PropValue::String(s.to_string())
    }
}

impl From<i64> for PropValue {
    fn from(i: i64) -> Self {
        PropValue::Int(i)
    }
}

impl From<i32> for PropValue {
    fn from(i: i32) -> Self {
        PropValue::Int(i as i64)
    }
}

impl From<f64> for PropValue {
    fn from(f: f64) -> Self {
        PropValue::Float(f)
    }
}

impl From<bool> for PropValue {
    fn from(b: bool) -> Self {
        PropValue::Bool(b)
    }
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;

    #[test]
    fn finite_float_serializes_as_json_number() {
        let v = PropValue::Float(1.5);
        assert_eq!(serde_json::to_value(&v).unwrap(), serde_json::json!(1.5));
    }

    #[test]
    fn nan_serializes_as_string_sentinel() {
        let v = PropValue::Float(f64::NAN);
        assert_eq!(serde_json::to_value(&v).unwrap(), serde_json::json!("NaN"));
    }

    #[test]
    fn infinity_serializes_as_string_sentinel() {
        assert_eq!(
            serde_json::to_value(PropValue::Float(f64::INFINITY)).unwrap(),
            serde_json::json!("Infinity")
        );
        assert_eq!(
            serde_json::to_value(PropValue::Float(f64::NEG_INFINITY)).unwrap(),
            serde_json::json!("-Infinity")
        );
    }

    #[test]
    fn properties_serialize_transparently_as_json_object() {
        let mut props = Properties::new();
        props.set("title", "Hello");
        props.set("count", 3i64);
        let json = serde_json::to_value(&props).unwrap();
        assert_eq!(json, serde_json::json!({"title": "Hello", "count": 3}));
    }

    #[test]
    fn nested_list_and_map_serialize() {
        let v = PropValue::List(vec![PropValue::Int(1), PropValue::String("a".into())]);
        assert_eq!(
            serde_json::to_value(&v).unwrap(),
            serde_json::json!([1, "a"])
        );

        let mut m = HashMap::new();
        m.insert("k".to_string(), PropValue::Bool(true));
        let v = PropValue::Map(m);
        assert_eq!(
            serde_json::to_value(&v).unwrap(),
            serde_json::json!({"k": true})
        );
    }
}
