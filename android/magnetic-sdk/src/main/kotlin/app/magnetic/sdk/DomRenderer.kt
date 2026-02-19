package app.magnetic.sdk

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp

/**
 * Render a DomNode tree as live Jetpack Compose UI.
 *
 * This is the runtime equivalent of magnetic-render-kotlin's compile-time codegen.
 * It walks the DomNode tree and emits Compose UI elements based on tag names,
 * wiring events to the onAction callback.
 *
 * @param node   The root DomNode to render
 * @param onAction  Callback when a user action fires (click, submit, input)
 * @param onNavigate  Callback for link navigation
 */
@Composable
fun RenderDomNode(
    node: DomNode,
    onAction: (action: String, payload: Map<String, String>) -> Unit,
    onNavigate: (path: String) -> Unit = {},
) {
    // Skip magnetic:head nodes — not relevant for native
    if (node.tag == "magnetic:head") return

    when (node.tag) {
        // ── Text elements ──────────────────────────────────────────
        "h1" -> RenderHeading(node, MaterialTheme.typography.headlineLarge)
        "h2" -> RenderHeading(node, MaterialTheme.typography.headlineMedium)
        "h3" -> RenderHeading(node, MaterialTheme.typography.headlineSmall)
        "h4" -> RenderHeading(node, MaterialTheme.typography.titleLarge)
        "h5" -> RenderHeading(node, MaterialTheme.typography.titleMedium)
        "h6" -> RenderHeading(node, MaterialTheme.typography.titleSmall)
        "p", "span", "label" -> RenderText(node)

        // ── Interactive ────────────────────────────────────────────
        "button" -> RenderButton(node, onAction)
        "input" -> RenderInput(node, onAction)
        "form" -> RenderForm(node, onAction, onNavigate)
        "a" -> RenderLink(node, onAction, onNavigate)

        // ── Layout ─────────────────────────────────────────────────
        "nav" -> RenderHStack(node, onAction, onNavigate)
        "ul", "ol" -> RenderList(node, onAction, onNavigate)
        "li" -> RenderListItem(node, onAction, onNavigate)
        "hr" -> HorizontalDivider(modifier = Modifier.padding(vertical = 8.dp))

        // ── Default: div, section, article, main, etc. ─────────
        else -> RenderContainer(node, onAction, onNavigate)
    }
}

// ════════════════════════════════════════════════════════════════════
// Text Rendering
// ════════════════════════════════════════════════════════════════════

@Composable
private fun RenderHeading(node: DomNode, style: androidx.compose.ui.text.TextStyle) {
    val text = collectText(node)
    if (text.isNotEmpty()) {
        Text(
            text = text,
            style = style,
            modifier = Modifier.padding(vertical = 4.dp),
        )
    }
}

@Composable
private fun RenderText(node: DomNode) {
    val text = collectText(node)
    if (text.isNotEmpty()) {
        Text(text = text)
    }
}

// ════════════════════════════════════════════════════════════════════
// Button
// ════════════════════════════════════════════════════════════════════

@Composable
private fun RenderButton(
    node: DomNode,
    onAction: (String, Map<String, String>) -> Unit,
) {
    val action = node.event("click") ?: return
    val label = collectText(node)
    val isSubmit = node.attr("type") == "submit"

    Button(
        onClick = { onAction(action, emptyMap()) },
        modifier = Modifier.padding(vertical = 4.dp),
    ) {
        Text(label)
    }
}

// ════════════════════════════════════════════════════════════════════
// Input
// ════════════════════════════════════════════════════════════════════

@Composable
private fun RenderInput(
    node: DomNode,
    onAction: (String, Map<String, String>) -> Unit,
) {
    val name = node.attr("name") ?: "input"
    val placeholder = node.attr("placeholder") ?: ""
    val type = node.attr("type") ?: "text"
    val inputAction = node.event("input")
    val value = node.attr("value") ?: ""

    var text by remember(node.key ?: name) { mutableStateOf(value) }

    OutlinedTextField(
        value = text,
        onValueChange = { newValue ->
            text = newValue
            if (inputAction != null) {
                onAction(inputAction, mapOf("value" to newValue))
            }
        },
        placeholder = { Text(placeholder) },
        visualTransformation = if (type == "password") {
            PasswordVisualTransformation()
        } else {
            VisualTransformation.None
        },
        keyboardOptions = KeyboardOptions(
            keyboardType = when (type) {
                "email" -> KeyboardType.Email
                "number" -> KeyboardType.Number
                "tel" -> KeyboardType.Phone
                "password" -> KeyboardType.Password
                else -> KeyboardType.Text
            },
            imeAction = ImeAction.Done,
        ),
        singleLine = type != "textarea",
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
    )
}

// ════════════════════════════════════════════════════════════════════
// Form — collects input values, submits as payload
// ════════════════════════════════════════════════════════════════════

@Composable
private fun RenderForm(
    node: DomNode,
    onAction: (String, Map<String, String>) -> Unit,
    onNavigate: (String) -> Unit,
) {
    val submitAction = node.event("submit") ?: "submit"

    // Collect form field values using a mutable map
    val formValues = remember { mutableStateMapOf<String, String>() }

    // Wrap onAction to intercept input changes and capture form values
    val formOnAction: (String, Map<String, String>) -> Unit = { action, payload ->
        // If this is an input change within the form, capture the value
        val inputName = payload.entries.firstOrNull()
        if (inputName != null && payload.containsKey("value")) {
            // Store under the field name for form submission
            // The action name for inputs within forms is the input event action
        }
        onAction(action, payload)
    }

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
    ) {
        node.children?.forEach { child ->
            if (child.tag == "input") {
                // Render input with form-aware value tracking
                val name = child.attr("name") ?: "input"
                RenderFormInput(child, name, formValues, onAction)
            } else if (child.tag == "button" && child.attr("type") == "submit") {
                // Submit button sends form values as payload
                val label = collectText(child)
                Button(
                    onClick = { onAction(submitAction, formValues.toMap()) },
                    modifier = Modifier.padding(vertical = 4.dp),
                ) {
                    Text(label)
                }
            } else {
                RenderDomNode(child, onAction, onNavigate)
            }
        }
    }
}

@Composable
private fun RenderFormInput(
    node: DomNode,
    fieldName: String,
    formValues: MutableMap<String, String>,
    onAction: (String, Map<String, String>) -> Unit,
) {
    val placeholder = node.attr("placeholder") ?: ""
    val type = node.attr("type") ?: "text"
    val initialValue = node.attr("value") ?: ""

    var text by remember(node.key ?: fieldName) { mutableStateOf(initialValue) }

    // Keep formValues in sync
    LaunchedEffect(text) {
        formValues[fieldName] = text
    }

    OutlinedTextField(
        value = text,
        onValueChange = { newValue ->
            text = newValue
            formValues[fieldName] = newValue
        },
        placeholder = { Text(placeholder) },
        visualTransformation = if (type == "password") {
            PasswordVisualTransformation()
        } else {
            VisualTransformation.None
        },
        keyboardOptions = KeyboardOptions(
            keyboardType = when (type) {
                "email" -> KeyboardType.Email
                "number" -> KeyboardType.Number
                "tel" -> KeyboardType.Phone
                "password" -> KeyboardType.Password
                else -> KeyboardType.Text
            },
            imeAction = ImeAction.Done,
        ),
        singleLine = type != "textarea",
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
    )
}

// ════════════════════════════════════════════════════════════════════
// Link / Anchor
// ════════════════════════════════════════════════════════════════════

@Composable
private fun RenderLink(
    node: DomNode,
    onAction: (String, Map<String, String>) -> Unit,
    onNavigate: (String) -> Unit,
) {
    val href = node.attr("href")
    val clickAction = node.event("click")
    val label = collectText(node)

    TextButton(
        onClick = {
            when {
                clickAction != null -> onAction(clickAction, emptyMap())
                href != null -> onNavigate(href)
            }
        },
    ) {
        Text(label)
    }
}

// ════════════════════════════════════════════════════════════════════
// Layout Containers
// ════════════════════════════════════════════════════════════════════

@Composable
private fun RenderContainer(
    node: DomNode,
    onAction: (String, Map<String, String>) -> Unit,
    onNavigate: (String) -> Unit,
) {
    val clickAction = node.event("click")

    val modifier = Modifier
        .fillMaxWidth()
        .padding(vertical = 2.dp)
        .let { mod ->
            if (clickAction != null) {
                mod.clickable { onAction(clickAction, emptyMap()) }
            } else {
                mod
            }
        }

    if (node.isRowLayout()) {
        Row(
            modifier = modifier,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            RenderNodeContent(node, onAction, onNavigate)
        }
    } else {
        Column(
            modifier = modifier,
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            RenderNodeContent(node, onAction, onNavigate)
        }
    }
}

@Composable
private fun RenderHStack(
    node: DomNode,
    onAction: (String, Map<String, String>) -> Unit,
    onNavigate: (String) -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        RenderNodeContent(node, onAction, onNavigate)
    }
}

@Composable
private fun RenderNodeContent(
    node: DomNode,
    onAction: (String, Map<String, String>) -> Unit,
    onNavigate: (String) -> Unit,
) {
    // Text content
    node.text?.let { text ->
        Text(text)
    }
    // Children
    node.children?.forEach { child ->
        RenderDomNode(child, onAction, onNavigate)
    }
}

// ════════════════════════════════════════════════════════════════════
// Lists
// ════════════════════════════════════════════════════════════════════

@Composable
private fun RenderList(
    node: DomNode,
    onAction: (String, Map<String, String>) -> Unit,
    onNavigate: (String) -> Unit,
) {
    val items = node.children ?: return

    LazyColumn(
        modifier = Modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        itemsIndexed(
            items = items,
            key = { index, item -> item.key ?: index.toString() },
        ) { _, item ->
            RenderDomNode(item, onAction, onNavigate)
        }
    }
}

@Composable
private fun RenderListItem(
    node: DomNode,
    onAction: (String, Map<String, String>) -> Unit,
    onNavigate: (String) -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 2.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        RenderNodeContent(node, onAction, onNavigate)
    }
}

// ════════════════════════════════════════════════════════════════════
// Utilities
// ════════════════════════════════════════════════════════════════════

/** Recursively collect all text content from a node tree. */
private fun collectText(node: DomNode): String {
    val buf = StringBuilder()
    collectTextInner(node, buf)
    return buf.toString()
}

private fun collectTextInner(node: DomNode, buf: StringBuilder) {
    node.text?.let { buf.append(it) }
    node.children?.forEach { collectTextInner(it, buf) }
}
