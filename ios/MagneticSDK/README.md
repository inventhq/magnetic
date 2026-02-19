# Magnetic iOS/macOS SDK

Render Magnetic server-driven UI as native SwiftUI — one line of code.

## Quick Start

### 1. Add the Swift Package

In Xcode: **File → Add Package Dependencies** → enter the repository URL:

```
https://github.com/inventhq/magnetic.git
```

Select the `MagneticSDK` library.

Or in `Package.swift`:

```swift
dependencies: [
    .package(url: "https://github.com/inventhq/magnetic.git", from: "0.1.0"),
],
targets: [
    .target(
        name: "MyApp",
        dependencies: [
            .product(name: "MagneticSDK", package: "magnetic"),
        ]
    ),
]
```

### 2. Add to your App

```swift
import MagneticSDK

struct ContentView: View {
    var body: some View {
        MagneticView(serverURL: "https://my-app.magnetic.app")
    }
}
```

That's it. The SDK connects to your Magnetic server via SSE, receives DOM snapshots, and renders them as native SwiftUI. User interactions (taps, form submits, text input) are sent back to the server as actions.

## How It Works

```
┌──────────────────────┐              ┌──────────────────────┐
│  iOS/macOS App       │              │  Magnetic Server     │
│                      │◄── SSE ────│  (Rust + V8)          │
│  MagneticView        │              │  Your TSX pages      │
│  (SwiftUI)           │── POST ──►│  Your state.ts         │
│                      │              │                      │
│  DomNode → SwiftUI   │              │  DomNode JSON        │
└──────────────────────┘              └──────────────────────┘
```

The server sends JSON DOM snapshots. The SDK maps them to SwiftUI:

| HTML Tag | SwiftUI View |
|----------|-------------|
| `div` | `VStack` / `HStack` (auto-detected) |
| `h1`–`h6` | `Text` with font modifiers (`.largeTitle`, `.title`, etc.) |
| `p`, `span` | `Text` |
| `button` | `Button` |
| `input` | `TextField` / `SecureField` |
| `form` | `VStack` with submit handling |
| `a` | `Button` (plain style, tinted) |
| `ul`, `ol` | `LazyVStack` |
| `li` | `HStack` |
| `nav` | `HStack` |
| `hr` | `Divider` |

## Advanced Usage

### Custom loading/error views

```swift
MagneticView(
    serverURL: "https://my-app.magnetic.app",
    loading: { MyCustomLoader() },
    error: { message in MyCustomError(message: message) }
)
```

### Direct client access

For sending actions from outside the view tree (e.g., from a view model):

```swift
@StateObject var client = MagneticClient(serverURL: "https://my-app.magnetic.app")

var body: some View {
    MagneticClientView(client: client)
        .onAppear { client.connect() }
        .onDisappear { client.disconnect() }
}

// Send action programmatically
client.sendAction("refresh")

// Observe connection state
switch client.connectionState {
case .connecting: // ...
case .connected:  // ...
case .disconnected: // ...
case .error: // check client.lastError
}
```

### Navigation

```swift
// Programmatic navigation
client.navigate(to: "/about")
```

Links (`<a href="/about">`) automatically trigger navigation via the `onNavigate` callback.

## Simulator Setup

When running in the iOS Simulator, `localhost` works directly since the Simulator shares the host network:

```swift
MagneticView(serverURL: "http://localhost:3003")
```

For a physical device on the same network, use your Mac's local IP:

```swift
MagneticView(serverURL: "http://192.168.1.100:3003")
```

## Requirements

- iOS 16+ / macOS 13+
- Swift 5.9+
- SwiftUI

## Server Protocol

The SDK communicates with the Magnetic server using:

- **SSE** (`GET /sse`): Receives `event: message\ndata: {"root": {...}}\n\n` snapshots
- **Actions** (`POST /actions/{action_name}`): Sends `{"key": "value"}` payload
- **Navigate** (`POST /actions/navigate`): Sends `{"path": "/route"}`

These match the exact protocol used by the web client (`magnetic.js`).

## Developer writes ZERO native UI code

The same TSX pages and `state.ts` that power the web app are rendered natively on iOS and macOS. No React Native, no Flutter, no WebView — real SwiftUI driven by your Magnetic server.
