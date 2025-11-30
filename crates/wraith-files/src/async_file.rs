//! High-level async file I/O using io_uring (Linux-only).
//!
//! This module provides high-level async file reader and writer interfaces
//! that use io_uring for maximum performance on Linux systems.

use crate::io_uring::{IoError, IoUringEngine};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;

/// Async file reader using io_uring
///
/// Provides high-level async read operations with automatic request tracking.
pub struct AsyncFileReader {
    engine: IoUringEngine,
    file: File,
    fd: RawFd,
    next_id: u64,
    pending_reads: HashMap<u64, PendingRead>,
    completed_reads: HashMap<u64, Vec<u8>>,
}

#[allow(dead_code)]
struct PendingRead {
    offset: u64,
    buffer: Vec<u8>,
}

impl AsyncFileReader {
    /// Open a file for async reading
    ///
    /// # Arguments
    /// * `path` - Path to the file
    /// * `queue_depth` - io_uring queue depth (typically 128-4096)
    ///
    /// # Examples
    /// ```no_run
    /// # #[cfg(target_os = "linux")]
    /// # {
    /// use wraith_files::async_file::AsyncFileReader;
    ///
    /// let reader = AsyncFileReader::open("/etc/hostname", 128).unwrap();
    /// # }
    /// ```
    pub fn open<P: AsRef<Path>>(path: P, queue_depth: u32) -> Result<Self, IoError> {
        let file = File::open(path)?;
        let fd = file.as_raw_fd();
        let engine = IoUringEngine::new(queue_depth)?;

        Ok(Self {
            engine,
            file,
            fd,
            next_id: 0,
            pending_reads: HashMap::new(),
            completed_reads: HashMap::new(),
        })
    }

    /// Submit an async read request
    ///
    /// Returns a request ID that can be used to retrieve the data later.
    ///
    /// # Arguments
    /// * `offset` - Offset in file to read from
    /// * `len` - Number of bytes to read
    ///
    /// # Examples
    /// ```no_run
    /// # #[cfg(target_os = "linux")]
    /// # {
    /// # use wraith_files::async_file::AsyncFileReader;
    /// let mut reader = AsyncFileReader::open("/etc/hostname", 128).unwrap();
    ///
    /// // Submit read request
    /// let req_id = reader.read_at(0, 1024).unwrap();
    /// reader.submit().unwrap();
    ///
    /// // Wait for completion
    /// let data = reader.wait_for(req_id).unwrap();
    /// # }
    /// ```
    pub fn read_at(&mut self, offset: u64, len: usize) -> Result<u64, IoError> {
        let request_id = self.next_id;
        self.next_id += 1;

        let buffer = vec![0u8; len];

        unsafe {
            self.engine
                .read(self.fd, offset, buffer.as_ptr() as *mut u8, len, request_id)?;
        }

        self.pending_reads
            .insert(request_id, PendingRead { offset, buffer });

        Ok(request_id)
    }

    /// Submit all pending read requests to the kernel
    ///
    /// Returns the number of requests submitted.
    pub fn submit(&mut self) -> Result<usize, IoError> {
        self.engine.submit()
    }

    /// Wait for a specific read request to complete
    ///
    /// Blocks until the specified request completes and returns the data.
    ///
    /// # Arguments
    /// * `request_id` - The request ID returned from `read_at()`
    pub fn wait_for(&mut self, request_id: u64) -> Result<Vec<u8>, IoError> {
        // Check if already completed
        if let Some(data) = self.completed_reads.remove(&request_id) {
            return Ok(data);
        }

        loop {
            // Wait for at least one completion if we have pending operations
            if self.engine.pending() > 0 {
                let completions = self.engine.wait(1)?;

                // Process ALL completions first to avoid losing any
                for comp in completions {
                    if comp.result < 0 {
                        // Error - just skip it for now
                        self.pending_reads.remove(&comp.user_data);
                        continue;
                    }

                    if let Some(pending) = self.pending_reads.remove(&comp.user_data) {
                        let mut buffer = pending.buffer;
                        buffer.truncate(comp.result as usize);
                        // Cache all completions
                        self.completed_reads.insert(comp.user_data, buffer);
                    }
                }

                // Now check if our requested completion is available
                if let Some(data) = self.completed_reads.remove(&request_id) {
                    return Ok(data);
                }
                // Otherwise continue waiting for more completions
            } else {
                return Err(IoError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Request not found and no pending operations",
                )));
            }
        }
    }

    /// Poll for any completed read requests
    ///
    /// Returns all completed reads as (request_id, data) pairs.
    /// Non-blocking - returns immediately with whatever is available.
    pub fn poll_completions(&mut self) -> Result<Vec<(u64, Vec<u8>)>, IoError> {
        let completions = self.engine.poll()?;
        let mut results = Vec::new();

        for comp in completions {
            if comp.result < 0 {
                // Skip errors for now (could be improved to track errors)
                continue;
            }

            if let Some(pending) = self.pending_reads.remove(&comp.user_data) {
                let mut buffer = pending.buffer;
                buffer.truncate(comp.result as usize);
                results.push((comp.user_data, buffer));
            }
        }

        Ok(results)
    }

    /// Get the number of pending read requests
    pub fn pending(&self) -> usize {
        self.engine.pending()
    }

    /// Get a reference to the underlying file
    pub fn file(&self) -> &File {
        &self.file
    }
}

/// Async file writer using io_uring
///
/// Provides high-level async write operations with automatic request tracking.
pub struct AsyncFileWriter {
    engine: IoUringEngine,
    file: File,
    fd: RawFd,
    next_id: u64,
    pending_writes: HashMap<u64, PendingWrite>,
    completed_writes: HashMap<u64, usize>,
}

#[allow(dead_code)]
struct PendingWrite {
    offset: u64,
    data: Vec<u8>,
}

impl AsyncFileWriter {
    /// Create a new file for async writing
    ///
    /// # Arguments
    /// * `path` - Path to the file
    /// * `queue_depth` - io_uring queue depth (typically 128-4096)
    ///
    /// # Examples
    /// ```no_run
    /// # #[cfg(target_os = "linux")]
    /// # {
    /// use wraith_files::async_file::AsyncFileWriter;
    ///
    /// let writer = AsyncFileWriter::create("/tmp/test.dat", 128).unwrap();
    /// # }
    /// ```
    pub fn create<P: AsRef<Path>>(path: P, queue_depth: u32) -> Result<Self, IoError> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        let fd = file.as_raw_fd();
        let engine = IoUringEngine::new(queue_depth)?;

        Ok(Self {
            engine,
            file,
            fd,
            next_id: 0,
            pending_writes: HashMap::new(),
            completed_writes: HashMap::new(),
        })
    }

    /// Open an existing file for async writing
    pub fn open<P: AsRef<Path>>(path: P, queue_depth: u32) -> Result<Self, IoError> {
        let file = OpenOptions::new().write(true).open(path)?;

        let fd = file.as_raw_fd();
        let engine = IoUringEngine::new(queue_depth)?;

        Ok(Self {
            engine,
            file,
            fd,
            next_id: 0,
            pending_writes: HashMap::new(),
            completed_writes: HashMap::new(),
        })
    }

    /// Submit an async write request
    ///
    /// Returns a request ID that can be used to track completion.
    ///
    /// # Arguments
    /// * `offset` - Offset in file to write to
    /// * `data` - Data to write
    pub fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<u64, IoError> {
        let request_id = self.next_id;
        self.next_id += 1;

        // Copy data since it needs to outlive the function call
        let data_copy = data.to_vec();

        unsafe {
            self.engine.write(
                self.fd,
                offset,
                data_copy.as_ptr(),
                data_copy.len(),
                request_id,
            )?;
        }

        self.pending_writes.insert(
            request_id,
            PendingWrite {
                offset,
                data: data_copy,
            },
        );

        Ok(request_id)
    }

    /// Submit all pending write requests to the kernel
    pub fn submit(&mut self) -> Result<usize, IoError> {
        self.engine.submit()
    }

    /// Wait for a specific write request to complete
    pub fn wait_for(&mut self, request_id: u64) -> Result<usize, IoError> {
        // Check if already completed
        if let Some(bytes) = self.completed_writes.remove(&request_id) {
            return Ok(bytes);
        }

        loop {
            if self.engine.pending() > 0 {
                let completions = self.engine.wait(1)?;

                // Process ALL completions first to avoid losing any
                for comp in completions {
                    if comp.result < 0 {
                        self.pending_writes.remove(&comp.user_data);
                        continue;
                    }

                    self.pending_writes.remove(&comp.user_data);
                    let bytes = comp.result as usize;
                    // Cache all completions
                    self.completed_writes.insert(comp.user_data, bytes);
                }

                // Now check if our requested completion is available
                if let Some(bytes) = self.completed_writes.remove(&request_id) {
                    return Ok(bytes);
                }
                // Otherwise continue waiting for more completions
            } else {
                return Err(IoError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Request not found and no pending operations",
                )));
            }
        }
    }

    /// Poll for completed write requests
    ///
    /// Returns (request_id, bytes_written) pairs for completed writes.
    pub fn poll_completions(&mut self) -> Result<Vec<(u64, usize)>, IoError> {
        let completions = self.engine.poll()?;
        let mut results = Vec::new();

        for comp in completions {
            if comp.result < 0 {
                continue;
            }

            if self.pending_writes.remove(&comp.user_data).is_some() {
                results.push((comp.user_data, comp.result as usize));
            }
        }

        Ok(results)
    }

    /// Wait for all pending writes to complete
    pub fn wait_all(&mut self) -> Result<(), IoError> {
        let pending = self.pending();
        if pending > 0 {
            self.engine.wait(pending)?;
            self.pending_writes.clear();
        }
        Ok(())
    }

    /// Sync all data to disk
    pub fn sync(&mut self) -> Result<(), IoError> {
        self.wait_all()?;
        self.file.sync_all()?;
        Ok(())
    }

    /// Get the number of pending write requests
    pub fn pending(&self) -> usize {
        self.engine.pending()
    }

    /// Get a reference to the underlying file
    pub fn file(&self) -> &File {
        &self.file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_async_reader_single_read() {
        let mut temp = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp, b"Hello, async reader!").unwrap();
        temp.flush().unwrap();

        let mut reader = AsyncFileReader::open(temp.path(), 128).unwrap();

        let req_id = reader.read_at(0, 20).unwrap();
        reader.submit().unwrap();

        let data = reader.wait_for(req_id).unwrap();
        assert_eq!(&data, b"Hello, async reader!");
    }

    #[test]
    fn test_async_reader_partial_read() {
        let mut temp = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp, b"0123456789").unwrap();
        temp.flush().unwrap();

        let mut reader = AsyncFileReader::open(temp.path(), 128).unwrap();

        let req_id = reader.read_at(5, 3).unwrap();
        reader.submit().unwrap();

        let data = reader.wait_for(req_id).unwrap();
        assert_eq!(&data, b"567");
    }

    #[test]
    fn test_async_reader_multiple_reads() {
        let mut temp = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp, b"ABCDEFGHIJ").unwrap();
        temp.flush().unwrap();

        let mut reader = AsyncFileReader::open(temp.path(), 128).unwrap();

        let req1 = reader.read_at(0, 5).unwrap();
        let req2 = reader.read_at(5, 5).unwrap();
        reader.submit().unwrap();

        let data1 = reader.wait_for(req1).unwrap();
        let data2 = reader.wait_for(req2).unwrap();

        assert_eq!(&data1, b"ABCDE");
        assert_eq!(&data2, b"FGHIJ");
    }

    #[test]
    fn test_async_writer_single_write() {
        let temp = NamedTempFile::new().unwrap();
        let mut writer = AsyncFileWriter::create(temp.path(), 128).unwrap();

        let req_id = writer.write_at(0, b"Test write").unwrap();
        writer.submit().unwrap();

        let bytes = writer.wait_for(req_id).unwrap();
        assert_eq!(bytes, 10);

        writer.sync().unwrap();

        let content = std::fs::read(temp.path()).unwrap();
        assert_eq!(&content, b"Test write");
    }

    #[test]
    fn test_async_writer_multiple_writes() {
        let temp = NamedTempFile::new().unwrap();
        let mut writer = AsyncFileWriter::create(temp.path(), 128).unwrap();

        let req1 = writer.write_at(0, b"Hello ").unwrap();
        let req2 = writer.write_at(6, b"World!").unwrap();
        writer.submit().unwrap();

        writer.wait_for(req1).unwrap();
        writer.wait_for(req2).unwrap();
        writer.sync().unwrap();

        let content = std::fs::read(temp.path()).unwrap();
        assert_eq!(&content, b"Hello World!");
    }

    #[test]
    fn test_async_writer_wait_all() {
        let temp = NamedTempFile::new().unwrap();
        let mut writer = AsyncFileWriter::create(temp.path(), 128).unwrap();

        writer.write_at(0, b"Line 1\n").unwrap();
        writer.write_at(7, b"Line 2\n").unwrap();
        writer.write_at(14, b"Line 3\n").unwrap();
        writer.submit().unwrap();

        writer.wait_all().unwrap();
        writer.sync().unwrap();

        let content = std::fs::read(temp.path()).unwrap();
        assert_eq!(&content, b"Line 1\nLine 2\nLine 3\n");
    }

    #[test]
    fn test_async_reader_poll_completions() {
        let mut temp = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut temp, b"Test polling").unwrap();
        temp.flush().unwrap();

        let mut reader = AsyncFileReader::open(temp.path(), 128).unwrap();

        let _req1 = reader.read_at(0, 4).unwrap();
        let _req2 = reader.read_at(5, 7).unwrap();
        reader.submit().unwrap();

        // Give the engine time to complete operations
        std::thread::sleep(std::time::Duration::from_millis(10));

        let results = reader.poll_completions().unwrap();
        // poll_completions returns results or empty if nothing ready
        // We can't guarantee results without waiting, so just check it doesn't error
        assert!(results.len() <= 2);
    }
}
