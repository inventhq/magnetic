import SwiftUI

/// Render a DomNode tree as live SwiftUI views.
///
/// This is the runtime equivalent of magnetic-render-swift's compile-time codegen.
/// It walks the DomNode tree and emits SwiftUI views based on tag names,
/// wiring events to the onAction callback.
///
/// - Parameters:
///   - node: The DomNode to render
///   - onAction: Callback when a user action fires (click, submit, input)
///   - onNavigate: Callback for link navigation
public struct RenderDomNode: View {
    let node: DomNode
    let onAction: (String, [String: String]) -> Void
    let onNavigate: (String) -> Void

    public init(
        node: DomNode,
        onAction: @escaping (String, [String: String]) -> Void,
        onNavigate: @escaping (String) -> Void = { _ in }
    ) {
        self.node = node
        self.onAction = onAction
        self.onNavigate = onNavigate
    }

    public var body: some View {
        renderNode(node)
    }

    @ViewBuilder
    private func renderNode(_ node: DomNode) -> some View {
        // Skip magnetic:head nodes — not relevant for native
        if node.tag == "magnetic:head" {
            EmptyView()
        } else {
            switch node.tag {
            // ── Text elements ──────────────────────────────────────
            case "h1": headingView(node, font: .largeTitle)
            case "h2": headingView(node, font: .title)
            case "h3": headingView(node, font: .title2)
            case "h4": headingView(node, font: .title3)
            case "h5": headingView(node, font: .headline)
            case "h6": headingView(node, font: .subheadline)
            case "p", "span", "label": textView(node)

            // ── Interactive ────────────────────────────────────────
            case "button": buttonView(node)
            case "input": inputView(node)
            case "form": formView(node)
            case "a": linkView(node)

            // ── Layout ─────────────────────────────────────────────
            case "nav": hStackView(node)
            case "ul", "ol": listView(node)
            case "li": listItemView(node)
            case "hr": Divider().padding(.vertical, 4)

            // ── Default: div, section, article, main, etc. ────────
            default: containerView(node)
            }
        }
    }

    // ════════════════════════════════════════════════════════════════
    // Text
    // ════════════════════════════════════════════════════════════════

    @ViewBuilder
    private func headingView(_ node: DomNode, font: Font) -> some View {
        let text = collectText(node)
        if !text.isEmpty {
            Text(text)
                .font(font)
                .fontWeight(.bold)
                .padding(.vertical, 2)
        }
    }

    @ViewBuilder
    private func textView(_ node: DomNode) -> some View {
        let text = collectText(node)
        if !text.isEmpty {
            Text(text)
        }
    }

    // ════════════════════════════════════════════════════════════════
    // Button
    // ════════════════════════════════════════════════════════════════

    @ViewBuilder
    private func buttonView(_ node: DomNode) -> some View {
        if let action = node.event("click") {
            let label = collectText(node)
            Button(label) {
                onAction(action, [:])
            }
            .padding(.vertical, 4)
        }
    }

    // ════════════════════════════════════════════════════════════════
    // Input
    // ════════════════════════════════════════════════════════════════

    private func inputView(_ node: DomNode) -> some View {
        InputFieldView(
            node: node,
            onAction: onAction
        )
    }

    // ════════════════════════════════════════════════════════════════
    // Form — collects input values, submits as payload
    // ════════════════════════════════════════════════════════════════

    private func formView(_ node: DomNode) -> some View {
        FormContainerView(
            node: node,
            onAction: onAction,
            onNavigate: onNavigate,
            renderNode: { child in
                RenderDomNode(node: child, onAction: onAction, onNavigate: onNavigate)
            }
        )
    }

    // ════════════════════════════════════════════════════════════════
    // Link / Anchor
    // ════════════════════════════════════════════════════════════════

    @ViewBuilder
    private func linkView(_ node: DomNode) -> some View {
        let href = node.attr("href")
        let clickAction = node.event("click")
        let label = collectText(node)

        Button(label) {
            if let action = clickAction {
                onAction(action, [:])
            } else if let href = href {
                onNavigate(href)
            }
        }
        .buttonStyle(.plain)
        .foregroundStyle(.tint)
    }

    // ════════════════════════════════════════════════════════════════
    // Layout Containers
    // ════════════════════════════════════════════════════════════════

    @ViewBuilder
    private func containerView(_ node: DomNode) -> some View {
        let clickAction = node.event("click")

        Group {
            if node.isRowLayout {
                HStack(spacing: 8) {
                    nodeContent(node)
                }
            } else {
                VStack(alignment: .leading, spacing: 4) {
                    nodeContent(node)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
        .onTapGesture {
            if let action = clickAction {
                onAction(action, [:])
            }
        }
    }

    @ViewBuilder
    private func hStackView(_ node: DomNode) -> some View {
        HStack(spacing: 8) {
            nodeContent(node)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, 4)
    }

    @ViewBuilder
    private func nodeContent(_ node: DomNode) -> some View {
        if let text = node.text {
            Text(text)
        }
        ForEach(Array(node.childNodes.enumerated()), id: \.element.id) { _, child in
            renderNode(child)
        }
    }

    // ════════════════════════════════════════════════════════════════
    // Lists
    // ════════════════════════════════════════════════════════════════

    @ViewBuilder
    private func listView(_ node: DomNode) -> some View {
        LazyVStack(alignment: .leading, spacing: 4) {
            ForEach(Array(node.childNodes.enumerated()), id: \.element.id) { _, child in
                renderNode(child)
            }
        }
    }

    @ViewBuilder
    private func listItemView(_ node: DomNode) -> some View {
        HStack(spacing: 8) {
            nodeContent(node)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, 2)
    }

    // ════════════════════════════════════════════════════════════════
    // Utilities
    // ════════════════════════════════════════════════════════════════

    private func collectText(_ node: DomNode) -> String {
        var buf = ""
        collectTextInner(node, buf: &buf)
        return buf
    }

    private func collectTextInner(_ node: DomNode, buf: inout String) {
        if let text = node.text { buf += text }
        for child in node.childNodes { collectTextInner(child, buf: &buf) }
    }
}

// ════════════════════════════════════════════════════════════════════
// Input Field — needs @State, so must be its own View struct
// ════════════════════════════════════════════════════════════════════

struct InputFieldView: View {
    let node: DomNode
    let onAction: (String, [String: String]) -> Void

    @State private var text: String = ""

    private var name: String { node.attr("name") ?? "input" }
    private var placeholder: String { node.attr("placeholder") ?? "" }
    private var inputType: String { node.attr("type") ?? "text" }
    private var inputAction: String? { node.event("input") }

    var body: some View {
        Group {
            if inputType == "password" {
                SecureField(placeholder, text: $text)
            } else {
                TextField(placeholder, text: $text)
                    .keyboardType(keyboardType)
                    .autocorrectionDisabled(inputType == "email" || inputType == "url")
                    .textInputAutocapitalization(
                        inputType == "email" ? .never : .sentences
                    )
            }
        }
        .textFieldStyle(.roundedBorder)
        .padding(.vertical, 4)
        .onChange(of: text) { _, newValue in
            if let action = inputAction {
                onAction(action, ["value": newValue])
            }
        }
        .onAppear {
            text = node.attr("value") ?? ""
        }
    }

    private var keyboardType: UIKeyboardType {
        switch inputType {
        case "email": return .emailAddress
        case "number": return .numberPad
        case "tel": return .phonePad
        case "url": return .URL
        default: return .default
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// Form Container — tracks field values for submit
// ════════════════════════════════════════════════════════════════════

struct FormContainerView<Content: View>: View {
    let node: DomNode
    let onAction: (String, [String: String]) -> Void
    let onNavigate: (String) -> Void
    let renderNode: (DomNode) -> Content

    @State private var formValues: [String: String] = [:]

    private var submitAction: String { node.event("submit") ?? "submit" }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(Array(node.childNodes.enumerated()), id: \.element.id) { _, child in
                if child.tag == "input" {
                    FormInputView(
                        node: child,
                        formValues: $formValues,
                        onAction: onAction
                    )
                } else if child.tag == "button" && child.attr("type") == "submit" {
                    let label = child.text ?? "Submit"
                    Button(label) {
                        onAction(submitAction, formValues)
                    }
                    .padding(.vertical, 4)
                } else {
                    renderNode(child)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, 4)
    }
}

struct FormInputView: View {
    let node: DomNode
    @Binding var formValues: [String: String]
    let onAction: (String, [String: String]) -> Void

    @State private var text: String = ""

    private var name: String { node.attr("name") ?? "input" }
    private var placeholder: String { node.attr("placeholder") ?? "" }
    private var inputType: String { node.attr("type") ?? "text" }

    var body: some View {
        Group {
            if inputType == "password" {
                SecureField(placeholder, text: $text)
            } else {
                TextField(placeholder, text: $text)
            }
        }
        .textFieldStyle(.roundedBorder)
        .padding(.vertical, 4)
        .onChange(of: text) { _, newValue in
            formValues[name] = newValue
        }
        .onAppear {
            text = node.attr("value") ?? ""
            formValues[name] = text
        }
    }
}
