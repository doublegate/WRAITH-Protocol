// WRAITH iOS - Sessions View

import SwiftUI

struct SessionsView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        NavigationView {
            Group {
                if appState.sessions.isEmpty {
                    VStack(spacing: 20) {
                        Image(systemName: "person.2.circle")
                            .font(.system(size: 64))
                            .foregroundColor(.secondary)
                        Text("No active sessions")
                            .font(.headline)
                            .foregroundColor(.secondary)
                        Text("Establish a session from the Home tab")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .multilineTextAlignment(.center)
                    }
                    .padding()
                } else {
                    List {
                        ForEach(appState.sessions, id: \.sessionId) { session in
                            SessionRow(session: session)
                        }
                    }
                }
            }
            .navigationTitle("Sessions")
        }
    }
}

// MARK: - Session Row

struct SessionRow: View {
    let session: SessionInfo

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text("Session")
                    .font(.headline)
                Spacer()
                Circle()
                    .fill(session.connected ? Color.green : Color.red)
                    .frame(width: 10, height: 10)
            }

            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text("ID:")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(session.sessionId.prefix(16) + "...")
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.primary)
                }

                HStack {
                    Text("Peer:")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(session.peerId.prefix(16) + "...")
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.primary)
                }

                HStack {
                    Text("Status:")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(session.connected ? "Connected" : "Disconnected")
                        .font(.caption)
                        .foregroundColor(session.connected ? .green : .red)
                }
            }
        }
        .padding(.vertical, 4)
    }
}

struct SessionsView_Previews: PreviewProvider {
    static var previews: some View {
        SessionsView()
            .environmentObject(AppState())
    }
}
