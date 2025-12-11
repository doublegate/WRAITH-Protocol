# WRAITH-Transfer Client - Sprint Planning

**Client Name:** WRAITH-Transfer
**Tier:** 1 (High Priority)
**Description:** Cross-platform P2P file transfer GUI application
**Target Platforms:** Windows, macOS, Linux
**UI Framework:** Tauri 2.0 + React/TypeScript + Zustand
**Timeline:** 12 weeks (3 sprints × 4 weeks)
**Total Story Points:** 156

**Status:** v1.5.8 - Basic functionality complete, Sprint 1 partially complete

---

## Implementation Status (v1.5.8)

**Current Implementation:**
- Simplified architecture using wraith-core Node API directly
- Basic Tauri application with React UI and Zustand state management
- 10 IPC commands for node/session/transfer management
- File selection with native dialog (fixed in v1.5.8)
- Node ID display with click-to-copy (fixed in v1.5.8)
- Transfer progress tracking
- Session management
- NAT detection with STUN (fixed in v1.5.8)

**Divergence from Plan:**
The actual implementation is simpler than originally planned. Instead of building a complex TransferManager with separate sender/receiver/progress modules, the application uses wraith-core's Node API directly. This provides all protocol functionality with minimal application-layer code.

---

## Overview

WRAITH-Transfer is the flagship client application providing an intuitive graphical interface for secure peer-to-peer file transfers using the WRAITH protocol. It targets non-technical users who need military-grade security without complexity.

**Core Value Proposition:**
- Click-to-send file transfer between any two devices
- No account creation or central servers required
- Encrypted peer-to-peer connections
- Cross-platform native performance
- Real-time transfer progress and session status

---

## Success Criteria

**User Experience:**
- [ ] Transfer initiated in <3 clicks from file selection
- [ ] Progress feedback updates at <100ms intervals
- [ ] Peer connection established in <5 seconds
- [ ] Application startup in <2 seconds
- [ ] <20 MB installer size per platform

**Performance:**
- [ ] Saturates 1 Gbps link with single large file
- [ ] Handles 10,000+ small files efficiently
- [ ] Resumes interrupted transfers automatically
- [ ] CPU usage <10% during idle peer connection
- [ ] Memory usage <150 MB baseline

**Platform Support:**
- [ ] Windows 10+ (x64, ARM64)
- [ ] macOS 11+ (Intel, Apple Silicon)
- [ ] Linux (AppImage, deb, rpm)
- [ ] Native installer per platform
- [ ] Auto-update mechanism

**Security:**
- [ ] Peer identity verification via QR code or key fingerprint
- [ ] Optional password protection for transfers
- [ ] Sandboxed file system access
- [ ] No data leaves encrypted channel
- [ ] Key management UI

---

## Dependencies

**Protocol Dependencies:**
- WRAITH protocol library (wraith-core, wraith-files)
- Protocol Phases 1-6 completed (weeks 1-36)
- CLI functionality validated

**External Dependencies:**
- Tauri 2.x framework
- React 18+ with TypeScript 5+
- Node.js 20+ for build tooling
- Platform-specific code signing certificates
- GitHub Actions runners (Windows, macOS, Ubuntu)

**Team Dependencies:**
- UI/UX designer for wireframes and visual design
- Frontend developer for React components
- Rust developer for Tauri backend integration
- QA tester for cross-platform validation

---

## Deliverables

**Sprint 1 (Weeks 37-40): Foundation & Core UI**
1. Tauri project scaffold with React frontend
2. Main application window with basic layout
3. File selection dialog (native file picker)
4. Peer connection screen (manual key exchange)
5. Transfer progress display (single file)
6. System tray integration (Windows/macOS/Linux)
7. Application settings panel
8. Dark/light theme toggle

**Sprint 2 (Weeks 41-44): Advanced Features & Platform Polish**
1. Multi-file batch transfer support
2. Drag-and-drop file input
3. Transfer history log with search
4. QR code peer pairing (camera + display)
5. Password-protected transfers
6. Resume interrupted transfers
7. Platform-specific installers (MSI, DMG, deb)
8. Auto-update mechanism (Tauri updater)

**Sprint 3 (Weeks 45-48): Hardening & Distribution**
1. Comprehensive error handling and user feedback
2. Accessibility improvements (keyboard navigation, screen readers)
3. Internationalization (i18n) for 5 languages
4. Performance optimization (lazy loading, virtual scrolling)
5. Cross-platform integration testing
6. Code signing for all platforms
7. Documentation (user guide, FAQ, troubleshooting)
8. Public release preparation (website, download links)

---

## Sprint 1: Foundation & Core UI (Weeks 37-40)

### Sprint Goal
Establish the Tauri application scaffold and implement core file transfer workflow with basic UI.

**Total Story Points:** 52
**Actual Points Completed:** ~35 (67%)

### Sprint 1 Actual Status (v1.5.8)

| Task | Planned | Actual | Status | Notes |
|------|---------|--------|--------|-------|
| S1.1: Tauri Scaffold | 8 pts | 8 pts | ✅ COMPLETE | Tauri 2.0 with React + TypeScript + Zustand |
| S1.2: Main Window Layout | 5 pts | 5 pts | ✅ COMPLETE | Simple header, transfer list, session panel layout |
| S1.3: File Selection | 8 pts | 8 pts | ✅ COMPLETE | Native dialog with browse button (fixed v1.5.8) |
| S1.4: Peer Connection | 13 pts | 8 pts | ⚠️ PARTIAL | Manual node ID exchange, no discovery UI yet |
| S1.5: Transfer Progress | 8 pts | 6 pts | ✅ COMPLETE | Basic progress display, no pause/resume yet |
| S1.6: System Tray | 5 pts | 0 pts | ❌ REMOVED | tray-icon dependency removed in v1.5.6 |
| S1.7: Settings Panel | 3 pts | 0 pts | ❌ NOT IMPL | No settings UI yet |
| S1.8: Theme Toggle | 2 pts | 0 pts | ❌ NOT IMPL | Single dark theme only |

**Key Achievements:**
- Working Tauri desktop application with React UI
- Node lifecycle management (start/stop)
- File transfer initiation and progress tracking
- Session management UI
- Click-to-copy node ID with tooltip (v1.5.8)
- Browse button with proper permissions (v1.5.8)
- NAT detection with Google STUN servers (v1.5.8)

**Deferred to Future Sprints:**
- Settings panel with configuration options
- Theme customization (light/dark toggle)
- System tray integration (may use different approach)
- Advanced peer discovery UI
- Transfer pause/resume functionality

---

### S1.1: Tauri Project Scaffold (8 points)

**Task:** Initialize Tauri 2.x project with React frontend and TypeScript configuration.

**Acceptance Criteria:**
- [ ] `npm create tauri-app` executed with React + TypeScript template
- [ ] Build succeeds on all target platforms (Windows, macOS, Linux)
- [ ] Hot module reloading works in development mode
- [ ] Production build generates optimized bundles
- [ ] Tauri CLI commands documented in README

**Implementation:**

```bash
# Initialize Tauri project
npm create tauri-app@latest -- --name wraith-transfer --template react-ts

cd wraith-transfer

# Install dependencies
npm install

# Development build
npm run tauri dev

# Production build
npm run tauri build
```

**Cargo.toml additions:**
```toml
[dependencies]
tauri = { version = "2.0", features = ["protocol-asset", "fs-read-dir", "dialog-all"] }
wraith-core = { path = "../../crates/wraith-core" }
wraith-files = { path = "../../crates/wraith-files" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.40", features = ["full"] }
log = "0.4"
env_logger = "0.11"
```

**tauri.conf.json:**
```json
{
  "productName": "WRAITH Transfer",
  "version": "0.1.0",
  "identifier": "com.wraith.transfer",
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devPath": "http://localhost:1420",
    "distDir": "../dist"
  },
  "tauri": {
    "allowlist": {
      "all": false,
      "fs": {
        "scope": ["$DOWNLOAD/*", "$DOCUMENT/*", "$HOME/*"],
        "readDir": true,
        "readFile": true,
        "writeFile": true
      },
      "dialog": {
        "all": true,
        "save": true,
        "open": true
      },
      "path": {
        "all": true
      },
      "notification": {
        "all": true
      }
    },
    "windows": [
      {
        "title": "WRAITH Transfer",
        "width": 1200,
        "height": 800,
        "minWidth": 800,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false
      }
    ]
  }
}
```

---

### S1.2: Main Application Window Layout (5 points)

**Task:** Design and implement the main application window with navigation and content areas.

**Acceptance Criteria:**
- [ ] Responsive layout works on 800×600 minimum resolution
- [ ] Navigation sidebar with icons for Send, Receive, History, Settings
- [ ] Content area updates based on navigation selection
- [ ] Window controls (minimize, maximize, close) work correctly
- [ ] Application icon displays in taskbar/dock

**React Component Structure:**
```tsx
// src/App.tsx
import React, { useState } from 'react';
import { Sidebar } from './components/Sidebar';
import { SendView } from './views/SendView';
import { ReceiveView } from './views/ReceiveView';
import { HistoryView } from './views/HistoryView';
import { SettingsView } from './views/SettingsView';

type View = 'send' | 'receive' | 'history' | 'settings';

export default function App() {
  const [currentView, setCurrentView] = useState<View>('send');

  const renderView = () => {
    switch (currentView) {
      case 'send':
        return <SendView />;
      case 'receive':
        return <ReceiveView />;
      case 'history':
        return <HistoryView />;
      case 'settings':
        return <SettingsView />;
    }
  };

  return (
    <div className="app-container">
      <Sidebar currentView={currentView} onNavigate={setCurrentView} />
      <main className="content-area">
        {renderView()}
      </main>
    </div>
  );
}
```

**Sidebar Component:**
```tsx
// src/components/Sidebar.tsx
import React from 'react';
import { Send, Download, History, Settings } from 'lucide-react';

interface SidebarProps {
  currentView: string;
  onNavigate: (view: string) => void;
}

export function Sidebar({ currentView, onNavigate }: SidebarProps) {
  const menuItems = [
    { id: 'send', label: 'Send', icon: Send },
    { id: 'receive', label: 'Receive', icon: Download },
    { id: 'history', label: 'History', icon: History },
    { id: 'settings', label: 'Settings', icon: Settings },
  ];

  return (
    <nav className="sidebar">
      <div className="sidebar-header">
        <img src="/logo.svg" alt="WRAITH" />
        <h1>WRAITH</h1>
      </div>
      <ul className="nav-menu">
        {menuItems.map(item => (
          <li key={item.id}>
            <button
              className={currentView === item.id ? 'active' : ''}
              onClick={() => onNavigate(item.id)}
            >
              <item.icon size={24} />
              <span>{item.label}</span>
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}
```

**CSS (Tailwind or styled-components):**
```css
.app-container {
  display: flex;
  height: 100vh;
  background: var(--bg-primary);
  color: var(--text-primary);
}

.sidebar {
  width: 240px;
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
}

.sidebar-header {
  padding: 24px;
  border-bottom: 1px solid var(--border-color);
}

.nav-menu li button {
  width: 100%;
  padding: 12px 24px;
  display: flex;
  align-items: center;
  gap: 12px;
  border: none;
  background: transparent;
  cursor: pointer;
  transition: background 0.2s;
}

.nav-menu li button.active {
  background: var(--bg-active);
  border-left: 3px solid var(--accent-color);
}

.content-area {
  flex: 1;
  overflow-y: auto;
  padding: 32px;
}
```

---

### S1.3: File Selection Dialog (8 points)

**Task:** Implement native file picker dialog for selecting files/folders to transfer.

**Acceptance Criteria:**
- [ ] Single file selection works
- [ ] Multiple file selection works
- [ ] Folder selection works (recursive)
- [ ] File size and count displayed after selection
- [ ] Invalid/inaccessible files show error message
- [ ] Selected files listed with remove option

**Tauri Backend (Rust):**
```rust
// src-tauri/src/commands.rs
use tauri::api::dialog::{FileDialogBuilder, MessageDialogBuilder};
use std::path::PathBuf;
use std::fs;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct FileInfo {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
}

#[tauri::command]
pub async fn select_files() -> Result<Vec<FileInfo>, String> {
    let files = FileDialogBuilder::new()
        .add_filter("All Files", &["*"])
        .set_title("Select Files to Transfer")
        .pick_files()
        .await
        .ok_or("No files selected")?;

    files.into_iter()
        .map(|path| get_file_info(&path))
        .collect()
}

#[tauri::command]
pub async fn select_folder() -> Result<Vec<FileInfo>, String> {
    let folder = FileDialogBuilder::new()
        .set_title("Select Folder to Transfer")
        .pick_folder()
        .await
        .ok_or("No folder selected")?;

    collect_files_recursive(&folder)
}

fn get_file_info(path: &PathBuf) -> Result<FileInfo, String> {
    let metadata = fs::metadata(path)
        .map_err(|e| format!("Cannot access file: {}", e))?;

    Ok(FileInfo {
        path: path.to_string_lossy().to_string(),
        name: path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        size: metadata.len(),
        is_dir: metadata.is_dir(),
    })
}

fn collect_files_recursive(dir: &PathBuf) -> Result<Vec<FileInfo>, String> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)
        .map_err(|e| format!("Cannot read directory: {}", e))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        if path.is_file() {
            files.push(get_file_info(&path)?);
        } else if path.is_dir() {
            files.extend(collect_files_recursive(&path)?);
        }
    }

    Ok(files)
}
```

**React Frontend:**
```tsx
// src/views/SendView.tsx
import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { FileInfo } from '../types';

export function SendView() {
  const [selectedFiles, setSelectedFiles] = useState<FileInfo[]>([]);
  const [loading, setLoading] = useState(false);

  const handleSelectFiles = async () => {
    setLoading(true);
    try {
      const files = await invoke<FileInfo[]>('select_files');
      setSelectedFiles(prev => [...prev, ...files]);
    } catch (error) {
      console.error('File selection failed:', error);
    } finally {
      setLoading(false);
    }
  };

  const handleSelectFolder = async () => {
    setLoading(true);
    try {
      const files = await invoke<FileInfo[]>('select_folder');
      setSelectedFiles(prev => [...prev, ...files]);
    } catch (error) {
      console.error('Folder selection failed:', error);
    } finally {
      setLoading(false);
    }
  };

  const totalSize = selectedFiles.reduce((sum, f) => sum + f.size, 0);

  return (
    <div className="send-view">
      <h2>Send Files</h2>

      <div className="file-selection">
        <button onClick={handleSelectFiles} disabled={loading}>
          Select Files
        </button>
        <button onClick={handleSelectFolder} disabled={loading}>
          Select Folder
        </button>
      </div>

      {selectedFiles.length > 0 && (
        <div className="selected-files">
          <h3>Selected: {selectedFiles.length} files ({formatBytes(totalSize)})</h3>
          <ul>
            {selectedFiles.map((file, idx) => (
              <li key={idx}>
                <span>{file.name}</span>
                <span>{formatBytes(file.size)}</span>
                <button onClick={() => {
                  setSelectedFiles(prev => prev.filter((_, i) => i !== idx));
                }}>
                  Remove
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}
```

---

### S1.4: Peer Connection Screen (13 points)

**Task:** Implement UI for establishing peer connections via manual key exchange.

**Acceptance Criteria:**
- [ ] Display local peer ID (public key fingerprint)
- [ ] Copy peer ID to clipboard button
- [ ] Input field for remote peer ID
- [ ] Connection status indicator (connecting/connected/failed)
- [ ] Peer identity verification UI (key fingerprint comparison)
- [ ] Save trusted peers to local storage
- [ ] Recent peers list for quick reconnection

**Tauri Backend:**
```rust
// src-tauri/src/peer.rs
use wraith_core::{PeerId, Connection};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub struct PeerManager {
    local_peer: PeerId,
    connections: Arc<Mutex<HashMap<String, Connection>>>,
}

impl PeerManager {
    pub fn new() -> Self {
        let local_peer = PeerId::generate();
        Self {
            local_peer,
            connections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn local_id(&self) -> String {
        self.local_peer.to_base64()
    }

    pub fn fingerprint(&self) -> String {
        self.local_peer.fingerprint()
    }

    pub async fn connect_to_peer(&self, remote_id: String) -> Result<(), String> {
        let peer_id = PeerId::from_base64(&remote_id)
            .map_err(|e| format!("Invalid peer ID: {}", e))?;

        // Attempt connection via DHT discovery
        let conn = Connection::dial(peer_id)
            .await
            .map_err(|e| format!("Connection failed: {}", e))?;

        let mut conns = self.connections.lock().unwrap();
        conns.insert(remote_id, conn);

        Ok(())
    }

    pub fn disconnect_peer(&self, remote_id: &str) {
        let mut conns = self.connections.lock().unwrap();
        conns.remove(remote_id);
    }

    pub fn connection_status(&self, remote_id: &str) -> Option<String> {
        let conns = self.connections.lock().unwrap();
        conns.get(remote_id).map(|c| c.status().to_string())
    }
}

#[tauri::command]
pub fn get_local_peer_id(peer_mgr: tauri::State<PeerManager>) -> String {
    peer_mgr.local_id()
}

#[tauri::command]
pub fn get_peer_fingerprint(peer_mgr: tauri::State<PeerManager>) -> String {
    peer_mgr.fingerprint()
}

#[tauri::command]
pub async fn connect_peer(
    remote_id: String,
    peer_mgr: tauri::State<'_, PeerManager>
) -> Result<(), String> {
    peer_mgr.connect_to_peer(remote_id).await
}

#[tauri::command]
pub fn disconnect_peer(
    remote_id: String,
    peer_mgr: tauri::State<PeerManager>
) {
    peer_mgr.disconnect_peer(&remote_id);
}

#[tauri::command]
pub fn peer_status(
    remote_id: String,
    peer_mgr: tauri::State<PeerManager>
) -> Option<String> {
    peer_mgr.connection_status(&remote_id)
}
```

**React Component:**
```tsx
// src/components/PeerConnection.tsx
import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { writeText } from '@tauri-apps/api/clipboard';

export function PeerConnection() {
  const [localPeerId, setLocalPeerId] = useState('');
  const [localFingerprint, setLocalFingerprint] = useState('');
  const [remotePeerId, setRemotePeerId] = useState('');
  const [connectionStatus, setConnectionStatus] = useState<'idle' | 'connecting' | 'connected' | 'failed'>('idle');
  const [trustedPeers, setTrustedPeers] = useState<string[]>([]);

  useEffect(() => {
    loadLocalPeerInfo();
    loadTrustedPeers();
  }, []);

  const loadLocalPeerInfo = async () => {
    const peerId = await invoke<string>('get_local_peer_id');
    const fingerprint = await invoke<string>('get_peer_fingerprint');
    setLocalPeerId(peerId);
    setLocalFingerprint(fingerprint);
  };

  const loadTrustedPeers = () => {
    const saved = localStorage.getItem('trustedPeers');
    if (saved) {
      setTrustedPeers(JSON.parse(saved));
    }
  };

  const copyPeerId = async () => {
    await writeText(localPeerId);
    // Show toast notification
  };

  const handleConnect = async () => {
    if (!remotePeerId.trim()) return;

    setConnectionStatus('connecting');
    try {
      await invoke('connect_peer', { remoteId: remotePeerId });
      setConnectionStatus('connected');

      // Add to trusted peers
      if (!trustedPeers.includes(remotePeerId)) {
        const updated = [...trustedPeers, remotePeerId];
        setTrustedPeers(updated);
        localStorage.setItem('trustedPeers', JSON.stringify(updated));
      }
    } catch (error) {
      console.error('Connection failed:', error);
      setConnectionStatus('failed');
    }
  };

  return (
    <div className="peer-connection">
      <section className="local-peer">
        <h3>Your Peer ID</h3>
        <div className="peer-id-display">
          <code>{localPeerId}</code>
          <button onClick={copyPeerId}>Copy</button>
        </div>
        <div className="fingerprint">
          <span>Fingerprint:</span>
          <code>{localFingerprint}</code>
        </div>
      </section>

      <section className="remote-peer">
        <h3>Connect to Peer</h3>
        <input
          type="text"
          placeholder="Paste peer ID here..."
          value={remotePeerId}
          onChange={e => setRemotePeerId(e.target.value)}
        />
        <button
          onClick={handleConnect}
          disabled={connectionStatus === 'connecting'}
        >
          {connectionStatus === 'connecting' ? 'Connecting...' : 'Connect'}
        </button>

        {connectionStatus === 'connected' && (
          <div className="status success">Connected</div>
        )}
        {connectionStatus === 'failed' && (
          <div className="status error">Connection failed</div>
        )}
      </section>

      {trustedPeers.length > 0 && (
        <section className="trusted-peers">
          <h3>Recent Peers</h3>
          <ul>
            {trustedPeers.map(peerId => (
              <li key={peerId}>
                <code>{peerId.slice(0, 16)}...{peerId.slice(-8)}</code>
                <button onClick={() => setRemotePeerId(peerId)}>
                  Use
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
```

---

### S1.5: Transfer Progress Display (8 points)

**Task:** Implement real-time transfer progress UI for single file transfers.

**Acceptance Criteria:**
- [ ] Progress bar updates every 100ms
- [ ] Shows transfer speed (MB/s)
- [ ] Shows time remaining estimate
- [ ] Shows bytes transferred / total bytes
- [ ] Cancel button works and cleans up connection
- [ ] Pause/resume functionality (if protocol supports)
- [ ] Transfer completion notification

**Tauri Backend:**
```rust
// src-tauri/src/transfer.rs
use wraith_files::{FileTransfer, TransferProgress};
use std::sync::{Arc, Mutex};
use tauri::Window;

#[derive(Clone, serde::Serialize)]
struct ProgressUpdate {
    transfer_id: String,
    bytes_transferred: u64,
    total_bytes: u64,
    speed_mbps: f64,
    eta_seconds: u64,
}

pub struct TransferManager {
    active_transfers: Arc<Mutex<HashMap<String, FileTransfer>>>,
}

impl TransferManager {
    pub fn new() -> Self {
        Self {
            active_transfers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_transfer(
        &self,
        window: Window,
        file_path: String,
        remote_peer: String,
    ) -> Result<String, String> {
        let transfer_id = uuid::Uuid::new_v4().to_string();

        let transfer = FileTransfer::new(file_path)
            .map_err(|e| e.to_string())?;

        // Clone for progress callback
        let transfer_id_clone = transfer_id.clone();
        let window_clone = window.clone();

        transfer.on_progress(move |progress: TransferProgress| {
            let update = ProgressUpdate {
                transfer_id: transfer_id_clone.clone(),
                bytes_transferred: progress.bytes_sent,
                total_bytes: progress.total_bytes,
                speed_mbps: progress.speed_mbps,
                eta_seconds: progress.eta_seconds,
            };

            window_clone.emit("transfer-progress", update).ok();
        });

        // Start transfer in background
        let transfer_clone = transfer.clone();
        tokio::spawn(async move {
            if let Err(e) = transfer_clone.send_to(&remote_peer).await {
                eprintln!("Transfer failed: {}", e);
                window.emit("transfer-failed", transfer_id).ok();
            } else {
                window.emit("transfer-complete", transfer_id).ok();
            }
        });

        let mut transfers = self.active_transfers.lock().unwrap();
        transfers.insert(transfer_id.clone(), transfer);

        Ok(transfer_id)
    }

    pub fn cancel_transfer(&self, transfer_id: &str) -> Result<(), String> {
        let mut transfers = self.active_transfers.lock().unwrap();
        if let Some(transfer) = transfers.remove(transfer_id) {
            transfer.cancel();
            Ok(())
        } else {
            Err("Transfer not found".to_string())
        }
    }
}

#[tauri::command]
pub async fn send_file(
    file_path: String,
    remote_peer: String,
    window: Window,
    transfer_mgr: tauri::State<'_, TransferManager>
) -> Result<String, String> {
    transfer_mgr.start_transfer(window, file_path, remote_peer).await
}

#[tauri::command]
pub fn cancel_transfer(
    transfer_id: String,
    transfer_mgr: tauri::State<TransferManager>
) -> Result<(), String> {
    transfer_mgr.cancel_transfer(&transfer_id)
}
```

**React Component:**
```tsx
// src/components/TransferProgress.tsx
import React, { useState, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';

interface ProgressData {
  transfer_id: string;
  bytes_transferred: number;
  total_bytes: number;
  speed_mbps: number;
  eta_seconds: number;
}

interface TransferProgressProps {
  transferId: string;
  fileName: string;
  onComplete?: () => void;
  onFailed?: () => void;
}

export function TransferProgress({ transferId, fileName, onComplete, onFailed }: TransferProgressProps) {
  const [progress, setProgress] = useState<ProgressData | null>(null);
  const [status, setStatus] = useState<'active' | 'complete' | 'failed' | 'cancelled'>('active');

  useEffect(() => {
    const unlistenProgress = listen<ProgressData>('transfer-progress', (event) => {
      if (event.payload.transfer_id === transferId) {
        setProgress(event.payload);
      }
    });

    const unlistenComplete = listen<string>('transfer-complete', (event) => {
      if (event.payload === transferId) {
        setStatus('complete');
        onComplete?.();
      }
    });

    const unlistenFailed = listen<string>('transfer-failed', (event) => {
      if (event.payload === transferId) {
        setStatus('failed');
        onFailed?.();
      }
    });

    return () => {
      unlistenProgress.then(fn => fn());
      unlistenComplete.then(fn => fn());
      unlistenFailed.then(fn => fn());
    };
  }, [transferId]);

  const handleCancel = async () => {
    try {
      await invoke('cancel_transfer', { transferId });
      setStatus('cancelled');
    } catch (error) {
      console.error('Cancel failed:', error);
    }
  };

  const percentComplete = progress
    ? (progress.bytes_transferred / progress.total_bytes * 100)
    : 0;

  return (
    <div className="transfer-progress">
      <div className="file-name">{fileName}</div>

      <div className="progress-bar">
        <div
          className="progress-fill"
          style={{ width: `${percentComplete}%` }}
        />
      </div>

      {progress && (
        <div className="stats">
          <span>{formatBytes(progress.bytes_transferred)} / {formatBytes(progress.total_bytes)}</span>
          <span>{progress.speed_mbps.toFixed(2)} MB/s</span>
          <span>ETA: {formatTime(progress.eta_seconds)}</span>
        </div>
      )}

      <div className="status">
        {status === 'active' && <span className="active">Transferring...</span>}
        {status === 'complete' && <span className="success">Complete ✓</span>}
        {status === 'failed' && <span className="error">Failed ✗</span>}
        {status === 'cancelled' && <span className="warning">Cancelled</span>}
      </div>

      {status === 'active' && (
        <button onClick={handleCancel} className="cancel-btn">
          Cancel
        </button>
      )}
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(2)} ${sizes[i]}`;
}

function formatTime(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ${seconds % 60}s`;
  const hours = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  return `${hours}h ${mins}m`;
}
```

---

### S1.6: System Tray Integration (5 points)

**Task:** Add system tray icon with menu for quick access to app features.

**Acceptance Criteria:**
- [ ] Tray icon displays on Windows/macOS/Linux
- [ ] Double-click tray icon shows/hides main window
- [ ] Context menu has options: Show, Quit, Pause All Transfers
- [ ] Tray icon tooltip shows active transfer count
- [ ] Badge/overlay icon indicates transfer activity

**Tauri Configuration:**
```rust
// src-tauri/src/tray.rs
use tauri::{
    CustomMenuItem, SystemTray, SystemTrayMenu, SystemTrayEvent,
    SystemTrayMenuItem, AppHandle, Manager
};

pub fn create_tray() -> SystemTray {
    let show = CustomMenuItem::new("show".to_string(), "Show Window");
    let pause = CustomMenuItem::new("pause".to_string(), "Pause All Transfers");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");

    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(pause)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    SystemTray::new().with_menu(tray_menu)
}

pub fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            let window = app.get_window("main").unwrap();
            if window.is_visible().unwrap() {
                window.hide().unwrap();
            } else {
                window.show().unwrap();
                window.set_focus().unwrap();
            }
        }
        SystemTrayEvent::MenuItemClick { id, .. } => {
            match id.as_str() {
                "show" => {
                    let window = app.get_window("main").unwrap();
                    window.show().unwrap();
                    window.set_focus().unwrap();
                }
                "pause" => {
                    // Emit event to pause all transfers
                    app.emit_all("pause-all-transfers", ()).unwrap();
                }
                "quit" => {
                    std::process::exit(0);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

#[tauri::command]
pub fn update_tray_tooltip(
    app: AppHandle,
    active_transfers: usize
) -> Result<(), String> {
    app.tray_handle()
        .set_tooltip(&format!("WRAITH Transfer - {} active", active_transfers))
        .map_err(|e| e.to_string())
}
```

**Main.rs Integration:**
```rust
// src-tauri/src/main.rs
mod tray;

fn main() {
    let tray = tray::create_tray();

    tauri::Builder::default()
        .system_tray(tray)
        .on_system_tray_event(tray::handle_tray_event)
        .invoke_handler(tauri::generate_handler![
            tray::update_tray_tooltip,
            // ... other commands
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

### S1.7: Application Settings Panel (3 points)

**Task:** Create settings UI for configuring application behavior.

**Acceptance Criteria:**
- [ ] Settings persisted to local storage
- [ ] Network settings (port, relay server)
- [ ] UI preferences (theme, language, notifications)
- [ ] Storage location for received files
- [ ] Bandwidth throttling options
- [ ] Reset to defaults button

**React Component:**
```tsx
// src/views/SettingsView.tsx
import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { open } from '@tauri-apps/api/dialog';

interface Settings {
  downloadPath: string;
  listenPort: number;
  relayServer: string;
  theme: 'light' | 'dark' | 'system';
  language: string;
  notificationsEnabled: boolean;
  maxBandwidthMbps: number | null;
}

const DEFAULT_SETTINGS: Settings = {
  downloadPath: '~/Downloads',
  listenPort: 0,
  relayServer: 'wss://relay.wraith.io',
  theme: 'system',
  language: 'en',
  notificationsEnabled: true,
  maxBandwidthMbps: null,
};

export function SettingsView() {
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    const saved = localStorage.getItem('settings');
    if (saved) {
      setSettings(JSON.parse(saved));
    }
  };

  const saveSettings = async () => {
    localStorage.setItem('settings', JSON.stringify(settings));
    await invoke('apply_settings', { settings });
    setDirty(false);
  };

  const selectDownloadPath = async () => {
    const path = await open({
      directory: true,
      multiple: false,
      title: 'Select Download Location'
    });

    if (path) {
      updateSetting('downloadPath', path as string);
    }
  };

  const updateSetting = <K extends keyof Settings>(key: K, value: Settings[K]) => {
    setSettings(prev => ({ ...prev, [key]: value }));
    setDirty(true);
  };

  const resetToDefaults = () => {
    setSettings(DEFAULT_SETTINGS);
    setDirty(true);
  };

  return (
    <div className="settings-view">
      <h2>Settings</h2>

      <section>
        <h3>Storage</h3>
        <div className="setting-row">
          <label>Download Location:</label>
          <div className="path-selector">
            <input
              type="text"
              value={settings.downloadPath}
              readOnly
            />
            <button onClick={selectDownloadPath}>Browse</button>
          </div>
        </div>
      </section>

      <section>
        <h3>Network</h3>
        <div className="setting-row">
          <label>Listen Port:</label>
          <input
            type="number"
            value={settings.listenPort}
            onChange={e => updateSetting('listenPort', parseInt(e.target.value))}
            placeholder="0 = auto"
          />
        </div>
        <div className="setting-row">
          <label>Relay Server:</label>
          <input
            type="text"
            value={settings.relayServer}
            onChange={e => updateSetting('relayServer', e.target.value)}
          />
        </div>
        <div className="setting-row">
          <label>Max Bandwidth (Mbps):</label>
          <input
            type="number"
            value={settings.maxBandwidthMbps ?? ''}
            onChange={e => updateSetting('maxBandwidthMbps', e.target.value ? parseInt(e.target.value) : null)}
            placeholder="Unlimited"
          />
        </div>
      </section>

      <section>
        <h3>Appearance</h3>
        <div className="setting-row">
          <label>Theme:</label>
          <select
            value={settings.theme}
            onChange={e => updateSetting('theme', e.target.value as 'light' | 'dark' | 'system')}
          >
            <option value="light">Light</option>
            <option value="dark">Dark</option>
            <option value="system">System</option>
          </select>
        </div>
        <div className="setting-row">
          <label>Language:</label>
          <select
            value={settings.language}
            onChange={e => updateSetting('language', e.target.value)}
          >
            <option value="en">English</option>
            <option value="es">Español</option>
            <option value="fr">Français</option>
            <option value="de">Deutsch</option>
            <option value="zh">中文</option>
          </select>
        </div>
      </section>

      <section>
        <h3>Notifications</h3>
        <div className="setting-row">
          <label>
            <input
              type="checkbox"
              checked={settings.notificationsEnabled}
              onChange={e => updateSetting('notificationsEnabled', e.target.checked)}
            />
            Enable transfer notifications
          </label>
        </div>
      </section>

      <div className="actions">
        <button onClick={resetToDefaults} className="secondary">
          Reset to Defaults
        </button>
        <button
          onClick={saveSettings}
          disabled={!dirty}
          className="primary"
        >
          Save Changes
        </button>
      </div>
    </div>
  );
}
```

---

### S1.8: Dark/Light Theme Toggle (2 points)

**Task:** Implement theme switching with CSS custom properties.

**Acceptance Criteria:**
- [ ] Theme persists across app restarts
- [ ] Smooth transition between themes (<200ms)
- [ ] All UI components respect theme
- [ ] System theme detection works
- [ ] Manual override option in settings

**CSS Variables:**
```css
/* src/styles/themes.css */
:root[data-theme="light"] {
  --bg-primary: #ffffff;
  --bg-secondary: #f5f5f5;
  --bg-sidebar: #fafafa;
  --bg-active: #e3f2fd;
  --text-primary: #212121;
  --text-secondary: #757575;
  --border-color: #e0e0e0;
  --accent-color: #2196f3;
  --success-color: #4caf50;
  --error-color: #f44336;
  --warning-color: #ff9800;
}

:root[data-theme="dark"] {
  --bg-primary: #1e1e1e;
  --bg-secondary: #252525;
  --bg-sidebar: #181818;
  --bg-active: #2d2d30;
  --text-primary: #d4d4d4;
  --text-secondary: #808080;
  --border-color: #3e3e42;
  --accent-color: #0d7dd8;
  --success-color: #89d185;
  --error-color: #f48771;
  --warning-color: #ffb74d;
}

* {
  transition: background-color 0.15s ease, color 0.15s ease, border-color 0.15s ease;
}
```

**Theme Hook:**
```tsx
// src/hooks/useTheme.ts
import { useState, useEffect } from 'react';

type Theme = 'light' | 'dark' | 'system';

export function useTheme() {
  const [theme, setTheme] = useState<Theme>('system');
  const [resolvedTheme, setResolvedTheme] = useState<'light' | 'dark'>('light');

  useEffect(() => {
    const saved = localStorage.getItem('theme') as Theme;
    if (saved) {
      setTheme(saved);
    }
  }, []);

  useEffect(() => {
    const root = document.documentElement;

    if (theme === 'system') {
      const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      const handleChange = (e: MediaQueryListEvent) => {
        setResolvedTheme(e.matches ? 'dark' : 'light');
      };

      setResolvedTheme(mediaQuery.matches ? 'dark' : 'light');
      mediaQuery.addEventListener('change', handleChange);

      return () => mediaQuery.removeEventListener('change', handleChange);
    } else {
      setResolvedTheme(theme);
    }
  }, [theme]);

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', resolvedTheme);
  }, [resolvedTheme]);

  const changeTheme = (newTheme: Theme) => {
    setTheme(newTheme);
    localStorage.setItem('theme', newTheme);
  };

  return { theme, resolvedTheme, changeTheme };
}
```

---

## Sprint 1 Definition of Done

- [ ] All code compiles without warnings on Windows, macOS, Linux
- [ ] Application launches in <2 seconds
- [ ] Window rendering is smooth (60 FPS)
- [ ] File selection works for single/multiple files and folders
- [ ] Peer connection established via manual key exchange
- [ ] Transfer progress displays real-time updates
- [ ] System tray integration functional on all platforms
- [ ] Settings persist correctly
- [ ] Theme switching works seamlessly
- [ ] No console errors during normal operation
- [ ] Code reviewed and approved
- [ ] Basic user documentation written

---

## Sprint 2: Advanced Features & Platform Polish (Weeks 41-44)

### Sprint Goal
Add multi-file batch transfer support, drag-and-drop, QR code pairing, and platform-specific installers.

**Total Story Points:** 52

### Tasks:
- **S2.1:** Multi-file batch transfer (8 pts) - Queue management, parallel/sequential modes
- **S2.2:** Drag-and-drop file input (3 pts) - Drop zone UI, file validation
- **S2.3:** Transfer history log (5 pts) - SQLite storage, search/filter, export CSV
- **S2.4:** QR code peer pairing (13 pts) - Camera access, QR generation/scanning, key encoding
- **S2.5:** Password-protected transfers (8 pts) - PBKDF2 key derivation, UI for password entry
- **S2.6:** Resume interrupted transfers (8 pts) - Checkpointing, partial file state
- **S2.7:** Platform installers (5 pts) - MSI (Windows), DMG (macOS), deb/rpm (Linux)
- **S2.8:** Auto-update mechanism (2 pts) - Tauri updater integration, version checking

---

## Sprint 3: Hardening & Distribution (Weeks 45-48)

### Sprint Goal
Comprehensive error handling, accessibility, internationalization, performance optimization, and public release preparation.

**Total Story Points:** 52

### Tasks:
- **S3.1:** Error handling & user feedback (8 pts) - Toast notifications, error dialogs, retry logic
- **S3.2:** Accessibility (5 pts) - Keyboard navigation, ARIA labels, screen reader support
- **S3.3:** Internationalization (8 pts) - i18n library integration, 5 language translations
- **S3.4:** Performance optimization (8 pts) - Lazy loading, virtual scrolling, bundle size reduction
- **S3.5:** Cross-platform testing (13 pts) - Manual testing on Win10/11, macOS 11-14, Ubuntu/Fedora
- **S3.6:** Code signing (5 pts) - Certificate procurement, signing pipeline
- **S3.7:** Documentation (3 pts) - User guide, FAQ, troubleshooting, video tutorials
- **S3.8:** Release preparation (2 pts) - Website landing page, download links, GitHub release

---

## Risk Mitigation

**Technical Risks:**
| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Platform-specific bugs | High | Medium | Early cross-platform testing, CI/CD for all targets |
| Performance on low-end hardware | Medium | Medium | Profile on min-spec machines, lazy loading |
| Tauri API limitations | Low | High | Prototype critical features early, fallback to native modules |
| File system permission issues | Medium | High | Comprehensive error handling, permission request UI |

**Schedule Risks:**
| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Protocol API changes | Medium | High | Pin protocol version, coordinate releases |
| UI/UX design iterations | High | Medium | Wireframe approval before implementation |
| Code signing delays | Medium | Low | Start certificate procurement early |
| Translation delays | Low | Low | Use machine translation as fallback |

---

## Completion Checklist

### Sprint 1 (Weeks 37-40):
- [ ] Tauri project builds on all platforms
- [ ] Main window layout complete
- [ ] File selection working
- [ ] Peer connection screen functional
- [ ] Transfer progress displays correctly
- [ ] System tray integration complete
- [ ] Settings panel functional
- [ ] Theme switching works

### Sprint 2 (Weeks 41-44):
- [ ] Multi-file batch transfers work
- [ ] Drag-and-drop implemented
- [ ] Transfer history searchable
- [ ] QR code pairing functional
- [ ] Password protection works
- [ ] Resume transfers implemented
- [ ] Platform installers generated
- [ ] Auto-update mechanism working

### Sprint 3 (Weeks 45-48):
- [ ] Error handling comprehensive
- [ ] Accessibility requirements met
- [ ] 5 languages supported
- [ ] Performance targets achieved
- [ ] Cross-platform testing complete
- [ ] Code signing functional
- [ ] Documentation published
- [ ] Public release deployed

**Target Release Date:** Week 48 (3 months from protocol completion)

---

*WRAITH-Transfer Sprint Planning v1.0.0*
