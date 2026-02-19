package app.magnetic.sdk

import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

/**
 * A single node in the Magnetic DOM tree.
 *
 * Mirrors the server's DomNode JSON format exactly:
 * { tag, key?, attrs?, events?, text?, children? }
 */
@Serializable
data class DomNode(
    val tag: String,
    val key: String? = null,
    val attrs: Map<String, String>? = null,
    val events: Map<String, String>? = null,
    val text: String? = null,
    val children: List<DomNode>? = null,
) {
    /** Get an event action by event name (e.g., "click" → "increment") */
    fun event(name: String): String? = events?.get(name)

    /** Get the CSS class attribute */
    fun cssClass(): String? = attrs?.get("class")

    /** Get an attribute by name */
    fun attr(name: String): String? = attrs?.get(name)

    /** Whether this node should render as a horizontal layout */
    fun isRowLayout(): Boolean {
        val cls = cssClass() ?: return tag == "nav" || tag == "header"
        return cls.contains("row") ||
                cls.contains("flex-row") ||
                cls.contains("inline") ||
                cls.contains("horizontal")
    }
}

/**
 * JSON parser configured for Magnetic DOM snapshots.
 * Ignores unknown keys for forward compatibility.
 */
internal val magneticJson = Json {
    ignoreUnknownKeys = true
    isLenient = true
}

/** Parse a DomNode from a JSON string */
fun parseDomNode(json: String): DomNode = magneticJson.decodeFromString(json)
