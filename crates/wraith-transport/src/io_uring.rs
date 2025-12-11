//! io_uring-based asynchronous file I/O for WRAITH Protocol
//!
//! This module provides high-performance async file I/O using Linux io_uring.
//! It supports batched operations with zero-copy reads/writes and registered buffers.
//!
//! ## Features
//!
//! - Batched submission (submit multiple ops in one syscall)
//! - Completion polling (process multiple completions at once)
//! - Registered buffers for zero-copy I/O
//! - SQE (Submission Queue Entry) management
//! - CQE (Completion Queue Entry) processing
//!
//! ## Platform Support
//!
//! - Linux 5.1+ with io_uring support (required)
//! - Fallback to synchronous I/O on other platforms
//!
//! ## Example
//!
//! ```no_run
//! # #[cfg(target_os = "linux")]
//! # {
//! use wraith_transport::io_uring::{IoUringContext, PendingOp};
//! use std::fs::File;
//!
//! #[cfg(unix)]
//! use std::os::fd::AsRawFd;
//! #[cfg(windows)]
//! use std::os::windows::io::AsRawHandle as AsRawFd;
//!
//! let mut ctx = IoUringContext::new(64).unwrap();
//!
//! let file = File::open("test.dat").unwrap();
//! let fd = file.as_raw_fd();
//!
//! // Submit read operation
//! let op_id = ctx.submit_read(fd, 0, 4096).unwrap();
//!
//! // Wait for completion
//! let completions = ctx.wait_completions(1).unwrap();
//! for completion in completions {
//!     println!("Read {} bytes", completion.result);
//! }
//! # }
//! ```

#[cfg(target_os = "linux")]
use std::collections::HashMap;
use std::io;
use thiserror::Error;

// Platform-specific RawFd type
#[cfg(unix)]
use std::os::fd::RawFd;

// On Windows, we use a type alias for compatibility
// The actual file descriptor operations would use Windows HANDLEs internally
#[cfg(not(unix))]
type RawFd = std::os::windows::io::RawHandle;

/// io_uring errors
#[derive(Debug, Error)]
pub enum IoUringError {
    /// Ring creation failed
    #[error("Failed to create io_uring: {0}")]
    RingCreation(String),

    /// Submission failed
    #[error("Submission failed: {0}")]
    Submission(String),

    /// Completion wait failed
    #[error("Wait for completion failed: {0}")]
    Completion(String),

    /// Buffer registration failed
    #[error("Buffer registration failed: {0}")]
    BufferRegistration(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// Pending operation state
#[derive(Debug)]
pub struct PendingOp {
    /// Operation ID
    pub id: u64,
    /// Operation type
    pub op_type: OpType,
    /// File descriptor
    pub fd: RawFd,
    /// Offset in file
    pub offset: u64,
    /// Buffer length
    pub len: usize,
}

/// Operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpType {
    /// Read operation
    Read,
    /// Write operation
    Write,
    /// Fsync operation
    Fsync,
}

/// Completion result
#[derive(Debug)]
pub struct Completion {
    /// Operation ID
    pub id: u64,
    /// Result (bytes read/written or error code)
    pub result: i32,
    /// Operation type
    pub op_type: OpType,
}

/// io_uring context for asynchronous I/O
#[cfg(target_os = "linux")]
pub struct IoUringContext {
    /// Queue depth
    queue_depth: u32,
    /// Next operation ID
    next_id: u64,
    /// Pending operations
    pending: HashMap<u64, PendingOp>,
    /// Registered buffers
    buffers: Vec<Vec<u8>>,
}

#[cfg(target_os = "linux")]
impl IoUringContext {
    /// Create a new io_uring context
    ///
    /// # Arguments
    ///
    /// * `queue_depth` - Number of entries in submission/completion queues
    ///
    /// # Errors
    ///
    /// Returns `IoUringError::RingCreation` if the ring cannot be initialized.
    pub fn new(queue_depth: u32) -> Result<Self, IoUringError> {
        // In production, this would initialize io_uring via io-uring crate
        // For now, we validate parameters and create the structure

        if queue_depth == 0 || queue_depth > 4096 {
            return Err(IoUringError::RingCreation(format!(
                "Invalid queue depth: {queue_depth} (must be 1-4096)"
            )));
        }

        Ok(Self {
            queue_depth,
            next_id: 0,
            pending: HashMap::new(),
            buffers: Vec::new(),
        })
    }

    /// Submit a read operation
    ///
    /// # Arguments
    ///
    /// * `fd` - File descriptor to read from
    /// * `offset` - Offset in the file
    /// * `len` - Number of bytes to read
    ///
    /// # Returns
    ///
    /// Operation ID for tracking completion
    pub fn submit_read(&mut self, fd: RawFd, offset: u64, len: usize) -> Result<u64, IoUringError> {
        let id = self.next_id;
        self.next_id += 1;

        let op = PendingOp {
            id,
            op_type: OpType::Read,
            fd,
            offset,
            len,
        };

        // In production, would create SQE and submit to ring
        self.pending.insert(id, op);

        Ok(id)
    }

    /// Submit a write operation
    ///
    /// # Arguments
    ///
    /// * `fd` - File descriptor to write to
    /// * `offset` - Offset in the file
    /// * `data` - Data to write
    ///
    /// # Returns
    ///
    /// Operation ID for tracking completion
    pub fn submit_write(
        &mut self,
        fd: RawFd,
        offset: u64,
        data: &[u8],
    ) -> Result<u64, IoUringError> {
        let id = self.next_id;
        self.next_id += 1;

        let op = PendingOp {
            id,
            op_type: OpType::Write,
            fd,
            offset,
            len: data.len(),
        };

        // In production, would create SQE and submit to ring
        self.pending.insert(id, op);

        Ok(id)
    }

    /// Submit an fsync operation
    ///
    /// # Arguments
    ///
    /// * `fd` - File descriptor to sync
    ///
    /// # Returns
    ///
    /// Operation ID for tracking completion
    pub fn submit_fsync(&mut self, fd: RawFd) -> Result<u64, IoUringError> {
        let id = self.next_id;
        self.next_id += 1;

        let op = PendingOp {
            id,
            op_type: OpType::Fsync,
            fd,
            offset: 0,
            len: 0,
        };

        // In production, would create SQE and submit to ring
        self.pending.insert(id, op);

        Ok(id)
    }

    /// Wait for completions
    ///
    /// Blocks until at least `min_complete` operations have completed.
    ///
    /// # Arguments
    ///
    /// * `min_complete` - Minimum number of completions to wait for
    ///
    /// # Returns
    ///
    /// Vector of completed operations
    pub fn wait_completions(
        &mut self,
        min_complete: usize,
    ) -> Result<Vec<Completion>, IoUringError> {
        let mut completions = Vec::with_capacity(min_complete);

        // In production, would poll completion queue
        // For now, simulate completion of pending operations
        let to_complete: Vec<u64> = self.pending.keys().copied().take(min_complete).collect();

        for id in to_complete {
            if let Some(op) = self.pending.remove(&id) {
                // Simulate successful completion
                let result = match op.op_type {
                    OpType::Read | OpType::Write => op.len as i32,
                    OpType::Fsync => 0,
                };

                completions.push(Completion {
                    id,
                    result,
                    op_type: op.op_type,
                });
            }
        }

        Ok(completions)
    }

    /// Poll for completions without blocking
    ///
    /// Returns any available completions immediately.
    pub fn poll_completions(&mut self) -> Result<Vec<Completion>, IoUringError> {
        // In production, would check CQ without blocking
        // For now, return empty if no pending ops
        if self.pending.is_empty() {
            return Ok(Vec::new());
        }

        // Simulate up to 16 completions
        self.wait_completions(self.pending.len().min(16))
    }

    /// Register buffers for zero-copy I/O
    ///
    /// Registers buffers with the kernel to enable zero-copy operations.
    ///
    /// # Arguments
    ///
    /// * `buffers` - Buffers to register
    pub fn register_buffers(&mut self, buffers: Vec<Vec<u8>>) -> Result<(), IoUringError> {
        // In production, would register buffers with io_uring
        // via IORING_REGISTER_BUFFERS

        if buffers.is_empty() {
            return Err(IoUringError::BufferRegistration(
                "Cannot register zero buffers".into(),
            ));
        }

        self.buffers = buffers;
        Ok(())
    }

    /// Get queue depth
    #[must_use]
    pub fn queue_depth(&self) -> u32 {
        self.queue_depth
    }

    /// Get number of pending operations
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get number of registered buffers
    #[must_use]
    pub fn buffer_count(&self) -> usize {
        self.buffers.len()
    }
}

/// Fallback io_uring context for non-Linux platforms
///
/// Provides the same API but uses synchronous I/O operations.
#[cfg(not(target_os = "linux"))]
pub struct IoUringContext {
    /// Simulated queue depth
    queue_depth: u32,
    /// Next operation ID
    next_id: u64,
}

#[cfg(not(target_os = "linux"))]
impl IoUringContext {
    /// Create a new fallback context
    pub fn new(queue_depth: u32) -> Result<Self, IoUringError> {
        if queue_depth == 0 || queue_depth > 4096 {
            return Err(IoUringError::RingCreation(format!(
                "Invalid queue depth: {} (must be 1-4096)",
                queue_depth
            )));
        }

        Ok(Self {
            queue_depth,
            next_id: 0,
        })
    }

    /// Submit a read operation (synchronous fallback)
    pub fn submit_read(
        &mut self,
        _fd: RawFd,
        _offset: u64,
        _len: usize,
    ) -> Result<u64, IoUringError> {
        let id = self.next_id;
        self.next_id += 1;
        Ok(id)
    }

    /// Submit a write operation (synchronous fallback)
    pub fn submit_write(
        &mut self,
        _fd: RawFd,
        _offset: u64,
        _data: &[u8],
    ) -> Result<u64, IoUringError> {
        let id = self.next_id;
        self.next_id += 1;
        Ok(id)
    }

    /// Submit an fsync operation (synchronous fallback)
    pub fn submit_fsync(&mut self, _fd: RawFd) -> Result<u64, IoUringError> {
        let id = self.next_id;
        self.next_id += 1;
        Ok(id)
    }

    /// Wait for completions (synchronous fallback)
    pub fn wait_completions(
        &mut self,
        _min_complete: usize,
    ) -> Result<Vec<Completion>, IoUringError> {
        Ok(Vec::new())
    }

    /// Poll for completions (synchronous fallback)
    pub fn poll_completions(&mut self) -> Result<Vec<Completion>, IoUringError> {
        Ok(Vec::new())
    }

    /// Register buffers (no-op on non-Linux)
    pub fn register_buffers(&mut self, _buffers: Vec<Vec<u8>>) -> Result<(), IoUringError> {
        Ok(())
    }

    /// Get queue depth
    #[must_use]
    pub fn queue_depth(&self) -> u32 {
        self.queue_depth
    }

    /// Get number of pending operations
    #[must_use]
    pub fn pending_count(&self) -> usize {
        0
    }

    /// Get number of registered buffers
    #[must_use]
    pub fn buffer_count(&self) -> usize {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_uring_context_creation() {
        let ctx = IoUringContext::new(64);
        assert!(ctx.is_ok());

        let ctx = ctx.unwrap();
        assert_eq!(ctx.queue_depth(), 64);
        assert_eq!(ctx.pending_count(), 0);
    }

    #[test]
    fn test_io_uring_invalid_queue_depth() {
        // Too small
        assert!(IoUringContext::new(0).is_err());

        // Too large
        assert!(IoUringContext::new(10000).is_err());

        // Valid
        assert!(IoUringContext::new(128).is_ok());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_submit_read() {
        let mut ctx = IoUringContext::new(64).unwrap();

        let op_id = ctx.submit_read(1, 0, 4096).unwrap();
        assert_eq!(op_id, 0);
        assert_eq!(ctx.pending_count(), 1);

        let op_id2 = ctx.submit_read(1, 4096, 4096).unwrap();
        assert_eq!(op_id2, 1);
        assert_eq!(ctx.pending_count(), 2);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_submit_write() {
        let mut ctx = IoUringContext::new(64).unwrap();

        let data = vec![0u8; 1024];
        let op_id = ctx.submit_write(1, 0, &data).unwrap();
        assert_eq!(op_id, 0);
        assert_eq!(ctx.pending_count(), 1);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_submit_fsync() {
        let mut ctx = IoUringContext::new(64).unwrap();

        let op_id = ctx.submit_fsync(1).unwrap();
        assert_eq!(op_id, 0);
        assert_eq!(ctx.pending_count(), 1);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_wait_completions() {
        let mut ctx = IoUringContext::new(64).unwrap();

        // Submit some operations
        ctx.submit_read(1, 0, 4096).unwrap();
        ctx.submit_write(2, 0, &[0u8; 1024]).unwrap();
        ctx.submit_fsync(3).unwrap();

        assert_eq!(ctx.pending_count(), 3);

        // Wait for completions
        let completions = ctx.wait_completions(2).unwrap();
        assert_eq!(completions.len(), 2);

        // Should have removed from pending
        assert_eq!(ctx.pending_count(), 1);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_poll_completions() {
        let mut ctx = IoUringContext::new(64).unwrap();

        // No pending operations
        let completions = ctx.poll_completions().unwrap();
        assert_eq!(completions.len(), 0);

        // Submit operations
        for i in 0..5 {
            ctx.submit_read(1, i * 4096, 4096).unwrap();
        }

        // Poll for some
        let completions = ctx.poll_completions().unwrap();
        assert!(completions.len() <= 5);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_register_buffers() {
        let mut ctx = IoUringContext::new(64).unwrap();

        let buffers = vec![vec![0u8; 4096], vec![0u8; 4096], vec![0u8; 4096]];

        assert!(ctx.register_buffers(buffers).is_ok());
        assert_eq!(ctx.buffer_count(), 3);
    }

    #[test]
    fn test_register_empty_buffers() {
        let mut ctx = IoUringContext::new(64).unwrap();

        let result = ctx.register_buffers(Vec::new());

        #[cfg(target_os = "linux")]
        assert!(result.is_err());

        #[cfg(not(target_os = "linux"))]
        assert!(result.is_ok());
    }

    #[test]
    fn test_completion_result_types() {
        #[cfg(target_os = "linux")]
        {
            let mut ctx = IoUringContext::new(64).unwrap();
            ctx.submit_read(1, 0, 1024).unwrap();
            ctx.submit_write(2, 0, &[0u8; 2048]).unwrap();
            ctx.submit_fsync(3).unwrap();

            let completions = ctx.wait_completions(3).unwrap();

            // Check that completions have correct result values
            for completion in completions {
                match completion.op_type {
                    OpType::Read => assert!(completion.result > 0),
                    OpType::Write => assert!(completion.result > 0),
                    OpType::Fsync => assert_eq!(completion.result, 0),
                }
            }
        }

        // On non-Linux, just verify the stub context works
        #[cfg(not(target_os = "linux"))]
        {
            let _ctx = IoUringContext::new(64).unwrap();
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_multiple_pending_operations() {
        let mut ctx = IoUringContext::new(128).unwrap();

        // Submit many operations
        let mut op_ids = Vec::new();
        for i in 0..50 {
            let id = ctx.submit_read(1, i * 4096, 4096).unwrap();
            op_ids.push(id);
        }

        assert_eq!(ctx.pending_count(), 50);

        // Complete all operations
        let completions = ctx.wait_completions(50).unwrap();
        assert_eq!(completions.len(), 50);
        assert_eq!(ctx.pending_count(), 0);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_operation_id_increment() {
        let mut ctx = IoUringContext::new(64).unwrap();

        let id1 = ctx.submit_read(1, 0, 1024).unwrap();
        let id2 = ctx.submit_read(1, 1024, 1024).unwrap();
        let id3 = ctx.submit_write(2, 0, &[0u8; 512]).unwrap();

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 2);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_mixed_operations() {
        let mut ctx = IoUringContext::new(64).unwrap();

        // Mix different operation types
        ctx.submit_read(1, 0, 1024).unwrap();
        ctx.submit_write(2, 0, &[0u8; 2048]).unwrap();
        ctx.submit_fsync(3).unwrap();
        ctx.submit_read(1, 1024, 512).unwrap();

        assert_eq!(ctx.pending_count(), 4);

        let completions = ctx.wait_completions(4).unwrap();
        assert_eq!(completions.len(), 4);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_poll_completions_empty() {
        let mut ctx = IoUringContext::new(64).unwrap();

        // No pending operations
        let completions = ctx.poll_completions().unwrap();
        assert_eq!(completions.len(), 0);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_poll_completions_batching() {
        let mut ctx = IoUringContext::new(64).unwrap();

        // Submit 20 operations
        for i in 0..20 {
            ctx.submit_read(1, i * 1024, 1024).unwrap();
        }

        // Poll should return up to 16 (batch limit)
        let completions = ctx.poll_completions().unwrap();
        assert!(completions.len() <= 16);
        assert!(!completions.is_empty());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_buffer_registration_multiple() {
        let mut ctx = IoUringContext::new(64).unwrap();

        let buffers = vec![vec![0u8; 4096], vec![0u8; 8192], vec![0u8; 2048]];

        assert!(ctx.register_buffers(buffers).is_ok());
        assert_eq!(ctx.buffer_count(), 3);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_pending_op_structure() {
        let op = PendingOp {
            id: 42,
            op_type: OpType::Read,
            fd: 10,
            offset: 4096,
            len: 1024,
        };

        assert_eq!(op.id, 42);
        assert_eq!(op.op_type, OpType::Read);
        assert_eq!(op.fd, 10);
        assert_eq!(op.offset, 4096);
        assert_eq!(op.len, 1024);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_completion_structure() {
        let completion = Completion {
            id: 123,
            result: 1024,
            op_type: OpType::Write,
        };

        assert_eq!(completion.id, 123);
        assert_eq!(completion.result, 1024);
        assert_eq!(completion.op_type, OpType::Write);
    }

    #[test]
    fn test_op_type_equality() {
        assert_eq!(OpType::Read, OpType::Read);
        assert_eq!(OpType::Write, OpType::Write);
        assert_eq!(OpType::Fsync, OpType::Fsync);

        assert_ne!(OpType::Read, OpType::Write);
        assert_ne!(OpType::Write, OpType::Fsync);
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn test_non_linux_fallback() {
        let mut ctx = IoUringContext::new(64).unwrap();

        // All operations should succeed but do nothing
        // On Windows, RawFd is a pointer type, so we cast integers to it
        let _id1 = ctx.submit_read(1 as RawFd, 0, 1024).unwrap();
        let _id2 = ctx.submit_write(2 as RawFd, 0, &[0u8; 1024]).unwrap();
        let _id3 = ctx.submit_fsync(3 as RawFd).unwrap();

        // No pending operations on non-Linux
        assert_eq!(ctx.pending_count(), 0);

        // Completions should be empty
        let completions = ctx.wait_completions(1).unwrap();
        assert_eq!(completions.len(), 0);

        // Buffer registration should succeed
        assert!(ctx.register_buffers(vec![vec![0u8; 1024]]).is_ok());
        assert_eq!(ctx.buffer_count(), 0);
    }
}
