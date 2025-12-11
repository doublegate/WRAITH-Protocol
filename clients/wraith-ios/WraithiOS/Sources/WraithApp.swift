// WRAITH iOS - Main Application Entry Point

import SwiftUI

@main
struct WraithApp: App {
    @StateObject private var appState = AppState()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
        }
    }
}

// MARK: - App State

@MainActor
class AppState: ObservableObject {
    @Published var node: WraithNode?
    @Published var isStarted = false
    @Published var localPeerId: String = ""
    @Published var sessions: [SessionInfo] = []
    @Published var transfers: [TransferInfo] = []
    @Published var errorMessage: String?

    private let config: NodeConfig

    init() {
        // Default configuration
        self.config = NodeConfig(
            maxSessions: 100,
            maxTransfers: 10,
            bufferSize: 65536
        )
    }

    // MARK: - Node Operations

    func startNode(listenAddr: String = "0.0.0.0:0") {
        do {
            let node = try WraithNode(config: config)
            try node.start(listenAddr: listenAddr)

            self.node = node
            self.isStarted = true
            self.localPeerId = node.localPeerId()
            self.errorMessage = nil
        } catch {
            self.errorMessage = "Failed to start node: \(error.localizedDescription)"
        }
    }

    func shutdownNode() {
        guard let node = node else { return }

        do {
            try node.shutdown()
            self.node = nil
            self.isStarted = false
            self.localPeerId = ""
            self.sessions.removeAll()
            self.transfers.removeAll()
        } catch {
            self.errorMessage = "Failed to shutdown node: \(error.localizedDescription)"
        }
    }

    func establishSession(peerId: String) {
        guard let node = node else {
            self.errorMessage = "Node not started"
            return
        }

        do {
            let session = try node.establishSession(peerId: peerId)
            self.sessions.append(session)
            self.errorMessage = nil
        } catch {
            self.errorMessage = "Failed to establish session: \(error.localizedDescription)"
        }
    }

    func sendFile(peerId: String, filePath: String) {
        guard let node = node else {
            self.errorMessage = "Node not started"
            return
        }

        do {
            let transfer = try node.sendFile(peerId: peerId, filePath: filePath)
            self.transfers.append(transfer)
            self.errorMessage = nil
        } catch {
            self.errorMessage = "Failed to send file: \(error.localizedDescription)"
        }
    }

    func refreshStatus() {
        guard let node = node else { return }

        do {
            let status = try node.getStatus()
            self.isStarted = status.running
            self.localPeerId = status.localPeerId
        } catch {
            self.errorMessage = "Failed to get status: \(error.localizedDescription)"
        }
    }
}
