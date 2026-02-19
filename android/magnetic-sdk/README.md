# Magnetic Android SDK

Render Magnetic server-driven UI as native Jetpack Compose — one line of code.

## Quick Start

### 1. Add the dependency

```kotlin
// settings.gradle.kts
dependencyResolutionManagement {
    repositories {
        // ... your existing repos
        maven { url = uri("https://jitpack.io") }
    }
}

// app/build.gradle.kts
dependencies {
    implementation("app.magnetic:sdk:0.1.0")
}
```

### 2. Add to your Activity/Screen

```kotlin
import app.magnetic.sdk.MagneticView

@Composable
fun MyScreen() {
    MagneticView(
        serverUrl = "https://my-app.magnetic.app",
        modifier = Modifier.fillMaxSize()
    )
}
```

That's it. The SDK connects to your Magnetic server via SSE, receives DOM snapshots, and renders them as native Compose UI. User interactions (clicks, form submits, text input) are sent back to the server as actions.

## How It Works

```
┌──────────────────────┐              ┌──────────────────────┐
│  Android App         │              │  Magnetic Server     │
│                      │◄── SSE ────│  (Rust + V8)          │
│  MagneticView        │              │  Your TSX pages      │
│  (Compose UI)        │── POST ──►│  Your state.ts         │
│                      │              │                      │
│  DomNode → Compose   │              │  DomNode JSON        │
└──────────────────────┘              └──────────────────────┘
```

The server sends JSON DOM snapshots. The SDK maps them to Compose:

| HTML Tag | Compose Widget |
|----------|---------------|
| `div` | `Column` / `Row` (auto-detected) |
| `h1`–`h6` | `Text` with Material3 typography |
| `p`, `span` | `Text` |
| `button` | `Button` |
| `input` | `OutlinedTextField` |
| `form` | `Column` with submit handling |
| `a` | `TextButton` (navigation) |
| `ul`, `ol` | `LazyColumn` |
| `li` | `Row` |
| `nav` | `Row` |
| `hr` | `HorizontalDivider` |

## Advanced Usage

### Custom loading/error states

```kotlin
MagneticView(
    serverUrl = "https://my-app.magnetic.app",
    loading = { MyCustomLoader() },
    error = { message -> MyCustomError(message) },
)
```

### Direct client access

For sending actions from outside the UI tree (e.g., from a ViewModel):

```kotlin
val client = remember { MagneticClient("https://my-app.magnetic.app") }

DisposableEffect(client) {
    client.connect()
    onDispose { client.destroy() }
}

// Send action programmatically
client.sendAction("refresh")

// Observe DOM
val dom by client.dom.collectAsStateWithLifecycle()

// Render
MagneticView(client = client, modifier = Modifier.fillMaxSize())
```

### Connection state

```kotlin
val state by client.connectionState.collectAsStateWithLifecycle()
// ConnectionState.CONNECTING, CONNECTED, DISCONNECTED, ERROR
```

### Navigation

```kotlin
// Programmatic navigation
client.navigate("/about")
```

Links (`<a href="/about">`) automatically trigger navigation via the `onNavigate` callback.

## Emulator Setup

When running in the Android emulator, `localhost` refers to the emulator itself. Use `10.0.2.2` to reach the host machine:

```kotlin
MagneticView(
    serverUrl = "http://10.0.2.2:3003",
    modifier = Modifier.fillMaxSize()
)
```

For a physical device on the same network, use your Mac's local IP:

```kotlin
MagneticView(
    serverUrl = "http://192.168.1.100:3003",
    modifier = Modifier.fillMaxSize()
)
```

## Server Protocol

The SDK communicates with the Magnetic server using:

- **SSE** (`GET /sse`): Receives `event: message\ndata: {"root": {...}}\n\n` snapshots
- **Actions** (`POST /actions/{action_name}`): Sends `{"key": "value"}` payload
- **Navigate** (`POST /actions/navigate`): Sends `{"path": "/route"}`

These match the exact protocol used by the web client (`magnetic.js`) and the iOS/macOS SDK.

## Requirements

- Android API 26+ (Android 8.0)
- Jetpack Compose with Material3
- Internet permission (added by SDK manifest)

## Dependencies

- **OkHttp 4.12** — HTTP client + SSE
- **kotlinx.serialization 1.6** — JSON parsing
- **kotlinx.coroutines 1.7** — Async/Flow
- **Compose BOM 2024.01** — UI rendering

## Developer writes ZERO native UI code

The same TSX pages and `state.ts` that power the web app are rendered natively on Android. No React Native, no Flutter, no WebView — real Compose UI driven by your Magnetic server.
