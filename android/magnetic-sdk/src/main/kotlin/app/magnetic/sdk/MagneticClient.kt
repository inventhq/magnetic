package app.magnetic.sdk

import kotlinx.coroutines.*
import kotlinx.coroutines.flow.*
import kotlinx.serialization.json.jsonObject
import okhttp3.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.sse.EventSource
import okhttp3.sse.EventSourceListener
import okhttp3.sse.EventSources
import java.io.IOException
import java.util.concurrent.TimeUnit

/**
 * Connection state for the Magnetic SSE link.
 */
enum class ConnectionState {
    CONNECTING,
    CONNECTED,
    DISCONNECTED,
    ERROR,
}

/**
 * Magnetic server client.
 *
 * Connects to a Magnetic server via SSE, receives DOM snapshots,
 * and sends user actions via POST.
 *
 * Usage:
 *   val client = MagneticClient("https://my-app.magnetic.app")
 *   client.dom.collect { domNode -> render(domNode) }
 *   client.sendAction("increment")
 */
class MagneticClient(
    private val serverUrl: String,
    private val scope: CoroutineScope = CoroutineScope(Dispatchers.IO + SupervisorJob()),
) {
    private val httpClient = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .readTimeout(0, TimeUnit.SECONDS)     // SSE = infinite read
        .writeTimeout(10, TimeUnit.SECONDS)
        .retryOnConnectionFailure(true)
        .build()

    private val _dom = MutableStateFlow<DomNode?>(null)
    /** Latest DOM snapshot from the server. Null until first snapshot arrives. */
    val dom: StateFlow<DomNode?> = _dom.asStateFlow()

    private val _connectionState = MutableStateFlow(ConnectionState.DISCONNECTED)
    /** Current SSE connection state. */
    val connectionState: StateFlow<ConnectionState> = _connectionState.asStateFlow()

    private val _error = MutableSharedFlow<String>(extraBufferCapacity = 8)
    /** Error messages (connection failures, action errors, etc.) */
    val errors: SharedFlow<String> = _error.asSharedFlow()

    private var eventSource: EventSource? = null
    private var reconnectJob: Job? = null

    private val baseUrl = serverUrl.trimEnd('/')

    // ── SSE Connection ─────────────────────────────────────────────

    /** Connect to the server's SSE endpoint and start receiving DOM snapshots. */
    fun connect() {
        if (_connectionState.value == ConnectionState.CONNECTING ||
            _connectionState.value == ConnectionState.CONNECTED
        ) return

        _connectionState.value = ConnectionState.CONNECTING

        val request = Request.Builder()
            .url("$baseUrl/sse")
            .header("Accept", "text/event-stream")
            .build()

        val factory = EventSources.createFactory(httpClient)
        eventSource = factory.newEventSource(request, object : EventSourceListener() {

            override fun onOpen(eventSource: EventSource, response: Response) {
                _connectionState.value = ConnectionState.CONNECTED
                reconnectJob?.cancel()
            }

            override fun onEvent(
                eventSource: EventSource,
                id: String?,
                type: String?,
                data: String,
            ) {
                when (type) {
                    // The server sends "event: message\ndata: {\"root\":...}"
                    "message", null -> {
                        try {
                            // Server wraps in {"root": ...} — extract the root node
                            val snapshot = magneticJson
                                .parseToJsonElement(data)
                            val rootElement = snapshot
                                .jsonObject["root"]
                                ?: throw IllegalStateException("Missing 'root' in snapshot")
                            val node = magneticJson.decodeFromJsonElement(
                                DomNode.serializer(), rootElement
                            )
                            _dom.value = node
                        } catch (e: Exception) {
                            scope.launch { _error.emit("Parse error: ${e.message}") }
                        }
                    }
                    "navigate" -> {
                        // Server-initiated navigation — request the new page
                        scope.launch { navigate(data) }
                    }
                }
            }

            override fun onClosed(eventSource: EventSource) {
                _connectionState.value = ConnectionState.DISCONNECTED
                scheduleReconnect()
            }

            override fun onFailure(
                eventSource: EventSource,
                t: Throwable?,
                response: Response?,
            ) {
                _connectionState.value = ConnectionState.ERROR
                scope.launch {
                    _error.emit("SSE error: ${t?.message ?: response?.message ?: "unknown"}")
                }
                scheduleReconnect()
            }
        })
    }

    /** Disconnect from the server. */
    fun disconnect() {
        reconnectJob?.cancel()
        eventSource?.cancel()
        eventSource = null
        _connectionState.value = ConnectionState.DISCONNECTED
    }

    private fun scheduleReconnect() {
        reconnectJob?.cancel()
        reconnectJob = scope.launch {
            delay(2000) // Wait 2s before reconnecting
            connect()
        }
    }

    // ── Actions ────────────────────────────────────────────────────

    /**
     * Send a user action to the server.
     *
     * Server endpoint: POST /actions/{action_name}
     * Body: JSON payload (form values, etc.) or empty object
     *
     * @param action The action name (e.g., "increment", "delete_42")
     * @param payload Optional key-value payload (e.g., form field values)
     */
    fun sendAction(action: String, payload: Map<String, String> = emptyMap()) {
        scope.launch {
            try {
                val jsonBody = if (payload.isEmpty()) {
                    "{}"
                } else {
                    buildString {
                        append("{")
                        payload.entries.forEachIndexed { i, (k, v) ->
                            if (i > 0) append(",")
                            append("\"")
                            append(k.replace("\"", "\\\""))
                            append("\":\"")
                            append(v.replace("\"", "\\\""))
                            append("\"")
                        }
                        append("}")
                    }
                }

                val encoded = action.replace(" ", "%20")
                val body = jsonBody.toRequestBody("application/json".toMediaType())
                val request = Request.Builder()
                    .url("$baseUrl/actions/$encoded")
                    .post(body)
                    .build()

                httpClient.newCall(request).execute().use { response ->
                    if (!response.isSuccessful) {
                        _error.emit("Action failed (${response.code}): ${response.body?.string()}")
                    }
                }
            } catch (e: IOException) {
                _error.emit("Action error: ${e.message}")
            }
        }
    }

    /**
     * Request a page navigation from the server.
     *
     * Server endpoint: POST /actions/navigate
     * Body: { "path": "/about" }
     */
    fun navigate(path: String) {
        scope.launch {
            try {
                val body = "{\"path\":\"${path.replace("\"", "\\\"")}\"}"
                    .toRequestBody("application/json".toMediaType())
                val request = Request.Builder()
                    .url("$baseUrl/actions/navigate")
                    .post(body)
                    .build()

                httpClient.newCall(request).execute().use { response ->
                    if (!response.isSuccessful) {
                        _error.emit("Navigate failed (${response.code})")
                    }
                }
            } catch (e: IOException) {
                _error.emit("Navigate error: ${e.message}")
            }
        }
    }

    /** Clean up resources. Call when the hosting Activity/Fragment is destroyed. */
    fun destroy() {
        disconnect()
        scope.cancel()
    }
}
