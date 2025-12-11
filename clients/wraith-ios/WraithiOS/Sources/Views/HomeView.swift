// WRAITH iOS - Home View

import SwiftUI

struct HomeView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingNewSessionSheet = false

    var body: some View {
        NavigationView {
            VStack(spacing: 20) {
                // Node Status Card
                NodeStatusCard()
                    .padding()

                Spacer()

                // Quick Actions
                VStack(spacing: 16) {
                    if appState.isStarted {
                        Button(action: {
                            showingNewSessionSheet = true
                        }) {
                            Label("New Session", systemImage: "plus.circle.fill")
                                .frame(maxWidth: .infinity)
                                .padding()
                                .background(Color.blue)
                                .foregroundColor(.white)
                                .cornerRadius(10)
                        }

                        Button(action: {
                            appState.shutdownNode()
                        }) {
                            Label("Stop Node", systemImage: "stop.circle.fill")
                                .frame(maxWidth: .infinity)
                                .padding()
                                .background(Color.red)
                                .foregroundColor(.white)
                                .cornerRadius(10)
                        }
                    } else {
                        Button(action: {
                            appState.startNode()
                        }) {
                            Label("Start Node", systemImage: "play.circle.fill")
                                .frame(maxWidth: .infinity)
                                .padding()
                                .background(Color.green)
                                .foregroundColor(.white)
                                .cornerRadius(10)
                        }
                    }
                }
                .padding()

                Spacer()
            }
            .navigationTitle("WRAITH")
            .sheet(isPresented: $showingNewSessionSheet) {
                NewSessionView()
            }
        }
    }
}

// MARK: - Node Status Card

struct NodeStatusCard: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("Node Status")
                    .font(.headline)
                Spacer()
                Circle()
                    .fill(appState.isStarted ? Color.green : Color.gray)
                    .frame(width: 12, height: 12)
            }

            Divider()

            if appState.isStarted {
                StatusRow(label: "Status", value: "Running")
                StatusRow(label: "Peer ID", value: appState.localPeerId)
                StatusRow(label: "Sessions", value: "\(appState.sessions.count)")
                StatusRow(label: "Transfers", value: "\(appState.transfers.count)")
            } else {
                Text("Node is not running")
                    .foregroundColor(.secondary)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(radius: 2)
    }
}

struct StatusRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
            Spacer()
            Text(value)
                .font(.system(.body, design: .monospaced))
                .lineLimit(1)
                .truncationMode(.middle)
        }
    }
}

// MARK: - New Session View

struct NewSessionView: View {
    @Environment(\.dismiss) var dismiss
    @EnvironmentObject var appState: AppState
    @State private var peerId = ""

    var body: some View {
        NavigationView {
            Form {
                Section(header: Text("Peer Information")) {
                    TextField("Peer ID (hex)", text: $peerId)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                        .font(.system(.body, design: .monospaced))
                }

                Section {
                    Button("Establish Session") {
                        appState.establishSession(peerId: peerId)
                        dismiss()
                    }
                    .disabled(peerId.isEmpty)
                }
            }
            .navigationTitle("New Session")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
            }
        }
    }
}

struct HomeView_Previews: PreviewProvider {
    static var previews: some View {
        HomeView()
            .environmentObject(AppState())
    }
}
