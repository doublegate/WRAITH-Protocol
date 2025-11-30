//! File chunking.

use crate::DEFAULT_CHUNK_SIZE;

/// Chunk a file into fixed-size pieces
pub struct FileChunker {
    chunk_size: usize,
}

impl FileChunker {
    /// Create a new chunker with default chunk size
    #[must_use]
    pub fn new() -> Self {
        Self::with_chunk_size(DEFAULT_CHUNK_SIZE)
    }

    /// Create a new chunker with custom chunk size
    #[must_use]
    pub fn with_chunk_size(size: usize) -> Self {
        Self { chunk_size: size }
    }

    /// Get chunk size
    #[must_use]
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    /// Calculate number of chunks for a file
    #[must_use]
    pub fn chunk_count(&self, file_size: u64) -> u64 {
        file_size.div_ceil(self.chunk_size as u64)
    }
}

impl Default for FileChunker {
    fn default() -> Self {
        Self::new()
    }
}
