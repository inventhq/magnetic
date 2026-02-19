import Foundation

/// A single node in the Magnetic DOM tree.
///
/// Mirrors the server's JSON format exactly:
/// `{ tag, key?, attrs?, events?, text?, children? }`
public struct DomNode: Codable, Identifiable, Equatable {
    public let tag: String
    public let key: String?
    public let attrs: [String: String]?
    public let events: [String: String]?
    public let text: String?
    public let children: [DomNode]?

    /// Stable identity for SwiftUI — uses `key` if present, falls back to tag + index hash.
    public var id: String { key ?? tag }

    /// Get an event action by event name (e.g., "click" → "increment").
    public func event(_ name: String) -> String? {
        events?[name]
    }

    /// Get the CSS class attribute.
    public var cssClass: String? {
        attrs?["class"]
    }

    /// Get an attribute by name.
    public func attr(_ name: String) -> String? {
        attrs?[name]
    }

    /// Whether this node should render as a horizontal layout.
    public var isRowLayout: Bool {
        if let cls = cssClass {
            return cls.contains("row") ||
                   cls.contains("flex-row") ||
                   cls.contains("inline") ||
                   cls.contains("horizontal")
        }
        return tag == "nav" || tag == "header"
    }

    /// Children or empty array.
    public var childNodes: [DomNode] { children ?? [] }
}

/// Server SSE snapshot wrapper: `{ "root": { ... } }`
struct Snapshot: Codable {
    let root: DomNode
}

/// JSON decoder configured for Magnetic DOM snapshots.
let magneticDecoder: JSONDecoder = {
    let d = JSONDecoder()
    return d
}()

/// Parse a DomNode from a JSON string.
public func parseDomNode(_ json: String) throws -> DomNode {
    let data = Data(json.utf8)
    return try magneticDecoder.decode(DomNode.self, from: data)
}

/// Parse a Snapshot (root wrapper) from a JSON string.
func parseSnapshot(_ json: String) throws -> Snapshot {
    let data = Data(json.utf8)
    return try magneticDecoder.decode(Snapshot.self, from: data)
}
