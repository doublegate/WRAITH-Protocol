# Phase 6: Integration & File Transfer Sprint Planning

**Duration:** Weeks 32-36 (4-5 weeks)
**Total Story Points:** 98
**Risk Level:** Medium (integration complexity)

---

## Phase Overview

**Goal:** Integrate all protocol components (crypto, transport, obfuscation, discovery) into a cohesive file transfer engine with chunking, tree hashing, multi-peer downloads, and congestion control. Implement the CLI interface for end-to-end testing.

### Success Criteria

- [ ] Complete file transfer (1GB): <10 seconds (1 Gbps LAN)
- [ ] Resume works after interruption
- [ ] Multi-peer speedup: ~linear up to 5 peers
- [ ] BBR achieves >95% bandwidth utilization
- [ ] CLI functional for send/receive
- [ ] Integration tests pass
- [ ] End-to-end encryption verified

### Dependencies

- Phases 1-5 complete (all protocol components)
- All crates (wraith-core, wraith-crypto, wraith-transport, wraith-obfuscation, wraith-discovery) functional

### Deliverables

1. File chunking engine (256 KiB chunks)
2. BLAKE3 tree hashing
3. Transfer state machine
4. Resume/seek support
5. Multi-peer parallel download
6. Progress tracking
7. BBR congestion control
8. Flow control
9. Loss detection & recovery
10. CLI implementation (`wraith send/recv`)
11. Configuration system
12. Integration tests

---

## Sprint Breakdown

### Sprint 6.1: File Chunking & Hashing (Weeks 32-33)

**Duration:** 1.5 weeks
**Story Points:** 21

**6.1.1: File Chunking** (8 SP)

```rust
// wraith-files/src/chunking.rs

use std::path::Path;
use std::io::{self, Read, Seek, SeekFrom};
use std::fs::File;

/// Default chunk size (256 KiB)
pub const DEFAULT_CHUNK_SIZE: usize = 256 * 1024;

/// File chunk metadata
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    pub index: u64,
    pub offset: u64,
    pub size: usize,
    pub hash: [u8; 32], // BLAKE3 hash
}

/// File chunker
pub struct FileChunker {
    file: File,
    chunk_size: usize,
    total_size: u64,
    current_offset: u64,
}

impl FileChunker {
    pub fn new<P: AsRef<Path>>(path: P, chunk_size: usize) -> io::Result<Self> {
        let file = File::open(path)?;
        let total_size = file.metadata()?.len();

        Ok(Self {
            file,
            chunk_size,
            total_size,
            current_offset: 0,
        })
    }

    /// Get total number of chunks
    pub fn num_chunks(&self) -> u64 {
        (self.total_size + self.chunk_size as u64 - 1) / self.chunk_size as u64
    }

    /// Read next chunk
    pub fn read_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        if self.current_offset >= self.total_size {
            return Ok(None);
        }

        let remaining = self.total_size - self.current_offset;
        let chunk_len = remaining.min(self.chunk_size as u64) as usize;

        let mut buffer = vec![0u8; chunk_len];
        self.file.read_exact(&mut buffer)?;

        self.current_offset += chunk_len as u64;

        Ok(Some(buffer))
    }

    /// Seek to specific chunk
    pub fn seek_to_chunk(&mut self, chunk_index: u64) -> io::Result<()> {
        let offset = chunk_index * self.chunk_size as u64;

        if offset >= self.total_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Chunk index out of bounds"
            ));
        }

        self.file.seek(SeekFrom::Start(offset))?;
        self.current_offset = offset;

        Ok(())
    }

    /// Read specific chunk by index
    pub fn read_chunk_at(&mut self, chunk_index: u64) -> io::Result<Vec<u8>> {
        self.seek_to_chunk(chunk_index)?;
        self.read_chunk()?.ok_or_else(|| {
            io::Error::new(io::ErrorKind::UnexpectedEof, "Chunk not found")
        })
    }

    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

/// File reassembler (receive side)
pub struct FileReassembler {
    file: File,
    chunk_size: usize,
    total_chunks: u64,
    received_chunks: std::collections::HashSet<u64>,
}

impl FileReassembler {
    pub fn new<P: AsRef<Path>>(
        path: P,
        total_size: u64,
        chunk_size: usize,
    ) -> io::Result<Self> {
        use std::fs::OpenOptions;

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(path)?;

        // Pre-allocate file
        file.set_len(total_size)?;

        let total_chunks = (total_size + chunk_size as u64 - 1) / chunk_size as u64;

        Ok(Self {
            file,
            chunk_size,
            total_chunks,
            received_chunks: std::collections::HashSet::new(),
        })
    }

    /// Write chunk at specific index
    pub fn write_chunk(&mut self, chunk_index: u64, data: &[u8]) -> io::Result<()> {
        use std::io::Write;

        if chunk_index >= self.total_chunks {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Chunk index out of bounds"
            ));
        }

        let offset = chunk_index * self.chunk_size as u64;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(data)?;

        self.received_chunks.insert(chunk_index);

        Ok(())
    }

    /// Check if chunk is received
    pub fn has_chunk(&self, chunk_index: u64) -> bool {
        self.received_chunks.contains(&chunk_index)
    }

    /// Get missing chunk indices
    pub fn missing_chunks(&self) -> Vec<u64> {
        (0..self.total_chunks)
            .filter(|i| !self.received_chunks.contains(i))
            .collect()
    }

    /// Check if transfer is complete
    pub fn is_complete(&self) -> bool {
        self.received_chunks.len() as u64 == self.total_chunks
    }

    /// Sync file to disk
    pub fn sync(&mut self) -> io::Result<()> {
        self.file.sync_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_chunking_roundtrip() {
        // Create test file
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = vec![0xAA; 1024 * 1024]; // 1 MB
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        // Chunk file
        let mut chunker = FileChunker::new(temp_file.path(), DEFAULT_CHUNK_SIZE).unwrap();
        assert_eq!(chunker.num_chunks(), 4); // 1MB / 256KB = 4 chunks

        // Read all chunks
        let mut chunks = Vec::new();
        while let Some(chunk) = chunker.read_chunk().unwrap() {
            chunks.push(chunk);
        }

        assert_eq!(chunks.len(), 4);

        // Reassemble
        let output_file = NamedTempFile::new().unwrap();
        let mut reassembler = FileReassembler::new(
            output_file.path(),
            data.len() as u64,
            DEFAULT_CHUNK_SIZE
        ).unwrap();

        for (i, chunk) in chunks.iter().enumerate() {
            reassembler.write_chunk(i as u64, chunk).unwrap();
        }

        assert!(reassembler.is_complete());
        reassembler.sync().unwrap();

        // Verify
        let reconstructed = std::fs::read(output_file.path()).unwrap();
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_seek_to_chunk() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&vec![0u8; 1024 * 1024]).unwrap();
        temp_file.flush().unwrap();

        let mut chunker = FileChunker::new(temp_file.path(), DEFAULT_CHUNK_SIZE).unwrap();

        // Read chunk 2 directly
        chunker.seek_to_chunk(2).unwrap();
        let chunk = chunker.read_chunk().unwrap().unwrap();

        assert_eq!(chunk.len(), DEFAULT_CHUNK_SIZE);
    }
}
```

**Acceptance Criteria:**
- [ ] File chunking works for any file size
- [ ] Chunk seeking functional
- [ ] Reassembly handles out-of-order chunks
- [ ] Resume support (missing chunks tracking)
- [ ] Pre-allocation for faster writes

---

**6.1.2: BLAKE3 Tree Hashing** (13 SP)

```rust
// wraith-files/src/tree_hash.rs

use blake3::Hasher;
use std::path::Path;
use std::io::{self, Read};
use std::fs::File;

/// File tree hash (BLAKE3)
#[derive(Debug, Clone)]
pub struct FileTreeHash {
    /// Root hash
    pub root: [u8; 32],
    /// Chunk hashes (leaf nodes)
    pub chunks: Vec<[u8; 32]>,
}

/// Compute tree hash for file
pub fn compute_tree_hash<P: AsRef<Path>>(
    path: P,
    chunk_size: usize,
) -> io::Result<FileTreeHash> {
    let mut file = File::open(path)?;
    let total_size = file.metadata()?.len();

    let num_chunks = (total_size + chunk_size as u64 - 1) / chunk_size as u64;
    let mut chunk_hashes = Vec::with_capacity(num_chunks as usize);

    // Hash each chunk
    let mut buffer = vec![0u8; chunk_size];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let chunk_hash = blake3::hash(&buffer[..bytes_read]);
        chunk_hashes.push(chunk_hash.into());
    }

    // Build Merkle tree
    let root = compute_merkle_root(&chunk_hashes);

    Ok(FileTreeHash {
        root,
        chunks: chunk_hashes,
    })
}

/// Compute Merkle root from leaf hashes
fn compute_merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }

    if leaves.len() == 1 {
        return leaves[0];
    }

    let mut current_level = leaves.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::new();

        for pair in current_level.chunks(2) {
            let hash = if pair.len() == 2 {
                // Hash concatenation of two nodes
                let mut hasher = Hasher::new();
                hasher.update(&pair[0]);
                hasher.update(&pair[1]);
                hasher.finalize().into()
            } else {
                // Odd number, promote single node
                pair[0]
            };

            next_level.push(hash);
        }

        current_level = next_level;
    }

    current_level[0]
}

/// Verify chunk against tree
pub fn verify_chunk(
    chunk_index: usize,
    chunk_data: &[u8],
    tree: &FileTreeHash,
) -> bool {
    if chunk_index >= tree.chunks.len() {
        return false;
    }

    let computed_hash = blake3::hash(chunk_data);
    computed_hash.as_bytes() == &tree.chunks[chunk_index]
}

/// Incremental tree hasher (for streaming)
pub struct IncrementalTreeHasher {
    chunk_hashes: Vec<[u8; 32]>,
    current_buffer: Vec<u8>,
    chunk_size: usize,
}

impl IncrementalTreeHasher {
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_hashes: Vec::new(),
            current_buffer: Vec::new(),
            chunk_size,
        }
    }

    /// Update with new data
    pub fn update(&mut self, data: &[u8]) {
        self.current_buffer.extend_from_slice(data);

        // Process complete chunks
        while self.current_buffer.len() >= self.chunk_size {
            let chunk = self.current_buffer.drain(..self.chunk_size).collect::<Vec<_>>();
            let hash = blake3::hash(&chunk);
            self.chunk_hashes.push(hash.into());
        }
    }

    /// Finalize and get tree hash
    pub fn finalize(mut self) -> FileTreeHash {
        // Hash remaining data
        if !self.current_buffer.is_empty() {
            let hash = blake3::hash(&self.current_buffer);
            self.chunk_hashes.push(hash.into());
        }

        let root = compute_merkle_root(&self.chunk_hashes);

        FileTreeHash {
            root,
            chunks: self.chunk_hashes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_tree_hash_computation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = vec![0xAA; 1024 * 1024]; // 1 MB
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        let tree = compute_tree_hash(temp_file.path(), 256 * 1024).unwrap();

        assert_eq!(tree.chunks.len(), 4); // 1MB / 256KB
        assert_ne!(tree.root, [0u8; 32]);
    }

    #[test]
    fn test_chunk_verification() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = vec![0xAA; 512 * 1024]; // 512 KB
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        let tree = compute_tree_hash(temp_file.path(), 256 * 1024).unwrap();

        // Verify first chunk
        let chunk = vec![0xAA; 256 * 1024];
        assert!(verify_chunk(0, &chunk, &tree));

        // Verify with wrong data
        let wrong_chunk = vec![0xBB; 256 * 1024];
        assert!(!verify_chunk(0, &wrong_chunk, &tree));
    }

    #[test]
    fn test_incremental_hasher() {
        let data = vec![0xAA; 1024 * 1024];

        let mut hasher = IncrementalTreeHasher::new(256 * 1024);

        // Feed data in 64KB chunks
        for chunk in data.chunks(64 * 1024) {
            hasher.update(chunk);
        }

        let tree = hasher.finalize();

        assert_eq!(tree.chunks.len(), 4);
    }

    #[test]
    fn test_merkle_root_single_leaf() {
        let leaf = [[1u8; 32]];
        let root = compute_merkle_root(&leaf);
        assert_eq!(root, leaf[0]);
    }

    #[test]
    fn test_merkle_root_multiple_leaves() {
        let leaves = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        let root = compute_merkle_root(&leaves);

        // Root should be different from any leaf
        for leaf in &leaves {
            assert_ne!(root, *leaf);
        }
    }
}
```

**Acceptance Criteria:**
- [ ] Tree hash computed correctly
- [ ] Chunk verification works
- [ ] Merkle tree structure valid
- [ ] Incremental hashing functional
- [ ] Performance: >500 MB/s hashing

---

### Sprint 6.2: Transfer State Machine (Week 33-34)

**Duration:** 1.5 weeks
**Story Points:** 26

**6.2.1: Transfer Session** (13 SP)

```rust
// wraith-core/src/transfer/session.rs

use std::path::PathBuf;
use std::time::Instant;
use std::collections::{HashMap, HashSet};

/// Transfer session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    Initializing,
    Handshaking,
    Transferring,
    Paused,
    Completing,
    Complete,
    Failed,
}

/// Transfer direction
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Send,
    Receive,
}

/// Transfer session
pub struct TransferSession {
    pub id: [u8; 32],
    pub direction: Direction,
    pub file_path: PathBuf,
    pub file_size: u64,
    pub chunk_size: usize,
    pub tree_hash: crate::files::FileTreeHash,
    state: TransferState,

    // Progress tracking
    transferred_chunks: HashSet<u64>,
    total_chunks: u64,
    bytes_transferred: u64,

    // Performance tracking
    started_at: Option<Instant>,
    completed_at: Option<Instant>,

    // Peer tracking (for multi-peer downloads)
    peers: HashMap<PeerId, PeerTransferState>,
}

type PeerId = [u8; 32];

struct PeerTransferState {
    assigned_chunks: HashSet<u64>,
    downloaded_chunks: u64,
    last_activity: Instant,
}

impl TransferSession {
    pub fn new_send(
        id: [u8; 32],
        file_path: PathBuf,
        file_size: u64,
        chunk_size: usize,
        tree_hash: crate::files::FileTreeHash,
    ) -> Self {
        let total_chunks = (file_size + chunk_size as u64 - 1) / chunk_size as u64;

        Self {
            id,
            direction: Direction::Send,
            file_path,
            file_size,
            chunk_size,
            tree_hash,
            state: TransferState::Initializing,
            transferred_chunks: HashSet::new(),
            total_chunks,
            bytes_transferred: 0,
            started_at: None,
            completed_at: None,
            peers: HashMap::new(),
        }
    }

    pub fn new_receive(
        id: [u8; 32],
        file_path: PathBuf,
        file_size: u64,
        chunk_size: usize,
        tree_hash: crate::files::FileTreeHash,
    ) -> Self {
        let total_chunks = (file_size + chunk_size as u64 - 1) / chunk_size as u64;

        Self {
            id,
            direction: Direction::Receive,
            file_path,
            file_size,
            chunk_size,
            tree_hash,
            state: TransferState::Initializing,
            transferred_chunks: HashSet::new(),
            total_chunks,
            bytes_transferred: 0,
            started_at: None,
            completed_at: None,
            peers: HashMap::new(),
        }
    }

    /// Start transfer
    pub fn start(&mut self) {
        self.state = TransferState::Transferring;
        self.started_at = Some(Instant::now());
    }

    /// Mark chunk as transferred
    pub fn mark_chunk_transferred(&mut self, chunk_index: u64, chunk_size: usize) {
        if self.transferred_chunks.insert(chunk_index) {
            self.bytes_transferred += chunk_size as u64;
        }

        // Check if complete
        if self.transferred_chunks.len() as u64 == self.total_chunks {
            self.state = TransferState::Complete;
            self.completed_at = Some(Instant::now());
        }
    }

    /// Get progress (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        self.bytes_transferred as f64 / self.file_size as f64
    }

    /// Get transfer speed (bytes/sec)
    pub fn speed(&self) -> Option<f64> {
        self.started_at.map(|start| {
            let elapsed = start.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                self.bytes_transferred as f64 / elapsed
            } else {
                0.0
            }
        })
    }

    /// Get ETA (seconds)
    pub fn eta(&self) -> Option<f64> {
        if let Some(speed) = self.speed() {
            if speed > 0.0 {
                let remaining = self.file_size - self.bytes_transferred;
                return Some(remaining as f64 / speed);
            }
        }
        None
    }

    /// Get missing chunks
    pub fn missing_chunks(&self) -> Vec<u64> {
        (0..self.total_chunks)
            .filter(|i| !self.transferred_chunks.contains(i))
            .collect()
    }

    /// Add peer to transfer
    pub fn add_peer(&mut self, peer_id: PeerId) {
        self.peers.insert(peer_id, PeerTransferState {
            assigned_chunks: HashSet::new(),
            downloaded_chunks: 0,
            last_activity: Instant::now(),
        });
    }

    /// Assign chunk to peer
    pub fn assign_chunk_to_peer(&mut self, peer_id: &PeerId, chunk_index: u64) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.assigned_chunks.insert(chunk_index);
        }
    }

    /// Get next chunk to request from peers
    pub fn next_chunk_to_request(&self) -> Option<u64> {
        // Find chunk not transferred and not assigned
        let assigned: HashSet<u64> = self.peers.values()
            .flat_map(|p| p.assigned_chunks.iter())
            .copied()
            .collect();

        (0..self.total_chunks)
            .find(|i| !self.transferred_chunks.contains(i) && !assigned.contains(i))
    }

    pub fn state(&self) -> TransferState {
        self.state
    }

    pub fn is_complete(&self) -> bool {
        self.state == TransferState::Complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_progress() {
        let tree_hash = crate::files::FileTreeHash {
            root: [0u8; 32],
            chunks: vec![[0u8; 32]; 4],
        };

        let mut session = TransferSession::new_receive(
            [1u8; 32],
            PathBuf::from("/tmp/test.dat"),
            1024 * 1024, // 1 MB
            256 * 1024,  // 256 KB chunks
            tree_hash,
        );

        session.start();

        assert_eq!(session.progress(), 0.0);

        // Transfer first chunk
        session.mark_chunk_transferred(0, 256 * 1024);
        assert_eq!(session.progress(), 0.25);

        // Transfer remaining chunks
        session.mark_chunk_transferred(1, 256 * 1024);
        session.mark_chunk_transferred(2, 256 * 1024);
        session.mark_chunk_transferred(3, 256 * 1024);

        assert_eq!(session.progress(), 1.0);
        assert!(session.is_complete());
    }

    #[test]
    fn test_missing_chunks() {
        let tree_hash = crate::files::FileTreeHash {
            root: [0u8; 32],
            chunks: vec![[0u8; 32]; 10],
        };

        let mut session = TransferSession::new_receive(
            [1u8; 32],
            PathBuf::from("/tmp/test.dat"),
            10 * 256 * 1024,
            256 * 1024,
            tree_hash,
        );

        session.mark_chunk_transferred(0, 256 * 1024);
        session.mark_chunk_transferred(2, 256 * 1024);
        session.mark_chunk_transferred(5, 256 * 1024);

        let missing = session.missing_chunks();
        assert_eq!(missing.len(), 7);
        assert!(missing.contains(&1));
        assert!(missing.contains(&3));
    }
}
```

**Acceptance Criteria:**
- [ ] Session state machine works
- [ ] Progress tracking accurate
- [ ] Speed/ETA calculation correct
- [ ] Multi-peer chunk assignment
- [ ] Resume support (missing chunks)

---

**6.2.2: Flow Control & Congestion Control** (13 SP)

```rust
// wraith-core/src/transfer/congestion.rs

use std::time::{Duration, Instant};

/// BBR congestion control
pub struct BbrCongestionControl {
    /// Current sending rate (bytes/sec)
    sending_rate: f64,
    /// Estimated bottleneck bandwidth
    bottleneck_bw: f64,
    /// Minimum RTT observed
    min_rtt: Duration,
    /// Delivery rate measurements
    delivery_rates: Vec<DeliveryRateSample>,
    /// RTT measurements
    rtt_samples: Vec<Duration>,
    /// Current state
    state: BbrState,
    /// Cycle count
    cycle_count: u64,
}

#[derive(Debug, Clone, Copy)]
enum BbrState {
    Startup,
    Drain,
    ProbeBW,
    ProbeRTT,
}

struct DeliveryRateSample {
    rate: f64,
    time: Instant,
}

impl BbrCongestionControl {
    pub fn new(initial_rate: f64) -> Self {
        Self {
            sending_rate: initial_rate,
            bottleneck_bw: initial_rate,
            min_rtt: Duration::from_millis(1),
            delivery_rates: Vec::new(),
            rtt_samples: Vec::new(),
            state: BbrState::Startup,
            cycle_count: 0,
        }
    }

    /// Update with ACK
    pub fn on_ack(&mut self, bytes_acked: usize, rtt: Duration) {
        // Record RTT
        self.rtt_samples.push(rtt);
        if rtt < self.min_rtt {
            self.min_rtt = rtt;
        }

        // Calculate delivery rate
        let delivery_rate = bytes_acked as f64 / rtt.as_secs_f64();
        self.delivery_rates.push(DeliveryRateSample {
            rate: delivery_rate,
            time: Instant::now(),
        });

        // Estimate bottleneck bandwidth
        self.update_bottleneck_bw();

        // Update state
        match self.state {
            BbrState::Startup => self.startup(),
            BbrState::Drain => self.drain(),
            BbrState::ProbeBW => self.probe_bw(),
            BbrState::ProbeRTT => self.probe_rtt(),
        }

        // Prune old samples
        self.prune_samples();
    }

    fn update_bottleneck_bw(&mut self) {
        // Use max delivery rate from recent window
        if let Some(max_rate) = self.delivery_rates.iter().map(|s| s.rate).max_by(|a, b| a.partial_cmp(b).unwrap()) {
            self.bottleneck_bw = max_rate;
        }
    }

    fn startup(&mut self) {
        // Increase sending rate aggressively
        self.sending_rate = self.bottleneck_bw * 2.0;

        // Exit startup if bandwidth not increasing
        if self.delivery_rates.len() >= 3 {
            let recent_rates: Vec<f64> = self.delivery_rates.iter().rev().take(3).map(|s| s.rate).collect();

            if recent_rates.windows(2).all(|w| w[0] <= w[1] * 1.25) {
                // Bandwidth not growing, enter drain
                self.state = BbrState::Drain;
            }
        }
    }

    fn drain(&mut self) {
        // Drain queue built up during startup
        self.sending_rate = self.bottleneck_bw;

        // Enter ProbeBW after draining
        self.state = BbrState::ProbeBW;
        self.cycle_count = 0;
    }

    fn probe_bw(&mut self) {
        // Cycle through gain values to probe bandwidth
        let gain_cycle = [1.25, 0.75, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let gain = gain_cycle[self.cycle_count as usize % gain_cycle.len()];

        self.sending_rate = self.bottleneck_bw * gain;

        self.cycle_count += 1;

        // Periodically probe RTT
        if self.cycle_count % 64 == 0 {
            self.state = BbrState::ProbeRTT;
        }
    }

    fn probe_rtt(&mut self) {
        // Reduce rate to measure min RTT
        self.sending_rate = self.bottleneck_bw * 0.5;

        // Return to ProbeBW after measuring
        if self.rtt_samples.len() >= 10 {
            self.state = BbrState::ProbeBW;
        }
    }

    fn prune_samples(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(10);

        self.delivery_rates.retain(|s| s.time > cutoff);
        self.rtt_samples.retain(|_| self.rtt_samples.len() < 100);
    }

    /// Get current sending rate
    pub fn sending_rate(&self) -> f64 {
        self.sending_rate
    }

    /// Get pacing interval for next packet
    pub fn pacing_interval(&self, packet_size: usize) -> Duration {
        if self.sending_rate > 0.0 {
            let interval_secs = packet_size as f64 / self.sending_rate;
            Duration::from_secs_f64(interval_secs)
        } else {
            Duration::from_millis(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbr_startup() {
        let mut bbr = BbrCongestionControl::new(1_000_000.0); // 1 MB/s

        // Simulate ACKs with increasing bandwidth
        for i in 0..10 {
            let bytes_acked = 1500;
            let rtt = Duration::from_millis(50);
            bbr.on_ack(bytes_acked, rtt);

            println!("Iteration {}: rate = {:.2} MB/s", i, bbr.sending_rate() / 1_000_000.0);
        }

        // Should have increased rate during startup
        assert!(bbr.sending_rate() > 1_000_000.0);
    }

    #[test]
    fn test_pacing_interval() {
        let bbr = BbrCongestionControl::new(10_000_000.0); // 10 MB/s

        let interval = bbr.pacing_interval(1500);

        // At 10 MB/s, 1500 bytes should take 0.15 ms
        assert!(interval < Duration::from_micros(200));
        assert!(interval > Duration::from_micros(100));
    }
}
```

**Acceptance Criteria:**
- [ ] BBR congestion control implemented
- [ ] Bandwidth probing works
- [ ] RTT measurement accurate
- [ ] Pacing prevents bursts
- [ ] Achieves >95% link utilization

---

### Sprint 6.3: CLI Implementation (Week 34-35)

**Duration:** 1.5 weeks
**Story Points:** 26

**6.3.1: CLI Commands** (13 SP)

```rust
// wraith-cli/src/main.rs

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "wraith")]
#[command(about = "WRAITH Protocol - Secure P2P File Transfer", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a file to peer
    Send {
        /// File to send
        file: PathBuf,

        /// Recipient peer ID or discovery name
        #[arg(short, long)]
        to: String,

        /// Obfuscation level (none, low, medium, high, paranoid)
        #[arg(short, long, default_value = "medium")]
        obfuscation: String,
    },

    /// Receive files
    Receive {
        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: PathBuf,

        /// Listen address
        #[arg(short, long, default_value = "0.0.0.0:0")]
        bind: String,
    },

    /// Start daemon mode
    Daemon {
        /// Listen address
        #[arg(short, long, default_value = "0.0.0.0:40000")]
        bind: String,

        /// Enable XDP kernel bypass
        #[arg(long)]
        xdp: bool,

        /// Network interface for XDP
        #[arg(long)]
        interface: Option<String>,
    },

    /// List active transfers
    Transfers,

    /// Show peer information
    Peers,

    /// Show node status
    Status,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    match cli.command {
        Commands::Send { file, to, obfuscation } => {
            send_file(file, to, obfuscation).await?;
        }

        Commands::Receive { output, bind } => {
            receive_files(output, bind).await?;
        }

        Commands::Daemon { bind, xdp, interface } => {
            run_daemon(bind, xdp, interface).await?;
        }

        Commands::Transfers => {
            list_transfers().await?;
        }

        Commands::Peers => {
            list_peers().await?;
        }

        Commands::Status => {
            show_status().await?;
        }
    }

    Ok(())
}

async fn send_file(file: PathBuf, to: String, obfuscation: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Sending {} to {}...", file.display(), to);

    // TODO: Implement file send logic

    Ok(())
}

async fn receive_files(output: PathBuf, bind: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Receiving files to {} (listening on {})...", output.display(), bind);

    // TODO: Implement file receive logic

    Ok(())
}

async fn run_daemon(bind: String, xdp: bool, interface: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting WRAITH daemon on {}...", bind);

    if xdp {
        if let Some(iface) = interface {
            println!("Using XDP on interface: {}", iface);
        } else {
            eprintln!("Error: --interface required when --xdp is enabled");
            std::process::exit(1);
        }
    }

    // TODO: Implement daemon logic

    Ok(())
}

async fn list_transfers() -> Result<(), Box<dyn std::error::Error>> {
    println!("Active transfers:");
    // TODO: Query daemon for active transfers
    Ok(())
}

async fn list_peers() -> Result<(), Box<dyn std::error::Error>> {
    println!("Connected peers:");
    // TODO: Query daemon for connected peers
    Ok(())
}

async fn show_status() -> Result<(), Box<dyn std::error::Error>> {
    println!("WRAITH Node Status:");
    // TODO: Query daemon for status
    Ok(())
}
```

**Acceptance Criteria:**
- [ ] CLI argument parsing works
- [ ] Send command implemented
- [ ] Receive command implemented
- [ ] Daemon mode functional
- [ ] Status/list commands work

---

**6.3.2: Progress Display** (8 SP)

```rust
// wraith-cli/src/progress.rs

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub struct TransferProgress {
    bar: ProgressBar,
}

impl TransferProgress {
    pub fn new(total_bytes: u64, filename: &str) -> Self {
        let bar = ProgressBar::new(total_bytes);

        bar.set_style(
            ProgressStyle::default_bar()
                .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );

        bar.set_message(format!("Transferring: {}", filename));

        Self { bar }
    }

    pub fn update(&self, transferred_bytes: u64) {
        self.bar.set_position(transferred_bytes);
    }

    pub fn finish(&self) {
        self.bar.finish_with_message("Transfer complete!");
    }

    pub fn set_message(&self, msg: String) {
        self.bar.set_message(msg);
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_idx])
}

pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();

    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
```

**Acceptance Criteria:**
- [ ] Progress bar displays correctly
- [ ] Speed/ETA shown
- [ ] Byte formatting human-readable
- [ ] Clean terminal output

---

**6.3.3: Configuration System** (5 SP)

```toml
# config.toml

[node]
# Node public key (hex)
public_key = "..."

# Private key file path
private_key_file = "~/.wraith/private_key"

[network]
# Listen address
listen_addr = "0.0.0.0:40000"

# Enable XDP kernel bypass
enable_xdp = false

# Network interface for XDP
xdp_interface = "eth0"

# Enable UDP fallback
udp_fallback = true

[obfuscation]
# Default obfuscation level (none, low, medium, high, paranoid)
default_level = "medium"

# Enable TLS mimicry
tls_mimicry = true

# Enable cover traffic
cover_traffic = false

[discovery]
# DHT bootstrap nodes
bootstrap_nodes = [
    "bootstrap1.wraith.network:40000",
    "bootstrap2.wraith.network:40000",
]

# DERP relay servers
relay_servers = [
    "relay1.wraith.network:40001",
    "relay2.wraith.network:40001",
]

[transfer]
# Chunk size (bytes)
chunk_size = 262144  # 256 KB

# Maximum concurrent transfers
max_concurrent = 10

# Resume incomplete transfers
enable_resume = true

[logging]
# Log level (error, warn, info, debug, trace)
level = "info"

# Log file path
file = "~/.wraith/wraith.log"
```

**Acceptance Criteria:**
- [ ] TOML config parsing works
- [ ] Defaults for all settings
- [ ] Config validation
- [ ] Per-user config support

---

### Sprint 6.4: Integration Testing (Week 35-36)

**Duration:** 1.5 weeks
**Story Points:** 25

**6.4.1: End-to-End Tests** (13 SP)

```rust
// tests/integration_test.rs

use wraith_core::*;
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_file_transfer_single_peer() {
    // Create sender and receiver nodes
    let sender = Node::new_random().await.unwrap();
    let receiver = Node::new_random().await.unwrap();

    // Create test file
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.dat");
    std::fs::write(&test_file, vec![0xAA; 1024 * 1024]).unwrap(); // 1 MB

    // Send file
    let transfer_id = sender.send_file(&test_file, &receiver.public_key()).await.unwrap();

    // Wait for transfer to complete
    let output_file = temp_dir.path().join("received.dat");
    receiver.receive_file(transfer_id, &output_file).await.unwrap();

    // Verify
    let original = std::fs::read(&test_file).unwrap();
    let received = std::fs::read(&output_file).unwrap();

    assert_eq!(original, received);
}

#[tokio::test]
async fn test_file_transfer_with_resume() {
    // TODO: Test interrupted transfer and resume
}

#[tokio::test]
async fn test_multi_peer_download() {
    // TODO: Test downloading from multiple peers simultaneously
}

#[tokio::test]
async fn test_nat_traversal() {
    // TODO: Test NAT hole punching
}

#[tokio::test]
async fn test_relay_fallback() {
    // TODO: Test relay fallback when direct connection fails
}
```

**Acceptance Criteria:**
- [ ] Basic file transfer test passes
- [ ] Resume test passes
- [ ] Multi-peer test passes
- [ ] NAT traversal test passes
- [ ] All integration tests pass in CI

---

**6.4.2: Performance Testing** (12 SP)

```rust
// benches/transfer.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

fn bench_file_transfer(c: &mut Criterion) {
    let mut group = c.benchmark_group("file_transfer");

    for size in [1_000_000, 10_000_000, 100_000_000] {
        group.throughput(Throughput::Bytes(size));

        group.bench_with_input(format!("{}MB", size / 1_000_000), &size, |b, &size| {
            b.to_async(tokio::runtime::Runtime::new().unwrap()).iter(|| async {
                // Benchmark file transfer
                // TODO: Implement benchmark
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_file_transfer);
criterion_main!(benches);
```

**Acceptance Criteria:**
- [ ] Throughput benchmarks implemented
- [ ] Latency benchmarks implemented
- [ ] Performance targets documented
- [ ] Benchmark results in CI

---

## Definition of Done (Phase 6)

### Code Quality
- [ ] All code passes `cargo clippy`
- [ ] Code formatted with `rustfmt`
- [ ] Public APIs documented
- [ ] Test coverage >80%

### Functionality
- [ ] File transfer works end-to-end
- [ ] Chunking and reassembly functional
- [ ] Tree hashing verifies integrity
- [ ] Resume support works
- [ ] Multi-peer downloads functional
- [ ] CLI commands work

### Performance
- [ ] 1GB transfer <10 seconds (1 Gbps LAN)
- [ ] BBR achieves >95% utilization
- [ ] Multi-peer speedup ~linear (up to 5 peers)

### Integration
- [ ] All protocol components integrated
- [ ] Crypto + Transport + Obfuscation + Discovery working together
- [ ] End-to-end tests pass
- [ ] CI/CD pipeline functional

### Documentation
- [ ] CLI usage documented
- [ ] Configuration guide
- [ ] API documentation complete
- [ ] Integration examples

---

## Risk Mitigation

### Integration Complexity
**Risk**: Components don't integrate smoothly
**Mitigation**: Incremental integration, extensive testing, clear interfaces

### Performance Targets
**Risk**: Cannot achieve 10s for 1GB
**Mitigation**: Profile early, optimize hot paths, document actual performance

### Multi-Peer Coordination
**Risk**: Chunk deduplication and coordination difficult
**Mitigation**: Well-defined chunk assignment algorithm, extensive testing

---

## Phase 6 Completion Checklist

- [ ] Sprint 6.1: File chunking & tree hashing
- [ ] Sprint 6.2: Transfer state machine & congestion control
- [ ] Sprint 6.3: CLI implementation & configuration
- [ ] Sprint 6.4: Integration & performance testing
- [ ] All performance targets met
- [ ] End-to-end tests pass
- [ ] Documentation complete

**Estimated Completion:** Week 36 (end of Phase 6)
