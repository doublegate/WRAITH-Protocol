// WRAITH iOS - Settings View

import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var appState: AppState
    @AppStorage("listenAddress") private var listenAddress = "0.0.0.0:0"
    @AppStorage("maxSessions") private var maxSessions = 100
    @AppStorage("maxTransfers") private var maxTransfers = 10
    @AppStorage("bufferSize") private var bufferSize = 65536

    var body: some View {
        NavigationView {
            Form {
                Section(header: Text("Node Configuration")) {
                    HStack {
                        Text("Listen Address")
                        Spacer()
                        TextField("Address", text: $listenAddress)
                            .multilineTextAlignment(.trailing)
                            .font(.system(.body, design: .monospaced))
                    }

                    HStack {
                        Text("Max Sessions")
                        Spacer()
                        Text("\(maxSessions)")
                            .foregroundColor(.secondary)
                    }

                    HStack {
                        Text("Max Transfers")
                        Spacer()
                        Text("\(maxTransfers)")
                            .foregroundColor(.secondary)
                    }

                    HStack {
                        Text("Buffer Size")
                        Spacer()
                        Text("\(formatBytes(UInt64(bufferSize)))")
                            .foregroundColor(.secondary)
                    }
                }

                Section(header: Text("Node Information")) {
                    if appState.isStarted {
                        HStack {
                            Text("Status")
                            Spacer()
                            Text("Running")
                                .foregroundColor(.green)
                        }

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Local Peer ID")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(appState.localPeerId)
                                .font(.system(.body, design: .monospaced))
                                .textSelection(.enabled)
                        }
                    } else {
                        HStack {
                            Text("Status")
                            Spacer()
                            Text("Stopped")
                                .foregroundColor(.gray)
                        }
                    }
                }

                Section(header: Text("About")) {
                    HStack {
                        Text("Version")
                        Spacer()
                        Text("1.0.0")
                            .foregroundColor(.secondary)
                    }

                    HStack {
                        Text("Protocol")
                        Spacer()
                        Text("WRAITH v1.5.8")
                            .foregroundColor(.secondary)
                    }

                    Link("GitHub Repository", destination: URL(string: "https://github.com/doublegate/WRAITH-Protocol")!)
                }

                Section {
                    Button(action: appState.refreshStatus) {
                        HStack {
                            Text("Refresh Status")
                            Spacer()
                            Image(systemName: "arrow.clockwise")
                        }
                    }
                }
            }
            .navigationTitle("Settings")
        }
    }

    private func formatBytes(_ bytes: UInt64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .file
        return formatter.string(fromByteCount: Int64(bytes))
    }
}

struct SettingsView_Previews: PreviewProvider {
    static var previews: some View {
        SettingsView()
            .environmentObject(AppState())
    }
}
