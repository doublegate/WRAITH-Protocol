# WRAITH-Sync Client - Sprint Planning

**Client Name:** WRAITH-Sync
**Tier:** 2 (Medium Priority)
**Description:** Decentralized file synchronization across devices
**Target Platforms:** Windows, macOS, Linux, iOS, Android
**UI Framework:** Tauri (desktop), React Native (mobile)
**Timeline:** 10 weeks (2.5 sprints × 4 weeks)
**Total Story Points:** 130

---

## Overview

WRAITH-Sync provides Dropbox-like file synchronization without central servers. Files are encrypted end-to-end and synced peer-to-peer across all user devices using the WRAITH protocol.

**Core Value Proposition:**
- Sync folders across unlimited devices
- No cloud storage accounts required
- Military-grade encryption for all synced data
- Selective sync (choose which folders to sync per device)
- Conflict resolution with version history
- Offline-first operation

---

## Success Criteria

**Performance:**
- [ ] Detects file changes within 1 second
- [ ] Syncs 10,000 files in <5 minutes (1 Gbps network)
- [ ] Handles 1GB+ files efficiently
- [ ] Delta sync reduces bandwidth by 90% for modified files
- [ ] Background sync with <50 MB RAM overhead

**Functionality:**
- [ ] Real-time sync across all online devices
- [ ] Conflict detection and resolution UI
- [ ] Version history (30 days or 10 versions)
- [ ] Selective sync configuration per device
- [ ] Pause/resume sync globally or per folder

**Platform Support:**
- [ ] Desktop: Windows 10+, macOS 11+, Linux
- [ ] Mobile: iOS 14+, Android 10+ (selective folders)
- [ ] System tray/menu bar integration
- [ ] Native file system notifications

---

## Dependencies

**Protocol:**
- WRAITH protocol Phases 1-6 (file transfer, DHT discovery)
- File chunking and deduplication support
- Device authentication and key exchange

**External:**
- File system watching (chokidar on desktop, native APIs mobile)
- SQLite for sync metadata database
- rsync-style diff algorithm (rdiff or similar)

---

## Deliverables

**Sprint 1 (Weeks 41-44): Core Sync Engine**
1. File system watcher integration
2. Change detection and event queue
3. Sync metadata database (file hashes, modification times)
4. Device discovery and pairing
5. Bidirectional sync algorithm
6. Conflict detection logic
7. Delta sync implementation (rdiff)
8. Basic CLI for testing

**Sprint 2 (Weeks 45-48): GUI & Advanced Features**
1. Tauri desktop GUI (folder selection, status)
2. React Native mobile GUI
3. Conflict resolution UI
4. Version history viewer
5. Selective sync configuration
6. Bandwidth throttling
7. Sync pause/resume controls
8. System tray/menu bar integration

**Sprint 3 (Weeks 49-50): Polish & Distribution**
1. Comprehensive error handling
2. Large file optimization (chunked uploads)
3. Network interruption recovery
4. Performance profiling and optimization
5. Cross-platform integration testing
6. User documentation
7. Platform-specific installers
8. Public release

---

## Sprint 1: Core Sync Engine (Weeks 41-44)

### S1.1: File System Watcher (8 points)

**Task:** Integrate file system watcher to detect file/folder changes.

**Implementation:**
```typescript
// src/sync/FileWatcher.ts
import chokidar from 'chokidar';
import { EventEmitter } from 'events';

export interface FileChange {
  type: 'add' | 'change' | 'unlink' | 'addDir' | 'unlinkDir';
  path: string;
  stats?: {
    size: number;
    mtime: number;
  };
}

export class FileWatcher extends EventEmitter {
  private watcher: chokidar.FSWatcher | null = null;
  private watchedPaths = new Set<string>();

  addPath(path: string): void {
    if (this.watchedPaths.has(path)) return;

    if (!this.watcher) {
      this.watcher = chokidar.watch(path, {
        persistent: true,
        ignoreInitial: false,
        awaitWriteFinish: {
          stabilityThreshold: 2000,
          pollInterval: 100,
        },
        ignored: [
          /(^|[\/\\])\../, // Hidden files
          '**/.wraith-sync/**', // Metadata folder
          '**/node_modules/**',
        ],
      });

      this.watcher
        .on('add', (path, stats) => this.emit('change', { type: 'add', path, stats }))
        .on('change', (path, stats) => this.emit('change', { type: 'change', path, stats }))
        .on('unlink', path => this.emit('change', { type: 'unlink', path }))
        .on('addDir', path => this.emit('change', { type: 'addDir', path }))
        .on('unlinkDir', path => this.emit('change', { type: 'unlinkDir', path }));
    } else {
      this.watcher.add(path);
    }

    this.watchedPaths.add(path);
  }

  removePath(path: string): void {
    this.watcher?.unwatch(path);
    this.watchedPaths.delete(path);
  }

  close(): void {
    this.watcher?.close();
    this.watcher = null;
    this.watchedPaths.clear();
  }
}
```

---

### S1.2: Sync Metadata Database (13 points)

**Task:** Design database schema for tracking file sync state.

**Schema:**
```sql
-- Synced folders configuration
CREATE TABLE sync_folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    local_path TEXT UNIQUE NOT NULL,
    remote_path TEXT NOT NULL, -- Virtual path shared across devices
    enabled INTEGER DEFAULT 1,
    created_at INTEGER NOT NULL
);

-- File metadata (one row per file per folder)
CREATE TABLE file_metadata (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    folder_id INTEGER NOT NULL,
    relative_path TEXT NOT NULL,
    size INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    hash BLOB NOT NULL, -- BLAKE3 hash
    is_directory INTEGER DEFAULT 0,
    synced INTEGER DEFAULT 0,
    deleted INTEGER DEFAULT 0,
    FOREIGN KEY (folder_id) REFERENCES sync_folders(id) ON DELETE CASCADE,
    UNIQUE (folder_id, relative_path)
);

-- File versions for history
CREATE TABLE file_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL,
    version INTEGER NOT NULL,
    hash BLOB NOT NULL,
    size INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (file_id) REFERENCES file_metadata(id) ON DELETE CASCADE
);

-- Devices in sync group
CREATE TABLE devices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    device_id TEXT UNIQUE NOT NULL, -- Peer ID
    device_name TEXT NOT NULL,
    last_seen INTEGER NOT NULL,
    public_key BLOB NOT NULL
);

-- Sync conflicts
CREATE TABLE conflicts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id INTEGER NOT NULL,
    local_hash BLOB NOT NULL,
    remote_hash BLOB NOT NULL,
    local_modified_at INTEGER NOT NULL,
    remote_modified_at INTEGER NOT NULL,
    device_id TEXT NOT NULL,
    resolved INTEGER DEFAULT 0,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (file_id) REFERENCES file_metadata(id) ON DELETE CASCADE
);

CREATE INDEX idx_file_metadata_folder ON file_metadata(folder_id);
CREATE INDEX idx_file_metadata_path ON file_metadata(relative_path);
CREATE INDEX idx_file_versions_file ON file_versions(file_id);
CREATE INDEX idx_conflicts_resolved ON conflicts(resolved);
```

---

### S1.3: Bidirectional Sync Algorithm (13 points)

**Task:** Implement sync algorithm to reconcile file states across devices.

**Algorithm:**
```typescript
// src/sync/SyncEngine.ts
import { Database } from '../database';
import { WraithClient } from '../wraith';
import { computeBlake3 } from '../crypto';
import * as fs from 'fs/promises';
import * as path from 'path';

export interface SyncState {
  path: string;
  hash: string;
  size: number;
  modifiedAt: number;
  isDirectory: boolean;
}

export class SyncEngine {
  constructor(
    private db: Database,
    private wraith: WraithClient
  ) {}

  async syncFolder(folderId: number, remoteDeviceId: string): Promise<void> {
    // Get local and remote file states
    const localFiles = await this.db.getFolderFiles(folderId);
    const remoteFiles = await this.getRemoteFileStates(remoteDeviceId, folderId);

    const operations: SyncOperation[] = [];

    // Build map of remote files for quick lookup
    const remoteMap = new Map(remoteFiles.map(f => [f.path, f]));

    // Check local files
    for (const localFile of localFiles) {
      const remoteFile = remoteMap.get(localFile.relativePath);

      if (!remoteFile) {
        if (localFile.deleted) {
          // Already deleted, no action
        } else {
          // Local file doesn't exist remotely - upload
          operations.push({
            type: 'upload',
            path: localFile.relativePath,
            localFile,
          });
        }
      } else {
        // File exists on both sides - check for differences
        if (localFile.hash !== remoteFile.hash) {
          // Hashes differ - potential conflict
          if (localFile.modifiedAt > remoteFile.modifiedAt) {
            operations.push({ type: 'upload', path: localFile.relativePath, localFile });
          } else if (localFile.modifiedAt < remoteFile.modifiedAt) {
            operations.push({ type: 'download', path: localFile.relativePath, remoteFile });
          } else {
            // Same timestamp but different content - conflict
            operations.push({
              type: 'conflict',
              path: localFile.relativePath,
              localFile,
              remoteFile,
            });
          }
        }
        // Else: Files are identical, no action needed
      }

      remoteMap.delete(localFile.relativePath);
    }

    // Remaining files in remoteMap don't exist locally - download
    for (const [path, remoteFile] of remoteMap) {
      operations.push({ type: 'download', path, remoteFile });
    }

    // Execute operations
    await this.executeOperations(folderId, remoteDeviceId, operations);
  }

  private async executeOperations(
    folderId: number,
    remoteDeviceId: string,
    operations: SyncOperation[]
  ): Promise<void> {
    for (const op of operations) {
      try {
        switch (op.type) {
          case 'upload':
            await this.uploadFile(folderId, remoteDeviceId, op.localFile!);
            break;
          case 'download':
            await this.downloadFile(folderId, remoteDeviceId, op.remoteFile!);
            break;
          case 'conflict':
            await this.handleConflict(folderId, op.localFile!, op.remoteFile!);
            break;
        }
      } catch (error) {
        console.error(`Sync operation failed for ${op.path}:`, error);
      }
    }
  }

  private async uploadFile(
    folderId: number,
    remoteDeviceId: string,
    localFile: FileMetadata
  ): Promise<void> {
    const folder = await this.db.getSyncFolder(folderId);
    const fullPath = path.join(folder.localPath, localFile.relativePath);

    const fileData = await fs.readFile(fullPath);

    await this.wraith.sendFile({
      peerId: remoteDeviceId,
      virtualPath: path.join(folder.remotePath, localFile.relativePath),
      data: fileData,
      metadata: {
        hash: localFile.hash,
        modifiedAt: localFile.modifiedAt,
        size: localFile.size,
      },
    });

    await this.db.markFileSynced(localFile.id);
  }

  private async downloadFile(
    folderId: number,
    remoteDeviceId: string,
    remoteFile: SyncState
  ): Promise<void> {
    const folder = await this.db.getSyncFolder(folderId);
    const fullPath = path.join(folder.localPath, remoteFile.path);

    const fileData = await this.wraith.receiveFile({
      peerId: remoteDeviceId,
      virtualPath: path.join(folder.remotePath, remoteFile.path),
    });

    // Write file to disk
    await fs.mkdir(path.dirname(fullPath), { recursive: true });
    await fs.writeFile(fullPath, fileData);

    // Verify hash
    const actualHash = await computeBlake3(fileData);
    if (actualHash !== remoteFile.hash) {
      throw new Error('Downloaded file hash mismatch');
    }

    // Update database
    await this.db.upsertFileMetadata(folderId, {
      relativePath: remoteFile.path,
      size: remoteFile.size,
      modifiedAt: remoteFile.modifiedAt,
      hash: remoteFile.hash,
      synced: true,
    });
  }

  private async handleConflict(
    folderId: number,
    localFile: FileMetadata,
    remoteFile: SyncState
  ): Promise<void> {
    // Record conflict in database
    await this.db.createConflict({
      fileId: localFile.id,
      localHash: localFile.hash,
      remoteHash: remoteFile.hash,
      localModifiedAt: localFile.modifiedAt,
      remoteModifiedAt: remoteFile.modifiedAt,
      deviceId: remoteDeviceId,
    });

    // Create conflict copy
    const folder = await this.db.getSyncFolder(folderId);
    const basePath = path.join(folder.localPath, localFile.relativePath);
    const conflictPath = `${basePath}.conflict-${Date.now()}`;

    await fs.copyFile(basePath, conflictPath);

    // Download remote version
    await this.downloadFile(folderId, remoteDeviceId, remoteFile);
  }

  private async getRemoteFileStates(deviceId: string, folderId: number): Promise<SyncState[]> {
    const folder = await this.db.getSyncFolder(folderId);

    const response = await this.wraith.requestFileList({
      peerId: deviceId,
      remotePath: folder.remotePath,
    });

    return response.files;
  }
}

interface SyncOperation {
  type: 'upload' | 'download' | 'conflict';
  path: string;
  localFile?: FileMetadata;
  remoteFile?: SyncState;
}
```

---

### S1.4: Delta Sync (rdiff) (13 points)

**Task:** Implement delta sync to minimize bandwidth for large file modifications.

**Implementation:**
```rust
// src-tauri/src/delta_sync.rs
use blake3;
use std::fs::File;
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;

const BLOCK_SIZE: usize = 4096;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BlockSignature {
    offset: u64,
    weak_hash: u32, // Rolling checksum
    strong_hash: [u8; 32], // BLAKE3
}

#[derive(serde::Serialize, serde::Deserialize)]
pub enum DeltaOp {
    Copy { offset: u64, length: usize },
    Insert { data: Vec<u8> },
}

pub fn generate_signature(file_path: &Path) -> Result<Vec<BlockSignature>, std::io::Error> {
    let mut file = File::open(file_path)?;
    let mut signatures = Vec::new();
    let mut offset = 0u64;
    let mut buffer = vec![0u8; BLOCK_SIZE];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 { break; }

        let block = &buffer[..n];

        signatures.push(BlockSignature {
            offset,
            weak_hash: rolling_checksum(block),
            strong_hash: blake3::hash(block).into(),
        });

        offset += n as u64;
    }

    Ok(signatures)
}

pub fn generate_delta(
    new_file_path: &Path,
    base_signatures: &[BlockSignature],
) -> Result<Vec<DeltaOp>, std::io::Error> {
    let mut file = File::open(new_file_path)?;
    let mut delta_ops = Vec::new();
    let mut buffer = vec![0u8; BLOCK_SIZE * 2];
    let mut window_start = 0usize;

    // Build hash map of base signatures for quick lookup
    let mut sig_map = std::collections::HashMap::new();
    for sig in base_signatures {
        sig_map.insert(sig.weak_hash, sig);
    }

    let mut pending_insert = Vec::new();

    loop {
        let n = file.read(&mut buffer[window_start..])?;
        if n == 0 { break; }

        let total_len = window_start + n;
        let mut i = 0;

        while i < total_len {
            let window_end = std::cmp::min(i + BLOCK_SIZE, total_len);
            let window = &buffer[i..window_end];

            let weak = rolling_checksum(window);

            if let Some(sig) = sig_map.get(&weak) {
                // Potential match - verify with strong hash
                let strong = blake3::hash(window);
                if strong.as_bytes() == &sig.strong_hash {
                    // Match found - flush pending insert and add copy op
                    if !pending_insert.is_empty() {
                        delta_ops.push(DeltaOp::Insert {
                            data: pending_insert.clone(),
                        });
                        pending_insert.clear();
                    }

                    delta_ops.push(DeltaOp::Copy {
                        offset: sig.offset,
                        length: window.len(),
                    });

                    i += window.len();
                    continue;
                }
            }

            // No match - add byte to pending insert
            pending_insert.push(buffer[i]);
            i += 1;
        }

        // Move unprocessed bytes to start of buffer
        buffer.copy_within(i..total_len, 0);
        window_start = total_len - i;
    }

    // Flush remaining pending insert
    if !pending_insert.is_empty() {
        delta_ops.push(DeltaOp::Insert { data: pending_insert });
    }

    Ok(delta_ops)
}

pub fn apply_delta(
    base_file_path: &Path,
    delta_ops: &[DeltaOp],
    output_path: &Path,
) -> Result<(), std::io::Error> {
    let mut base_file = File::open(base_file_path)?;
    let mut output_file = File::create(output_path)?;

    for op in delta_ops {
        match op {
            DeltaOp::Copy { offset, length } => {
                base_file.seek(SeekFrom::Start(*offset))?;
                let mut buffer = vec![0u8; *length];
                base_file.read_exact(&mut buffer)?;
                output_file.write_all(&buffer)?;
            }
            DeltaOp::Insert { data } => {
                output_file.write_all(data)?;
            }
        }
    }

    Ok(())
}

fn rolling_checksum(data: &[u8]) -> u32 {
    // Adler-32 rolling checksum
    const MOD_ADLER: u32 = 65521;

    let mut a = 1u32;
    let mut b = 0u32;

    for &byte in data {
        a = (a + byte as u32) % MOD_ADLER;
        b = (b + a) % MOD_ADLER;
    }

    (b << 16) | a
}

#[tauri::command]
pub fn create_file_signature(path: String) -> Result<Vec<BlockSignature>, String> {
    generate_signature(Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_delta(
    new_file: String,
    base_sigs: Vec<BlockSignature>
) -> Result<Vec<DeltaOp>, String> {
    generate_delta(Path::new(&new_file), &base_sigs).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn patch_file(
    base_file: String,
    delta: Vec<DeltaOp>,
    output_file: String
) -> Result<(), String> {
    apply_delta(
        Path::new(&base_file),
        &delta,
        Path::new(&output_file)
    ).map_err(|e| e.to_string())
}
```

---

## Sprint 2-3 Summary

**Sprint 2 (Weeks 45-48):**
- Tauri desktop GUI (folder selection, sync status dashboard)
- Mobile app (selective sync, battery-optimized)
- Conflict resolution UI
- Version history browser
- Bandwidth throttling and scheduling

**Sprint 3 (Weeks 49-50):**
- Performance optimization (parallel file transfers)
- Network interruption recovery
- Large file chunking (multi-GB files)
- Cross-platform testing
- Installer packaging and release

---

## Completion Checklist

- [ ] File watcher detects changes reliably
- [ ] Bidirectional sync works correctly
- [ ] Delta sync reduces bandwidth by >80%
- [ ] Conflicts detected and UI presented
- [ ] Version history functional
- [ ] Cross-device sync tested (3+ devices)
- [ ] Mobile apps approved for app stores
- [ ] Desktop installers published

**Target Release Date:** Week 50 (10 weeks from start)

---

*WRAITH-Sync Sprint Planning v1.0.0*
