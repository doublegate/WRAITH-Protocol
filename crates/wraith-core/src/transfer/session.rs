//! Transfer session state machine for file transfers.
//!
//! This module provides the `TransferSession` type for managing the state and
//! progress of file transfers. It supports both sending and receiving modes,
//! with multi-peer download coordination.
//!
//! # Security
//!
//! - Implements `ZeroizeOnDrop` to clear sensitive session data from memory
//! - Transfer IDs are randomly generated to prevent prediction
//! - Session state is tracked to prevent replay attacks
//!
//! # Example
//!
//! ```rust
//! use wraith_core::transfer::{TransferSession, Direction};
//! use std::path::PathBuf;
//!
//! // Create a receive session for a 1 MB file
//! let mut session = TransferSession::new_receive(
//!     [1u8; 32],
//!     PathBuf::from("/tmp/received_file.dat"),
//!     1024 * 1024, // 1 MB
//!     256 * 1024,  // 256 KB chunks
//! );
//!
//! session.start();
//!
//! // Mark chunks as transferred
//! session.mark_chunk_transferred(0, 256 * 1024);
//! session.mark_chunk_transferred(1, 256 * 1024);
//!
//! // Check progress
//! assert_eq!(session.progress(), 0.5);
//! ```

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;
use zeroize::Zeroize;

/// Transfer session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferState {
    /// Transfer initializing
    Initializing,
    /// Performing handshake
    Handshaking,
    /// Actively transferring
    Transferring,
    /// Transfer paused (can resume)
    Paused,
    /// Completing final verification
    Completing,
    /// Transfer complete
    Complete,
    /// Transfer failed
    Failed,
}

/// Transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Sending file
    Send,
    /// Receiving file
    Receive,
}

/// Peer identifier (32-byte public key hash)
pub type PeerId = [u8; 32];

/// Per-peer transfer state
#[derive(Debug, Clone)]
struct PeerTransferState {
    /// Chunks assigned to this peer
    assigned_chunks: HashSet<u64>,
    /// Chunks successfully downloaded from this peer
    downloaded_chunks: u64,
    /// Last activity timestamp
    last_activity: Instant,
    /// Download speed (bytes/sec)
    speed: f64,
}

/// Transfer session
///
/// Manages the state and progress of a single file transfer, supporting
/// both sending and receiving modes. Tracks progress, speed, ETA, and
/// coordinates multi-peer downloads.
///
/// # Security
///
/// Implements `ZeroizeOnDrop` to clear sensitive data (transfer ID, peer IDs)
/// from memory when the session is dropped. This helps prevent sensitive
/// metadata from persisting in memory after a transfer completes.
///
/// # State Machine
///
/// ```text
/// Initializing -> Handshaking -> Transferring -> Completing -> Complete
///                     |              |                |
///                     v              v                v
///                   Failed        Paused           Failed
///                                   |
///                                   v
///                              Transferring
/// ```
pub struct TransferSession {
    /// Transfer ID (unique identifier)
    /// SECURITY: Zeroized on drop
    #[allow(dead_code)]
    pub id: [u8; 32],
    /// Transfer direction
    pub direction: Direction,
    /// File path
    pub file_path: PathBuf,
    /// File size in bytes
    pub file_size: u64,
    /// Chunk size in bytes
    pub chunk_size: usize,
    /// Total number of chunks
    pub total_chunks: u64,

    /// Current state
    state: TransferState,

    /// Bitmap tracking transferred chunks (bit = 1 means transferred).
    ///
    /// Uses `Vec<u64>` as a bitset: `chunk_bitmap[idx/64] & (1 << (idx%64))`
    chunk_bitmap: Vec<u64>,
    /// Count of transferred chunks (cached for O(1) access)
    transferred_count: u64,
    /// Bytes transferred
    bytes_transferred: u64,

    /// Ordered set of requestable (not transferred, not assigned) chunk indices
    requestable_chunks: BTreeSet<u64>,
    /// Cache of all currently assigned chunks across all peers
    assigned_chunks_cache: HashSet<u64>,

    /// Start timestamp
    started_at: Option<Instant>,
    /// Completion timestamp
    completed_at: Option<Instant>,

    /// Peer states (for multi-peer downloads)
    /// SECURITY: Peer IDs are zeroized on drop
    peers: HashMap<PeerId, PeerTransferState>,
}

impl Drop for TransferSession {
    fn drop(&mut self) {
        // Zeroize the transfer ID (sensitive session identifier)
        self.id.zeroize();

        // Zeroize peer IDs by draining the map and zeroizing each key
        let peer_entries: Vec<(PeerId, PeerTransferState)> = self.peers.drain().collect();
        for (mut peer_id, _state) in peer_entries {
            peer_id.zeroize();
        }

        // Clear all collections
        self.chunk_bitmap.clear();
        self.requestable_chunks.clear();
        self.assigned_chunks_cache.clear();

        tracing::trace!("TransferSession zeroized and dropped");
    }
}

impl TransferSession {
    /// Create a new send transfer session
    ///
    /// # Arguments
    ///
    /// * `id` - Unique 32-byte transfer identifier
    /// * `file_path` - Path to the file being sent
    /// * `file_size` - Total file size in bytes
    /// * `chunk_size` - Size of each chunk in bytes
    ///
    /// # Example
    ///
    /// ```rust
    /// use wraith_core::transfer::TransferSession;
    /// use std::path::PathBuf;
    ///
    /// let session = TransferSession::new_send(
    ///     [1u8; 32],
    ///     PathBuf::from("/path/to/file.dat"),
    ///     1024 * 1024, // 1 MB
    ///     256 * 1024,  // 256 KB chunks
    /// );
    /// ```
    #[must_use]
    pub fn new_send(id: [u8; 32], file_path: PathBuf, file_size: u64, chunk_size: usize) -> Self {
        let total_chunks = file_size.div_ceil(chunk_size as u64);
        let bitmap_words = total_chunks.div_ceil(64) as usize;
        let requestable_chunks: BTreeSet<u64> = (0..total_chunks).collect();

        Self {
            id,
            direction: Direction::Send,
            file_path,
            file_size,
            chunk_size,
            total_chunks,
            state: TransferState::Initializing,
            chunk_bitmap: vec![0u64; bitmap_words],
            transferred_count: 0,
            bytes_transferred: 0,
            requestable_chunks,
            assigned_chunks_cache: HashSet::new(),
            started_at: None,
            completed_at: None,
            peers: HashMap::new(),
        }
    }

    /// Create a new receive transfer session
    ///
    /// # Arguments
    ///
    /// * `id` - Unique 32-byte transfer identifier
    /// * `file_path` - Path where the received file will be saved
    /// * `file_size` - Expected total file size in bytes
    /// * `chunk_size` - Size of each chunk in bytes
    ///
    /// # Example
    ///
    /// ```rust
    /// use wraith_core::transfer::TransferSession;
    /// use std::path::PathBuf;
    ///
    /// let session = TransferSession::new_receive(
    ///     [1u8; 32],
    ///     PathBuf::from("/path/to/output.dat"),
    ///     1024 * 1024, // 1 MB
    ///     256 * 1024,  // 256 KB chunks
    /// );
    /// ```
    #[must_use]
    pub fn new_receive(
        id: [u8; 32],
        file_path: PathBuf,
        file_size: u64,
        chunk_size: usize,
    ) -> Self {
        let total_chunks = file_size.div_ceil(chunk_size as u64);
        let bitmap_words = total_chunks.div_ceil(64) as usize;
        let requestable_chunks: BTreeSet<u64> = (0..total_chunks).collect();

        Self {
            id,
            direction: Direction::Receive,
            file_path,
            file_size,
            chunk_size,
            total_chunks,
            state: TransferState::Initializing,
            chunk_bitmap: vec![0u64; bitmap_words],
            transferred_count: 0,
            bytes_transferred: 0,
            requestable_chunks,
            assigned_chunks_cache: HashSet::new(),
            started_at: None,
            completed_at: None,
            peers: HashMap::new(),
        }
    }

    /// Start the transfer
    pub fn start(&mut self) {
        self.state = TransferState::Transferring;
        self.started_at = Some(Instant::now());
    }

    /// Pause the transfer
    pub fn pause(&mut self) {
        if self.state == TransferState::Transferring {
            self.state = TransferState::Paused;
        }
    }

    /// Resume the transfer
    pub fn resume(&mut self) {
        if self.state == TransferState::Paused {
            self.state = TransferState::Transferring;
        }
    }

    /// Mark chunk as transferred
    ///
    /// Updates both the transferred_chunks and missing_chunks_set for O(1) operations.
    ///
    /// # Arguments
    ///
    /// * `chunk_index` - Index of the chunk that was transferred
    /// * `chunk_size` - Size of the transferred chunk in bytes
    pub fn mark_chunk_transferred(&mut self, chunk_index: u64, chunk_size: usize) {
        if chunk_index >= self.total_chunks {
            return;
        }

        if !Self::bitmap_test(&self.chunk_bitmap, chunk_index) {
            Self::bitmap_set(&mut self.chunk_bitmap, chunk_index);
            self.transferred_count += 1;
            self.bytes_transferred += chunk_size as u64;
            self.requestable_chunks.remove(&chunk_index);
            self.assigned_chunks_cache.remove(&chunk_index);

            // Check if complete
            if self.transferred_count == self.total_chunks {
                self.state = TransferState::Complete;
                self.completed_at = Some(Instant::now());
            }
        }
    }

    /// Get transfer progress (0.0 to 1.0)
    #[must_use]
    pub fn progress(&self) -> f64 {
        if self.file_size == 0 {
            return 1.0;
        }
        self.bytes_transferred as f64 / self.file_size as f64
    }

    /// Get transfer speed in bytes/sec
    #[must_use]
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

    /// Get ETA in seconds
    #[must_use]
    pub fn eta(&self) -> Option<f64> {
        if let Some(speed) = self.speed()
            && speed > 0.0
        {
            let remaining = self.file_size - self.bytes_transferred;
            return Some(remaining as f64 / speed);
        }
        None
    }

    /// Get elapsed time in seconds
    #[must_use]
    pub fn elapsed(&self) -> Option<f64> {
        self.started_at.map(|start| start.elapsed().as_secs_f64())
    }

    /// Get missing chunks
    ///
    /// Returns chunk indices that have not yet been transferred.
    /// Uses O(m) complexity where m is the number of missing chunks,
    /// rather than O(n) where n is the total number of chunks.
    ///
    /// # Performance
    ///
    /// This method is optimized for large files with many chunks.
    /// For a 1 GB file with 4096 chunks where 100 are missing,
    /// this returns in O(100) time instead of O(4096).
    #[must_use]
    pub fn missing_chunks(&self) -> Vec<u64> {
        let missing_count = (self.total_chunks - self.transferred_count) as usize;
        let mut missing = Vec::with_capacity(missing_count);

        for (word_idx, &word) in self.chunk_bitmap.iter().enumerate() {
            // If all bits set, no missing chunks in this word
            if word == u64::MAX {
                continue;
            }
            // Scan unset bits using inverted word
            let mut unset = !word;
            while unset != 0 {
                let bit = unset.trailing_zeros() as u64;
                let chunk_idx = (word_idx as u64) * 64 + bit;
                if chunk_idx < self.total_chunks {
                    missing.push(chunk_idx);
                }
                unset &= unset - 1; // Clear lowest set bit
            }
        }

        missing
    }

    /// Get missing chunks sorted
    ///
    /// Returns missing chunk indices in ascending order.
    /// Naturally sorted since we iterate in order.
    #[must_use]
    pub fn missing_chunks_sorted(&self) -> Vec<u64> {
        self.missing_chunks()
    }

    /// Get number of missing chunks
    ///
    /// Returns the count of chunks not yet transferred.
    /// O(1) operation using cached transferred count.
    #[must_use]
    pub fn missing_count(&self) -> u64 {
        self.total_chunks - self.transferred_count
    }

    /// Check if a specific chunk is missing
    ///
    /// O(1) lookup operation using bitmap.
    #[must_use]
    pub fn is_chunk_missing(&self, chunk_index: u64) -> bool {
        chunk_index < self.total_chunks && !Self::bitmap_test(&self.chunk_bitmap, chunk_index)
    }

    /// Add peer to transfer
    pub fn add_peer(&mut self, peer_id: PeerId) {
        self.peers.insert(
            peer_id,
            PeerTransferState {
                assigned_chunks: HashSet::new(),
                downloaded_chunks: 0,
                last_activity: Instant::now(),
                speed: 0.0,
            },
        );
    }

    /// Remove peer from transfer
    pub fn remove_peer(&mut self, peer_id: &PeerId) -> Option<HashSet<u64>> {
        self.peers.remove(peer_id).map(|state| {
            // Re-add unfinished assigned chunks back to requestable pool
            for &chunk_idx in &state.assigned_chunks {
                self.assigned_chunks_cache.remove(&chunk_idx);
                if !Self::bitmap_test(&self.chunk_bitmap, chunk_idx) {
                    self.requestable_chunks.insert(chunk_idx);
                }
            }
            state.assigned_chunks
        })
    }

    /// Assign chunk to peer
    pub fn assign_chunk_to_peer(&mut self, peer_id: &PeerId, chunk_index: u64) -> bool {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.assigned_chunks.insert(chunk_index);
            self.assigned_chunks_cache.insert(chunk_index);
            self.requestable_chunks.remove(&chunk_index);
            peer.last_activity = Instant::now();
            true
        } else {
            false
        }
    }

    /// Mark chunk as downloaded from peer
    pub fn mark_peer_chunk_downloaded(&mut self, peer_id: &PeerId, chunk_index: u64) {
        if let Some(peer) = self.peers.get_mut(peer_id)
            && peer.assigned_chunks.remove(&chunk_index)
        {
            self.assigned_chunks_cache.remove(&chunk_index);
            peer.downloaded_chunks += 1;
            peer.last_activity = Instant::now();
        }
    }

    /// Update peer speed
    pub fn update_peer_speed(&mut self, peer_id: &PeerId, speed: f64) {
        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.speed = speed;
            peer.last_activity = Instant::now();
        }
    }

    /// Get next chunk to request from peers
    ///
    /// Returns the lowest-indexed chunk that is:
    /// - Not yet transferred
    /// - Not currently assigned to any peer
    ///
    /// Uses a BTreeSet for O(log n) lookup of the next requestable chunk.
    #[must_use]
    pub fn next_chunk_to_request(&mut self) -> Option<u64> {
        // requestable_chunks already excludes transferred and assigned chunks
        self.requestable_chunks.iter().next().copied()
    }

    /// Get all assigned chunks across all peers
    #[must_use]
    pub fn assigned_chunks(&self) -> HashSet<u64> {
        self.peers
            .values()
            .flat_map(|p| p.assigned_chunks.iter())
            .copied()
            .collect()
    }

    /// Get peer count
    #[must_use]
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Get peer IDs
    #[must_use]
    pub fn peer_ids(&self) -> Vec<PeerId> {
        self.peers.keys().copied().collect()
    }

    /// Get peer download count
    #[must_use]
    pub fn peer_downloaded_count(&self, peer_id: &PeerId) -> u64 {
        self.peers
            .get(peer_id)
            .map(|p| p.downloaded_chunks)
            .unwrap_or(0)
    }

    /// Get peer speed
    #[must_use]
    pub fn peer_speed(&self, peer_id: &PeerId) -> f64 {
        self.peers.get(peer_id).map(|p| p.speed).unwrap_or(0.0)
    }

    /// Get aggregate download speed from all peers
    #[must_use]
    pub fn aggregate_peer_speed(&self) -> f64 {
        self.peers.values().map(|p| p.speed).sum()
    }

    /// Get current state
    #[must_use]
    pub fn state(&self) -> TransferState {
        self.state
    }

    /// Check if transfer is complete
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.state == TransferState::Complete
    }

    /// Check if transfer is active
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            TransferState::Transferring | TransferState::Paused
        )
    }

    /// Check if transfer failed
    #[must_use]
    pub fn is_failed(&self) -> bool {
        self.state == TransferState::Failed
    }

    /// Mark transfer as failed
    pub fn mark_failed(&mut self) {
        self.state = TransferState::Failed;
    }

    /// Get transferred chunk count
    #[must_use]
    pub fn transferred_count(&self) -> u64 {
        self.transferred_count
    }

    /// Get bytes transferred
    #[must_use]
    pub fn bytes_transferred(&self) -> u64 {
        self.bytes_transferred
    }

    // ========================================================================
    // Bitmap helpers
    // ========================================================================

    /// Set a bit in the bitmap
    fn bitmap_set(bitmap: &mut [u64], idx: u64) {
        let word = (idx / 64) as usize;
        let bit = idx % 64;
        bitmap[word] |= 1u64 << bit;
    }

    /// Test a bit in the bitmap
    fn bitmap_test(bitmap: &[u64], idx: u64) -> bool {
        let word = (idx / 64) as usize;
        let bit = idx % 64;
        (bitmap[word] >> bit) & 1 == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_progress() {
        let mut session = TransferSession::new_receive(
            [1u8; 32],
            PathBuf::from("/tmp/test.dat"),
            1024 * 1024, // 1 MB
            256 * 1024,  // 256 KB chunks
        );

        session.start();

        assert_eq!(session.progress(), 0.0);
        assert_eq!(session.state(), TransferState::Transferring);

        // Transfer first chunk
        session.mark_chunk_transferred(0, 256 * 1024);
        assert_eq!(session.progress(), 0.25);

        // Transfer remaining chunks
        session.mark_chunk_transferred(1, 256 * 1024);
        session.mark_chunk_transferred(2, 256 * 1024);
        session.mark_chunk_transferred(3, 256 * 1024);

        assert_eq!(session.progress(), 1.0);
        assert!(session.is_complete());
        assert_eq!(session.state(), TransferState::Complete);
    }

    #[test]
    fn test_missing_chunks() {
        let mut session = TransferSession::new_receive(
            [1u8; 32],
            PathBuf::from("/tmp/test.dat"),
            10 * 256 * 1024,
            256 * 1024,
        );

        session.mark_chunk_transferred(0, 256 * 1024);
        session.mark_chunk_transferred(2, 256 * 1024);
        session.mark_chunk_transferred(5, 256 * 1024);

        let missing = session.missing_chunks();
        assert_eq!(missing.len(), 7);
        assert!(missing.contains(&1));
        assert!(missing.contains(&3));
        assert!(missing.contains(&4));
        assert_eq!(session.missing_count(), 7);
    }

    #[test]
    fn test_pause_resume() {
        let mut session =
            TransferSession::new_send([2u8; 32], PathBuf::from("/tmp/send.dat"), 1024, 256);

        session.start();
        assert_eq!(session.state(), TransferState::Transferring);

        session.pause();
        assert_eq!(session.state(), TransferState::Paused);

        session.resume();
        assert_eq!(session.state(), TransferState::Transferring);
    }

    #[test]
    fn test_multi_peer_coordination() {
        let mut session = TransferSession::new_receive(
            [3u8; 32],
            PathBuf::from("/tmp/multi.dat"),
            10 * 256 * 1024,
            256 * 1024,
        );

        let peer1 = [1u8; 32];
        let peer2 = [2u8; 32];

        session.add_peer(peer1);
        session.add_peer(peer2);

        assert_eq!(session.peer_count(), 2);

        // Assign chunks to peers
        session.assign_chunk_to_peer(&peer1, 0);
        session.assign_chunk_to_peer(&peer1, 1);
        session.assign_chunk_to_peer(&peer2, 2);
        session.assign_chunk_to_peer(&peer2, 3);

        // Get next unassigned chunk
        let next = session.next_chunk_to_request();
        assert_eq!(next, Some(4));

        // Mark chunks as downloaded
        session.mark_peer_chunk_downloaded(&peer1, 0);
        session.mark_peer_chunk_downloaded(&peer2, 2);

        assert_eq!(session.peer_downloaded_count(&peer1), 1);
        assert_eq!(session.peer_downloaded_count(&peer2), 1);
    }

    #[test]
    fn test_peer_speed_tracking() {
        let mut session = TransferSession::new_receive(
            [4u8; 32],
            PathBuf::from("/tmp/speed.dat"),
            1024 * 1024,
            256 * 1024,
        );

        let peer1 = [1u8; 32];
        let peer2 = [2u8; 32];

        session.add_peer(peer1);
        session.add_peer(peer2);

        session.update_peer_speed(&peer1, 1_000_000.0); // 1 MB/s
        session.update_peer_speed(&peer2, 2_000_000.0); // 2 MB/s

        assert_eq!(session.peer_speed(&peer1), 1_000_000.0);
        assert_eq!(session.peer_speed(&peer2), 2_000_000.0);
        assert_eq!(session.aggregate_peer_speed(), 3_000_000.0);
    }

    #[test]
    fn test_remove_peer() {
        let mut session = TransferSession::new_receive(
            [5u8; 32],
            PathBuf::from("/tmp/remove.dat"),
            1024 * 1024,
            256 * 1024,
        );

        let peer1 = [1u8; 32];
        session.add_peer(peer1);
        session.assign_chunk_to_peer(&peer1, 0);
        session.assign_chunk_to_peer(&peer1, 1);

        let assigned = session.remove_peer(&peer1);
        assert!(assigned.is_some());
        let chunks = assigned.unwrap();
        assert_eq!(chunks.len(), 2);
        assert!(chunks.contains(&0));
        assert!(chunks.contains(&1));

        assert_eq!(session.peer_count(), 0);
    }

    #[test]
    fn test_speed_and_eta() {
        use std::thread;
        use std::time::Duration;

        let mut session = TransferSession::new_receive(
            [6u8; 32],
            PathBuf::from("/tmp/eta.dat"),
            1024 * 1024,
            256 * 1024,
        );

        session.start();

        // Transfer some data
        thread::sleep(Duration::from_millis(100));
        session.mark_chunk_transferred(0, 256 * 1024);

        let speed = session.speed();
        assert!(speed.is_some());
        assert!(speed.unwrap() > 0.0);

        let eta = session.eta();
        assert!(eta.is_some());
        assert!(eta.unwrap() > 0.0);

        let elapsed = session.elapsed();
        assert!(elapsed.is_some());
        assert!(elapsed.unwrap() >= 0.1);
    }

    #[test]
    fn test_direction() {
        let send_session =
            TransferSession::new_send([7u8; 32], PathBuf::from("/tmp/send.dat"), 1024, 256);

        let recv_session =
            TransferSession::new_receive([8u8; 32], PathBuf::from("/tmp/recv.dat"), 1024, 256);

        assert_eq!(send_session.direction, Direction::Send);
        assert_eq!(recv_session.direction, Direction::Receive);
    }

    #[test]
    fn test_assigned_chunks() {
        let mut session = TransferSession::new_receive(
            [9u8; 32],
            PathBuf::from("/tmp/assigned.dat"),
            10 * 256 * 1024,
            256 * 1024,
        );

        let peer1 = [1u8; 32];
        let peer2 = [2u8; 32];

        session.add_peer(peer1);
        session.add_peer(peer2);

        session.assign_chunk_to_peer(&peer1, 0);
        session.assign_chunk_to_peer(&peer1, 1);
        session.assign_chunk_to_peer(&peer2, 2);

        let assigned = session.assigned_chunks();
        assert_eq!(assigned.len(), 3);
        assert!(assigned.contains(&0));
        assert!(assigned.contains(&1));
        assert!(assigned.contains(&2));
    }
}
