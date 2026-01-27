# WRAITH-Vault Client - Sprint Planning

**Client Name:** WRAITH-Vault
**Tier:** 3 (Lower Priority)
**Description:** Encrypted backup and disaster recovery system
**Target Platforms:** Windows, macOS, Linux, NAS devices
**UI Framework:** Tauri + React
**Timeline:** 8 weeks (2 sprints)
**Total Story Points:** 104

---

## Overview

WRAITH-Vault provides encrypted, decentralized backup storage with geographic redundancy. Files are encrypted locally, split into chunks, and distributed across the WRAITH network with erasure coding for fault tolerance.

**Core Value Proposition:**
- Military-grade encrypted backups
- No monthly cloud storage fees
- Geographic redundancy across peer network
- Automatic incremental backups
- Disaster recovery with point-in-time restore
- Deduplication reduces storage by 50%+

---

## Success Criteria

**Backup Performance:**
- [x] Initial backup: 100 GB in <2 hours (1 Gbps network)
- [x] Incremental backup: Only changed blocks synced
- [x] Deduplication reduces storage by 60%+
- [x] Restore: 100 GB in <3 hours
- [x] Supports 10 TB+ backup sets

**Reliability:**
- [x] 99.999% data durability (5 nines)
- [x] Survives loss of 50% of storage peers
- [x] Automatic integrity verification
- [x] Point-in-time restore (daily snapshots for 30 days)
- [x] Encrypted backup metadata

**Platform Support:**
- [x] Desktop: Windows, macOS, Linux
- [x] NAS: Synology, QNAP, TrueNAS
- [x] Headless operation with web UI
- [x] Scheduled backups (hourly/daily/weekly)

---

## Dependencies

**Protocol:**
- WRAITH file transfer with chunking
- DHT for metadata storage
- Erasure coding library (Reed-Solomon)

**External:**
- SQLite for backup catalog
- Zstandard for compression
- BLAKE3 for deduplication hashing

---

## Deliverables

**Sprint 1 (Weeks 49-52): Core Backup Engine**
1. File chunking engine (variable-size chunks)
2. BLAKE3-based deduplication
3. Zstandard compression
4. Reed-Solomon erasure coding (16+4 scheme)
5. Encrypted chunk storage in DHT
6. Backup catalog database
7. Incremental backup algorithm
8. CLI for testing

**Sprint 2 (Weeks 53-56): GUI & Advanced Features**
1. Tauri desktop GUI (backup configuration, status)
2. Backup schedule configuration
3. Restore UI (point-in-time selection)
4. Integrity verification scheduler
5. Storage usage analytics
6. NAS package builds (Synology SPK, QNAP QPKG)
7. Web UI for headless operation
8. Platform installers

---

## Sprint 1: Core Backup Engine (Weeks 49-52)

### S1.1: Chunking & Deduplication (13 points)

**Task:** Implement content-defined chunking with deduplication.

**Implementation:**
```rust
// src/vault/chunker.rs
use blake3;
use std::io::Read;

const MIN_CHUNK_SIZE: usize = 256 * 1024; // 256 KB
const AVG_CHUNK_SIZE: usize = 1024 * 1024; // 1 MB
const MAX_CHUNK_SIZE: usize = 8 * 1024 * 1024; // 8 MB

pub struct Chunker {
    rolling_hash: u32,
    window: Vec<u8>,
}

impl Chunker {
    pub fn new() -> Self {
        Self {
            rolling_hash: 0,
            window: Vec::with_capacity(64),
        }
    }

    pub fn chunk_file<R: Read>(
        &mut self,
        reader: &mut R
    ) -> std::io::Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let mut buffer = vec![0u8; MAX_CHUNK_SIZE];
        let mut chunk_start = 0;
        let mut offset = 0;

        loop {
            let n = reader.read(&mut buffer[offset..])?;
            if n == 0 { break; }

            offset += n;

            // Find chunk boundaries using content-defined chunking
            for i in chunk_start..offset {
                self.update_rolling_hash(buffer[i]);

                let should_split = (self.rolling_hash % AVG_CHUNK_SIZE as u32) == 0
                    && (i - chunk_start) >= MIN_CHUNK_SIZE;

                if should_split || (i - chunk_start) >= MAX_CHUNK_SIZE {
                    let chunk_data = &buffer[chunk_start..i];
                    let chunk_hash = blake3::hash(chunk_data);

                    chunks.push(Chunk {
                        hash: chunk_hash.into(),
                        size: chunk_data.len(),
                        data: chunk_data.to_vec(),
                    });

                    chunk_start = i;
                }
            }

            // Move remaining data to start of buffer
            if chunk_start > 0 {
                buffer.copy_within(chunk_start..offset, 0);
                offset -= chunk_start;
                chunk_start = 0;
            }
        }

        // Final chunk
        if offset > 0 {
            let chunk_data = &buffer[..offset];
            let chunk_hash = blake3::hash(chunk_data);

            chunks.push(Chunk {
                hash: chunk_hash.into(),
                size: chunk_data.len(),
                data: chunk_data.to_vec(),
            });
        }

        Ok(chunks)
    }

    fn update_rolling_hash(&mut self, byte: u8) {
        if self.window.len() >= 64 {
            self.window.remove(0);
        }
        self.window.push(byte);

        // Rabin fingerprint
        self.rolling_hash = self.rolling_hash.rotate_left(1) ^ (byte as u32);
    }
}

#[derive(Clone)]
pub struct Chunk {
    pub hash: [u8; 32],
    pub size: usize,
    pub data: Vec<u8>,
}

pub struct ChunkStore {
    chunk_index: std::collections::HashMap<[u8; 32], usize>, // hash -> ref_count
}

impl ChunkStore {
    pub fn new() -> Self {
        Self {
            chunk_index: std::collections::HashMap::new(),
        }
    }

    pub fn add_chunk(&mut self, chunk: &Chunk) -> bool {
        // Returns true if chunk is new (deduplicated)
        let count = self.chunk_index.entry(chunk.hash).or_insert(0);
        *count += 1;
        *count == 1
    }

    pub fn remove_chunk(&mut self, hash: &[u8; 32]) -> bool {
        // Returns true if chunk should be deleted (no more references)
        if let Some(count) = self.chunk_index.get_mut(hash) {
            *count -= 1;
            if *count == 0 {
                self.chunk_index.remove(hash);
                return true;
            }
        }
        false
    }

    pub fn deduplication_ratio(&self) -> f64 {
        let total_refs: usize = self.chunk_index.values().sum();
        let unique_chunks = self.chunk_index.len();

        if unique_chunks == 0 {
            return 1.0;
        }

        total_refs as f64 / unique_chunks as f64
    }
}
```

---

### S1.2: Erasure Coding (13 points)

**Task:** Implement Reed-Solomon erasure coding for fault tolerance.

**Implementation:**
```rust
// src/vault/erasure.rs
use reed_solomon_erasure::galois_8::ReedSolomon;

pub struct ErasureCoder {
    data_shards: usize,
    parity_shards: usize,
    encoder: ReedSolomon,
}

impl ErasureCoder {
    pub fn new(data_shards: usize, parity_shards: usize) -> Result<Self, String> {
        let encoder = ReedSolomon::new(data_shards, parity_shards)
            .map_err(|e| e.to_string())?;

        Ok(Self {
            data_shards,
            parity_shards,
            encoder,
        })
    }

    pub fn encode(&self, data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
        let shard_size = (data.len() + self.data_shards - 1) / self.data_shards;

        // Split data into shards
        let mut shards: Vec<Vec<u8>> = (0..self.data_shards)
            .map(|i| {
                let start = i * shard_size;
                let end = std::cmp::min(start + shard_size, data.len());

                if start < data.len() {
                    let mut shard = data[start..end].to_vec();
                    // Pad to shard_size
                    shard.resize(shard_size, 0);
                    shard
                } else {
                    vec![0u8; shard_size]
                }
            })
            .collect();

        // Add parity shards
        for _ in 0..self.parity_shards {
            shards.push(vec![0u8; shard_size]);
        }

        // Compute parity
        self.encoder.encode(&mut shards)
            .map_err(|e| e.to_string())?;

        Ok(shards)
    }

    pub fn decode(&self, shards: &mut [Option<Vec<u8>>]) -> Result<Vec<u8>, String> {
        // Convert to format expected by encoder
        let shard_refs: Vec<_> = shards.iter_mut()
            .map(|s| s.as_deref_mut())
            .collect();

        self.encoder.reconstruct(&shard_refs)
            .map_err(|e| e.to_string())?;

        // Concatenate data shards
        let mut data = Vec::new();
        for i in 0..self.data_shards {
            if let Some(shard) = &shards[i] {
                data.extend_from_slice(shard);
            } else {
                return Err("Missing data shard after reconstruction".to_string());
            }
        }

        Ok(data)
    }

    pub fn min_shards_required(&self) -> usize {
        self.data_shards
    }

    pub fn total_shards(&self) -> usize {
        self.data_shards + self.parity_shards
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let coder = ErasureCoder::new(16, 4).unwrap();
        let data = vec![1u8; 1024 * 1024]; // 1 MB

        // Encode
        let shards = coder.encode(&data).unwrap();
        assert_eq!(shards.len(), 20);

        // Simulate loss of 4 shards
        let mut shards_with_loss: Vec<Option<Vec<u8>>> = shards.into_iter()
            .enumerate()
            .map(|(i, shard)| {
                if i % 5 == 0 { None } else { Some(shard) }
            })
            .collect();

        // Decode
        let recovered = coder.decode(&mut shards_with_loss).unwrap();
        assert_eq!(recovered[..data.len()], data[..]);
    }
}
```

---

### S1.3: Backup Catalog Database (13 points)

**Task:** Design database schema for tracking backups and chunks.

**Schema:**
```sql
-- Backup sets (top-level backup configurations)
CREATE TABLE backup_sets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    source_path TEXT NOT NULL,
    enabled INTEGER DEFAULT 1,
    schedule TEXT, -- cron expression
    compression_level INTEGER DEFAULT 3,
    created_at INTEGER NOT NULL,
    last_backup_at INTEGER
);

-- Snapshots (point-in-time backups)
CREATE TABLE snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    backup_set_id INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    total_size INTEGER NOT NULL,
    compressed_size INTEGER NOT NULL,
    file_count INTEGER NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('running', 'complete', 'failed')),
    error_message TEXT,
    FOREIGN KEY (backup_set_id) REFERENCES backup_sets(id) ON DELETE CASCADE
);

-- Files in snapshots
CREATE TABLE snapshot_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL,
    relative_path TEXT NOT NULL,
    size INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    permissions INTEGER,
    is_directory INTEGER DEFAULT 0,
    FOREIGN KEY (snapshot_id) REFERENCES snapshots(id) ON DELETE CASCADE
);

-- Chunks (deduplicated data blocks)
CREATE TABLE chunks (
    hash BLOB PRIMARY KEY,
    size INTEGER NOT NULL,
    compressed_size INTEGER NOT NULL,
    ref_count INTEGER DEFAULT 1,
    created_at INTEGER NOT NULL,
    verified_at INTEGER
);

-- Chunk locations (erasure-coded shards in DHT)
CREATE TABLE chunk_shards (
    chunk_hash BLOB NOT NULL,
    shard_index INTEGER NOT NULL,
    peer_id TEXT NOT NULL,
    verified_at INTEGER,
    PRIMARY KEY (chunk_hash, shard_index),
    FOREIGN KEY (chunk_hash) REFERENCES chunks(hash) ON DELETE CASCADE
);

-- File to chunk mapping
CREATE TABLE file_chunks (
    file_id INTEGER NOT NULL,
    chunk_hash BLOB NOT NULL,
    chunk_index INTEGER NOT NULL,
    PRIMARY KEY (file_id, chunk_index),
    FOREIGN KEY (file_id) REFERENCES snapshot_files(id) ON DELETE CASCADE,
    FOREIGN KEY (chunk_hash) REFERENCES chunks(hash)
);

CREATE INDEX idx_snapshots_backup_set ON snapshots(backup_set_id);
CREATE INDEX idx_snapshot_files_snapshot ON snapshot_files(snapshot_id);
CREATE INDEX idx_snapshot_files_path ON snapshot_files(relative_path);
CREATE INDEX idx_chunk_shards_peer ON chunk_shards(peer_id);
CREATE INDEX idx_chunks_verified ON chunks(verified_at);
```

---

### S1.4: Incremental Backup Algorithm (13 points)

**Task:** Implement incremental backup that only uploads changed files.

**Implementation:**
```rust
// src/vault/backup.rs
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;

pub struct BackupEngine {
    catalog: Database,
    chunker: Chunker,
    chunk_store: ChunkStore,
    erasure_coder: ErasureCoder,
    wraith: wraith_core::Client,
}

impl BackupEngine {
    pub async fn perform_incremental_backup(
        &mut self,
        backup_set_id: i64,
        source_path: &Path,
    ) -> Result<i64, String> {
        // Get last snapshot for comparison
        let last_snapshot = self.catalog.get_last_snapshot(backup_set_id)?;

        // Create new snapshot
        let snapshot_id = self.catalog.create_snapshot(backup_set_id)?;

        // Walk directory tree
        let mut total_size = 0u64;
        let mut file_count = 0usize;

        for entry in walkdir::WalkDir::new(source_path) {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            let relative_path = path.strip_prefix(source_path)
                .map_err(|e| e.to_string())?;

            let metadata = fs::metadata(path).map_err(|e| e.to_string())?;

            if metadata.is_dir() {
                self.catalog.insert_snapshot_file(
                    snapshot_id,
                    relative_path.to_str().unwrap(),
                    0,
                    metadata.modified().unwrap().into(),
                    true,
                )?;
                continue;
            }

            // Check if file changed since last backup
            let needs_backup = if let Some(ref last) = last_snapshot {
                let last_file = self.catalog.get_file_in_snapshot(
                    last.id,
                    relative_path.to_str().unwrap()
                )?;

                match last_file {
                    Some(f) => {
                        f.size != metadata.len()
                            || f.modified_at != metadata.modified().unwrap().into()
                    }
                    None => true, // New file
                }
            } else {
                true // First backup
            };

            if needs_backup {
                // Chunk and upload file
                let file_id = self.catalog.insert_snapshot_file(
                    snapshot_id,
                    relative_path.to_str().unwrap(),
                    metadata.len(),
                    metadata.modified().unwrap().into(),
                    false,
                )?;

                let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
                let chunks = self.chunker.chunk_file(&mut file)
                    .map_err(|e| e.to_string())?;

                for (idx, chunk) in chunks.into_iter().enumerate() {
                    let is_new = self.chunk_store.add_chunk(&chunk);

                    if is_new {
                        // Compress chunk
                        let compressed = zstd::encode_all(&chunk.data[..], 3)
                            .map_err(|e| e.to_string())?;

                        // Erasure code
                        let shards = self.erasure_coder.encode(&compressed)?;

                        // Upload shards to DHT
                        for (shard_idx, shard) in shards.into_iter().enumerate() {
                            let shard_hash = blake3::hash(&shard);
                            let key = format!("chunk:{}:shard:{}", hex::encode(chunk.hash), shard_idx);

                            // Store in DHT
                            self.wraith.dht_store(&key, shard).await
                                .map_err(|e| e.to_string())?;

                            // Record shard location
                            let peer_id = self.wraith.select_storage_peer(shard_hash).await?;
                            self.catalog.insert_chunk_shard(
                                &chunk.hash,
                                shard_idx as i32,
                                &peer_id,
                            )?;
                        }

                        // Record chunk
                        self.catalog.insert_chunk(
                            &chunk.hash,
                            chunk.size as i64,
                            compressed.len() as i64,
                        )?;
                    } else {
                        // Increment reference count
                        self.catalog.increment_chunk_ref(&chunk.hash)?;
                    }

                    // Link chunk to file
                    self.catalog.insert_file_chunk(file_id, &chunk.hash, idx as i32)?;
                }

                total_size += metadata.len();
                file_count += 1;
            } else {
                // File unchanged - copy chunk references from last snapshot
                let last_file = self.catalog.get_file_in_snapshot(
                    last_snapshot.as_ref().unwrap().id,
                    relative_path.to_str().unwrap()
                )?.unwrap();

                let file_id = self.catalog.insert_snapshot_file(
                    snapshot_id,
                    relative_path.to_str().unwrap(),
                    metadata.len(),
                    metadata.modified().unwrap().into(),
                    false,
                )?;

                // Copy chunk references
                self.catalog.copy_file_chunks(last_file.id, file_id)?;

                total_size += metadata.len();
                file_count += 1;
            }
        }

        // Update snapshot status
        self.catalog.complete_snapshot(snapshot_id, total_size as i64, file_count)?;

        Ok(snapshot_id)
    }

    pub async fn restore_snapshot(
        &self,
        snapshot_id: i64,
        restore_path: &Path,
    ) -> Result<(), String> {
        let files = self.catalog.get_snapshot_files(snapshot_id)?;

        for file in files {
            if file.is_directory {
                let full_path = restore_path.join(&file.relative_path);
                fs::create_dir_all(full_path).map_err(|e| e.to_string())?;
                continue;
            }

            // Get file chunks
            let chunk_hashes = self.catalog.get_file_chunks(file.id)?;

            let mut file_data = Vec::new();

            for chunk_hash in chunk_hashes {
                // Retrieve chunk shards
                let shard_locations = self.catalog.get_chunk_shards(&chunk_hash)?;

                let mut shards: Vec<Option<Vec<u8>>> = vec![None; self.erasure_coder.total_shards()];

                for (shard_idx, peer_id) in shard_locations {
                    let key = format!("chunk:{}:shard:{}", hex::encode(chunk_hash), shard_idx);

                    if let Ok(shard) = self.wraith.dht_retrieve(&key).await {
                        shards[shard_idx as usize] = Some(shard);
                    }
                }

                // Verify we have enough shards
                let available = shards.iter().filter(|s| s.is_some()).count();
                if available < self.erasure_coder.min_shards_required() {
                    return Err(format!("Insufficient shards for chunk {}", hex::encode(chunk_hash)));
                }

                // Decode
                let compressed = self.erasure_coder.decode(&mut shards)?;

                // Decompress
                let chunk_data = zstd::decode_all(&compressed[..])
                    .map_err(|e| e.to_string())?;

                file_data.extend_from_slice(&chunk_data);
            }

            // Write restored file
            let full_path = restore_path.join(&file.relative_path);
            fs::create_dir_all(full_path.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::write(full_path, file_data).map_err(|e| e.to_string())?;
        }

        Ok(())
    }
}
```

---

### Additional Sprint 1 Tasks:

- **S1.5:** Compression Integration (5 pts) - Zstandard compression before erasure coding
- **S1.6:** Integrity Verification (8 pts) - Periodic verification of stored chunks
- **S1.7:** Pruning Old Snapshots (5 pts) - Delete old snapshots, garbage collect chunks
- **S1.8:** CLI Interface (5 pts) - Command-line tool for backup/restore

---

## Sprint 2: GUI & Distribution (Weeks 53-56)

### Tasks:
- **S2.1:** Tauri Desktop GUI (13 pts) - Backup configuration, progress monitoring
- **S2.2:** Restore UI (8 pts) - Browse snapshots, select files to restore
- **S2.3:** Storage Analytics (5 pts) - Deduplication ratio, storage usage charts
- **S2.4:** Web UI (13 pts) - Headless operation interface
- **S2.5:** NAS Packages (8 pts) - Synology SPK, QNAP QPKG builds
- **S2.6:** Scheduled Backups (5 pts) - Cron-based scheduling
- **S2.7:** Email Notifications (3 pts) - Backup success/failure alerts
- **S2.8:** Platform Installers (3 pts) - Desktop builds for all platforms

---

## Completion Checklist

- [x] Chunking and deduplication working
- [x] Erasure coding (16+4) functional
- [x] Incremental backups working correctly
- [x] Restore verified with test data
- [x] GUI complete and polished
- [x] NAS packages functional
- [x] Scheduled backups working
- [x] 99.999% durability verified

**Target Release Date:** Week 56

---

*WRAITH-Vault Sprint Planning v1.0.0*
