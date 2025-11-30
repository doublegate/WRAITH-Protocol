//! io_uring-based high-performance file I/O engine (Linux-only).
//!
//! This module provides a high-performance async file I/O engine using
//! Linux's io_uring interface for zero-copy, batched I/O operations.
//!
//! Target performance: >2 GB/s file I/O throughput

use io_uring::{IoUring, Probe, opcode, types};
use std::os::unix::io::RawFd;
use thiserror::Error;

/// io_uring-based async file I/O engine
///
/// Provides high-performance async file operations using Linux io_uring.
/// Supports batching multiple I/O operations for improved throughput.
pub struct IoUringEngine {
    ring: IoUring,
    pending: usize,
}

impl IoUringEngine {
    /// Create a new io_uring instance with specified queue depth
    ///
    /// # Arguments
    /// * `queue_depth` - Number of submission queue entries (power of 2, max 4096)
    ///
    /// # Examples
    /// ```no_run
    /// # #[cfg(target_os = "linux")]
    /// # {
    /// use wraith_files::io_uring::IoUringEngine;
    ///
    /// let engine = IoUringEngine::new(128).unwrap();
    /// # }
    /// ```
    pub fn new(queue_depth: u32) -> Result<Self, IoError> {
        let ring = IoUring::new(queue_depth)?;

        // Probe for supported operations
        let mut probe = Probe::new();
        ring.submitter().register_probe(&mut probe)?;

        // Verify read/write operations are supported
        if !probe.is_supported(opcode::Read::CODE) {
            return Err(IoError::UnsupportedOperation("read"));
        }

        if !probe.is_supported(opcode::Write::CODE) {
            return Err(IoError::UnsupportedOperation("write"));
        }

        Ok(Self { ring, pending: 0 })
    }

    /// Submit an async read request
    ///
    /// # Arguments
    /// * `fd` - File descriptor to read from
    /// * `offset` - Offset in file to read from
    /// * `buf` - Buffer to read into
    /// * `user_data` - User-defined request identifier
    ///
    /// # Safety
    /// The caller must ensure the buffer remains valid until the completion event.
    pub unsafe fn read(
        &mut self,
        fd: RawFd,
        offset: u64,
        buf: *mut u8,
        len: usize,
        user_data: u64,
    ) -> Result<(), IoError> {
        let read_op = opcode::Read::new(types::Fd(fd), buf, len as u32)
            .offset(offset)
            .build()
            .user_data(user_data);

        unsafe {
            self.ring
                .submission()
                .push(&read_op)
                .map_err(|_| IoError::QueueFull)?;
        }

        self.pending += 1;
        Ok(())
    }

    /// Submit an async write request
    ///
    /// # Arguments
    /// * `fd` - File descriptor to write to
    /// * `offset` - Offset in file to write to
    /// * `buf` - Buffer containing data to write
    /// * `user_data` - User-defined request identifier
    ///
    /// # Safety
    /// The caller must ensure the buffer remains valid until the completion event.
    pub unsafe fn write(
        &mut self,
        fd: RawFd,
        offset: u64,
        buf: *const u8,
        len: usize,
        user_data: u64,
    ) -> Result<(), IoError> {
        let write_op = opcode::Write::new(types::Fd(fd), buf, len as u32)
            .offset(offset)
            .build()
            .user_data(user_data);

        unsafe {
            self.ring
                .submission()
                .push(&write_op)
                .map_err(|_| IoError::QueueFull)?;
        }

        self.pending += 1;
        Ok(())
    }

    /// Submit all pending operations to the kernel
    ///
    /// Returns the number of operations submitted.
    pub fn submit(&mut self) -> Result<usize, IoError> {
        let submitted = self.ring.submit()?;
        Ok(submitted)
    }

    /// Wait for at least `min_complete` operations to complete
    ///
    /// Returns all available completion events.
    pub fn wait(&mut self, min_complete: usize) -> Result<Vec<Completion>, IoError> {
        self.ring.submit_and_wait(min_complete)?;

        let mut completions = Vec::new();

        for cqe in self.ring.completion() {
            completions.push(Completion {
                user_data: cqe.user_data(),
                result: cqe.result(),
            });
            self.pending = self.pending.saturating_sub(1);
        }

        Ok(completions)
    }

    /// Poll for completions without waiting
    ///
    /// Returns all available completion events immediately.
    pub fn poll(&mut self) -> Result<Vec<Completion>, IoError> {
        self.wait(0)
    }

    /// Get the number of pending operations
    pub fn pending(&self) -> usize {
        self.pending
    }

    /// Get the queue depth of this engine
    pub fn queue_depth(&self) -> u32 {
        self.ring.params().sq_entries()
    }
}

/// Completion event from io_uring
#[derive(Debug, Clone, Copy)]
pub struct Completion {
    /// User-defined request identifier
    pub user_data: u64,
    /// Result of the operation (bytes transferred or negative error code)
    pub result: i32,
}

impl Completion {
    /// Check if this completion represents a successful operation
    pub fn is_success(&self) -> bool {
        self.result >= 0
    }

    /// Get the number of bytes transferred (if successful)
    pub fn bytes_transferred(&self) -> Option<usize> {
        if self.result >= 0 {
            Some(self.result as usize)
        } else {
            None
        }
    }

    /// Get the error code (if failed)
    pub fn error_code(&self) -> Option<i32> {
        if self.result < 0 {
            Some(-self.result)
        } else {
            None
        }
    }
}

/// Error types for io_uring operations
#[derive(Debug, Error)]
pub enum IoError {
    /// I/O error from io_uring
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Submission queue is full
    #[error("Submission queue is full")]
    QueueFull,

    /// Operation not supported by kernel
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::io::AsRawFd;
    use tempfile::NamedTempFile;

    #[test]
    fn test_io_uring_creation() {
        let engine = IoUringEngine::new(128).unwrap();
        assert_eq!(engine.pending(), 0);
        assert!(engine.queue_depth() >= 128);
    }

    #[test]
    fn test_io_uring_read() {
        let mut engine = IoUringEngine::new(128).unwrap();

        // Create a test file
        let mut temp = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp, b"Hello, io_uring!").unwrap();
        temp.flush().unwrap();

        let file = File::open(temp.path()).unwrap();
        let fd = file.as_raw_fd();

        let mut buf = vec![0u8; 1024];
        unsafe {
            engine.read(fd, 0, buf.as_mut_ptr(), buf.len(), 1).unwrap();
        }

        engine.submit().unwrap();
        let completions = engine.wait(1).unwrap();

        assert_eq!(completions.len(), 1);
        assert!(completions[0].is_success());
        assert_eq!(completions[0].bytes_transferred(), Some(16));
        assert_eq!(&buf[..16], b"Hello, io_uring!");
    }

    #[test]
    fn test_io_uring_write() {
        let mut engine = IoUringEngine::new(128).unwrap();

        let temp = NamedTempFile::new().unwrap();
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(temp.path())
            .unwrap();
        let fd = file.as_raw_fd();

        let data = b"Write test data";
        unsafe {
            engine.write(fd, 0, data.as_ptr(), data.len(), 1).unwrap();
        }

        engine.submit().unwrap();
        let completions = engine.wait(1).unwrap();

        assert_eq!(completions.len(), 1);
        assert!(completions[0].is_success());
        assert_eq!(completions[0].bytes_transferred(), Some(15));

        // Verify the write
        drop(file);
        let content = std::fs::read(temp.path()).unwrap();
        assert_eq!(&content[..15], b"Write test data");
    }

    #[test]
    fn test_io_uring_batching() {
        let mut engine = IoUringEngine::new(128).unwrap();

        let file = File::open("/etc/hostname").unwrap();
        let fd = file.as_raw_fd();

        // Submit multiple reads
        let mut buffers = vec![vec![0u8; 64]; 4];
        for (i, buf) in buffers.iter_mut().enumerate() {
            unsafe {
                engine
                    .read(fd, 0, buf.as_mut_ptr(), buf.len(), i as u64)
                    .unwrap();
            }
        }

        assert_eq!(engine.pending(), 4);

        engine.submit().unwrap();
        let completions = engine.wait(4).unwrap();

        assert_eq!(completions.len(), 4);
        for comp in completions {
            assert!(comp.is_success());
        }
    }

    #[test]
    fn test_completion_methods() {
        let success = Completion {
            user_data: 1,
            result: 100,
        };
        assert!(success.is_success());
        assert_eq!(success.bytes_transferred(), Some(100));
        assert_eq!(success.error_code(), None);

        let failure = Completion {
            user_data: 2,
            result: -5, // EIO
        };
        assert!(!failure.is_success());
        assert_eq!(failure.bytes_transferred(), None);
        assert_eq!(failure.error_code(), Some(5));
    }
}
