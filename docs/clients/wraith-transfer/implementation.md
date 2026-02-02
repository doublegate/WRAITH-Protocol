# WRAITH-Transfer Implementation

**Document Version:** 2.0.0
**Last Updated:** 2025-12-11
**Client Version:** 1.0.0 (v1.5.8)

---

## Overview

This document describes the actual implementation of WRAITH-Transfer, a simplified desktop application that provides a React UI over the wraith-core Node API. The implementation uses Tauri 2.0 for the desktop shell and integrates directly with wraith-core for all protocol functionality.

---

## Technology Stack

### Backend (Rust)

**Core Dependencies:**
```toml
[dependencies]
# WRAITH Protocol - Node API integration
wraith-core = { path = "../../../crates/wraith-core" }

# Tauri Framework
tauri = { version = "2.1.1", features = [] }
tauri-plugin-dialog = "2.0.3"
tauri-plugin-fs = "2.0.3"
tauri-plugin-log = "2.0.1"
tauri-plugin-shell = "2.0.2"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Utilities
hex = "0.4"
tracing = "0.1"
```

**Note:** The implementation uses wraith-core's Node API directly rather than reimplementing transfer management. No separate wraith-files, wraith-crypto, or wraith-transport dependencies are needed as these are already integrated into wraith-core.

### Frontend (TypeScript + React)

**Core Dependencies:**
```json
{
  "dependencies": {
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "@tauri-apps/api": "^2.1.1",
    "@tauri-apps/plugin-dialog": "^2.0.0",
    "@tauri-apps/plugin-fs": "^2.0.0",
    "@tauri-apps/plugin-shell": "^2.0.0",
    "zustand": "^5.0.2"
  },
  "devDependencies": {
    "@types/react": "^18.3.12",
    "@types/react-dom": "^18.3.1",
    "@vitejs/plugin-react": "^4.3.4",
    "typescript": "~5.6.2",
    "vite": "^5.4.11",
    "tailwindcss": "^4.0.0"
  }
}
```

---

## Code Architecture

### Actual Project Structure

```
clients/wraith-transfer/
├── src-tauri/              # Rust backend (84 lines)
│   ├── src/
│   │   ├── lib.rs          # Main entry point, IPC handler registration
│   │   ├── commands.rs     # 10 Tauri IPC commands (315 lines)
│   │   ├── state.rs        # AppState with Arc<RwLock<Option<Node>>>
│   │   └── error.rs        # AppError enum with Serialize for frontend
│   ├── Cargo.toml
│   ├── tauri.conf.json     # Tauri configuration with plugin permissions
│   └── icons/              # Application icons
│
├── frontend/               # React frontend
│   ├── src/
│   │   ├── main.tsx        # Vite entry point
│   │   ├── App.tsx         # Main application component
│   │   ├── components/     # UI components
│   │   │   ├── Header.tsx           # Status bar with node ID (click-to-copy)
│   │   │   ├── TransferList.tsx     # Active transfers display
│   │   │   ├── SessionPanel.tsx     # Session management
│   │   │   ├── NewTransferDialog.tsx # File selection and send
│   │   │   └── StatusBar.tsx        # Connection status
│   │   ├── stores/         # Zustand state management
│   │   │   ├── nodeStore.ts         # Node status, start/stop
│   │   │   ├── transferStore.ts     # Transfer list, progress
│   │   │   └── sessionStore.ts      # Session list
│   │   ├── types/
│   │   │   └── index.ts    # TypeScript types (NodeStatus, TransferInfo, SessionInfo)
│   │   └── lib/
│   │       └── tauri.ts    # Type-safe IPC wrappers
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   └── tailwind.config.js  # Tailwind CSS v4 configuration
│
└── package.json            # Workspace configuration
```

### Core Components

#### Application State (Rust)

The backend uses a simple AppState structure that wraps the wraith-core Node API:

```rust
// src-tauri/src/state.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use wraith_core::node::Node;
use crate::TransferInfo;

/// Global application state
#[derive(Default)]
pub struct AppState {
    /// The WRAITH protocol node (None when stopped)
    pub node: Arc<RwLock<Option<Node>>>,

    /// Tracked transfers (for UI state)
    pub transfers: Arc<RwLock<HashMap<String, TransferInfo>>>,

    /// Download directory preference
    pub download_dir: Arc<RwLock<Option<String>>>,
}

impl AppState {
    pub async fn is_node_running(&self) -> bool {
        self.node.read().await.is_some()
    }

    pub async fn get_node_id_hex(&self) -> Option<String> {
        let node = self.node.read().await;
        node.as_ref().map(|n| hex::encode(n.node_id()))
    }
}
```

#### Tauri IPC Commands

The application exposes 10 commands to the frontend:

```rust
// src-tauri/src/commands.rs (simplified excerpt)
use tauri::State;
use wraith_core::node::{Node, NodeConfig};

/// Start the WRAITH node
#[tauri::command]
pub async fn start_node(state: State<'_, AppState>) -> AppResult<NodeStatus> {
    let config = NodeConfig::default();
    let node = Node::new_with_config(config).await?;

    node.start().await?;
    let node_id = hex::encode(node.node_id());

    *state.node.write().await = Some(node);

    Ok(NodeStatus {
        running: true,
        node_id: Some(node_id),
        active_sessions: 0,
        active_transfers: 0,
    })
}

/// Send a file to a peer
#[tauri::command]
pub async fn send_file(
    state: State<'_, AppState>,
    peer_id: String,
    file_path: String,
) -> AppResult<String> {
    let node = state.node.read().await;
    let Some(node) = node.as_ref() else {
        return Err(AppError::NodeNotRunning);
    };

    let peer_bytes = hex::decode(&peer_id)?;
    let mut peer_id_arr = [0u8; 32];
    peer_id_arr.copy_from_slice(&peer_bytes);

    let transfer_id = node.send_file(PathBuf::from(&file_path), &peer_id_arr).await?;

    Ok(hex::encode(transfer_id))
}

/// Get transfer progress
#[tauri::command]
pub async fn get_transfer_progress(
    state: State<'_, AppState>,
    transfer_id: String,
) -> AppResult<Option<TransferInfo>> {
    let node = state.node.read().await;
    let Some(node) = node.as_ref() else {
        return Err(AppError::NodeNotRunning);
    };

    let transfer_bytes = hex::decode(&transfer_id)?;
    let mut transfer_id_arr = [0u8; 32];
    transfer_id_arr.copy_from_slice(&transfer_bytes);

    if let Some(progress) = node.get_transfer_progress(&transfer_id_arr).await {
        Ok(Some(TransferInfo {
            id: transfer_id,
            total_bytes: progress.bytes_total,
            transferred_bytes: progress.bytes_sent,
            progress: (progress.bytes_sent as f32) / (progress.bytes_total as f32),
            status: if progress.bytes_sent >= progress.bytes_total {
                "completed"
            } else {
                "in_progress"
            }.to_string(),
            // ... other fields
        }))
    } else {
        Ok(None)
    }
}
```

**Complete Command List:**
1. `get_node_status()` - Returns node running state, ID, session/transfer counts
2. `start_node()` - Creates and starts wraith-core Node
3. `stop_node()` - Stops and cleans up Node
4. `get_node_id()` - Returns hex-encoded node ID
5. `get_sessions()` - Lists active sessions with stats
6. `close_session(peer_id)` - Closes a session
7. `send_file(peer_id, file_path)` - Initiates file transfer
8. `get_transfers()` - Lists all active transfers
9. `get_transfer_progress(transfer_id)` - Gets specific transfer progress
10. `cancel_transfer(transfer_id)` - Cancels a transfer

#### React Frontend (Zustand Stores)

The frontend uses Zustand for state management with three stores:

```typescript
// frontend/src/stores/nodeStore.ts
import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface NodeStatus {
  running: boolean;
  node_id: string | null;
  active_sessions: number;
  active_transfers: number;
}

interface NodeStore {
  status: NodeStatus | null;
  loading: boolean;
  error: string | null;

  startNode: () => Promise<void>;
  stopNode: () => Promise<void>;
  refreshStatus: () => Promise<void>;
}

export const useNodeStore = create<NodeStore>((set) => ({
  status: null,
  loading: false,
  error: null,

  startNode: async () => {
    set({ loading: true, error: null });
    try {
      const status = await invoke<NodeStatus>('start_node');
      set({ status, loading: false });
    } catch (err) {
      set({ error: String(err), loading: false });
    }
  },

  stopNode: async () => {
    await invoke('stop_node');
    set({ status: null });
  },

  refreshStatus: async () => {
    const status = await invoke<NodeStatus>('get_node_status');
    set({ status });
  },
}));
```

```typescript
// frontend/src/stores/transferStore.ts
import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

interface TransferInfo {
  id: string;
  peer_id: string;
  file_name: string;
  total_bytes: number;
  transferred_bytes: number;
  progress: number;
  status: string;
  direction: string;
}

interface TransferStore {
  transfers: TransferInfo[];
  loading: boolean;
  error: string | null;

  sendFile: (peerId: string, filePath: string) => Promise<string | null>;
  refreshTransfers: () => Promise<void>;
  cancelTransfer: (transferId: string) => Promise<void>;
}

export const useTransferStore = create<TransferStore>((set) => ({
  transfers: [],
  loading: false,
  error: null,

  sendFile: async (peerId, filePath) => {
    set({ loading: true, error: null });
    try {
      const transferId = await invoke<string>('send_file', { peerId, filePath });
      set({ loading: false });
      return transferId;
    } catch (err) {
      set({ error: String(err), loading: false });
      return null;
    }
  },

  refreshTransfers: async () => {
    const transfers = await invoke<TransferInfo[]>('get_transfers');
    set({ transfers });
  },

  cancelTransfer: async (transferId) => {
    await invoke('cancel_transfer', { transferId });
  },
}));
```

---

## Implementation Details

### Protocol Integration

WRAITH Transfer delegates all protocol operations to `wraith-core::node::Node`:

- **File Transfer:** `Node::send_file(path, peer_id)` handles chunking, encryption, transmission
- **Session Management:** `Node::active_sessions()`, `Node::close_session(peer_id)`
- **Progress Tracking:** `Node::get_transfer_progress(transfer_id)` provides real-time stats
- **NAT Traversal:** Automatic STUN-based NAT type detection via wraith-discovery
- **Peer Discovery:** Future DHT integration (currently requires manual peer ID exchange)

The application layer focuses on UI/UX and does not reimplement protocol logic.

### Bug Fixes (v1.5.8)

Three critical issues were resolved:

**1. NAT Detection Timeout (FIXED)**
- **Issue:** STUN servers using invalid IPs caused startup blocking
- **Fix:** Updated to Google Public STUN servers (74.125.250.129:19302, 74.125.250.130:19302)
- **Location:** `crates/wraith-discovery/src/nat/types.rs`, `crates/wraith-discovery/src/manager.rs`

**2. Node ID Not Copyable (FIXED)**
- **Issue:** Node ID displayed as truncated text, not clickable
- **Fix:** Added click-to-copy with `navigator.clipboard.writeText()`, tooltip, visual feedback
- **Location:** `clients/wraith-transfer/frontend/src/components/Header.tsx`

**3. Browse Button Not Working (FIXED)**
- **Issue:** Missing Tauri plugin permissions in `tauri.conf.json`
- **Fix:** Added `plugins` section with `dialog` and `fs` permissions
- **Location:** `clients/wraith-transfer/src-tauri/tauri.conf.json`

For details, see [WRAITH_TRANSFER_FIXES.md](../../troubleshooting/WRAITH_TRANSFER_FIXES.md).

---

## Build and Deployment

### Development Build

```bash
# From repository root
cd clients/wraith-transfer

# Install frontend dependencies
cd frontend
npm install
cd ..

# Run development server (starts frontend dev server and Tauri)
npm run tauri dev
```

The development build:
- Hot-reloads React UI changes automatically
- Compiles Rust backend in debug mode
- Enables dev tools in the window
- Logs to console

### Production Build

```bash
# From clients/wraith-transfer/
npm run tauri build
```

**Build Outputs:**
- **Linux:** `src-tauri/target/release/wraith-transfer` (binary)
- **Windows:** `src-tauri/target/release/wraith-transfer.exe`
- **macOS:** `src-tauri/target/release/bundle/macos/WRAITH Transfer.app`

### Platform-Specific Notes

**Linux (Wayland/X11):**
- Application automatically handles Wayland Error 71 on KDE Plasma 6
- Falls back to X11 backend when needed
- Sets `GDK_BACKEND=x11` and `WEBKIT_DISABLE_COMPOSITING_MODE=1` for compatibility
- See [WRAITH_TRANSFER_FIXES.md](../../troubleshooting/WRAITH_TRANSFER_FIXES.md) for details

**Windows:**
- Requires WebView2 Runtime (usually pre-installed on Windows 10/11)
- Application bundle includes installer

**macOS:**
- Requires macOS 11+ for Apple Silicon compatibility
- Both Intel and ARM64 builds supported

---

## Testing

### Backend Tests

The Tauri backend includes unit tests for the command module:

```bash
# Run backend tests
cd clients/wraith-transfer/src-tauri
cargo test
```

**Test Coverage:**
- AppState initialization and node management
- IPC command error handling
- Transfer tracking state mutations
- Node ID hex encoding/decoding

**Example Test:**
```rust
#[tokio::test]
async fn test_app_state_node_not_running() {
    let state = mock_app_state();
    let running = state.is_node_running().await;
    assert!(!running, "Node should not be running");
}
```

### Frontend Tests

Frontend testing uses Vitest (not yet implemented in v1.5.8):

```bash
# Run frontend tests (future)
cd clients/wraith-transfer/frontend
npm test
```

### Integration Testing

**Manual Testing Checklist:**
1. Start node (verify node ID displayed)
2. Copy node ID to clipboard
3. Browse and select file
4. Enter peer ID and initiate transfer
5. Monitor progress updates
6. Verify transfer completion
7. Close sessions
8. Stop node

### Protocol Testing

Protocol-level testing is handled by wraith-core test suite:
- 1,303 total tests (1,280 passing, 23 ignored)
- File chunking, integrity, reassembly
- NAT traversal and session management
- Encryption and key exchange

```bash
# Run protocol tests
cargo test --workspace
```

---

## See Also

- [Architecture](architecture.md) - System design and component interactions
- [Features](features.md) - Complete feature list and roadmap
- [WRAITH Transfer Fixes](../../troubleshooting/WRAITH_TRANSFER_FIXES.md) - Bug fixes and troubleshooting
- [Client Overview](../overview.md) - All WRAITH clients
- [Protocol Implementation Guide](../../../ref-docs/protocol_implementation_guide.md) - Protocol specification

---

**Document History:**
- **v2.0.0 (2025-12-11):** Updated to reflect actual simplified implementation using wraith-core Node API, documented v1.5.8 bug fixes
- **v1.0.0 (2025-11-28):** Initial draft with planned complex architecture (superseded)
