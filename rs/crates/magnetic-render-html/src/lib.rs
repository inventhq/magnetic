//! magnetic-render-html — Render Magnetic DomNode trees to HTML strings
//!
//! Produces SSR-ready HTML with data-key and data-a_ attributes for
//! magnetic.js client hydration.

use magnetic_dom::DomNode;

/// Void elements that must not have closing tags
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input",
    "link", "meta", "param", "source", "track", "wbr",
];

/// Render a DomNode tree to an HTML string.
pub fn render_to_html(node: &DomNode) -> String {
    let mut buf = String::with_capacity(4096);
    write_node(node, &mut buf);
    buf
}

/// Render a full HTML page with SSR content, scripts, and styles.
pub fn render_page(opts: &PageOptions) -> String {
    let body_html = render_to_html(&opts.root);

    // Extract <magnetic:head> nodes for <head> injection
    let mut head_extra = String::new();
    extract_head_html(&opts.root, &mut head_extra);

    // Extract title from <Head> component if present
    let extracted_title = extract_title(&opts.root);

    let mut html = String::with_capacity(body_html.len() + 2048);
    html.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\" />\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n");

    // Use extracted <Head><title> if present, otherwise fall back to opts.title
    if let Some(t) = &extracted_title {
        html.push_str(&format!("<title>{}</title>\n", escape_html(t)));
    } else if let Some(title) = &opts.title {
        html.push_str(&format!("<title>{}</title>\n", escape_html(title)));
    }
    if let Some(desc) = &opts.description {
        html.push_str(&format!("<meta name=\"description\" content=\"{}\" />\n", escape_attr(desc)));
    }

    // Injected head elements from <Head> (excluding <title> since we handled it above)
    let head_no_title = remove_title_from_head_html(&head_extra);
    html.push_str(&head_no_title);

    // Inline CSS
    if let Some(css) = &opts.inline_css {
        html.push_str(&format!("<style>{}</style>", css));
    }

    // Linked stylesheets
    for href in &opts.styles {
        html.push_str(&format!("<link rel=\"stylesheet\" href=\"{}\" />", escape_attr(href)));
    }

    html.push_str("\n</head>\n<body>\n");

    // Mount point with SSR content
    let mount = opts.mount_selector.as_deref().unwrap_or("#app");
    let id = mount.trim_start_matches('#');
    html.push_str(&format!("<div id=\"{}\">{}</div>\n", id, body_html));

    // Scripts
    for src in &opts.scripts {
        html.push_str(&format!("<script src=\"{}\"></script>\n", escape_attr(src)));
    }

    // Inline scripts (e.g. client-side renderers for delta mode)
    for script in &opts.inline_scripts {
        html.push_str("<script>\n");
        html.push_str(script);
        html.push_str("\n</script>\n");
    }

    // Magnetic client bootstrap
    if let Some(sse_url) = &opts.sse_url {
        html.push_str("<script>\n");
        html.push_str(&format!("Magnetic.connect(\"{}\", \"{}\");\n", sse_url, mount));
        if let Some(wasm_url) = &opts.wasm_url {
            html.push_str(&format!("Magnetic.loadWasm(\"{}\");\n", wasm_url));
        }
        html.push_str("</script>\n");
    }

    html.push_str("</body>\n</html>");
    html
}

/// Options for rendering a full HTML page.
pub struct PageOptions {
    pub root: DomNode,
    pub scripts: Vec<String>,
    pub styles: Vec<String>,
    pub inline_css: Option<String>,
    pub sse_url: Option<String>,
    pub mount_selector: Option<String>,
    pub wasm_url: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    /// Inline script blocks injected after external scripts but before SSE bootstrap.
    /// Used for registering client-side renderers for delta mode.
    pub inline_scripts: Vec<String>,
}

fn write_node(node: &DomNode, buf: &mut String) {
    // Skip magnetic:head nodes from body output
    if node.is_head() {
        return;
    }

    let is_void = VOID_ELEMENTS.contains(&node.tag.as_str());

    buf.push('<');
    buf.push_str(&node.tag);

    // data-key attribute
    if let Some(key) = &node.key {
        buf.push_str(" data-key=\"");
        buf.push_str(&escape_attr(key));
        buf.push('"');
    }

    // HTML attributes
    if let Some(attrs) = &node.attrs {
        // Sort for deterministic output
        let mut keys: Vec<&String> = attrs.keys().collect();
        keys.sort();
        for k in keys {
            let v = &attrs[k];
            buf.push(' ');
            buf.push_str(k);
            buf.push_str("=\"");
            buf.push_str(&escape_attr(v));
            buf.push('"');
        }
    }

    // Event attributes → data-a_ prefix
    if let Some(events) = &node.events {
        let mut keys: Vec<&String> = events.keys().collect();
        keys.sort();
        for k in keys {
            let v = &events[k];
            buf.push_str(" data-a_");
            buf.push_str(k);
            buf.push_str("=\"");
            buf.push_str(&escape_attr(v));
            buf.push('"');
        }
    }

    buf.push('>');

    // Text content
    if let Some(text) = &node.text {
        buf.push_str(&escape_html(text));
    }

    // Children
    for child in node.children_iter() {
        write_node(child, buf);
    }

    // Closing tag (skip for void elements)
    if !is_void {
        buf.push_str("</");
        buf.push_str(&node.tag);
        buf.push('>');
    }
}

/// Extract magnetic:head children and render them as HTML
fn extract_head_html(node: &DomNode, buf: &mut String) {
    if node.is_head() {
        for child in node.children_iter() {
            write_node(child, buf);
        }
        return;
    }
    for child in node.children_iter() {
        extract_head_html(child, buf);
    }
}

/// Extract the text content of a <title> node inside <magnetic:head>
fn extract_title(node: &DomNode) -> Option<String> {
    if node.is_head() {
        for child in node.children_iter() {
            if child.tag == "title" {
                return child.text.clone();
            }
        }
        return None;
    }
    for child in node.children_iter() {
        if let Some(t) = extract_title(child) {
            return Some(t);
        }
    }
    None
}

/// Remove <title>...</title> from already-rendered head HTML to avoid duplicates
fn remove_title_from_head_html(html: &str) -> String {
    // Simple approach: strip <title>...</title> tags
    let mut result = html.to_string();
    while let Some(start) = result.find("<title>") {
        if let Some(end) = result[start..].find("</title>") {
            result = format!("{}{}", &result[..start], &result[start + end + 8..]);
        } else {
            break;
        }
    }
    result
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use magnetic_dom::DomNode;
    use std::collections::HashMap;

    #[test]
    fn test_simple_render() {
        let node = DomNode {
            tag: "div".into(),
            key: Some("app".into()),
            attrs: Some(HashMap::from([("class".into(), "container".into())])),
            events: None,
            text: None,
            children: Some(vec![
                DomNode::text("h1", "Hello"),
                DomNode {
                    tag: "button".into(),
                    key: Some("btn".into()),
                    attrs: None,
                    events: Some(HashMap::from([("click".into(), "increment".into())])),
                    text: Some("+".into()),
                    children: None,
                },
            ]),
        };

        let html = render_to_html(&node);
        assert!(html.contains("data-key=\"app\""));
        assert!(html.contains("class=\"container\""));
        assert!(html.contains("data-a_click=\"increment\""));
        assert!(html.contains("<h1>Hello</h1>"));
    }

    #[test]
    fn test_void_element() {
        let node = DomNode {
            tag: "input".into(),
            key: None,
            attrs: Some(HashMap::from([("type".into(), "text".into())])),
            events: None,
            text: None,
            children: None,
        };
        let html = render_to_html(&node);
        assert!(html.contains("<input"));
        assert!(!html.contains("</input>"));
    }
}
