//! magnetic-render-kotlin — Render Magnetic DomNode trees to Jetpack Compose Kotlin
//!
//! Translates the JSON DOM tree into idiomatic Compose UI code.
//! The output is a complete @Composable function that can be compiled by the
//! Kotlin compiler with the Compose plugin.
//!
//! Mapping strategy:
//!   div          → Column (vertical) / Row (if class contains "row" or "flex-row")
//!   span         → Row (inline)
//!   h1..h6       → Text(..., style = MaterialTheme.typography.headlineX)
//!   p            → Text(...)
//!   button       → Button(onClick = { onAction("action") }) { Text("label") }
//!   input        → OutlinedTextField(value = "", onValueChange = {}, ...)
//!   form         → Column (wraps children, submit → onAction)
//!   a / Link     → TextButton(onClick = { onAction("navigate:href") }) { Text("label") }
//!   img          → AsyncImage(model = src, contentDescription = alt)
//!   nav          → Row (navigation bar)
//!   ul/ol        → LazyColumn
//!   li           → item { Text/Row }

use magnetic_dom::DomNode;

/// Render a DomNode tree to a Jetpack Compose @Composable function.
pub fn render_to_kotlin(node: &DomNode, fn_name: &str) -> String {
    let mut buf = String::with_capacity(4096);

    // File header
    buf.push_str("package com.magnetic.app\n\n");
    buf.push_str("import androidx.compose.foundation.layout.*\n");
    buf.push_str("import androidx.compose.foundation.lazy.LazyColumn\n");
    buf.push_str("import androidx.compose.foundation.lazy.items\n");
    buf.push_str("import androidx.compose.material3.*\n");
    buf.push_str("import androidx.compose.runtime.*\n");
    buf.push_str("import androidx.compose.ui.Alignment\n");
    buf.push_str("import androidx.compose.ui.Modifier\n");
    buf.push_str("import androidx.compose.ui.unit.dp\n\n");

    buf.push_str(&format!("@Composable\nfun {}(onAction: (String) -> Unit) {{\n", fn_name));
    write_kotlin_node(node, &mut buf, 1);
    buf.push_str("}\n");

    buf
}

fn indent(buf: &mut String, depth: usize) {
    for _ in 0..depth {
        buf.push_str("    ");
    }
}

fn write_kotlin_node(node: &DomNode, buf: &mut String, depth: usize) {
    match node.tag.as_str() {
        // Skip magnetic:head nodes (not relevant for native)
        "magnetic:head" => return,

        // Headings → Text with typography style
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
            let style = match node.tag.as_str() {
                "h1" => "headlineLarge",
                "h2" => "headlineMedium",
                "h3" => "headlineSmall",
                "h4" => "titleLarge",
                "h5" => "titleMedium",
                _ => "titleSmall",
            };
            if let Some(text) = collect_text(node) {
                indent(buf, depth);
                buf.push_str(&format!(
                    "Text(\"{}\", style = MaterialTheme.typography.{})\n",
                    escape_kotlin(&text), style
                ));
            }
        }

        // Paragraph → Text
        "p" | "span" | "label" => {
            if let Some(text) = collect_text(node) {
                indent(buf, depth);
                buf.push_str(&format!("Text(\"{}\")\n", escape_kotlin(&text)));
            }
        }

        // Button → Button composable
        "button" => {
            let action = node.event("click").unwrap_or("noop");
            let label = collect_text(node).unwrap_or_default();
            indent(buf, depth);
            buf.push_str(&format!(
                "Button(onClick = {{ onAction(\"{}\") }}) {{\n",
                escape_kotlin(action)
            ));
            indent(buf, depth + 1);
            buf.push_str(&format!("Text(\"{}\")\n", escape_kotlin(&label)));
            indent(buf, depth);
            buf.push_str("}\n");
        }

        // Input → OutlinedTextField
        "input" => {
            let input_type = node.attrs.as_ref()
                .and_then(|a| a.get("type"))
                .map(|s| s.as_str())
                .unwrap_or("text");
            let placeholder = node.attrs.as_ref()
                .and_then(|a| a.get("placeholder"))
                .map(|s| s.as_str())
                .unwrap_or("");
            let name = node.attrs.as_ref()
                .and_then(|a| a.get("name"))
                .map(|s| s.as_str())
                .unwrap_or("input");
            let action = node.event("input").unwrap_or("");

            indent(buf, depth);
            buf.push_str(&format!(
                "var {name}Value by remember {{ mutableStateOf(\"\") }}\n"
            ));
            indent(buf, depth);
            buf.push_str(&format!(
                "OutlinedTextField(\n"
            ));
            indent(buf, depth + 1);
            buf.push_str(&format!("value = {}Value,\n", name));
            indent(buf, depth + 1);
            buf.push_str(&format!("onValueChange = {{ {}Value = it", name));
            if !action.is_empty() {
                buf.push_str(&format!("; onAction(\"{}\")", escape_kotlin(action)));
            }
            buf.push_str(" },\n");
            indent(buf, depth + 1);
            buf.push_str(&format!("placeholder = {{ Text(\"{}\") }},\n", escape_kotlin(placeholder)));
            indent(buf, depth + 1);
            buf.push_str("modifier = Modifier.fillMaxWidth()\n");
            indent(buf, depth);
            buf.push_str(")\n");
        }

        // Anchor / Link → TextButton with navigate action
        "a" => {
            let action = node.event("click")
                .or_else(|| node.attrs.as_ref()?.get("href").map(|h| h.as_str()))
                .unwrap_or("");
            let label = collect_text(node).unwrap_or_default();
            indent(buf, depth);
            buf.push_str(&format!(
                "TextButton(onClick = {{ onAction(\"{}\") }}) {{\n",
                escape_kotlin(action)
            ));
            indent(buf, depth + 1);
            buf.push_str(&format!("Text(\"{}\")\n", escape_kotlin(&label)));
            indent(buf, depth);
            buf.push_str("}\n");
        }

        // Form → Column with submit handler
        "form" => {
            let action = node.event("submit").unwrap_or("submit");
            indent(buf, depth);
            buf.push_str("Column(\n");
            indent(buf, depth + 1);
            buf.push_str("modifier = Modifier.fillMaxWidth()\n");
            indent(buf, depth);
            buf.push_str(") {\n");
            for child in node.children_iter() {
                write_kotlin_node(child, buf, depth + 1);
            }
            // Submit button if form has action
            indent(buf, depth + 1);
            buf.push_str(&format!(
                "// Form submit: onAction(\"{}\")\n",
                escape_kotlin(action)
            ));
            indent(buf, depth);
            buf.push_str("}\n");
        }

        // Nav → Row
        "nav" => {
            indent(buf, depth);
            buf.push_str("Row(\n");
            indent(buf, depth + 1);
            buf.push_str("horizontalArrangement = Arrangement.spacedBy(8.dp),\n");
            indent(buf, depth + 1);
            buf.push_str("modifier = Modifier.fillMaxWidth()\n");
            indent(buf, depth);
            buf.push_str(") {\n");
            for child in node.children_iter() {
                write_kotlin_node(child, buf, depth + 1);
            }
            indent(buf, depth);
            buf.push_str("}\n");
        }

        // Lists
        "ul" | "ol" => {
            indent(buf, depth);
            buf.push_str("LazyColumn {\n");
            for (i, child) in node.children_iter().iter().enumerate() {
                indent(buf, depth + 1);
                buf.push_str(&format!("item(key = \"{}\") {{\n", child.key.as_deref().unwrap_or(&i.to_string())));
                write_kotlin_node(child, buf, depth + 2);
                indent(buf, depth + 1);
                buf.push_str("}\n");
            }
            indent(buf, depth);
            buf.push_str("}\n");
        }

        "li" => {
            // Render children directly
            for child in node.children_iter() {
                write_kotlin_node(child, buf, depth);
            }
            if let Some(text) = &node.text {
                indent(buf, depth);
                buf.push_str(&format!("Text(\"{}\")\n", escape_kotlin(text)));
            }
        }

        // Default: div and everything else → Column or Row
        _ => {
            let is_row = is_row_layout(node);
            let composable = if is_row { "Row" } else { "Column" };

            indent(buf, depth);
            buf.push_str(&format!("{}(\n", composable));

            // Modifiers from CSS class
            indent(buf, depth + 1);
            if is_row {
                buf.push_str("horizontalArrangement = Arrangement.spacedBy(8.dp),\n");
                indent(buf, depth + 1);
                buf.push_str("verticalAlignment = Alignment.CenterVertically,\n");
            } else {
                buf.push_str("verticalArrangement = Arrangement.spacedBy(8.dp),\n");
            }
            indent(buf, depth + 1);
            buf.push_str("modifier = Modifier.fillMaxWidth()\n");
            indent(buf, depth);
            buf.push_str(") {\n");

            // Text content
            if let Some(text) = &node.text {
                indent(buf, depth + 1);
                buf.push_str(&format!("Text(\"{}\")\n", escape_kotlin(text)));
            }

            // Children
            for child in node.children_iter() {
                write_kotlin_node(child, buf, depth + 1);
            }

            indent(buf, depth);
            buf.push_str("}\n");
        }
    }
}

/// Check if a node should be rendered as a Row (horizontal) vs Column (vertical)
fn is_row_layout(node: &DomNode) -> bool {
    if let Some(class) = node.class() {
        return class.contains("row")
            || class.contains("flex-row")
            || class.contains("topnav")
            || class.contains("add-form")
            || class.contains("filters")
            || class.contains("task-card");
    }
    // Elements that are typically horizontal
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

fn escape_kotlin(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('$', "\\$")
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
            events: Some(HashMap::from([("click".into(), "increment".into())])),
            text: Some("+".into()),
            children: None,
        };
        let kt = render_to_kotlin(&node, "TestScreen");
        assert!(kt.contains("@Composable"));
        assert!(kt.contains("onAction(\"increment\")"));
        assert!(kt.contains("Text(\"+\")"));
    }
}
