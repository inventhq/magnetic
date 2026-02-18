//! magnetic-dom — Shared DomNode types for Magnetic renderers
//!
//! This crate defines the canonical Rust representation of the Magnetic JSON DOM
//! snapshot format. All renderers (HTML, Kotlin, SwiftUI) consume this type.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single node in the Magnetic DOM tree.
///
/// Mirrors the JSON schema at contracts/schemas/dom/snapshot.schema.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomNode {
    /// HTML tag name (e.g. "div", "button", "input")
    pub tag: String,

    /// Stable identity for efficient DOM reuse
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,

    /// HTML attributes (class, placeholder, data-*, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attrs: Option<HashMap<String, String>>,

    /// Map of DOM event name → action name (e.g. "click" → "increment")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<HashMap<String, String>>,

    /// Text content for leaf nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Child nodes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<DomNode>>,
}

/// A complete snapshot wrapping the root DomNode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub root: DomNode,
}

impl DomNode {
    /// Create a simple text node
    pub fn text(tag: &str, content: &str) -> Self {
        DomNode {
            tag: tag.to_string(),
            key: None,
            attrs: None,
            events: None,
            text: Some(content.to_string()),
            children: None,
        }
    }

    /// Get a class attribute if present
    pub fn class(&self) -> Option<&str> {
        self.attrs.as_ref()?.get("class").map(|s| s.as_str())
    }

    /// Check if this node is a Magnetic head node (for SSR extraction)
    pub fn is_head(&self) -> bool {
        self.tag == "magnetic:head"
    }

    /// Iterate over children (empty slice if none)
    pub fn children_iter(&self) -> &[DomNode] {
        match &self.children {
            Some(c) => c,
            None => &[],
        }
    }

    /// Get an event action by event name
    pub fn event(&self, name: &str) -> Option<&str> {
        self.events.as_ref()?.get(name).map(|s| s.as_str())
    }
}

/// Parse a snapshot from a JSON string
pub fn parse_snapshot(json: &str) -> Result<Snapshot, serde_json::Error> {
    serde_json::from_str(json)
}

/// Parse a single DomNode from a JSON string
pub fn parse_node(json: &str) -> Result<DomNode, serde_json::Error> {
    serde_json::from_str(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_snapshot() {
        let json = r#"{
            "root": {
                "tag": "div",
                "key": "app",
                "children": [
                    { "tag": "h1", "text": "Count: 0" },
                    { "tag": "button", "events": { "click": "increment" }, "text": "+" }
                ]
            }
        }"#;

        let snap = parse_snapshot(json).unwrap();
        assert_eq!(snap.root.tag, "div");
        assert_eq!(snap.root.key.as_deref(), Some("app"));
        assert_eq!(snap.root.children_iter().len(), 2);
        assert_eq!(snap.root.children_iter()[1].event("click"), Some("increment"));
    }
}
