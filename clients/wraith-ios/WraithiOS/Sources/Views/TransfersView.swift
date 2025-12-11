// WRAITH iOS - Transfers View

import SwiftUI

struct TransfersView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingNewTransferSheet = false

    var body: some View {
        NavigationView {
            Group {
                if appState.transfers.isEmpty {
                    VStack(spacing: 20) {
                        Image(systemName: "arrow.up.arrow.down.circle")
                            .font(.system(size: 64))
                            .foregroundColor(.secondary)
                        Text("No active transfers")
                            .font(.headline)
                            .foregroundColor(.secondary)

                        if appState.isStarted {
                            Button(action: {
                                showingNewTransferSheet = true
                            }) {
                                Label("Send File", systemImage: "plus.circle.fill")
                                    .padding()
                                    .background(Color.blue)
                                    .foregroundColor(.white)
                                    .cornerRadius(10)
                            }
                        }
                    }
                } else {
                    List {
                        ForEach(appState.transfers, id: \.transferId) { transfer in
                            TransferRow(transfer: transfer)
                        }
                    }
                }
            }
            .navigationTitle("Transfers")
            .toolbar {
                if appState.isStarted && !appState.transfers.isEmpty {
                    ToolbarItem(placement: .primaryAction) {
                        Button(action: {
                            showingNewTransferSheet = true
                        }) {
                            Image(systemName: "plus")
                        }
                    }
                }
            }
            .sheet(isPresented: $showingNewTransferSheet) {
                NewTransferView()
            }
        }
    }
}

// MARK: - Transfer Row

struct TransferRow: View {
    let transfer: TransferInfo

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(URL(fileURLWithPath: transfer.filePath).lastPathComponent)
                    .font(.headline)
                Spacer()
                StatusBadge(status: transfer.status)
            }

            HStack {
                Text("Peer: \(transfer.peerId.prefix(16))...")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Text(formatBytes(transfer.bytesTransferred))
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            if transfer.fileSize > 0 {
                ProgressView(value: Double(transfer.bytesTransferred), total: Double(transfer.fileSize))
            }
        }
        .padding(.vertical, 4)
    }

    private func formatBytes(_ bytes: UInt64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .file
        return formatter.string(fromByteCount: Int64(bytes))
    }
}

// MARK: - Status Badge

struct StatusBadge: View {
    let status: TransferStatus

    var body: some View {
        Text(statusText)
            .font(.caption)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(statusColor.opacity(0.2))
            .foregroundColor(statusColor)
            .cornerRadius(4)
    }

    private var statusText: String {
        switch status {
        case .pending: return "Pending"
        case .sending: return "Sending"
        case .receiving: return "Receiving"
        case .completed: return "Completed"
        case .failed: return "Failed"
        case .cancelled: return "Cancelled"
        }
    }

    private var statusColor: Color {
        switch status {
        case .pending: return .orange
        case .sending, .receiving: return .blue
        case .completed: return .green
        case .failed: return .red
        case .cancelled: return .gray
        }
    }
}

// MARK: - New Transfer View

struct NewTransferView: View {
    @Environment(\.dismiss) var dismiss
    @EnvironmentObject var appState: AppState
    @State private var peerId = ""
    @State private var filePath = ""
    @State private var showingFilePicker = false

    var body: some View {
        NavigationView {
            Form {
                Section(header: Text("Recipient")) {
                    TextField("Peer ID (hex)", text: $peerId)
                        .autocapitalization(.none)
                        .disableAutocorrection(true)
                        .font(.system(.body, design: .monospaced))
                }

                Section(header: Text("File")) {
                    HStack {
                        TextField("File Path", text: $filePath)
                            .disabled(true)
                        Button("Browse") {
                            showingFilePicker = true
                        }
                    }
                }

                Section {
                    Button("Send File") {
                        appState.sendFile(peerId: peerId, filePath: filePath)
                        dismiss()
                    }
                    .disabled(peerId.isEmpty || filePath.isEmpty)
                }
            }
            .navigationTitle("Send File")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
            }
            .sheet(isPresented: $showingFilePicker) {
                // File picker would be implemented here
                // For iOS, you would use UIDocumentPickerViewController
                Text("File picker not yet implemented")
            }
        }
    }
}

struct TransfersView_Previews: PreviewProvider {
    static var previews: some View {
        TransfersView()
            .environmentObject(AppState())
    }
}
