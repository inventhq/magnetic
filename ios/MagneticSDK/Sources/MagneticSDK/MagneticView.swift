import SwiftUI

/// MagneticView — the single entry point for rendering a Magnetic app in iOS/macOS.
///
/// Connects to a Magnetic server, receives DOM snapshots via SSE,
/// and renders them as native SwiftUI views.
///
/// Usage:
/// ```swift
/// MagneticView(serverURL: "https://my-app.magnetic.app")
/// ```
///
/// - Parameters:
///   - serverURL: Base URL of the Magnetic server
///   - loading: Optional view shown while waiting for first snapshot
///   - error: Optional view shown on connection error
public struct MagneticView<Loading: View, ErrorContent: View>: View {
    @StateObject private var client: MagneticClient

    private let loading: () -> Loading
    private let errorContent: (String) -> ErrorContent

    public init(
        serverURL: String,
        @ViewBuilder loading: @escaping () -> Loading,
        @ViewBuilder error: @escaping (String) -> ErrorContent
    ) {
        _client = StateObject(wrappedValue: MagneticClient(serverURL: serverURL))
        self.loading = loading
        self.errorContent = error
    }

    public var body: some View {
        ZStack(alignment: .top) {
            Group {
                if let dom = client.dom {
                    ScrollView {
                        RenderDomNode(
                            node: dom,
                            onAction: { action, payload in
                                client.sendAction(action, payload: payload)
                            },
                            onNavigate: { path in
                                client.navigate(to: path)
                            }
                        )
                        .padding()
                    }
                } else if client.connectionState == .error {
                    errorContent(client.lastError ?? "Connection failed")
                } else {
                    loading()
                }
            }

            // Connection indicator
            if client.connectionState == .connecting {
                ProgressView()
                    .progressViewStyle(.linear)
                    .frame(maxWidth: .infinity)
            }
        }
        .onAppear { client.connect() }
        .onDisappear { client.disconnect() }
    }
}

// MARK: - Convenience initializer with default loading/error views

extension MagneticView where Loading == DefaultLoadingView, ErrorContent == DefaultErrorView {
    /// Create a MagneticView with default loading and error views.
    ///
    /// ```swift
    /// MagneticView(serverURL: "https://my-app.magnetic.app")
    /// ```
    public init(serverURL: String) {
        self.init(
            serverURL: serverURL,
            loading: { DefaultLoadingView() },
            error: { message in DefaultErrorView(message: message) }
        )
    }
}

// MARK: - Overload with explicit client

/// MagneticView with an explicit client instance.
///
/// Use this when you need direct access to the client for sending actions
/// outside of the view tree.
public struct MagneticClientView<Loading: View, ErrorContent: View>: View {
    @ObservedObject var client: MagneticClient

    private let loading: () -> Loading
    private let errorContent: (String) -> ErrorContent

    public init(
        client: MagneticClient,
        @ViewBuilder loading: @escaping () -> Loading,
        @ViewBuilder error: @escaping (String) -> ErrorContent
    ) {
        self.client = client
        self.loading = loading
        self.errorContent = error
    }

    public var body: some View {
        ZStack(alignment: .top) {
            Group {
                if let dom = client.dom {
                    ScrollView {
                        RenderDomNode(
                            node: dom,
                            onAction: { action, payload in
                                client.sendAction(action, payload: payload)
                            },
                            onNavigate: { path in
                                client.navigate(to: path)
                            }
                        )
                        .padding()
                    }
                } else if client.connectionState == .error {
                    errorContent(client.lastError ?? "Connection failed")
                } else {
                    loading()
                }
            }

            if client.connectionState == .connecting {
                ProgressView()
                    .progressViewStyle(.linear)
                    .frame(maxWidth: .infinity)
            }
        }
    }
}

extension MagneticClientView where Loading == DefaultLoadingView, ErrorContent == DefaultErrorView {
    public init(client: MagneticClient) {
        self.init(
            client: client,
            loading: { DefaultLoadingView() },
            error: { message in DefaultErrorView(message: message) }
        )
    }
}

// MARK: - Default views

public struct DefaultLoadingView: View {
    public var body: some View {
        VStack(spacing: 16) {
            Spacer()
            ProgressView()
            Text("Connecting...")
                .font(.body)
                .foregroundStyle(.secondary)
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

public struct DefaultErrorView: View {
    let message: String

    public var body: some View {
        VStack(spacing: 8) {
            Spacer()
            Text("Connection Error")
                .font(.title3)
                .foregroundStyle(.red)
            Text(message)
                .font(.body)
                .foregroundStyle(.secondary)
            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
