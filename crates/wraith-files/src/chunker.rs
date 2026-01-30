//! File chunking with seek support and reassembly.
//!
//! This module provides file chunking and reassembly with optional buffer pool
//! integration for reduced allocation overhead during high-throughput transfers.

use crate::DEFAULT_CHUNK_SIZE;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use wraith_transport::BufferPool;

/// Chunk metadata
#[derive(Debug, Clone)]
pub struct ChunkInfo {
    /// Chunk index
    pub index: u64,
    /// Byte offset in file
    pub offset: u64,
    /// Chunk size in bytes
    pub size: usize,
    /// BLAKE3 hash of chunk
    pub hash: [u8; 32],
}

/// File chunker with I/O support
///
/// Supports optional buffer pool integration for reduced allocation overhead
/// during high-throughput file transfers.
pub struct FileChunker {
    file: File,
    chunk_size: usize,
    total_size: u64,
    current_offset: u64,
    /// Optional buffer pool for chunk allocation
    buffer_pool: Option<BufferPool>,
}

impl FileChunker {
    /// Create a new chunker for a file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or metadata cannot be read.
    pub fn new<P: AsRef<Path>>(path: P, chunk_size: usize) -> io::Result<Self> {
        let file = File::open(path)?;
        let total_size = file.metadata()?.len();

        Ok(Self {
            file,
            chunk_size,
            total_size,
            current_offset: 0,
            buffer_pool: None,
        })
    }

    /// Create a chunker with a buffer pool for reduced allocation overhead
    ///
    /// When a buffer pool is provided, chunk buffers are acquired from the pool
    /// instead of being allocated fresh for each read operation.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to chunk
    /// * `chunk_size` - Size of each chunk in bytes
    /// * `buffer_pool` - Buffer pool for chunk allocation
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or metadata cannot be read.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use wraith_files::chunker::FileChunker;
    /// use wraith_transport::BufferPool;
    ///
    /// let pool = BufferPool::new(262144, 64); // 256KB chunks, 64 buffers
    /// let chunker = FileChunker::with_buffer_pool("file.dat", 262144, pool).unwrap();
    /// ```
    pub fn with_buffer_pool<P: AsRef<Path>>(
        path: P,
        chunk_size: usize,
        buffer_pool: BufferPool,
    ) -> io::Result<Self> {
        let file = File::open(path)?;
        let total_size = file.metadata()?.len();

        Ok(Self {
            file,
            chunk_size,
            total_size,
            current_offset: 0,
            buffer_pool: Some(buffer_pool),
        })
    }

    /// Create a chunker with default chunk size
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or metadata cannot be read.
    pub fn with_default_size<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::new(path, DEFAULT_CHUNK_SIZE)
    }

    /// Set a buffer pool for chunk allocation
    ///
    /// When set, subsequent `read_chunk` calls will use buffers from the pool.
    pub fn set_buffer_pool(&mut self, pool: BufferPool) {
        self.buffer_pool = Some(pool);
    }

    /// Get a reference to the buffer pool if configured
    pub fn buffer_pool(&self) -> Option<&BufferPool> {
        self.buffer_pool.as_ref()
    }

    /// Get total number of chunks
    #[must_use]
    pub fn num_chunks(&self) -> u64 {
        self.total_size.div_ceil(self.chunk_size as u64)
    }

    /// Get chunk size
    #[must_use]
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    /// Get total file size
    #[must_use]
    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    /// Read next chunk sequentially
    ///
    /// If a buffer pool is configured, acquires a buffer from the pool.
    /// Otherwise, allocates a new buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the file fails.
    ///
    /// # Buffer Pool Usage
    ///
    /// When using a buffer pool, the caller should release the returned buffer
    /// back to the pool after processing to enable buffer reuse.
    pub fn read_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        if self.current_offset >= self.total_size {
            return Ok(None);
        }

        let remaining = self.total_size - self.current_offset;
        let chunk_len = remaining.min(self.chunk_size as u64) as usize;

        // Acquire buffer from pool or allocate new one
        let mut buffer = if let Some(ref pool) = self.buffer_pool {
            let mut buf = pool.acquire();
            // Resize buffer to actual chunk length if needed (for last chunk)
            buf.truncate(chunk_len);
            if buf.len() < chunk_len {
                buf.resize(chunk_len, 0);
            }
            buf
        } else {
            vec![0u8; chunk_len]
        };

        self.file.read_exact(&mut buffer)?;

        self.current_offset += chunk_len as u64;

        Ok(Some(buffer))
    }

    /// Release a chunk buffer back to the pool
    ///
    /// If a buffer pool is configured, returns the buffer to the pool for reuse.
    /// Otherwise, the buffer is dropped.
    ///
    /// # Arguments
    ///
    /// * `buffer` - The buffer to release
    pub fn release_chunk(&self, buffer: Vec<u8>) {
        if let Some(ref pool) = self.buffer_pool {
            pool.release(buffer);
        }
        // Otherwise, buffer is dropped
    }

    /// Seek to specific chunk
    ///
    /// # Errors
    ///
    /// Returns an error if the chunk index is out of bounds or seeking fails.
    pub fn seek_to_chunk(&mut self, chunk_index: u64) -> io::Result<()> {
        let offset = chunk_index * self.chunk_size as u64;

        if offset >= self.total_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Chunk index out of bounds",
            ));
        }

        self.file.seek(SeekFrom::Start(offset))?;
        self.current_offset = offset;

        Ok(())
    }

    /// Read specific chunk by index
    ///
    /// # Errors
    ///
    /// Returns an error if the chunk index is invalid or reading fails.
    pub fn read_chunk_at(&mut self, chunk_index: u64) -> io::Result<Vec<u8>> {
        self.seek_to_chunk(chunk_index)?;
        self.read_chunk()?
            .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "Chunk not found"))
    }

    /// Get chunk info for a specific index
    ///
    /// # Errors
    ///
    /// Returns an error if reading the chunk fails.
    pub fn chunk_info(&mut self, chunk_index: u64) -> io::Result<ChunkInfo> {
        let offset = chunk_index * self.chunk_size as u64;

        if offset >= self.total_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Chunk index out of bounds",
            ));
        }

        let chunk_data = self.read_chunk_at(chunk_index)?;
        let hash = blake3::hash(&chunk_data);

        Ok(ChunkInfo {
            index: chunk_index,
            offset,
            size: chunk_data.len(),
            hash: *hash.as_bytes(),
        })
    }
}

/// File reassembler for receiving side
///
/// Supports out-of-order chunk writing for parallel downloads with O(1) chunk
/// tracking and O(m) missing chunk queries where m is the number of missing chunks.
pub struct FileReassembler {
    file: File,
    chunk_size: usize,
    total_chunks: u64,
    #[allow(dead_code)]
    total_size: u64,
    /// Bitmap tracking received chunks (bit = 1 means received)
    /// Uses Vec<u64> as a bitset: bitmap[idx/64] & (1 << (idx%64))
    chunk_bitmap: Vec<u64>,
    /// Count of received chunks (cached for O(1) access)
    received_count: u64,
}

impl FileReassembler {
    /// Create a new reassembler
    ///
    /// Pre-allocates the file to the expected size for faster writes.
    /// Initializes the missing_chunks_set with all chunk indices for O(m) queries.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or pre-allocated.
    ///
    /// # Performance
    ///
    /// Initialization is O(n) where n is total_chunks, but subsequent missing_chunks()
    /// queries are O(m) where m is the number of missing chunks.
    pub fn new<P: AsRef<Path>>(path: P, total_size: u64, chunk_size: usize) -> io::Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        // Pre-allocate file for faster writes
        file.set_len(total_size)?;

        let total_chunks = total_size.div_ceil(chunk_size as u64);
        let bitmap_words = total_chunks.div_ceil(64) as usize;

        Ok(Self {
            file,
            chunk_size,
            total_chunks,
            total_size,
            chunk_bitmap: vec![0u64; bitmap_words],
            received_count: 0,
        })
    }

    /// Write chunk at specific index
    ///
    /// Supports out-of-order chunk writes for parallel downloads.
    /// Updates both received_chunks and missing_chunks_set for O(1) operations.
    ///
    /// # Errors
    ///
    /// Returns an error if the chunk index is invalid or writing fails.
    ///
    /// # Performance
    ///
    /// Both insert into received_chunks and remove from missing_chunks_set are O(1).
    pub fn write_chunk(&mut self, chunk_index: u64, data: &[u8]) -> io::Result<()> {
        if chunk_index >= self.total_chunks {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Chunk index out of bounds",
            ));
        }

        let offset = chunk_index * self.chunk_size as u64;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(data)?;

        // O(1) bitmap operations
        if !Self::bitmap_test(&self.chunk_bitmap, chunk_index) {
            Self::bitmap_set(&mut self.chunk_bitmap, chunk_index);
            self.received_count += 1;
        }

        Ok(())
    }

    /// Check if chunk is received
    #[must_use]
    pub fn has_chunk(&self, chunk_index: u64) -> bool {
        chunk_index < self.total_chunks && Self::bitmap_test(&self.chunk_bitmap, chunk_index)
    }

    /// Get missing chunk indices
    ///
    /// Returns chunk indices that have not yet been received.
    ///
    /// # Performance
    ///
    /// This is O(m) where m is the number of missing chunks, not O(n) where
    /// n is total chunks. For large files with most chunks received, this is
    /// significantly faster than iterating all chunks.
    #[must_use]
    pub fn missing_chunks(&self) -> Vec<u64> {
        let missing_total = (self.total_chunks - self.received_count) as usize;
        let mut missing = Vec::with_capacity(missing_total);

        for (word_idx, &word) in self.chunk_bitmap.iter().enumerate() {
            if word == u64::MAX {
                continue;
            }
            let mut unset = !word;
            while unset != 0 {
                let bit = unset.trailing_zeros() as u64;
                let chunk_idx = (word_idx as u64) * 64 + bit;
                if chunk_idx < self.total_chunks {
                    missing.push(chunk_idx);
                }
                unset &= unset - 1;
            }
        }

        missing
    }

    /// Get missing chunks sorted
    ///
    /// Returns missing chunk indices in ascending order.
    /// Naturally sorted since we iterate bitmap words in order.
    #[must_use]
    pub fn missing_chunks_sorted(&self) -> Vec<u64> {
        self.missing_chunks() // Already sorted by bitmap iteration order
    }

    /// Get number of missing chunks
    ///
    /// O(1) operation using cached received count.
    #[must_use]
    pub fn missing_count(&self) -> u64 {
        self.total_chunks - self.received_count
    }

    /// Check if a specific chunk is missing
    ///
    /// O(1) lookup operation.
    #[must_use]
    pub fn is_chunk_missing(&self, chunk_index: u64) -> bool {
        chunk_index < self.total_chunks && !Self::bitmap_test(&self.chunk_bitmap, chunk_index)
    }

    /// Get number of received chunks
    #[must_use]
    pub fn received_count(&self) -> u64 {
        self.received_count
    }

    /// Get progress (0.0 to 1.0)
    #[must_use]
    pub fn progress(&self) -> f64 {
        if self.total_chunks == 0 {
            1.0
        } else {
            self.received_count as f64 / self.total_chunks as f64
        }
    }

    /// Check if transfer is complete
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.received_count == self.total_chunks
    }

    /// Sync file to disk
    ///
    /// # Errors
    ///
    /// Returns an error if syncing fails.
    pub fn sync(&mut self) -> io::Result<()> {
        self.file.sync_all()
    }

    /// Finalize and close the file
    ///
    /// # Errors
    ///
    /// Returns an error if not all chunks are received or syncing fails.
    pub fn finalize(mut self) -> io::Result<()> {
        if !self.is_complete() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Transfer incomplete: {}/{} chunks received",
                    self.received_count(),
                    self.total_chunks
                ),
            ));
        }

        self.sync()?;
        Ok(())
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_chunking_roundtrip() {
        // Create test file (4 MiB to produce 4 chunks with default 1 MiB chunk size)
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = vec![0xAA; 4 * 1024 * 1024]; // 4 MiB
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        // Chunk file
        let mut chunker = FileChunker::new(temp_file.path(), DEFAULT_CHUNK_SIZE).unwrap();
        assert_eq!(chunker.num_chunks(), 4); // 4 MiB / 1 MiB = 4 chunks

        // Read all chunks
        let mut chunks = Vec::new();
        while let Some(chunk) = chunker.read_chunk().unwrap() {
            chunks.push(chunk);
        }

        assert_eq!(chunks.len(), 4);

        // Reassemble
        let output_file = NamedTempFile::new().unwrap();
        let mut reassembler =
            FileReassembler::new(output_file.path(), data.len() as u64, DEFAULT_CHUNK_SIZE)
                .unwrap();

        for (i, chunk) in chunks.iter().enumerate() {
            reassembler.write_chunk(i as u64, chunk).unwrap();
        }

        assert!(reassembler.is_complete());
        assert_eq!(reassembler.progress(), 1.0);
        reassembler.finalize().unwrap();

        // Verify
        let reconstructed = std::fs::read(output_file.path()).unwrap();
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_seek_to_chunk() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(&vec![0u8; 4 * 1024 * 1024]).unwrap(); // 4 MiB
        temp_file.flush().unwrap();

        let mut chunker = FileChunker::new(temp_file.path(), DEFAULT_CHUNK_SIZE).unwrap();

        // Read chunk 2 directly
        chunker.seek_to_chunk(2).unwrap();
        let chunk = chunker.read_chunk().unwrap().unwrap();

        assert_eq!(chunk.len(), DEFAULT_CHUNK_SIZE);
    }

    #[test]
    fn test_out_of_order_reassembly() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = vec![0xBB; 2 * 1024 * 1024]; // 2 MiB (2 chunks)
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        let mut chunker = FileChunker::new(temp_file.path(), DEFAULT_CHUNK_SIZE).unwrap();
        let mut chunks = Vec::new();
        while let Some(chunk) = chunker.read_chunk().unwrap() {
            chunks.push(chunk);
        }

        // Reassemble in reverse order
        let output_file = NamedTempFile::new().unwrap();
        let mut reassembler =
            FileReassembler::new(output_file.path(), data.len() as u64, DEFAULT_CHUNK_SIZE)
                .unwrap();

        reassembler.write_chunk(1, &chunks[1]).unwrap();
        reassembler.write_chunk(0, &chunks[0]).unwrap();

        assert!(reassembler.is_complete());
        reassembler.finalize().unwrap();

        // Verify
        let reconstructed = std::fs::read(output_file.path()).unwrap();
        assert_eq!(reconstructed, data);
    }

    #[test]
    fn test_missing_chunks() {
        let output_file = NamedTempFile::new().unwrap();
        let mut reassembler = FileReassembler::new(
            output_file.path(),
            10 * DEFAULT_CHUNK_SIZE as u64,
            DEFAULT_CHUNK_SIZE,
        )
        .unwrap();

        reassembler
            .write_chunk(0, &vec![0u8; DEFAULT_CHUNK_SIZE])
            .unwrap();
        reassembler
            .write_chunk(2, &vec![0u8; DEFAULT_CHUNK_SIZE])
            .unwrap();
        reassembler
            .write_chunk(5, &vec![0u8; DEFAULT_CHUNK_SIZE])
            .unwrap();

        let missing = reassembler.missing_chunks();
        assert_eq!(missing.len(), 7);
        assert!(missing.contains(&1));
        assert!(missing.contains(&3));
        assert!(missing.contains(&4));
        assert!(!missing.contains(&0));
        assert!(!missing.contains(&2));
    }

    #[test]
    fn test_chunk_info() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = vec![0xCC; 1024 * 1024];
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        let mut chunker = FileChunker::new(temp_file.path(), DEFAULT_CHUNK_SIZE).unwrap();
        let info = chunker.chunk_info(0).unwrap();

        assert_eq!(info.index, 0);
        assert_eq!(info.offset, 0);
        assert_eq!(info.size, DEFAULT_CHUNK_SIZE);
        assert_ne!(info.hash, [0u8; 32]);
    }

    #[test]
    fn test_incomplete_finalize_fails() {
        let output_file = NamedTempFile::new().unwrap();
        let reassembler = FileReassembler::new(
            output_file.path(),
            10 * DEFAULT_CHUNK_SIZE as u64,
            DEFAULT_CHUNK_SIZE,
        )
        .unwrap();

        // Should fail - no chunks written
        assert!(reassembler.finalize().is_err());
    }

    // Buffer pool integration tests

    #[test]
    fn test_chunker_with_buffer_pool() {
        use wraith_transport::BufferPool;

        // Create test file
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = vec![0xDD; DEFAULT_CHUNK_SIZE * 2]; // 2 chunks
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        // Create buffer pool
        let pool = BufferPool::new(DEFAULT_CHUNK_SIZE, 4);
        assert_eq!(pool.available(), 4);

        // Create chunker with pool
        let mut chunker =
            FileChunker::with_buffer_pool(temp_file.path(), DEFAULT_CHUNK_SIZE, pool).unwrap();
        assert!(chunker.buffer_pool().is_some());
        assert_eq!(chunker.num_chunks(), 2);

        // Read chunks using pool
        let chunk1 = chunker.read_chunk().unwrap().unwrap();
        assert_eq!(chunk1.len(), DEFAULT_CHUNK_SIZE);
        assert!(chunk1.iter().all(|&b| b == 0xDD));

        let chunk2 = chunker.read_chunk().unwrap().unwrap();
        assert_eq!(chunk2.len(), DEFAULT_CHUNK_SIZE);
        assert!(chunk2.iter().all(|&b| b == 0xDD));

        // No more chunks
        assert!(chunker.read_chunk().unwrap().is_none());

        // Release chunks back to pool
        chunker.release_chunk(chunk1);
        chunker.release_chunk(chunk2);

        // Verify pool has buffers back
        assert_eq!(chunker.buffer_pool().unwrap().available(), 4);
    }

    #[test]
    fn test_chunker_set_buffer_pool() {
        use wraith_transport::BufferPool;

        // Create test file
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = vec![0xEE; DEFAULT_CHUNK_SIZE];
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        // Create chunker without pool
        let mut chunker = FileChunker::new(temp_file.path(), DEFAULT_CHUNK_SIZE).unwrap();
        assert!(chunker.buffer_pool().is_none());

        // Set buffer pool
        let pool = BufferPool::new(DEFAULT_CHUNK_SIZE, 2);
        chunker.set_buffer_pool(pool);
        assert!(chunker.buffer_pool().is_some());

        // Read chunk using pool
        let chunk = chunker.read_chunk().unwrap().unwrap();
        assert_eq!(chunk.len(), DEFAULT_CHUNK_SIZE);
        assert!(chunk.iter().all(|&b| b == 0xEE));

        // Pool should have 1 buffer remaining (one was acquired)
        assert_eq!(chunker.buffer_pool().unwrap().available(), 1);

        // Release chunk
        chunker.release_chunk(chunk);
        assert_eq!(chunker.buffer_pool().unwrap().available(), 2);
    }

    #[test]
    fn test_chunker_buffer_pool_last_chunk() {
        use wraith_transport::BufferPool;

        // Create test file with size not divisible by chunk size
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = vec![0xFF; DEFAULT_CHUNK_SIZE + 1000]; // 1 full chunk + 1000 bytes
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        // Create buffer pool with larger buffers
        let pool = BufferPool::new(DEFAULT_CHUNK_SIZE, 2);

        // Create chunker with pool
        let mut chunker =
            FileChunker::with_buffer_pool(temp_file.path(), DEFAULT_CHUNK_SIZE, pool).unwrap();
        assert_eq!(chunker.num_chunks(), 2);

        // First chunk should be full size
        let chunk1 = chunker.read_chunk().unwrap().unwrap();
        assert_eq!(chunk1.len(), DEFAULT_CHUNK_SIZE);

        // Last chunk should be smaller
        let chunk2 = chunker.read_chunk().unwrap().unwrap();
        assert_eq!(chunk2.len(), 1000);

        // Both chunks should contain correct data
        assert!(chunk1.iter().all(|&b| b == 0xFF));
        assert!(chunk2.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_chunker_release_without_pool() {
        // Create test file
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = vec![0xAB; DEFAULT_CHUNK_SIZE];
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        // Create chunker without pool
        let mut chunker = FileChunker::new(temp_file.path(), DEFAULT_CHUNK_SIZE).unwrap();
        assert!(chunker.buffer_pool().is_none());

        // Read chunk (allocated, not from pool)
        let chunk = chunker.read_chunk().unwrap().unwrap();

        // Release should work without panic (buffer is just dropped)
        chunker.release_chunk(chunk);
    }

    #[test]
    fn test_chunker_buffer_pool_roundtrip() {
        use wraith_transport::BufferPool;

        // Create test file
        let mut temp_file = NamedTempFile::new().unwrap();
        let data = vec![0x42; DEFAULT_CHUNK_SIZE * 4]; // 4 chunks
        temp_file.write_all(&data).unwrap();
        temp_file.flush().unwrap();

        // Create buffer pool with only 2 buffers (less than chunk count)
        let pool = BufferPool::new(DEFAULT_CHUNK_SIZE, 2);

        // Create chunker with pool
        let mut chunker =
            FileChunker::with_buffer_pool(temp_file.path(), DEFAULT_CHUNK_SIZE, pool).unwrap();

        // Read and verify all chunks, recycling buffers
        let mut all_data = Vec::new();
        while let Some(chunk) = chunker.read_chunk().unwrap() {
            all_data.extend_from_slice(&chunk);
            // Release buffer immediately for reuse
            chunker.release_chunk(chunk);
        }

        // Verify total data matches
        assert_eq!(all_data, data);
    }
}
