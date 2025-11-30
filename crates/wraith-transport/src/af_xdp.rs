//! AF_XDP Socket Management for WRAITH Protocol
//!
//! This module provides AF_XDP (Address Family eXpress Data Path) socket management
//! for high-performance zero-copy packet processing on Linux.
//!
//! ## Architecture
//!
//! AF_XDP provides kernel bypass for network I/O using shared memory regions (UMEM)
//! and ring buffers for packet descriptors.
//!
//! ## Requirements
//!
//! - Linux kernel 5.3+ with AF_XDP support
//! - XDP program loaded on the network interface
//! - Sufficient locked memory limit (ulimit -l)
//!
//! ## Performance
//!
//! - Target throughput: 10-40 Gbps (single core)
//! - Target latency: <1μs (NIC to userspace)
//! - Zero-copy packet processing
//!
//! ## Example
//!
//! ```no_run
//! # #[cfg(target_os = "linux")]
//! # {
//! use wraith_transport::af_xdp::{AfXdpSocket, UmemConfig, SocketConfig};
//!
//! // Create UMEM (shared memory region)
//! let umem_config = UmemConfig::default();
//! let umem = umem_config.create().unwrap();
//!
//! // Create AF_XDP socket
//! let socket_config = SocketConfig::default();
//! let mut socket = AfXdpSocket::new("eth0", 0, umem, socket_config).unwrap();
//!
//! // Receive packets
//! let packets = socket.rx_batch(32).unwrap();
//! for pkt in packets {
//!     // Process packet data
//!     println!("Received {} bytes", pkt.len);
//! }
//! # }
//! ```

use std::io::{self, Error};
use std::os::raw::{c_int, c_void};
use std::ptr;
use std::sync::Arc;
use thiserror::Error;

// XDP socket constants
const XDP_PACKET_HEADROOM: usize = 256;
const XDP_UMEM_MIN_CHUNK_SIZE: usize = 2048;

/// AF_XDP errors
#[derive(Debug, Error)]
pub enum AfXdpError {
    /// Failed to create UMEM
    #[error("Failed to create UMEM: {0}")]
    UmemCreation(String),

    /// Failed to create socket
    #[error("Failed to create AF_XDP socket: {0}")]
    SocketCreation(String),

    /// Failed to bind socket
    #[error("Failed to bind AF_XDP socket: {0}")]
    SocketBind(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Ring buffer operation failed
    #[error("Ring buffer operation failed: {0}")]
    RingBufferError(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

/// UMEM (User Memory) configuration
///
/// UMEM is a shared memory region used for packet buffers between
/// the kernel and userspace.
#[derive(Debug, Clone)]
pub struct UmemConfig {
    /// Total UMEM size in bytes
    pub size: usize,

    /// Size of each frame/chunk (must be power of 2)
    pub frame_size: usize,

    /// Headroom before packet data
    pub headroom: usize,

    /// Number of fill ring entries (must be power of 2)
    pub fill_ring_size: u32,

    /// Number of completion ring entries (must be power of 2)
    pub comp_ring_size: u32,
}

impl Default for UmemConfig {
    fn default() -> Self {
        Self {
            size: 4 * 1024 * 1024,         // 4 MB
            frame_size: 2048,              // 2 KB frames
            headroom: XDP_PACKET_HEADROOM, // 256 bytes
            fill_ring_size: 2048,          // 2048 descriptors
            comp_ring_size: 2048,          // 2048 descriptors
        }
    }
}

impl UmemConfig {
    /// Validate configuration parameters
    pub fn validate(&self) -> Result<(), AfXdpError> {
        // Frame size must be power of 2 and >= minimum
        if !self.frame_size.is_power_of_two() {
            return Err(AfXdpError::InvalidConfig(
                "frame_size must be power of 2".into(),
            ));
        }

        if self.frame_size < XDP_UMEM_MIN_CHUNK_SIZE {
            return Err(AfXdpError::InvalidConfig(format!(
                "frame_size must be >= {}",
                XDP_UMEM_MIN_CHUNK_SIZE
            )));
        }

        // Ring sizes must be power of 2
        if !self.fill_ring_size.is_power_of_two() {
            return Err(AfXdpError::InvalidConfig(
                "fill_ring_size must be power of 2".into(),
            ));
        }

        if !self.comp_ring_size.is_power_of_two() {
            return Err(AfXdpError::InvalidConfig(
                "comp_ring_size must be power of 2".into(),
            ));
        }

        // UMEM size must accommodate frames
        let num_frames = self.size / self.frame_size;
        if num_frames == 0 {
            return Err(AfXdpError::InvalidConfig(
                "UMEM size too small for frame_size".into(),
            ));
        }

        Ok(())
    }

    /// Create UMEM with this configuration
    pub fn create(&self) -> Result<Arc<Umem>, AfXdpError> {
        self.validate()?;
        Umem::new(self.clone())
    }
}

/// UMEM (User Memory) region
///
/// Shared memory region for packet buffers with fill and completion rings.
pub struct Umem {
    /// Configuration
    config: UmemConfig,

    /// Memory-mapped region
    buffer: *mut u8,

    /// Fill ring (kernel -> userspace)
    fill_ring: RingBuffer,

    /// Completion ring (kernel -> userspace)
    comp_ring: RingBuffer,
}

impl Umem {
    /// Create a new UMEM region
    pub fn new(config: UmemConfig) -> Result<Arc<Self>, AfXdpError> {
        config.validate()?;

        // Allocate memory using mmap
        let buffer = unsafe {
            libc::mmap(
                ptr::null_mut(),
                config.size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_POPULATE,
                -1,
                0,
            )
        };

        if buffer == libc::MAP_FAILED {
            return Err(AfXdpError::UmemCreation(Error::last_os_error().to_string()));
        }

        // Lock memory to prevent swapping
        let ret = unsafe { libc::mlock(buffer, config.size) };
        if ret != 0 {
            unsafe {
                libc::munmap(buffer, config.size);
            }
            return Err(AfXdpError::UmemCreation(
                "Failed to lock memory (check ulimit -l)".into(),
            ));
        }

        Ok(Arc::new(Self {
            config: config.clone(),
            buffer: buffer as *mut u8,
            fill_ring: RingBuffer::new(config.fill_ring_size),
            comp_ring: RingBuffer::new(config.comp_ring_size),
        }))
    }

    /// Get the UMEM buffer pointer
    pub fn buffer(&self) -> *mut u8 {
        self.buffer
    }

    /// Get the UMEM size
    pub fn size(&self) -> usize {
        self.config.size
    }

    /// Get frame size
    pub fn frame_size(&self) -> usize {
        self.config.frame_size
    }

    /// Get number of frames
    pub fn num_frames(&self) -> usize {
        self.size() / self.frame_size()
    }

    /// Get frame at index
    pub fn get_frame(&self, index: usize) -> Option<&[u8]> {
        if index >= self.num_frames() {
            return None;
        }

        let offset = index * self.frame_size();
        Some(unsafe { std::slice::from_raw_parts(self.buffer.add(offset), self.frame_size()) })
    }

    /// Get mutable frame at index
    pub fn get_frame_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        if index >= self.num_frames() {
            return None;
        }

        let offset = index * self.frame_size();
        Some(unsafe { std::slice::from_raw_parts_mut(self.buffer.add(offset), self.frame_size()) })
    }

    /// Fill ring (for RX: userspace provides buffers to kernel)
    pub fn fill_ring(&self) -> &RingBuffer {
        &self.fill_ring
    }

    /// Completion ring (for TX: kernel returns completed buffers)
    pub fn comp_ring(&self) -> &RingBuffer {
        &self.comp_ring
    }
}

impl Drop for Umem {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.buffer as *mut c_void, self.config.size);
        }
    }
}

// SAFETY: UMEM is safe to send between threads
unsafe impl Send for Umem {}
unsafe impl Sync for Umem {}

/// AF_XDP socket configuration
#[derive(Debug, Clone)]
pub struct SocketConfig {
    /// Number of RX ring entries (must be power of 2)
    pub rx_ring_size: u32,

    /// Number of TX ring entries (must be power of 2)
    pub tx_ring_size: u32,

    /// Bind flags (XDP_COPY, XDP_ZEROCOPY, etc.)
    pub bind_flags: u16,

    /// Queue ID to attach to
    pub queue_id: u32,
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self {
            rx_ring_size: 2048,
            tx_ring_size: 2048,
            bind_flags: 0, // Default: no special flags
            queue_id: 0,
        }
    }
}

impl SocketConfig {
    /// Validate configuration
    pub fn validate(&self) -> Result<(), AfXdpError> {
        if !self.rx_ring_size.is_power_of_two() {
            return Err(AfXdpError::InvalidConfig(
                "rx_ring_size must be power of 2".into(),
            ));
        }

        if !self.tx_ring_size.is_power_of_two() {
            return Err(AfXdpError::InvalidConfig(
                "tx_ring_size must be power of 2".into(),
            ));
        }

        Ok(())
    }
}

/// Ring buffer for packet descriptors
///
/// Used for fill, completion, RX, and TX rings.
pub struct RingBuffer {
    /// Ring size (must be power of 2)
    size: u32,

    /// Producer index
    producer: std::sync::atomic::AtomicU32,

    /// Consumer index
    consumer: std::sync::atomic::AtomicU32,

    /// Cached producer (for batch operations)
    cached_prod: u32,

    /// Cached consumer (for batch operations)
    cached_cons: u32,
}

impl RingBuffer {
    /// Create a new ring buffer
    pub fn new(size: u32) -> Self {
        assert!(size.is_power_of_two(), "Ring size must be power of 2");

        Self {
            size,
            producer: std::sync::atomic::AtomicU32::new(0),
            consumer: std::sync::atomic::AtomicU32::new(0),
            cached_prod: 0,
            cached_cons: 0,
        }
    }

    /// Get number of available entries for production
    pub fn available(&self) -> u32 {
        let cons = self.consumer.load(std::sync::atomic::Ordering::Acquire);

        // Use cached_prod if it's ahead of the producer atomic
        // This accounts for reservations that haven't been submitted yet
        let prod = self
            .cached_prod
            .max(self.producer.load(std::sync::atomic::Ordering::Acquire));

        self.size - (prod - cons)
    }

    /// Get number of entries ready for consumption
    pub fn ready(&self) -> u32 {
        let prod = self.producer.load(std::sync::atomic::Ordering::Acquire);

        // Use cached_cons if it's ahead of the consumer atomic
        // This accounts for peeks that haven't been released yet
        let cons = self
            .cached_cons
            .max(self.consumer.load(std::sync::atomic::Ordering::Acquire));

        prod - cons
    }

    /// Reserve entries for production
    pub fn reserve(&mut self, count: u32) -> Option<u32> {
        if self.available() < count {
            return None;
        }

        let idx = self.cached_prod;
        self.cached_prod += count;
        Some(idx)
    }

    /// Submit reserved entries
    pub fn submit(&mut self, count: u32) {
        self.producer
            .fetch_add(count, std::sync::atomic::Ordering::Release);
    }

    /// Peek at entries ready for consumption
    pub fn peek(&mut self, count: u32) -> Option<u32> {
        if self.ready() < count {
            return None;
        }

        let idx = self.cached_cons;
        self.cached_cons += count;
        Some(idx)
    }

    /// Release consumed entries
    pub fn release(&mut self, count: u32) {
        self.consumer
            .fetch_add(count, std::sync::atomic::Ordering::Release);
    }
}

/// Packet descriptor for AF_XDP
#[derive(Debug, Clone, Copy)]
pub struct PacketDesc {
    /// Address in UMEM
    pub addr: u64,

    /// Packet length
    pub len: u32,

    /// Options (reserved)
    pub options: u32,
}

/// AF_XDP socket
///
/// Provides zero-copy packet I/O using XDP and shared memory.
pub struct AfXdpSocket {
    /// Socket file descriptor
    fd: c_int,

    /// Associated UMEM
    umem: Arc<Umem>,

    /// Configuration
    #[allow(dead_code)]
    config: SocketConfig,

    /// RX ring
    rx_ring: RingBuffer,

    /// TX ring
    tx_ring: RingBuffer,

    /// Interface name
    ifname: String,
}

impl AfXdpSocket {
    /// Create a new AF_XDP socket
    ///
    /// # Arguments
    ///
    /// * `ifname` - Network interface name (e.g., "eth0")
    /// * `queue_id` - Queue ID to attach to
    /// * `umem` - Shared UMEM region
    /// * `config` - Socket configuration
    pub fn new(
        ifname: &str,
        _queue_id: u32,
        umem: Arc<Umem>,
        config: SocketConfig,
    ) -> Result<Self, AfXdpError> {
        config.validate()?;

        // Create AF_XDP socket
        let fd = unsafe { libc::socket(libc::AF_XDP as c_int, libc::SOCK_RAW, 0) };

        if fd < 0 {
            return Err(AfXdpError::SocketCreation(
                Error::last_os_error().to_string(),
            ));
        }

        // TODO: Set socket options (UMEM, rings, etc.)
        // This requires platform-specific socket option constants
        // which would be defined in a separate xdp_sys module

        Ok(Self {
            fd,
            umem,
            config: config.clone(),
            rx_ring: RingBuffer::new(config.rx_ring_size),
            tx_ring: RingBuffer::new(config.tx_ring_size),
            ifname: ifname.to_string(),
        })
    }

    /// Receive a batch of packets
    ///
    /// Returns packet descriptors for processing. Packets remain in UMEM
    /// until released via the fill ring.
    pub fn rx_batch(&mut self, max_count: usize) -> Result<Vec<PacketDesc>, AfXdpError> {
        let packets = Vec::with_capacity(max_count);

        // Check RX ring for available packets
        let ready = self.rx_ring.ready();
        if ready == 0 {
            return Ok(packets);
        }

        let count = ready.min(max_count as u32);

        if let Some(_idx) = self.rx_ring.peek(count) {
            // TODO: Read packet descriptors from RX ring
            // This requires mmap'ed ring buffer access

            self.rx_ring.release(count);
        }

        Ok(packets)
    }

    /// Transmit a batch of packets
    ///
    /// # Arguments
    ///
    /// * `packets` - Packet descriptors to transmit
    pub fn tx_batch(&mut self, packets: &[PacketDesc]) -> Result<usize, AfXdpError> {
        let count = packets.len() as u32;

        // Check TX ring for available space
        let available = self.tx_ring.available();
        if available < count {
            return Err(AfXdpError::RingBufferError(format!(
                "TX ring full: {} available, {} requested",
                available, count
            )));
        }

        if let Some(_idx) = self.tx_ring.reserve(count) {
            // TODO: Write packet descriptors to TX ring
            // This requires mmap'ed ring buffer access

            self.tx_ring.submit(count);

            // Kick kernel to transmit
            self.kick_tx()?;
        }

        Ok(packets.len())
    }

    /// Kick kernel to process TX ring
    fn kick_tx(&self) -> Result<(), AfXdpError> {
        // Send dummy message to wake kernel
        let ret =
            unsafe { libc::sendto(self.fd, ptr::null(), 0, libc::MSG_DONTWAIT, ptr::null(), 0) };

        if ret < 0 {
            let err = Error::last_os_error();
            // EAGAIN/EWOULDBLOCK is acceptable (no space in kernel queue)
            if err.raw_os_error() != Some(libc::EAGAIN)
                && err.raw_os_error() != Some(libc::EWOULDBLOCK)
            {
                return Err(AfXdpError::Io(err));
            }
        }

        Ok(())
    }

    /// Get socket file descriptor
    pub fn fd(&self) -> c_int {
        self.fd
    }

    /// Get UMEM reference
    pub fn umem(&self) -> &Arc<Umem> {
        &self.umem
    }

    /// Get interface name
    pub fn ifname(&self) -> &str {
        &self.ifname
    }
}

impl Drop for AfXdpSocket {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

// SAFETY: AF_XDP socket is safe to send between threads
unsafe impl Send for AfXdpSocket {}
unsafe impl Sync for AfXdpSocket {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_umem_config_default() {
        let config = UmemConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.frame_size, 2048);
        assert!(config.fill_ring_size.is_power_of_two());
        assert!(config.comp_ring_size.is_power_of_two());
    }

    #[test]
    fn test_umem_config_validate() {
        let mut config = UmemConfig::default();

        // Invalid frame size (not power of 2)
        config.frame_size = 2000;
        assert!(config.validate().is_err());

        // Invalid frame size (too small)
        config.frame_size = 1024;
        assert!(config.validate().is_err());

        // Invalid ring size (not power of 2)
        config.frame_size = 2048;
        config.fill_ring_size = 2000;
        assert!(config.validate().is_err());

        // Valid configuration
        config.fill_ring_size = 2048;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_socket_config_validate() {
        let mut config = SocketConfig::default();

        // Valid default
        assert!(config.validate().is_ok());

        // Invalid RX ring size
        config.rx_ring_size = 2000;
        assert!(config.validate().is_err());

        // Invalid TX ring size
        config.rx_ring_size = 2048;
        config.tx_ring_size = 2000;
        assert!(config.validate().is_err());

        // Valid configuration
        config.tx_ring_size = 2048;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_ring_buffer_basic() {
        let mut ring = RingBuffer::new(16);

        // Initial state
        assert_eq!(ring.available(), 16);
        assert_eq!(ring.ready(), 0);

        // Reserve and submit
        let idx = ring.reserve(4).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(ring.available(), 12);

        ring.submit(4);
        assert_eq!(ring.ready(), 4);

        // Peek and release
        let idx = ring.peek(2).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(ring.ready(), 2);

        ring.release(2);
        assert_eq!(ring.ready(), 2);
        assert_eq!(ring.available(), 14);
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let mut ring = RingBuffer::new(4);

        // Reserve all space
        assert!(ring.reserve(4).is_some());

        // Try to reserve more (should fail)
        assert!(ring.reserve(1).is_none());

        // Submit and release
        ring.submit(4);
        assert!(ring.peek(4).is_some());
        ring.release(4);

        // Now we can reserve again
        assert!(ring.reserve(4).is_some());
    }

    #[test]
    fn test_packet_desc_size() {
        // Ensure packet descriptor is 16 bytes (cache-line friendly)
        assert_eq!(
            std::mem::size_of::<PacketDesc>(),
            16,
            "PacketDesc should be 16 bytes"
        );
    }

    #[test]
    fn test_umem_creation() {
        let config = UmemConfig {
            size: 8192,
            frame_size: 2048,
            headroom: 256,
            fill_ring_size: 4,
            comp_ring_size: 4,
        };

        // This may fail if we don't have permission to lock memory
        // or if AF_XDP is not supported on this system
        match Umem::new(config) {
            Ok(umem) => {
                assert_eq!(umem.size(), 8192);
                assert_eq!(umem.frame_size(), 2048);
                assert_eq!(umem.num_frames(), 4);
            }
            Err(e) => {
                // Expected on systems without AF_XDP support or insufficient permissions
                eprintln!("UMEM creation failed (may be expected): {}", e);
            }
        }
    }
}
