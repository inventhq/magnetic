package app.magnetic.sdk

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle

/**
 * MagneticView — the single entry point for rendering a Magnetic app in Android.
 *
 * Connects to a Magnetic server, receives DOM snapshots via SSE,
 * and renders them as native Jetpack Compose UI.
 *
 * Usage:
 * ```kotlin
 * MagneticView(
 *     serverUrl = "https://my-app.magnetic.app",
 *     modifier = Modifier.fillMaxSize()
 * )
 * ```
 *
 * @param serverUrl  Base URL of the Magnetic server (e.g., "http://10.0.2.2:3003" for emulator)
 * @param modifier   Compose modifier for the root container
 * @param loading    Optional composable shown while waiting for first snapshot
 * @param error      Optional composable shown on connection error
 */
@Composable
fun MagneticView(
    serverUrl: String,
    modifier: Modifier = Modifier,
    loading: @Composable () -> Unit = { DefaultLoading() },
    error: @Composable (message: String) -> Unit = { DefaultError(it) },
) {
    val client = remember(serverUrl) { MagneticClient(serverUrl) }

    // Connect on first composition, disconnect on disposal
    DisposableEffect(client) {
        client.connect()
        onDispose { client.destroy() }
    }

    val dom by client.dom.collectAsStateWithLifecycle()
    val state by client.connectionState.collectAsStateWithLifecycle()

    // Collect errors
    var lastError by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(client) {
        client.errors.collect { lastError = it }
    }

    Box(modifier = modifier) {
        when {
            dom != null -> {
                RenderDomNode(
                    node = dom!!,
                    onAction = { action, payload -> client.sendAction(action, payload) },
                    onNavigate = { path -> client.navigate(path) },
                )
            }
            state == ConnectionState.ERROR -> {
                error(lastError ?: "Connection failed")
            }
            state == ConnectionState.CONNECTING || state == ConnectionState.CONNECTED -> {
                loading()
            }
            else -> {
                error("Disconnected")
            }
        }

        // Connection indicator
        if (state == ConnectionState.CONNECTING) {
            LinearProgressIndicator(
                modifier = Modifier
                    .fillMaxWidth()
                    .align(Alignment.TopCenter),
            )
        }
    }
}

/**
 * MagneticView with an explicit client instance.
 *
 * Use this when you need direct access to the client for sending actions
 * outside of the UI tree (e.g., from a ViewModel).
 */
@Composable
fun MagneticView(
    client: MagneticClient,
    modifier: Modifier = Modifier,
    loading: @Composable () -> Unit = { DefaultLoading() },
    error: @Composable (message: String) -> Unit = { DefaultError(it) },
) {
    val dom by client.dom.collectAsStateWithLifecycle()
    val state by client.connectionState.collectAsStateWithLifecycle()

    var lastError by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(client) {
        client.errors.collect { lastError = it }
    }

    Box(modifier = modifier) {
        when {
            dom != null -> {
                RenderDomNode(
                    node = dom!!,
                    onAction = { action, payload -> client.sendAction(action, payload) },
                    onNavigate = { path -> client.navigate(path) },
                )
            }
            state == ConnectionState.ERROR -> {
                error(lastError ?: "Connection failed")
            }
            state == ConnectionState.CONNECTING || state == ConnectionState.CONNECTED -> {
                loading()
            }
            else -> {
                error("Disconnected")
            }
        }

        if (state == ConnectionState.CONNECTING) {
            LinearProgressIndicator(
                modifier = Modifier
                    .fillMaxWidth()
                    .align(Alignment.TopCenter),
            )
        }
    }
}

// ── Default UI ─────────────────────────────────────────────────────

@Composable
private fun DefaultLoading() {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            CircularProgressIndicator()
            Spacer(modifier = Modifier.height(16.dp))
            Text(
                text = "Connecting...",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun DefaultError(message: String) {
    Box(
        modifier = Modifier.fillMaxSize(),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text(
                text = "Connection Error",
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.error,
            )
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = message,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}
