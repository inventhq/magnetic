//! magnetic-render-swift — Render Magnetic DomNode trees to SwiftUI code
//!
//! Translates the JSON DOM tree into idiomatic SwiftUI views.
//! The output is a complete SwiftUI View struct that can be compiled by Xcode.
//!
//! Mapping strategy:
//!   div          → VStack (vertical) / HStack (if row-like class)
//!   span         → HStack (inline)
//!   h1..h6       → Text("...").font(.largeTitle/.title/.headline/...)
//!   p            → Text("...")
//!   button       → Button("label") { onAction("action") }
//!   input        → TextField("placeholder", text: $binding)
//!   form         → VStack (wraps children, submit → onAction)
//!   a / Link     → Button("label") { onAction("navigate:href") }
//!   img          → AsyncImage(url: URL(string: src))
//!   nav          → HStack (navigation bar)
//!   ul/ol        → List / ForEach
//!   li           → direct children

use magnetic_dom::DomNode;

/// Render a DomNode tree to a SwiftUI View struct.
pub fn render_to_swift(node: &DomNode, struct_name: &str) -> String {
    let mut buf = String::with_capacity(4096);

    // File header
    buf.push_str("import SwiftUI\n\n");

    buf.push_str(&format!("struct {}: View {{\n", struct_name));
    buf.push_str("    var onAction: (String) -> Void\n\n");
    buf.push_str("    var body: some View {\n");
    write_swift_node(node, &mut buf, 2);
    buf.push_str("    }\n");
    buf.push_str("}\n");

    buf
}

fn indent(buf: &mut String, depth: usize) {
    for _ in 0..depth {
        buf.push_str("    ");
    }
}

fn write_swift_node(node: &DomNode, buf: &mut String, depth: usize) {
    match node.tag.as_str() {
        // Skip magnetic:head nodes (not relevant for native)
        "magnetic:head" => return,

        // Headings → Text with font modifier
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let font = match node.tag.as_str() {
                "h1" => ".largeTitle",
                "h2" => ".title",
                "h3" => ".title2",
                "h4" => ".title3",
                "h5" => ".headline",
                _ => ".subheadline",
            };
            if let Some(text) = collect_text(node) {
                indent(buf, depth);
                buf.push_str(&format!(
                    "Text(\"{}\")\n",
                    escape_swift(&text)
                ));
                indent(buf, depth + 1);
                buf.push_str(&format!(".font({})\n", font));
                indent(buf, depth + 1);
                buf.push_str(".fontWeight(.bold)\n");
            }
        }

        // Paragraph / span / label → Text
        "p" | "span" | "label" => {
            if let Some(text) = collect_text(node) {
                indent(buf, depth);
                buf.push_str(&format!("Text(\"{}\")\n", escape_swift(&text)));
            }
        }

        // Button → Button view
        "button" => {
            let action = node.event("click").unwrap_or("noop");
            let label = collect_text(node).unwrap_or_default();
            indent(buf, depth);
            buf.push_str(&format!(
                "Button(\"{}\") {{\n",
                escape_swift(&label)
            ));
            indent(buf, depth + 1);
            buf.push_str(&format!("onAction(\"{}\")\n", escape_swift(action)));
            indent(buf, depth);
            buf.push_str("}\n");
        }

        // Input → TextField
        "input" => {
            let placeholder = node.attrs.as_ref()
                .and_then(|a| a.get("placeholder"))
                .map(|s| s.as_str())
                .unwrap_or("Enter text");
            let name = node.attrs.as_ref()
                .and_then(|a| a.get("name"))
                .map(|s| s.as_str())
                .unwrap_or("input");

            indent(buf, depth);
            buf.push_str(&format!(
                "@State var {name}Text: String = \"\"\n"
            ));
            indent(buf, depth);
            buf.push_str(&format!(
                "TextField(\"{}\", text: ${name}Text)\n",
                escape_swift(placeholder)
            ));
            indent(buf, depth + 1);
            buf.push_str(".textFieldStyle(.roundedBorder)\n");
        }

        // Anchor / Link → Button with navigate action
        "a" => {
            let action = node.event("click")
                .or_else(|| node.attrs.as_ref()?.get("href").map(|h| h.as_str()))
                .unwrap_or("");
            let label = collect_text(node).unwrap_or_default();
            indent(buf, depth);
            buf.push_str(&format!(
                "Button(\"{}\") {{\n",
                escape_swift(&label)
            ));
            indent(buf, depth + 1);
            buf.push_str(&format!("onAction(\"{}\")\n", escape_swift(action)));
            indent(buf, depth);
            buf.push_str("}\n");
            indent(buf, depth);
            buf.push_str(".buttonStyle(.plain)\n");
            indent(buf, depth);
            buf.push_str(".foregroundColor(.accentColor)\n");
        }

        // Form → VStack with submit
        "form" => {
            let action = node.event("submit").unwrap_or("submit");
            indent(buf, depth);
            buf.push_str("VStack(spacing: 12) {\n");
            for child in node.children_iter() {
                write_swift_node(child, buf, depth + 1);
            }
            indent(buf, depth + 1);
            buf.push_str(&format!(
                "// Form submit: onAction(\"{}\")\n",
                escape_swift(action)
            ));
            indent(buf, depth);
            buf.push_str("}\n");
        }

        // Nav → HStack
        "nav" => {
            indent(buf, depth);
            buf.push_str("HStack(spacing: 12) {\n");
            for child in node.children_iter() {
                write_swift_node(child, buf, depth + 1);
            }
            indent(buf, depth);
            buf.push_str("}\n");
        }

        // Lists
        "ul" | "ol" => {
            indent(buf, depth);
            buf.push_str("VStack(alignment: .leading, spacing: 8) {\n");
            for child in node.children_iter() {
                write_swift_node(child, buf, depth + 1);
            }
            indent(buf, depth);
            buf.push_str("}\n");
        }

        "li" => {
            for child in node.children_iter() {
                write_swift_node(child, buf, depth);
            }
            if let Some(text) = &node.text {
                indent(buf, depth);
                buf.push_str(&format!("Text(\"{}\")\n", escape_swift(text)));
            }
        }

        // Default: div and everything else → VStack or HStack
        _ => {
            let is_row = is_row_layout(node);
            let stack = if is_row { "HStack" } else { "VStack" };

            indent(buf, depth);
            buf.push_str(&format!("{}(", stack));
            if is_row {
                buf.push_str("spacing: 8");
            } else {
                buf.push_str("alignment: .leading, spacing: 8");
            }
            buf.push_str(") {\n");

            // Text content
            if let Some(text) = &node.text {
                indent(buf, depth + 1);
                buf.push_str(&format!("Text(\"{}\")\n", escape_swift(text)));
            }

            // Children
            for child in node.children_iter() {
                write_swift_node(child, buf, depth + 1);
            }

            indent(buf, depth);
            buf.push_str("}\n");

            // Apply modifiers based on CSS class
            if let Some(class) = node.class() {
                if class.contains("task-board") || class.contains("about-page") || class.contains("not-found") {
                    indent(buf, depth);
                    buf.push_str(".padding()\n");
                    indent(buf, depth);
                    buf.push_str(".background(Color(.systemBackground))\n");
                    indent(buf, depth);
                    buf.push_str(".cornerRadius(16)\n");
                }
            }
        }
    }
}

/// Check if a node should be rendered as an HStack (horizontal)
fn is_row_layout(node: &DomNode) -> bool {
    if let Some(class) = node.class() {
        return class.contains("row")
            || class.contains("flex-row")
            || class.contains("topnav")
            || class.contains("add-form")
            || class.contains("filters")
            || class.contains("task-card");
    }
    matches!(node.tag.as_str(), "nav" | "header")
}

/// Collect all text content from a node and its children
fn collect_text(node: &DomNode) -> Option<String> {
    let mut text = String::new();
    collect_text_inner(node, &mut text);
    if text.is_empty() { None } else { Some(text) }
}

fn collect_text_inner(node: &DomNode, buf: &mut String) {
    if let Some(t) = &node.text {
        buf.push_str(t);
    }
    for child in node.children_iter() {
        collect_text_inner(child, buf);
    }
}

fn escape_swift(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use magnetic_dom::DomNode;
    use std::collections::HashMap;

    #[test]
    fn test_button_render() {
        let node = DomNode {
            tag: "button".into(),
            key: Some("btn".into()),
            attrs: None,
            events: Some(HashMap::from([("click".into(), "decrement".into())])),
            text: Some("-".into()),
            children: None,
        };
        let swift = render_to_swift(&node, "TestView");
        assert!(swift.contains("struct TestView: View"));
        assert!(swift.contains("onAction(\"decrement\")"));
        assert!(swift.contains("Button(\"-\")"));
    }

    #[test]
    fn test_heading_render() {
        let node = DomNode::text("h1", "Hello World");
        let swift = render_to_swift(&node, "HeadingView");
        assert!(swift.contains("Text(\"Hello World\")"));
        assert!(swift.contains(".font(.largeTitle)"));
    }
}
