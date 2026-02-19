import Foundation
import Combine

/// Connection state for the Magnetic SSE link.
public enum ConnectionState: String {
    case connecting
    case connected
    case disconnected
    case error
}

/// Magnetic server client.
///
/// Connects to a Magnetic server via SSE, receives DOM snapshots,
/// and sends user actions via POST.
///
/// Usage:
/// ```swift
/// let client = MagneticClient(serverURL: "https://my-app.magnetic.app")
/// client.connect()
/// // Observe client.dom for DomNode updates
/// client.sendAction("increment")
/// ```
@MainActor
public final class MagneticClient: ObservableObject {

    /// Latest DOM snapshot from the server. Nil until first snapshot arrives.
    @Published public private(set) var dom: DomNode?

    /// Current SSE connection state.
    @Published public private(set) var connectionState: ConnectionState = .disconnected

    /// Last error message.
    @Published public private(set) var lastError: String?

    private let baseURL: String
    private var actionSession: URLSession
    private var sseDelegate: SSESessionDelegate?
    private var sseSession: URLSession?
    private var sseTask: URLSessionDataTask?
    private var reconnectTask: Task<Void, Never>?

    public init(serverURL: String) {
        self.baseURL = serverURL.hasSuffix("/")
            ? String(serverURL.dropLast())
            : serverURL

        self.actionSession = URLSession.shared
    }

    // MARK: - SSE Connection

    /// Connect to the server's SSE endpoint and start receiving DOM snapshots.
    public func connect() {
        guard connectionState != .connecting && connectionState != .connected else { return }
        connectionState = .connecting

        guard let url = URL(string: "\(baseURL)/sse") else {
            connectionState = .error
            lastError = "Invalid SSE URL"
            return
        }

        var request = URLRequest(url: url)
        request.setValue("text/event-stream", forHTTPHeaderField: "Accept")
        request.timeoutInterval = .infinity

        let delegate = SSESessionDelegate { [weak self] eventType, data in
            guard let self = self else { return }
            Task { @MainActor in
                self.handleSSEEvent(type: eventType, data: data)
            }
        } onOpen: { [weak self] in
            Task { @MainActor in
                self?.connectionState = .connected
                self?.reconnectTask?.cancel()
            }
        } onError: { [weak self] error in
            Task { @MainActor in
                self?.connectionState = .error
                self?.lastError = "SSE error: \(error)"
                self?.scheduleReconnect()
            }
        } onComplete: { [weak self] in
            Task { @MainActor in
                self?.connectionState = .disconnected
                self?.scheduleReconnect()
            }
        }

        self.sseDelegate = delegate

        let config = URLSessionConfiguration.default
        config.timeoutIntervalForRequest = .infinity
        config.timeoutIntervalForResource = .infinity
        let session = URLSession(configuration: config, delegate: delegate, delegateQueue: nil)
        self.sseSession = session

        let task = session.dataTask(with: request)
        task.resume()
        self.sseTask = task
    }

    /// Disconnect from the server.
    public func disconnect() {
        reconnectTask?.cancel()
        reconnectTask = nil
        sseTask?.cancel()
        sseTask = nil
        sseSession?.invalidateAndCancel()
        sseSession = nil
        sseDelegate = nil
        connectionState = .disconnected
    }

    private func handleSSEEvent(type: String?, data: String) {
        switch type {
        case "message", nil:
            do {
                let snapshot = try parseSnapshot(data)
                self.dom = snapshot.root
            } catch {
                self.lastError = "Parse error: \(error.localizedDescription)"
            }
        case "navigate":
            navigate(to: data)
        default:
            break
        }
    }

    private func scheduleReconnect() {
        reconnectTask?.cancel()
        reconnectTask = Task {
            try? await Task.sleep(nanoseconds: 2_000_000_000) // 2 seconds
            guard !Task.isCancelled else { return }
            connect()
        }
    }

    // MARK: - Actions

    /// Send a user action to the server.
    ///
    /// Server endpoint: `POST /actions/{action_name}`
    ///
    /// - Parameters:
    ///   - action: The action name (e.g., "increment", "delete_42")
    ///   - payload: Optional key-value payload (e.g., form field values)
    public func sendAction(_ action: String, payload: [String: String] = [:]) {
        let encoded = action.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? action
        guard let url = URL(string: "\(baseURL)/actions/\(encoded)") else {
            lastError = "Invalid action URL"
            return
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try? JSONEncoder().encode(payload)

        Task {
            do {
                let (_, response) = try await actionSession.data(for: request)
                if let http = response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
                    await MainActor.run {
                        self.lastError = "Action failed (\(http.statusCode))"
                    }
                }
            } catch {
                await MainActor.run {
                    self.lastError = "Action error: \(error.localizedDescription)"
                }
            }
        }
    }

    /// Request a page navigation from the server.
    ///
    /// Server endpoint: `POST /actions/navigate`
    public func navigate(to path: String) {
        guard let url = URL(string: "\(baseURL)/actions/navigate") else {
            lastError = "Invalid navigate URL"
            return
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try? JSONEncoder().encode(["path": path])

        Task {
            do {
                let (_, response) = try await actionSession.data(for: request)
                if let http = response as? HTTPURLResponse, !(200..<300).contains(http.statusCode) {
                    await MainActor.run {
                        self.lastError = "Navigate failed (\(http.statusCode))"
                    }
                }
            } catch {
                await MainActor.run {
                    self.lastError = "Navigate error: \(error.localizedDescription)"
                }
            }
        }
    }

    /// Clean up resources.
    public func destroy() {
        disconnect()
    }
}

// MARK: - SSE URLSession Delegate

/// URLSessionDataDelegate that parses SSE events from a streaming HTTP response.
final class SSESessionDelegate: NSObject, URLSessionDataDelegate {
    private let onEvent: (String?, String) -> Void
    private let onOpen: () -> Void
    private let onError: (String) -> Void
    private let onComplete: () -> Void

    private var buffer = ""
    private var currentEvent: String? = nil
    private var currentData = ""
    private var opened = false

    init(
        onEvent: @escaping (String?, String) -> Void,
        onOpen: @escaping () -> Void,
        onError: @escaping (String) -> Void,
        onComplete: @escaping () -> Void
    ) {
        self.onEvent = onEvent
        self.onOpen = onOpen
        self.onError = onError
        self.onComplete = onComplete
    }

    // Called when data arrives from the SSE stream
    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        guard let text = String(data: data, encoding: .utf8) else { return }

        if !opened {
            opened = true
            onOpen()
        }

        buffer += text
        processBuffer()
    }

    // Called when the response headers arrive
    func urlSession(
        _ session: URLSession,
        dataTask: URLSessionDataTask,
        didReceive response: URLResponse,
        completionHandler: @escaping (URLSession.ResponseDisposition) -> Void
    ) {
        // Allow streaming — don't buffer the entire response
        completionHandler(.allow)
    }

    // Called when the task completes (success or error)
    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        if let error = error {
            // Cancelled tasks are not real errors
            if (error as NSError).code == NSURLErrorCancelled { return }
            onError(error.localizedDescription)
        } else {
            onComplete()
        }
    }

    private func processBuffer() {
        while let newlineRange = buffer.range(of: "\n") {
            let line = String(buffer[buffer.startIndex..<newlineRange.lowerBound])
            buffer = String(buffer[newlineRange.upperBound...])

            if line.isEmpty {
                // Empty line = end of event
                if !currentData.isEmpty {
                    onEvent(currentEvent, currentData)
                }
                currentEvent = nil
                currentData = ""
            } else if line.hasPrefix("event:") {
                currentEvent = String(line.dropFirst(6)).trimmingCharacters(in: .whitespaces)
            } else if line.hasPrefix("data:") {
                let value = String(line.dropFirst(5)).trimmingCharacters(in: .whitespaces)
                if !currentData.isEmpty { currentData += "\n" }
                currentData += value
            } else if line.hasPrefix(":") {
                // Comment / keepalive — ignore
            }
        }
    }
}
