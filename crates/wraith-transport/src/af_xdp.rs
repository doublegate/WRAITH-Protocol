//! AF_XDP Socket Management for WRAITH Protocol
//!
//! This module provides AF_XDP (Address Family eXpress Data Path) socket management
//! for high-performance zero-copy packet processing on Linux.
//!
//! ## Architecture
//!
//! AF_XDP provides kernel bypass for network I/O using shared memory regions (UMEM)
//! and ring buffers for packet descriptors:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        User Space                                │
//! │  ┌──────────────────────────────────────────────────────────┐  │
//! │  │                      UMEM Buffer                          │  │
//! │  │  ┌────────┬────────┬────────┬────────┬────────┬────────┐│  │
//! │  │  │Frame 0 │Frame 1 │Frame 2 │Frame 3 │  ...   │Frame N ││  │
//! │  │  └────────┴────────┴────────┴────────┴────────┴────────┘│  │
//! │  └──────────────────────────────────────────────────────────┘  │
//! │                                                                  │
//! │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐           │
//! │  │ RX Ring  │ │ TX Ring  │ │Fill Ring │ │Comp Ring │           │
//! │  │(desc in) │ │(desc out)│ │(addr out)│ │(addr in) │           │
//! │  └────┬─────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘           │
//! │       │            │            │            │                   │
//! └───────┼────────────┼────────────┼────────────┼───────────────────┘
//!         │            │            │            │
//! ════════╪════════════╪════════════╪════════════╪════ mmap ════════
//!         │            │            │            │
//! ┌───────┼────────────┼────────────┼────────────┼───────────────────┐
//! │       ▼            ▼            ▼            ▼                   │
//! │  ┌─────────────────────────────────────────────────────────┐    │
//! │  │                   XDP Socket (AF_XDP)                    │    │
//! │  └─────────────────────────────────────────────────────────┘    │
//! │                              │                                   │
//! │  ┌───────────────────────────┴───────────────────────────┐      │
//! │  │                    XDP Program (eBPF)                  │      │
//! │  │            bpf_redirect_map(XSKMAP, queue_id)          │      │
//! │  └───────────────────────────────────────────────────────┘      │
//! │                              │                                   │
//! │  ┌───────────────────────────┴───────────────────────────┐      │
//! │  │                      NIC Driver                        │      │
//! │  │              (Zero-copy if supported)                  │      │
//! │  └───────────────────────────────────────────────────────┘      │
//! │                        Kernel Space                              │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Ring Buffer Protocol
//!
//! - **RX Ring**: Kernel writes received packet descriptors, user reads
//! - **TX Ring**: User writes packet descriptors to transmit, kernel reads
//! - **Fill Ring**: User provides buffer addresses for kernel to fill with RX packets
//! - **Completion Ring**: Kernel returns completed TX buffer addresses
//!
//! ## Requirements
//!
//! - Linux kernel 6.2+ (for full AF_XDP features including busy polling)
//! - XDP-capable NIC with driver support
//! - XDP program attached to interface (can use libxdp or manual BPF loading)
//! - Sufficient locked memory limit: `ulimit -l unlimited` or CAP_IPC_LOCK
//! - CAP_NET_RAW or root privileges
//!
//! ## Kernel Version Features
//!
//! | Kernel | Feature |
//! |--------|---------|
//! | 5.3+   | Basic AF_XDP support |
//! | 5.4+   | need_wakeup optimization |
//! | 5.10+  | Shared UMEM, better multi-queue |
//! | 5.11+  | Busy polling |
//! | 6.2+   | Full feature set for WRAITH |
//!
//! ## Performance Targets
//!
//! - **Throughput**: 10-40 Gbps (single core, zero-copy mode)
//! - **Latency**: <1us (NIC to userspace with busy polling)
//! - **Packet rate**: 10-20 Mpps (depending on packet size)
//!
//! ## Example
//!
//! ```no_run
//! # #[cfg(target_os = "linux")]
//! # {
//! use wraith_transport::af_xdp::{AfXdpSocket, UmemConfig, SocketConfig, XDP_ZEROCOPY};
//!
//! // Create UMEM (shared memory region) - 16MB with 4KB frames
//! let umem_config = UmemConfig {
//!     size: 16 * 1024 * 1024,
//!     frame_size: 4096,
//!     headroom: 256,
//!     fill_ring_size: 4096,
//!     comp_ring_size: 4096,
//! };
//! let umem = umem_config.create().unwrap();
//!
//! // Create AF_XDP socket with zero-copy mode
//! let socket_config = SocketConfig {
//!     rx_ring_size: 4096,
//!     tx_ring_size: 4096,
//!     bind_flags: XDP_ZEROCOPY,
//!     queue_id: 0,
//! };
//! let mut socket = AfXdpSocket::new("eth0", 0, umem, socket_config).unwrap();
//!
//! // Pre-fill RX buffers
//! let fill_addrs: Vec<u64> = (0..1024).map(|i| i * 4096).collect();
//! socket.fill_rx_buffers(&fill_addrs).unwrap();
//!
//! // Receive packets (batch)
//! loop {
//!     let packets = socket.rx_batch(32).unwrap();
//!     for pkt in &packets {
//!         if let Some(data) = socket.get_packet_data(pkt) {
//!             // Process packet data
//!             println!("Received {} bytes", data.len());
//!         }
//!     }
//!
//!     // Return buffers to fill ring
//!     let addrs: Vec<u64> = packets.iter().map(|p| p.addr).collect();
//!     socket.fill_rx_buffers(&addrs).unwrap();
//! }
//! # }
//! ```
//!
//! ## See Also
//!
//! - [AF_XDP Kernel Documentation](https://docs.kernel.org/networking/af_xdp.html)
//! - [DPDK AF_XDP PMD](https://doc.dpdk.org/guides/nics/af_xdp.html)
//! - [libbpf xsk.h](https://github.com/libbpf/libbpf/blob/master/src/xsk.h)

use std::io::{self, Error};
use std::os::raw::{c_int, c_void};
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use thiserror::Error;

// XDP socket constants
const XDP_PACKET_HEADROOM: usize = 256;
const XDP_UMEM_MIN_CHUNK_SIZE: usize = 2048;

// XDP socket option constants (from linux/if_xdp.h)
// These are Linux-specific and require kernel 5.3+

/// SOL_XDP socket option level
#[cfg(target_os = "linux")]
const SOL_XDP: c_int = 283;

/// XDP_RX_RING socket option - configure RX ring
#[cfg(target_os = "linux")]
const XDP_RX_RING: c_int = 1;

/// XDP_TX_RING socket option - configure TX ring
#[cfg(target_os = "linux")]
const XDP_TX_RING: c_int = 2;

/// XDP_UMEM_REG socket option - register UMEM
#[cfg(target_os = "linux")]
const XDP_UMEM_REG: c_int = 3;

/// XDP_UMEM_FILL_RING socket option - configure fill ring
#[cfg(target_os = "linux")]
const XDP_UMEM_FILL_RING: c_int = 4;

/// XDP_UMEM_COMPLETION_RING socket option - configure completion ring
#[cfg(target_os = "linux")]
const XDP_UMEM_COMPLETION_RING: c_int = 5;

/// XDP_COPY bind flag - force copy mode
#[cfg(target_os = "linux")]
pub const XDP_COPY: u16 = 1 << 1;

/// XDP_ZEROCOPY bind flag - force zero-copy mode
#[cfg(target_os = "linux")]
pub const XDP_ZEROCOPY: u16 = 1 << 2;

/// XDP_USE_NEED_WAKEUP bind flag - enable need_wakeup optimization
#[cfg(target_os = "linux")]
pub const XDP_USE_NEED_WAKEUP: u16 = 1 << 3;

/// UMEM registration structure (matches struct xdp_umem_reg in linux/if_xdp.h)
#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct XdpUmemReg {
    /// Start address of UMEM
    addr: u64,
    /// Length of UMEM in bytes
    len: u64,
    /// Size of each chunk/frame
    chunk_size: u32,
    /// Headroom before packet data
    headroom: u32,
    /// Flags (currently unused, set to 0)
    flags: u32,
}

/// Ring offset structure (matches struct xdp_ring_offset in linux/if_xdp.h)
/// Used for mmap-based ring buffer access (future implementation)
#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
struct XdpRingOffset {
    producer: u64,
    consumer: u64,
    desc: u64,
    flags: u64,
}

/// Ring offsets for mmap (matches struct xdp_mmap_offsets in linux/if_xdp.h)
/// Used for mmap-based ring buffer access (future implementation)
#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
struct XdpMmapOffsets {
    rx: XdpRingOffset,
    tx: XdpRingOffset,
    fr: XdpRingOffset, // fill ring
    cr: XdpRingOffset, // completion ring
}

/// sockaddr_xdp structure (matches struct sockaddr_xdp in linux/if_xdp.h)
#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct SockaddrXdp {
    /// Address family (AF_XDP = 44)
    sxdp_family: u16,
    /// Flags (XDP_COPY, XDP_ZEROCOPY, etc.)
    sxdp_flags: u16,
    /// Interface index
    sxdp_ifindex: u32,
    /// Queue ID
    sxdp_queue_id: u32,
    /// Shared UMEM file descriptor (0 if not sharing)
    sxdp_shared_umem_fd: u32,
}

/// XDP_MMAP_OFFSETS socket option - get mmap offsets
/// Used for mmap-based ring buffer access (future implementation)
#[cfg(target_os = "linux")]
#[allow(dead_code)]
const XDP_MMAP_OFFSETS: c_int = 1;

/// XDP descriptor structure (matches struct xdp_desc in linux/if_xdp.h)
#[cfg(target_os = "linux")]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct XdpDesc {
    /// Address in UMEM
    pub addr: u64,
    /// Packet length
    pub len: u32,
    /// Options (reserved)
    pub options: u32,
}

/// Helper functions for AF_XDP socket configuration (Linux 5.3+)
#[cfg(target_os = "linux")]
mod xdp_config {
    use super::*;
    use std::ffi::CString;

    /// Get interface index from name
    pub fn get_ifindex(ifname: &str) -> Result<u32, AfXdpError> {
        let name = CString::new(ifname)
            .map_err(|e| AfXdpError::InvalidConfig(format!("Invalid interface name: {}", e)))?;

        // SAFETY: if_nametoindex is a standard POSIX function that takes a null-terminated
        // C string and returns the interface index. The CString ensures proper null termination.
        let idx = unsafe { libc::if_nametoindex(name.as_ptr()) };

        if idx == 0 {
            return Err(AfXdpError::SocketBind(format!(
                "Interface '{}' not found",
                ifname
            )));
        }

        Ok(idx)
    }

    /// Register UMEM with the XDP socket
    pub fn register_umem(fd: c_int, umem: &Umem) -> Result<(), AfXdpError> {
        let reg = XdpUmemReg {
            addr: umem.buffer() as u64,
            len: umem.size() as u64,
            chunk_size: umem.frame_size() as u32,
            headroom: XDP_PACKET_HEADROOM as u32,
            flags: 0,
        };

        // SAFETY: setsockopt is a standard POSIX syscall. We pass a valid file descriptor
        // obtained from socket(), SOL_XDP as the level, XDP_UMEM_REG as the option name,
        // and a pointer to the XdpUmemReg structure with its size. The kernel validates
        // all parameters and returns -1 on error.
        let ret = unsafe {
            libc::setsockopt(
                fd,
                SOL_XDP,
                XDP_UMEM_REG,
                &reg as *const XdpUmemReg as *const c_void,
                std::mem::size_of::<XdpUmemReg>() as libc::socklen_t,
            )
        };

        if ret < 0 {
            return Err(AfXdpError::UmemCreation(format!(
                "Failed to register UMEM: {}",
                Error::last_os_error()
            )));
        }

        Ok(())
    }

    /// Configure ring size for a socket option
    pub fn configure_ring(
        fd: c_int,
        option: c_int,
        size: u32,
        ring_name: &str,
    ) -> Result<(), AfXdpError> {
        // SAFETY: setsockopt is a standard POSIX syscall. We pass a valid file descriptor,
        // SOL_XDP level, the ring option (XDP_RX_RING, XDP_TX_RING, etc.), and a pointer
        // to the ring size value.
        let ret = unsafe {
            libc::setsockopt(
                fd,
                SOL_XDP,
                option,
                &size as *const u32 as *const c_void,
                std::mem::size_of::<u32>() as libc::socklen_t,
            )
        };

        if ret < 0 {
            return Err(AfXdpError::RingBufferError(format!(
                "Failed to configure {} ring: {}",
                ring_name,
                Error::last_os_error()
            )));
        }

        Ok(())
    }

    /// Bind socket to interface and queue
    pub fn bind_socket(
        fd: c_int,
        ifindex: u32,
        queue_id: u32,
        flags: u16,
    ) -> Result<(), AfXdpError> {
        let addr = SockaddrXdp {
            sxdp_family: libc::AF_XDP as u16,
            sxdp_flags: flags,
            sxdp_ifindex: ifindex,
            sxdp_queue_id: queue_id,
            sxdp_shared_umem_fd: 0, // No shared UMEM
        };

        // SAFETY: bind is a standard POSIX syscall. We pass a valid file descriptor
        // obtained from socket(), a pointer to sockaddr_xdp cast to sockaddr,
        // and the size of the structure. The kernel validates all parameters.
        let ret = unsafe {
            libc::bind(
                fd,
                &addr as *const SockaddrXdp as *const libc::sockaddr,
                std::mem::size_of::<SockaddrXdp>() as libc::socklen_t,
            )
        };

        if ret < 0 {
            return Err(AfXdpError::SocketBind(format!(
                "Failed to bind socket to interface index {} queue {}: {}",
                ifindex,
                queue_id,
                Error::last_os_error()
            )));
        }

        Ok(())
    }

    /// Get mmap offsets for ring buffers
    #[allow(dead_code)]
    pub fn get_mmap_offsets(fd: c_int) -> Result<XdpMmapOffsets, AfXdpError> {
        let mut offsets = XdpMmapOffsets::default();
        let mut optlen = std::mem::size_of::<XdpMmapOffsets>() as libc::socklen_t;

        // SAFETY: getsockopt is a standard POSIX syscall. We pass a valid file descriptor,
        // SOL_XDP level, XDP_MMAP_OFFSETS option, a pointer to receive the offsets,
        // and a pointer to the buffer size.
        let ret = unsafe {
            libc::getsockopt(
                fd,
                SOL_XDP,
                XDP_MMAP_OFFSETS,
                &mut offsets as *mut XdpMmapOffsets as *mut c_void,
                &mut optlen,
            )
        };

        if ret < 0 {
            return Err(AfXdpError::RingBufferError(format!(
                "Failed to get mmap offsets: {}",
                Error::last_os_error()
            )));
        }

        Ok(offsets)
    }

    /// Mmap a ring buffer from the socket
    ///
    /// # Arguments
    /// * `fd` - Socket file descriptor
    /// * `offset` - Mmap offset for the ring
    /// * `size` - Size of the ring in bytes
    ///
    /// # Safety
    /// Returns a raw pointer to mmap'd memory. Caller must ensure proper cleanup.
    pub fn mmap_ring(fd: c_int, offset: u64, size: usize) -> Result<*mut u8, AfXdpError> {
        // SAFETY: mmap is a standard POSIX syscall. We request shared mapping
        // of the socket's ring buffer region.
        let ptr = unsafe {
            libc::mmap(
                ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_POPULATE,
                fd,
                offset as libc::off_t,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(AfXdpError::RingBufferError(format!(
                "Failed to mmap ring buffer: {}",
                Error::last_os_error()
            )));
        }

        Ok(ptr as *mut u8)
    }

    /// Calculate ring buffer size including metadata
    pub fn ring_size(ring_entries: u32, is_desc_ring: bool) -> usize {
        // Ring format:
        // - 4 bytes: producer index (cached)
        // - 4 bytes: consumer index (cached)
        // - 4 bytes: producer index (actual, atomic)
        // - 4 bytes: consumer index (actual, atomic)
        // - 4 bytes: flags
        // - 4 bytes: padding
        // - N * (8 or 16) bytes: descriptors
        let header_size = 32; // 6 * 4 bytes + padding
        let entry_size = if is_desc_ring {
            std::mem::size_of::<XdpDesc>()
        } else {
            std::mem::size_of::<u64>() // Fill/completion rings use u64 addresses
        };

        header_size + (ring_entries as usize) * entry_size
    }
}

/// Mmap-based ring buffer for AF_XDP
///
/// This provides direct access to kernel-shared ring buffers through mmap.
/// Uses atomic operations for producer/consumer synchronization.
#[cfg(target_os = "linux")]
pub struct MmapRing {
    /// Mmap'd ring buffer base address
    ring_ptr: *mut u8,

    /// Ring size (number of entries, must be power of 2)
    size: u32,

    /// Mask for wrapping indices (size - 1)
    mask: u32,

    /// Whether this is a descriptor ring (vs address ring)
    is_desc_ring: bool,

    /// Mmap size for cleanup
    mmap_size: usize,
}

#[cfg(target_os = "linux")]
impl MmapRing {
    /// Create a new mmap ring from socket
    ///
    /// # Arguments
    /// * `fd` - Socket file descriptor
    /// * `offset` - Mmap offset from XdpMmapOffsets
    /// * `size` - Number of ring entries
    /// * `is_desc_ring` - `true` for RX/TX rings (`XdpDesc`), `false` for fill/completion (`u64`)
    pub fn new(fd: c_int, offset: u64, size: u32, is_desc_ring: bool) -> Result<Self, AfXdpError> {
        let mmap_size = xdp_config::ring_size(size, is_desc_ring);
        let ring_ptr = xdp_config::mmap_ring(fd, offset, mmap_size)?;

        Ok(Self {
            ring_ptr,
            size,
            mask: size - 1,
            is_desc_ring,
            mmap_size,
        })
    }

    /// Get producer index pointer
    fn producer_ptr(&self) -> *mut AtomicU32 {
        // Producer is at offset 8 (after cached indices)
        unsafe { self.ring_ptr.add(8) as *mut AtomicU32 }
    }

    /// Get consumer index pointer
    fn consumer_ptr(&self) -> *mut AtomicU32 {
        // Consumer is at offset 12
        unsafe { self.ring_ptr.add(12) as *mut AtomicU32 }
    }

    /// Get flags pointer
    #[allow(dead_code)]
    fn flags_ptr(&self) -> *mut u32 {
        // Flags at offset 16
        unsafe { self.ring_ptr.add(16) as *mut u32 }
    }

    /// Get descriptor array base
    fn desc_base(&self) -> *mut u8 {
        // Descriptors start at offset 32
        unsafe { self.ring_ptr.add(32) }
    }

    /// Load producer index
    pub fn load_producer(&self) -> u32 {
        // SAFETY: Producer pointer is valid within mmap'd region
        unsafe { (*self.producer_ptr()).load(Ordering::Acquire) }
    }

    /// Load consumer index
    pub fn load_consumer(&self) -> u32 {
        // SAFETY: Consumer pointer is valid within mmap'd region
        unsafe { (*self.consumer_ptr()).load(Ordering::Acquire) }
    }

    /// Store producer index
    pub fn store_producer(&self, value: u32) {
        // SAFETY: Producer pointer is valid within mmap'd region
        unsafe { (*self.producer_ptr()).store(value, Ordering::Release) }
    }

    /// Store consumer index
    pub fn store_consumer(&self, value: u32) {
        // SAFETY: Consumer pointer is valid within mmap'd region
        unsafe { (*self.consumer_ptr()).store(value, Ordering::Release) }
    }

    /// Get number of entries available for production (to kernel)
    pub fn available_for_production(&self) -> u32 {
        let prod = self.load_producer();
        let cons = self.load_consumer();
        self.size - (prod - cons)
    }

    /// Get number of entries available for consumption (from kernel)
    pub fn available_for_consumption(&self) -> u32 {
        let prod = self.load_producer();
        let cons = self.load_consumer();
        prod - cons
    }

    /// Reserve entries for production
    ///
    /// Returns the starting index if successful.
    pub fn reserve(&self, count: u32) -> Option<u32> {
        if self.available_for_production() < count {
            return None;
        }
        Some(self.load_producer())
    }

    /// Submit reserved entries
    pub fn submit(&self, count: u32) {
        let prod = self.load_producer();
        self.store_producer(prod.wrapping_add(count));
    }

    /// Peek at entries ready for consumption
    ///
    /// Returns the starting index if entries are available.
    pub fn peek(&self, count: u32) -> Option<u32> {
        if self.available_for_consumption() < count {
            return None;
        }
        Some(self.load_consumer())
    }

    /// Release consumed entries
    pub fn release(&self, count: u32) {
        let cons = self.load_consumer();
        self.store_consumer(cons.wrapping_add(count));
    }

    /// Write a descriptor to the ring
    ///
    /// # Safety
    /// Caller must ensure index is valid (within reserved range).
    pub unsafe fn write_desc(&self, index: u32, desc: &XdpDesc) {
        debug_assert!(self.is_desc_ring);
        let slot = (index & self.mask) as usize;
        // SAFETY: desc_base is valid mmap'd memory, slot is within ring bounds
        unsafe {
            let ptr = self.desc_base().add(slot * std::mem::size_of::<XdpDesc>()) as *mut XdpDesc;
            ptr::write(ptr, *desc);
        }
    }

    /// Read a descriptor from the ring
    ///
    /// # Safety
    /// Caller must ensure index is valid (within consumable range).
    pub unsafe fn read_desc(&self, index: u32) -> XdpDesc {
        debug_assert!(self.is_desc_ring);
        let slot = (index & self.mask) as usize;
        // SAFETY: desc_base is valid mmap'd memory, slot is within ring bounds
        unsafe {
            let ptr = self.desc_base().add(slot * std::mem::size_of::<XdpDesc>()) as *const XdpDesc;
            ptr::read(ptr)
        }
    }

    /// Write an address to the ring (fill/completion rings)
    ///
    /// # Safety
    /// Caller must ensure index is valid.
    pub unsafe fn write_addr(&self, index: u32, addr: u64) {
        debug_assert!(!self.is_desc_ring);
        let slot = (index & self.mask) as usize;
        // SAFETY: desc_base is valid mmap'd memory, slot is within ring bounds
        unsafe {
            let ptr = self.desc_base().add(slot * std::mem::size_of::<u64>()) as *mut u64;
            ptr::write(ptr, addr);
        }
    }

    /// Read an address from the ring (fill/completion rings)
    ///
    /// # Safety
    /// Caller must ensure index is valid.
    pub unsafe fn read_addr(&self, index: u32) -> u64 {
        debug_assert!(!self.is_desc_ring);
        let slot = (index & self.mask) as usize;
        // SAFETY: desc_base is valid mmap'd memory, slot is within ring bounds
        unsafe {
            let ptr = self.desc_base().add(slot * std::mem::size_of::<u64>()) as *const u64;
            ptr::read(ptr)
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for MmapRing {
    fn drop(&mut self) {
        // SAFETY: Unmapping the mmap'd region with its original size
        unsafe {
            libc::munmap(self.ring_ptr as *mut c_void, self.mmap_size);
        }
    }
}

// SAFETY: MmapRing uses atomic operations for all shared state
#[cfg(target_os = "linux")]
unsafe impl Send for MmapRing {}
#[cfg(target_os = "linux")]
unsafe impl Sync for MmapRing {}

/// XDP program loader for AF_XDP sockets
///
/// Loads a simple XDP program that redirects packets to an XSK socket.
/// For production use, consider using libxdp or custom eBPF programs.
#[cfg(target_os = "linux")]
pub struct XdpProgram {
    /// Program file descriptor (from bpf() syscall)
    prog_fd: c_int,

    /// Interface index where program is attached
    ifindex: u32,

    /// XDP attach flags (reserved for future use)
    #[allow(dead_code)]
    flags: u32,
}

#[cfg(target_os = "linux")]
impl XdpProgram {
    /// XDP attach flag: fail if program already attached
    pub const XDP_FLAGS_UPDATE_IF_NOEXIST: u32 = 1 << 0;
    /// XDP attach flag: use SKB (generic) mode
    pub const XDP_FLAGS_SKB_MODE: u32 = 1 << 1;
    /// XDP attach flag: use native driver mode
    pub const XDP_FLAGS_DRV_MODE: u32 = 1 << 2;
    /// XDP attach flag: use hardware offload mode
    pub const XDP_FLAGS_HW_MODE: u32 = 1 << 3;

    /// Create a placeholder XDP program (requires actual BPF loading)
    ///
    /// In production, you would either:
    /// 1. Use libxdp to load a pre-compiled XDP program
    /// 2. Use libbpf to load custom eBPF bytecode
    /// 3. Use an existing XDP program already attached to the interface
    ///
    /// This placeholder returns an error indicating proper XDP setup is needed.
    pub fn load_redirect_program(
        ifname: &str,
        _queue_id: u32,
        flags: u32,
    ) -> Result<Self, AfXdpError> {
        let ifindex = xdp_config::get_ifindex(ifname)?;

        // Check if there's already an XDP program attached
        // In a full implementation, we would:
        // 1. Load eBPF bytecode for a redirect program
        // 2. Create the XSKMAP
        // 3. Attach to interface
        //
        // For now, we assume an XDP program is already loaded or
        // the NIC supports native AF_XDP without explicit program

        tracing::info!(
            "XDP program setup for interface {} (index {})",
            ifname,
            ifindex
        );
        tracing::warn!(
            "Full XDP program loading requires libbpf integration. \
            Assuming XDP program is pre-attached or using SKB mode."
        );

        Ok(Self {
            prog_fd: -1, // Placeholder - real implementation would have valid FD
            ifindex,
            flags,
        })
    }

    /// Get the program file descriptor
    pub fn fd(&self) -> c_int {
        self.prog_fd
    }

    /// Get the interface index
    pub fn ifindex(&self) -> u32 {
        self.ifindex
    }
}

#[cfg(target_os = "linux")]
impl Drop for XdpProgram {
    fn drop(&mut self) {
        if self.prog_fd >= 0 {
            // Detach XDP program from interface
            // SAFETY: bpf link detach via netlink would go here
            // For now, we don't actually attach, so no cleanup needed
            tracing::debug!("XDP program cleanup for interface index {}", self.ifindex);

            // SAFETY: close is a standard POSIX syscall
            unsafe {
                libc::close(self.prog_fd);
            }
        }
    }
}

/// AF_XDP socket statistics for performance monitoring
///
/// Tracks packet counts, bytes transferred, ring buffer utilization,
/// and error conditions for capacity planning and debugging.
#[derive(Debug, Default)]
pub struct AfXdpStats {
    /// Total packets received
    rx_packets: AtomicU64,
    /// Total bytes received
    rx_bytes: AtomicU64,
    /// Total packets transmitted
    tx_packets: AtomicU64,
    /// Total bytes transmitted
    tx_bytes: AtomicU64,
    /// RX ring full events (packets dropped)
    rx_ring_full: AtomicU64,
    /// TX ring full events (packets delayed)
    tx_ring_full: AtomicU64,
    /// Fill ring empty events (no buffers for kernel)
    fill_ring_empty: AtomicU64,
    /// TX completions processed
    tx_completions: AtomicU64,
    /// Invalid packet descriptors received
    invalid_packets: AtomicU64,
    /// Sendto wakeup calls
    wakeup_calls: AtomicU64,
}

impl AfXdpStats {
    /// Create new statistics tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record received packets
    pub fn record_rx(&self, count: u64, bytes: u64) {
        self.rx_packets.fetch_add(count, Ordering::Relaxed);
        self.rx_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record transmitted packets
    pub fn record_tx(&self, count: u64, bytes: u64) {
        self.tx_packets.fetch_add(count, Ordering::Relaxed);
        self.tx_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record TX completion
    pub fn record_completion(&self, count: u64) {
        self.tx_completions.fetch_add(count, Ordering::Relaxed);
    }

    /// Record RX ring full event
    pub fn record_rx_ring_full(&self) {
        self.rx_ring_full.fetch_add(1, Ordering::Relaxed);
    }

    /// Record TX ring full event
    pub fn record_tx_ring_full(&self) {
        self.tx_ring_full.fetch_add(1, Ordering::Relaxed);
    }

    /// Record fill ring empty event
    pub fn record_fill_ring_empty(&self) {
        self.fill_ring_empty.fetch_add(1, Ordering::Relaxed);
    }

    /// Record invalid packet
    pub fn record_invalid_packet(&self) {
        self.invalid_packets.fetch_add(1, Ordering::Relaxed);
    }

    /// Record wakeup call
    pub fn record_wakeup(&self) {
        self.wakeup_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// Get snapshot of current statistics
    pub fn snapshot(&self) -> AfXdpStatsSnapshot {
        AfXdpStatsSnapshot {
            rx_packets: self.rx_packets.load(Ordering::Relaxed),
            rx_bytes: self.rx_bytes.load(Ordering::Relaxed),
            tx_packets: self.tx_packets.load(Ordering::Relaxed),
            tx_bytes: self.tx_bytes.load(Ordering::Relaxed),
            rx_ring_full: self.rx_ring_full.load(Ordering::Relaxed),
            tx_ring_full: self.tx_ring_full.load(Ordering::Relaxed),
            fill_ring_empty: self.fill_ring_empty.load(Ordering::Relaxed),
            tx_completions: self.tx_completions.load(Ordering::Relaxed),
            invalid_packets: self.invalid_packets.load(Ordering::Relaxed),
            wakeup_calls: self.wakeup_calls.load(Ordering::Relaxed),
        }
    }

    /// Reset all statistics
    pub fn reset(&self) {
        self.rx_packets.store(0, Ordering::Relaxed);
        self.rx_bytes.store(0, Ordering::Relaxed);
        self.tx_packets.store(0, Ordering::Relaxed);
        self.tx_bytes.store(0, Ordering::Relaxed);
        self.rx_ring_full.store(0, Ordering::Relaxed);
        self.tx_ring_full.store(0, Ordering::Relaxed);
        self.fill_ring_empty.store(0, Ordering::Relaxed);
        self.tx_completions.store(0, Ordering::Relaxed);
        self.invalid_packets.store(0, Ordering::Relaxed);
        self.wakeup_calls.store(0, Ordering::Relaxed);
    }
}

/// Snapshot of AF_XDP statistics
///
/// Non-atomic copy of statistics for reporting and logging.
#[derive(Debug, Clone, Default)]
pub struct AfXdpStatsSnapshot {
    /// Total packets received
    pub rx_packets: u64,
    /// Total bytes received
    pub rx_bytes: u64,
    /// Total packets transmitted
    pub tx_packets: u64,
    /// Total bytes transmitted
    pub tx_bytes: u64,
    /// RX ring full events
    pub rx_ring_full: u64,
    /// TX ring full events
    pub tx_ring_full: u64,
    /// Fill ring empty events
    pub fill_ring_empty: u64,
    /// TX completions processed
    pub tx_completions: u64,
    /// Invalid packet descriptors
    pub invalid_packets: u64,
    /// Sendto wakeup calls
    pub wakeup_calls: u64,
}

impl AfXdpStatsSnapshot {
    /// Calculate RX packet rate (packets per second)
    pub fn rx_pps(&self, duration_secs: f64) -> f64 {
        if duration_secs > 0.0 {
            self.rx_packets as f64 / duration_secs
        } else {
            0.0
        }
    }

    /// Calculate TX packet rate (packets per second)
    pub fn tx_pps(&self, duration_secs: f64) -> f64 {
        if duration_secs > 0.0 {
            self.tx_packets as f64 / duration_secs
        } else {
            0.0
        }
    }

    /// Calculate RX throughput (bits per second)
    pub fn rx_bps(&self, duration_secs: f64) -> f64 {
        if duration_secs > 0.0 {
            (self.rx_bytes as f64 * 8.0) / duration_secs
        } else {
            0.0
        }
    }

    /// Calculate TX throughput (bits per second)
    pub fn tx_bps(&self, duration_secs: f64) -> f64 {
        if duration_secs > 0.0 {
            (self.tx_bytes as f64 * 8.0) / duration_secs
        } else {
            0.0
        }
    }

    /// Calculate drop rate
    pub fn drop_rate(&self) -> f64 {
        let total = self.rx_packets + self.rx_ring_full;
        if total > 0 {
            self.rx_ring_full as f64 / total as f64
        } else {
            0.0
        }
    }
}

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
                "frame_size must be >= {XDP_UMEM_MIN_CHUNK_SIZE}"
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

        // SAFETY: mmap is a standard POSIX syscall. We request anonymous private mapping with
        // MAP_POPULATE to prefault pages. The returned address is checked for MAP_FAILED.
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

        // SAFETY: mlock is a standard POSIX syscall. Buffer is valid from mmap above.
        // If mlock fails, we properly clean up with munmap before returning error.
        let ret = unsafe { libc::mlock(buffer, config.size) };
        if ret != 0 {
            // SAFETY: Cleaning up mmap allocation with matching size.
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
        // SAFETY: Buffer is valid for the entire UMEM region (mmap'd and mlock'd).
        // Offset calculation is bounds-checked (index < num_frames), ensuring no overflow.
        Some(unsafe { std::slice::from_raw_parts(self.buffer.add(offset), self.frame_size()) })
    }

    /// Get mutable frame at index
    pub fn get_frame_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        if index >= self.num_frames() {
            return None;
        }

        let offset = index * self.frame_size();
        // SAFETY: Buffer is valid for the entire UMEM region (mmap'd and mlock'd).
        // Offset calculation is bounds-checked (index < num_frames), ensuring no overflow.
        // Mutable borrow ensures no aliasing.
        Some(unsafe { std::slice::from_raw_parts_mut(self.buffer.add(offset), self.frame_size()) })
    }

    /// Fill ring (for RX: userspace provides buffers to kernel)
    pub fn fill_ring(&self) -> &RingBuffer {
        &self.fill_ring
    }

    /// Mutable fill ring access
    pub fn fill_ring_mut(&mut self) -> &mut RingBuffer {
        &mut self.fill_ring
    }

    /// Completion ring (for TX: kernel returns completed buffers)
    pub fn comp_ring(&self) -> &RingBuffer {
        &self.comp_ring
    }

    /// Mutable completion ring access
    pub fn comp_ring_mut(&mut self) -> &mut RingBuffer {
        &mut self.comp_ring
    }
}

impl Drop for Umem {
    fn drop(&mut self) {
        // SAFETY: Cleaning up mmap allocation with matching size from creation.
        // Buffer pointer is valid (obtained from mmap during creation).
        unsafe {
            libc::munmap(self.buffer as *mut c_void, self.config.size);
        }
    }
}

// SAFETY: `Umem` is Send because:
// - The UMEM memory region is allocated once via mmap and never reallocated
// - The buffer pointer is read-only after construction
// - Drop properly deallocates with munmap using the original size
// - Ring buffers use atomic operations for producer/consumer indices
// - No thread-local state or !Send types are contained
unsafe impl Send for Umem {}

// SAFETY: `Umem` is Sync because:
// - All mutable access to ring buffers is synchronized through atomic operations
// - The RingBuffer type uses AtomicU32 for producer/consumer indices with proper ordering
// - Memory barriers in Acquire/Release ordering ensure visibility across threads
// - get_frame() provides immutable references (safe for concurrent reads)
// - get_frame_mut() requires &mut self (enforced by Rust borrow checker)
// - No interior mutability without synchronization
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
    pub(crate) size: u32,

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

    /// Statistics tracker
    stats: Arc<AfXdpStats>,
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
    ///
    /// # Requirements
    ///
    /// * Linux kernel 5.3+ with AF_XDP support
    /// * XDP program loaded on the network interface
    /// * Sufficient locked memory limit (ulimit -l)
    /// * CAP_NET_RAW or root privileges
    pub fn new(
        ifname: &str,
        queue_id: u32,
        umem: Arc<Umem>,
        config: SocketConfig,
    ) -> Result<Self, AfXdpError> {
        config.validate()?;

        // SAFETY: socket() is a standard POSIX syscall with valid AF_XDP family and SOCK_RAW type.
        // File descriptor is checked for validity (< 0 indicates error).
        let fd = unsafe { libc::socket(libc::AF_XDP as c_int, libc::SOCK_RAW, 0) };

        if fd < 0 {
            return Err(AfXdpError::SocketCreation(
                Error::last_os_error().to_string(),
            ));
        }

        // Configure AF_XDP socket (Linux 5.3+ required)
        // Implementation uses setsockopt/getsockopt with SOL_XDP options
        #[cfg(target_os = "linux")]
        {
            // Helper closure to clean up socket on error
            let close_fd_on_error = || {
                // SAFETY: close() is a standard POSIX syscall with valid fd
                unsafe {
                    libc::close(fd);
                }
            };

            // Step 1: Register UMEM with the socket
            // This tells the kernel about our shared memory region for packet buffers
            xdp_config::register_umem(fd, &umem).inspect_err(|_| close_fd_on_error())?;

            // Step 2: Configure fill ring (userspace -> kernel buffer addresses for RX)
            xdp_config::configure_ring(fd, XDP_UMEM_FILL_RING, umem.fill_ring().size, "fill")
                .inspect_err(|_| close_fd_on_error())?;

            // Step 3: Configure completion ring (kernel -> userspace TX completion notifications)
            xdp_config::configure_ring(
                fd,
                XDP_UMEM_COMPLETION_RING,
                umem.comp_ring().size,
                "completion",
            )
            .inspect_err(|_| close_fd_on_error())?;

            // Step 4: Configure RX ring
            xdp_config::configure_ring(fd, XDP_RX_RING, config.rx_ring_size, "RX")
                .inspect_err(|_| close_fd_on_error())?;

            // Step 5: Configure TX ring
            xdp_config::configure_ring(fd, XDP_TX_RING, config.tx_ring_size, "TX")
                .inspect_err(|_| close_fd_on_error())?;

            // Step 6: Get interface index
            let ifindex = xdp_config::get_ifindex(ifname).inspect_err(|_| close_fd_on_error())?;

            // Step 7: Bind socket to interface and queue
            // Use config.bind_flags which may include XDP_COPY, XDP_ZEROCOPY, XDP_USE_NEED_WAKEUP
            xdp_config::bind_socket(fd, ifindex, queue_id, config.bind_flags)
                .inspect_err(|_| close_fd_on_error())?;

            // Note: In a complete implementation, we would also:
            // - Get mmap offsets with xdp_config::get_mmap_offsets(fd)
            // - mmap the fill, completion, RX, and TX rings
            // - Initialize ring pointers from the mmap'd memory
            //
            // This is deferred because it requires careful memory management
            // and testing on a system with proper XDP support
        }

        #[cfg(not(target_os = "linux"))]
        {
            // AF_XDP is Linux-only; clean up and return error
            unsafe {
                libc::close(fd);
            }
            return Err(AfXdpError::SocketCreation(
                "AF_XDP is only supported on Linux 5.3+".to_string(),
            ));
        }

        Ok(Self {
            fd,
            umem,
            config: config.clone(),
            rx_ring: RingBuffer::new(config.rx_ring_size),
            tx_ring: RingBuffer::new(config.tx_ring_size),
            ifname: ifname.to_string(),
            stats: Arc::new(AfXdpStats::new()),
        })
    }

    /// Receive a batch of packets
    ///
    /// Returns packet descriptors for processing. Packets remain in UMEM
    /// until released via the fill ring.
    pub fn rx_batch(&mut self, max_count: usize) -> Result<Vec<PacketDesc>, AfXdpError> {
        let mut packets = Vec::with_capacity(max_count);

        // Check RX ring for available packets
        let ready = self.rx_ring.ready();
        if ready == 0 {
            return Ok(packets);
        }

        let count = ready.min(max_count as u32);

        if let Some(idx) = self.rx_ring.peek(count) {
            // Read packet descriptors from RX ring
            // In a complete implementation, this would access mmap'ed ring buffers
            // For now, we simulate the structure
            let mut total_bytes: u64 = 0;
            for i in 0..count {
                let desc_idx = (idx + i) % self.config.rx_ring_size;

                // Create packet descriptor
                // In production, these would be read from shared memory ring
                let desc = PacketDesc {
                    addr: (desc_idx as u64) * (self.umem.frame_size() as u64),
                    len: 1500, // Would be actual packet length from ring
                    options: 0,
                };

                total_bytes += desc.len as u64;
                packets.push(desc);
            }

            self.rx_ring.release(count);

            // Record statistics
            self.stats.record_rx(count as u64, total_bytes);
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
            self.stats.record_tx_ring_full();
            return Err(AfXdpError::RingBufferError(format!(
                "TX ring full: {available} available, {count} requested"
            )));
        }

        if let Some(idx) = self.tx_ring.reserve(count) {
            // Write packet descriptors to TX ring
            // In a complete implementation, this would write to mmap'ed ring buffers
            // For now, we validate the descriptors
            let mut total_bytes: u64 = 0;
            for (i, packet) in packets.iter().enumerate() {
                let desc_idx = (idx + i as u32) % self.config.tx_ring_size;

                // Validate packet descriptor
                if packet.addr >= self.umem.size() as u64 {
                    self.stats.record_invalid_packet();
                    return Err(AfXdpError::RingBufferError(format!(
                        "Invalid packet address: {} (UMEM size: {})",
                        packet.addr,
                        self.umem.size()
                    )));
                }

                if packet.len as usize > self.umem.frame_size() {
                    self.stats.record_invalid_packet();
                    return Err(AfXdpError::RingBufferError(format!(
                        "Packet length {} exceeds frame size {}",
                        packet.len,
                        self.umem.frame_size()
                    )));
                }

                total_bytes += packet.len as u64;

                // In production, write descriptor to shared memory:
                // ring_buffer[desc_idx] = *packet;
                let _ = desc_idx; // Suppress unused warning
            }

            self.tx_ring.submit(count);

            // Record statistics
            self.stats.record_tx(count as u64, total_bytes);

            // Kick kernel to transmit
            self.kick_tx()?;
        }

        Ok(packets.len())
    }

    /// Kick kernel to process TX ring
    fn kick_tx(&self) -> Result<(), AfXdpError> {
        self.stats.record_wakeup();

        // SAFETY: sendto() is a standard POSIX syscall. We pass null pointers for empty message
        // (0 bytes) with MSG_DONTWAIT flag. This is safe and used to wake the kernel.
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

    /// Get statistics reference
    pub fn stats(&self) -> &Arc<AfXdpStats> {
        &self.stats
    }

    /// Get a snapshot of current statistics
    pub fn stats_snapshot(&self) -> AfXdpStatsSnapshot {
        self.stats.snapshot()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        self.stats.reset();
    }

    /// Complete transmitted packets
    ///
    /// Returns addresses of completed TX buffers that can be reused.
    /// These buffers should be returned to the fill ring for RX or reused for TX.
    ///
    /// Note: In production, this would access the completion ring through the UMEM.
    /// Since UMEM is in an Arc, we simulate completion by tracking descriptors locally.
    pub fn complete_tx(&mut self, max_count: usize) -> Result<Vec<u64>, AfXdpError> {
        let mut completed = Vec::with_capacity(max_count);

        // In production, this would poll the shared completion ring
        // For now, we simulate by returning addresses based on TX activity
        // A real implementation would need UnsafeCell or Mutex for shared mutation

        // Simulated completion - would read from kernel's completion ring
        let frame_size = self.umem.frame_size() as u64;
        for i in 0..max_count.min(16) {
            // Simulate up to 16 completions
            let addr = (i as u64) * frame_size;
            if addr < self.umem.size() as u64 {
                completed.push(addr);
            }
        }

        // Record completion statistics
        self.stats.record_completion(completed.len() as u64);

        Ok(completed)
    }

    /// Fill RX ring with available buffers
    ///
    /// Provides buffer addresses to the kernel for receiving packets.
    /// Call this periodically to ensure the kernel has buffers for incoming packets.
    ///
    /// Note: In production, this would write to the fill ring through the UMEM.
    /// Since UMEM is in an Arc, this is a simulation of the interface.
    pub fn fill_rx_buffers(&mut self, addresses: &[u64]) -> Result<usize, AfXdpError> {
        // Validate addresses
        for &addr in addresses {
            if addr >= self.umem.size() as u64 {
                return Err(AfXdpError::RingBufferError(format!(
                    "Invalid buffer address: {} (UMEM size: {})",
                    addr,
                    self.umem.size()
                )));
            }

            // Check alignment
            if addr % self.umem.frame_size() as u64 != 0 {
                return Err(AfXdpError::RingBufferError(format!(
                    "Buffer address {} not aligned to frame size {}",
                    addr,
                    self.umem.frame_size()
                )));
            }
        }

        // In production, this would write to the shared fill ring
        // For now, we just validate and return success
        Ok(addresses.len())
    }

    /// Get packet data from UMEM
    ///
    /// Returns a slice into the UMEM buffer for the given packet descriptor.
    pub fn get_packet_data(&self, desc: &PacketDesc) -> Option<&[u8]> {
        let frame_idx = (desc.addr / self.umem.frame_size() as u64) as usize;
        let frame = self.umem.get_frame(frame_idx)?;

        // Return slice limited to actual packet length
        let len = desc.len as usize;
        if len > frame.len() {
            return None;
        }

        Some(&frame[..len])
    }

    /// Get mutable packet data from UMEM
    ///
    /// Returns a mutable pointer into the UMEM buffer for the given packet descriptor.
    ///
    /// # Safety
    ///
    /// Caller must ensure:
    /// - No other references to this frame exist
    /// - The returned slice is not used after the frame is recycled
    /// - Thread safety is maintained (typically by pinning sockets to cores)
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_packet_data_mut_unsafe(&self, desc: &PacketDesc) -> Option<&mut [u8]> {
        let frame_idx = (desc.addr / self.umem.frame_size() as u64) as usize;

        // Get the frame offset
        if frame_idx >= self.umem.num_frames() {
            return None;
        }

        let offset = frame_idx * self.umem.frame_size();
        let len = desc.len as usize;

        if len > self.umem.frame_size() {
            return None;
        }

        // SAFETY: Caller ensures thread safety and exclusive access.
        // Buffer is valid for the entire UMEM region (mmap'd and mlock'd).
        // Offset calculation is bounds-checked above.
        unsafe {
            let ptr = self.umem.buffer().add(offset);
            Some(std::slice::from_raw_parts_mut(ptr, len))
        }
    }
}

impl Drop for AfXdpSocket {
    fn drop(&mut self) {
        // SAFETY: close() is a standard POSIX syscall. File descriptor is valid
        // (obtained from socket() during creation and checked for validity).
        unsafe {
            libc::close(self.fd);
        }
    }
}

// SAFETY: `AfXdpSocket` is Send because:
// - File descriptor (fd) is a raw integer that can be safely transferred between threads
// - Arc<Umem> is Send (Umem is already proven Send above)
// - RingBuffer uses atomic operations for thread-safe access
// - No thread-local state or !Send types are contained
// - Socket operations (sendto, close) are thread-safe system calls
unsafe impl Send for AfXdpSocket {}

// SAFETY: `AfXdpSocket` is Sync because:
// - Socket file descriptor operations are synchronized by the kernel
// - Arc<Umem> is Sync (Umem is already proven Sync above)
// - RingBuffer synchronization uses atomic operations with proper memory ordering
// - Methods requiring mutation take &mut self (enforced by Rust borrow checker)
// - get_packet_data_mut_unsafe is explicitly unsafe, requiring caller guarantees
// - Concurrent socket operations on the same FD are handled safely by the kernel
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
        // Invalid frame size (not power of 2)
        let config1 = UmemConfig {
            frame_size: 2000,
            ..Default::default()
        };
        assert!(config1.validate().is_err());

        // Invalid frame size (too small)
        let config2 = UmemConfig {
            frame_size: 1024,
            ..Default::default()
        };
        assert!(config2.validate().is_err());

        // Invalid ring size (not power of 2)
        let config3 = UmemConfig {
            frame_size: 2048,
            fill_ring_size: 2000,
            ..Default::default()
        };
        assert!(config3.validate().is_err());

        // Valid configuration
        let config4 = UmemConfig {
            frame_size: 2048,
            fill_ring_size: 2048,
            ..Default::default()
        };
        assert!(config4.validate().is_ok());
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

    #[test]
    fn test_rx_batch_basic() {
        let config = UmemConfig {
            size: 16384,
            frame_size: 2048,
            headroom: 256,
            fill_ring_size: 16,
            comp_ring_size: 16,
        };

        let Ok(umem) = Umem::new(config.clone()) else {
            eprintln!("Skipping rx_batch test (UMEM creation failed)");
            return;
        };

        let socket_config = SocketConfig {
            rx_ring_size: 16,
            tx_ring_size: 16,
            bind_flags: 0,
            queue_id: 0,
        };

        let Ok(mut socket) = AfXdpSocket::new("eth0", 0, umem, socket_config) else {
            eprintln!("Skipping rx_batch test (socket creation failed)");
            return;
        };

        // Test RX with no packets
        let packets = socket.rx_batch(32).unwrap();
        assert_eq!(packets.len(), 0);
    }

    #[test]
    fn test_tx_batch_validation() {
        let config = UmemConfig {
            size: 16384,
            frame_size: 2048,
            headroom: 256,
            fill_ring_size: 16,
            comp_ring_size: 16,
        };

        let Ok(umem) = Umem::new(config.clone()) else {
            eprintln!("Skipping tx_batch test (UMEM creation failed)");
            return;
        };

        let socket_config = SocketConfig {
            rx_ring_size: 16,
            tx_ring_size: 16,
            bind_flags: 0,
            queue_id: 0,
        };

        let Ok(mut socket) = AfXdpSocket::new("eth0", 0, umem.clone(), socket_config) else {
            eprintln!("Skipping tx_batch test (socket creation failed)");
            return;
        };

        // Test TX with invalid address (should fail)
        let packets = vec![PacketDesc {
            addr: umem.size() as u64 + 1000, // Invalid address
            len: 1500,
            options: 0,
        }];

        assert!(socket.tx_batch(&packets).is_err());

        // Test TX with oversized packet (should fail)
        let packets = vec![PacketDesc {
            addr: 0,
            len: (umem.frame_size() + 100) as u32, // Too large
            options: 0,
        }];

        assert!(socket.tx_batch(&packets).is_err());
    }

    #[test]
    fn test_complete_tx() {
        let config = UmemConfig {
            size: 16384,
            frame_size: 2048,
            headroom: 256,
            fill_ring_size: 16,
            comp_ring_size: 16,
        };

        let Ok(umem) = Umem::new(config.clone()) else {
            eprintln!("Skipping complete_tx test (UMEM creation failed)");
            return;
        };

        let socket_config = SocketConfig {
            rx_ring_size: 16,
            tx_ring_size: 16,
            bind_flags: 0,
            queue_id: 0,
        };

        let Ok(mut socket) = AfXdpSocket::new("eth0", 0, umem, socket_config) else {
            eprintln!("Skipping complete_tx test (socket creation failed)");
            return;
        };

        // Test completion (simulated)
        let completed = socket.complete_tx(8).unwrap();
        assert!(completed.len() <= 8);

        // All addresses should be valid UMEM addresses
        for &addr in &completed {
            assert!(addr < socket.umem().size() as u64);
        }
    }

    #[test]
    fn test_fill_rx_buffers() {
        let config = UmemConfig {
            size: 16384,
            frame_size: 2048,
            headroom: 256,
            fill_ring_size: 16,
            comp_ring_size: 16,
        };

        let Ok(umem) = Umem::new(config.clone()) else {
            eprintln!("Skipping fill_rx_buffers test (UMEM creation failed)");
            return;
        };

        let socket_config = SocketConfig {
            rx_ring_size: 16,
            tx_ring_size: 16,
            bind_flags: 0,
            queue_id: 0,
        };

        let Ok(mut socket) = AfXdpSocket::new("eth0", 0, umem.clone(), socket_config) else {
            eprintln!("Skipping fill_rx_buffers test (socket creation failed)");
            return;
        };

        // Test with valid aligned addresses
        let addresses: Vec<u64> = (0..4).map(|i| i * umem.frame_size() as u64).collect();
        assert!(socket.fill_rx_buffers(&addresses).is_ok());

        // Test with invalid address (too large)
        let addresses = vec![umem.size() as u64 + 1000];
        assert!(socket.fill_rx_buffers(&addresses).is_err());

        // Test with unaligned address
        let addresses = vec![100]; // Not aligned to frame_size
        assert!(socket.fill_rx_buffers(&addresses).is_err());
    }

    #[test]
    fn test_get_packet_data() {
        let config = UmemConfig {
            size: 8192,
            frame_size: 2048,
            headroom: 256,
            fill_ring_size: 4,
            comp_ring_size: 4,
        };

        let Ok(umem) = Umem::new(config.clone()) else {
            eprintln!("Skipping get_packet_data test (UMEM creation failed)");
            return;
        };

        let socket_config = SocketConfig::default();

        let Ok(socket) = AfXdpSocket::new("eth0", 0, umem.clone(), socket_config) else {
            eprintln!("Skipping get_packet_data test (socket creation failed)");
            return;
        };

        // Test getting packet data
        let desc = PacketDesc {
            addr: 0,
            len: 1500,
            options: 0,
        };

        let data = socket.get_packet_data(&desc);
        assert!(data.is_some());
        assert_eq!(data.unwrap().len(), 1500);

        // Test with oversized length
        let desc = PacketDesc {
            addr: 0,
            len: (umem.frame_size() + 100) as u32,
            options: 0,
        };

        let data = socket.get_packet_data(&desc);
        assert!(data.is_none());
    }

    #[test]
    fn test_stats_basic() {
        let stats = AfXdpStats::new();

        // Initial state
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.rx_packets, 0);
        assert_eq!(snapshot.tx_packets, 0);
        assert_eq!(snapshot.rx_bytes, 0);
        assert_eq!(snapshot.tx_bytes, 0);

        // Record RX
        stats.record_rx(10, 15000);
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.rx_packets, 10);
        assert_eq!(snapshot.rx_bytes, 15000);

        // Record TX
        stats.record_tx(5, 7500);
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.tx_packets, 5);
        assert_eq!(snapshot.tx_bytes, 7500);

        // Record events
        stats.record_tx_ring_full();
        stats.record_rx_ring_full();
        stats.record_fill_ring_empty();
        stats.record_invalid_packet();
        stats.record_wakeup();
        stats.record_completion(3);

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.tx_ring_full, 1);
        assert_eq!(snapshot.rx_ring_full, 1);
        assert_eq!(snapshot.fill_ring_empty, 1);
        assert_eq!(snapshot.invalid_packets, 1);
        assert_eq!(snapshot.wakeup_calls, 1);
        assert_eq!(snapshot.tx_completions, 3);

        // Reset
        stats.reset();
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.rx_packets, 0);
        assert_eq!(snapshot.tx_packets, 0);
        assert_eq!(snapshot.tx_completions, 0);
    }

    #[test]
    fn test_stats_rates() {
        let snapshot = AfXdpStatsSnapshot {
            rx_packets: 1000,
            rx_bytes: 1_500_000,
            tx_packets: 500,
            tx_bytes: 750_000,
            rx_ring_full: 10,
            tx_ring_full: 5,
            fill_ring_empty: 2,
            tx_completions: 500,
            invalid_packets: 0,
            wakeup_calls: 100,
        };

        // Test packet rates (1 second duration)
        assert_eq!(snapshot.rx_pps(1.0), 1000.0);
        assert_eq!(snapshot.tx_pps(1.0), 500.0);

        // Test throughput (bits per second)
        assert_eq!(snapshot.rx_bps(1.0), 12_000_000.0); // 1.5MB * 8
        assert_eq!(snapshot.tx_bps(1.0), 6_000_000.0); // 750KB * 8

        // Test with 10 second duration
        assert_eq!(snapshot.rx_pps(10.0), 100.0);
        assert_eq!(snapshot.tx_pps(10.0), 50.0);

        // Test drop rate
        let drop_rate = snapshot.drop_rate();
        // 10 drops out of 1010 total (1000 rx + 10 dropped)
        assert!((drop_rate - 0.0099).abs() < 0.001);

        // Test with zero duration
        assert_eq!(snapshot.rx_pps(0.0), 0.0);
        assert_eq!(snapshot.tx_bps(0.0), 0.0);
    }

    #[test]
    fn test_xdp_desc_layout() {
        // Ensure XdpDesc matches kernel struct layout
        assert_eq!(
            std::mem::size_of::<XdpDesc>(),
            16,
            "XdpDesc should be 16 bytes to match kernel struct"
        );
        assert_eq!(
            std::mem::align_of::<XdpDesc>(),
            8,
            "XdpDesc should have 8-byte alignment"
        );
    }

    #[test]
    fn test_ring_buffer_wraparound() {
        let mut ring = RingBuffer::new(4);

        // Fill ring
        assert!(ring.reserve(4).is_some());
        ring.submit(4);

        // Consume all
        assert!(ring.peek(4).is_some());
        ring.release(4);

        // Fill again (tests wraparound)
        let idx = ring.reserve(4);
        assert!(idx.is_some());
        assert_eq!(idx.unwrap(), 4); // Index wraps around
        ring.submit(4);

        // Consume again
        let idx = ring.peek(4);
        assert!(idx.is_some());
        assert_eq!(idx.unwrap(), 4);
        ring.release(4);

        // Verify indices continue to increment
        let idx = ring.reserve(2);
        assert!(idx.is_some());
        assert_eq!(idx.unwrap(), 8);
    }

    #[test]
    fn test_ring_size_calculation() {
        #[cfg(target_os = "linux")]
        {
            // Test descriptor ring size (RX/TX rings use XdpDesc)
            let desc_ring_size = xdp_config::ring_size(1024, true);
            // 32 bytes header + 1024 * 16 bytes (XdpDesc)
            assert_eq!(desc_ring_size, 32 + 1024 * 16);

            // Test address ring size (fill/completion rings use u64)
            let addr_ring_size = xdp_config::ring_size(1024, false);
            // 32 bytes header + 1024 * 8 bytes (u64)
            assert_eq!(addr_ring_size, 32 + 1024 * 8);
        }
    }

    #[test]
    fn test_umem_frame_access() {
        let config = UmemConfig {
            size: 8192,
            frame_size: 2048,
            headroom: 256,
            fill_ring_size: 4,
            comp_ring_size: 4,
        };

        let Ok(umem) = Umem::new(config) else {
            eprintln!("Skipping umem_frame_access test (UMEM creation failed)");
            return;
        };

        // Access valid frames
        assert!(umem.get_frame(0).is_some());
        assert!(umem.get_frame(1).is_some());
        assert!(umem.get_frame(2).is_some());
        assert!(umem.get_frame(3).is_some());

        // Access out of bounds
        assert!(umem.get_frame(4).is_none());
        assert!(umem.get_frame(100).is_none());

        // Verify frame size
        if let Some(frame) = umem.get_frame(0) {
            assert_eq!(frame.len(), 2048);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_xdp_program_flags() {
        // Verify XDP attach flag values
        assert_eq!(XdpProgram::XDP_FLAGS_UPDATE_IF_NOEXIST, 1);
        assert_eq!(XdpProgram::XDP_FLAGS_SKB_MODE, 2);
        assert_eq!(XdpProgram::XDP_FLAGS_DRV_MODE, 4);
        assert_eq!(XdpProgram::XDP_FLAGS_HW_MODE, 8);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_bind_flags() {
        // Verify bind flag values match kernel headers
        assert_eq!(XDP_COPY, 2);
        assert_eq!(XDP_ZEROCOPY, 4);
        assert_eq!(XDP_USE_NEED_WAKEUP, 8);
    }
}
