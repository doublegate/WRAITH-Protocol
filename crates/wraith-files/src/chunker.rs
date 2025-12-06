//! File chunking with seek support and reassembly.

use crate::DEFAULT_CHUNK_SIZE;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

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
pub struct FileChunker {
    file: File,
    chunk_size: usize,
    total_size: u64,
    current_offset: u64,
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
    /// # Errors
    ///
    /// Returns an error if reading from the file fails.
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
    /// Chunks that have been received (for quick lookup)
    received_chunks: HashSet<u64>,
    /// Chunks that are still missing (for O(m) missing queries)
    /// This is the inverse of received_chunks for optimal missing chunk iteration
    missing_chunks_set: HashSet<u64>,
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

        // Initialize missing set with all chunk indices
        let missing_chunks_set: HashSet<u64> = (0..total_chunks).collect();

        Ok(Self {
            file,
            chunk_size,
            total_chunks,
            total_size,
            received_chunks: HashSet::new(),
            missing_chunks_set,
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

        // O(1) operations for both sets
        if self.received_chunks.insert(chunk_index) {
            self.missing_chunks_set.remove(&chunk_index);
        }

        Ok(())
    }

    /// Check if chunk is received
    #[must_use]
    pub fn has_chunk(&self, chunk_index: u64) -> bool {
        self.received_chunks.contains(&chunk_index)
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
        self.missing_chunks_set.iter().copied().collect()
    }

    /// Get missing chunks sorted
    ///
    /// Returns missing chunk indices in ascending order.
    /// Useful for sequential chunk requests.
    #[must_use]
    pub fn missing_chunks_sorted(&self) -> Vec<u64> {
        let mut missing: Vec<u64> = self.missing_chunks_set.iter().copied().collect();
        missing.sort_unstable();
        missing
    }

    /// Get number of missing chunks
    ///
    /// O(1) operation using the missing_chunks_set.
    #[must_use]
    pub fn missing_count(&self) -> u64 {
        self.missing_chunks_set.len() as u64
    }

    /// Check if a specific chunk is missing
    ///
    /// O(1) lookup operation.
    #[must_use]
    pub fn is_chunk_missing(&self, chunk_index: u64) -> bool {
        self.missing_chunks_set.contains(&chunk_index)
    }

    /// Get number of received chunks
    #[must_use]
    pub fn received_count(&self) -> u64 {
        self.received_chunks.len() as u64
    }

    /// Get progress (0.0 to 1.0)
    #[must_use]
    pub fn progress(&self) -> f64 {
        if self.total_chunks == 0 {
            // 0-byte file is always 100% complete
            1.0
        } else {
            self.received_chunks.len() as f64 / self.total_chunks as f64
        }
    }

    /// Check if transfer is complete
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.received_chunks.len() as u64 == self.total_chunks
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
        temp_file.write_all(&vec![0u8; 1024 * 1024]).unwrap();
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
        let data = vec![0xBB; 512 * 1024]; // 512 KB
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
}
