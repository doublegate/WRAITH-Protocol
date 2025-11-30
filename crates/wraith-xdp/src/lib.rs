//! # WRAITH XDP/eBPF Packet Filtering
//!
//! Linux-specific XDP (eXpress Data Path) packet filtering using eBPF programs.
//!
//! This crate provides:
//! - XDP program loading and attachment
//! - BPF map access for statistics and configuration
//! - High-performance packet steering to AF_XDP sockets
//!
//! ## Requirements
//!
//! - Linux kernel 5.3+ with XDP support
//! - libbpf-dev (Ubuntu/Debian) or libbpf-devel (Fedora/RHEL)
//! - clang and LLVM for compiling eBPF programs
//!
//! ## Target Performance
//!
//! - Packet processing: >24M pps (single core)
//! - Latency: <1μs (NIC to AF_XDP socket)
//!
//! ## Feature Flags
//!
//! - `libbpf` - Enable libbpf integration (requires system libbpf library)
//!
//! ## Example
//!
//! ```no_run
//! # #[cfg(all(target_os = "linux", feature = "libbpf"))]
//! # {
//! use wraith_xdp::{XdpProgram, XdpFlags};
//!
//! // Load XDP program from compiled object file
//! let prog = XdpProgram::load("target/xdp/xdp_filter.o").unwrap();
//!
//! // Attach to network interface
//! prog.attach("eth0", XdpFlags::DRV_MODE).unwrap();
//!
//! // Read statistics
//! let stats = prog.read_stats().unwrap();
//! println!("RX packets: {}", stats.rx_packets);
//! # }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![cfg(target_os = "linux")]

use std::ffi::NulError;
use std::fmt;
use thiserror::Error;

#[cfg(feature = "libbpf")]
use std::ffi::CString;

// XDP-specific constants
/// WRAITH port range minimum
pub const WRAITH_PORT_MIN: u16 = 40000;
/// WRAITH port range maximum
pub const WRAITH_PORT_MAX: u16 = 50000;

/// XDP attachment flags
#[derive(Debug, Clone, Copy)]
pub struct XdpFlags(u32);

impl XdpFlags {
    /// Update if no XDP program exists
    pub const UPDATE_IF_NOEXIST: Self = XdpFlags(1 << 0);
    /// Use SKB mode (no driver support needed)
    pub const SKB_MODE: Self = XdpFlags(1 << 1);
    /// Use driver mode (requires driver support)
    pub const DRV_MODE: Self = XdpFlags(1 << 2);
    /// Use hardware offload mode (requires NIC support)
    pub const HW_MODE: Self = XdpFlags(1 << 3);

    /// Get raw flags value
    pub fn bits(self) -> u32 {
        self.0
    }
}

/// XDP program statistics
#[derive(Debug, Default, Clone, Copy)]
pub struct XdpStats {
    /// Total packets received
    pub rx_packets: u64,
    /// Total bytes received
    pub rx_bytes: u64,
    /// Packets dropped
    pub dropped: u64,
    /// Packets redirected to AF_XDP
    pub redirected: u64,
}

impl fmt::Display for XdpStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RX: {} packets ({} bytes), Dropped: {}, Redirected: {}",
            self.rx_packets, self.rx_bytes, self.dropped, self.redirected
        )
    }
}

/// XDP program errors
#[derive(Debug, Error)]
pub enum XdpError {
    /// Failed to load BPF object
    #[error("Failed to load BPF object: {0}")]
    LoadFailed(String),

    /// BPF object or map not found
    #[error("BPF object not found: {0}")]
    NotFound(String),

    /// Invalid network interface
    #[error("Invalid network interface: {0}")]
    InvalidInterface(String),

    /// Failed to attach XDP program
    #[error("Failed to attach XDP program: {0}")]
    AttachFailed(String),

    /// Failed to detach XDP program
    #[error("Failed to detach XDP program: {0}")]
    DetachFailed(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Null byte in string
    #[error("Null byte in string: {0}")]
    Nul(#[from] NulError),

    /// Feature not available
    #[error("XDP/libbpf feature not enabled - recompile with --features libbpf")]
    FeatureNotEnabled,
}

// Conditional compilation based on libbpf feature
#[cfg(feature = "libbpf")]
mod libbpf_impl {
    use super::*;
    use std::os::raw::c_int;
    use std::ptr;

    /// XDP program handle (with libbpf support)
    pub struct XdpProgram {
        obj: *mut libbpf_sys::bpf_object,
        prog: *mut libbpf_sys::bpf_program,
        xsks_map_fd: c_int,
        stats_map_fd: c_int,
    }

    impl XdpProgram {
        /// Load XDP program from compiled ELF object file
        ///
        /// # Arguments
        /// * `path` - Path to the compiled BPF object file (typically .o)
        ///
        /// # Errors
        /// Returns an error if the file cannot be loaded or required maps are missing
        pub fn load(path: &str) -> Result<Self, XdpError> {
            let path_c = CString::new(path)?;

            unsafe {
                // Open BPF object file
                let obj = libbpf_sys::bpf_object__open(path_c.as_ptr());
                if obj.is_null() {
                    return Err(XdpError::LoadFailed("Failed to open BPF object".into()));
                }

                // Load BPF object into kernel
                if libbpf_sys::bpf_object__load(obj) != 0 {
                    libbpf_sys::bpf_object__close(obj);
                    return Err(XdpError::LoadFailed("Failed to load BPF object into kernel".into()));
                }

                // Find XDP program by name
                let prog_name = CString::new("xdp_wraith_filter")?;
                let prog = libbpf_sys::bpf_object__find_program_by_name(obj, prog_name.as_ptr());
                if prog.is_null() {
                    libbpf_sys::bpf_object__close(obj);
                    return Err(XdpError::NotFound("xdp_wraith_filter program not found".into()));
                }

                // Get xsks_map file descriptor
                let xsks_map_name = CString::new("xsks_map")?;
                let xsks_map = libbpf_sys::bpf_object__find_map_by_name(obj, xsks_map_name.as_ptr());
                let xsks_map_fd = if !xsks_map.is_null() {
                    libbpf_sys::bpf_map__fd(xsks_map)
                } else {
                    return Err(XdpError::NotFound("xsks_map not found".into()));
                };

                // Get stats_map file descriptor
                let stats_map_name = CString::new("stats_map")?;
                let stats_map = libbpf_sys::bpf_object__find_map_by_name(obj, stats_map_name.as_ptr());
                let stats_map_fd = if !stats_map.is_null() {
                    libbpf_sys::bpf_map__fd(stats_map)
                } else {
                    return Err(XdpError::NotFound("stats_map not found".into()));
                };

                Ok(Self {
                    obj,
                    prog,
                    xsks_map_fd,
                    stats_map_fd,
                })
            }
        }

        /// Attach XDP program to a network interface
        ///
        /// # Arguments
        /// * `ifname` - Interface name (e.g., "eth0", "ens160")
        /// * `flags` - XDP attachment flags (SKB, DRV, or HW mode)
        ///
        /// # Errors
        /// Returns an error if the interface doesn't exist or attachment fails
        pub fn attach(&self, ifname: &str, flags: XdpFlags) -> Result<(), XdpError> {
            let ifname_c = CString::new(ifname)?;
            let ifindex = unsafe { libc::if_nametoindex(ifname_c.as_ptr()) };

            if ifindex == 0 {
                return Err(XdpError::InvalidInterface(ifname.to_string()));
            }

            unsafe {
                let prog_fd = libbpf_sys::bpf_program__fd(self.prog);
                let ret = libbpf_sys::bpf_set_link_xdp_fd(ifindex as i32, prog_fd, flags.bits());
                if ret != 0 {
                    return Err(XdpError::AttachFailed(format!("errno: {}", -ret)));
                }
            }

            Ok(())
        }

        /// Detach XDP program from a network interface
        ///
        /// # Arguments
        /// * `ifname` - Interface name
        pub fn detach(&self, ifname: &str) -> Result<(), XdpError> {
            let ifname_c = CString::new(ifname)?;
            let ifindex = unsafe { libc::if_nametoindex(ifname_c.as_ptr()) };

            if ifindex == 0 {
                return Err(XdpError::InvalidInterface(ifname.to_string()));
            }

            unsafe {
                let ret = libbpf_sys::bpf_set_link_xdp_fd(ifindex as i32, -1, 0);
                if ret != 0 {
                    return Err(XdpError::DetachFailed(format!("errno: {}", -ret)));
                }
            }

            Ok(())
        }

        /// Get the xsks_map file descriptor
        ///
        /// This map is used to register AF_XDP sockets for packet redirection
        pub fn xsks_map_fd(&self) -> c_int {
            self.xsks_map_fd
        }

        /// Get the stats_map file descriptor
        pub fn stats_map_fd(&self) -> c_int {
            self.stats_map_fd
        }

        /// Read statistics from the XDP program
        ///
        /// Aggregates per-CPU statistics into a single XdpStats structure.
        pub fn read_stats(&self) -> Result<XdpStats, XdpError> {
            let mut stats = XdpStats::default();
            let num_cpus = num_cpus::get() as u32;

            unsafe {
                for stat_type in 0..4u32 {
                    let mut total = 0u64;

                    for cpu in 0..num_cpus {
                        let mut value = 0u64;
                        let ret = libbpf_sys::bpf_map_lookup_elem(
                            self.stats_map_fd,
                            &stat_type as *const u32 as *const _,
                            &mut value as *mut u64 as *mut _,
                        );

                        if ret == 0 {
                            total += value;
                        }
                    }

                    match stat_type {
                        0 => stats.rx_packets = total,
                        1 => stats.rx_bytes = total,
                        2 => stats.dropped = total,
                        3 => stats.redirected = total,
                        _ => {}
                    }
                }
            }

            Ok(stats)
        }
    }

    impl Drop for XdpProgram {
        fn drop(&mut self) {
            unsafe {
                if !self.obj.is_null() {
                    libbpf_sys::bpf_object__close(self.obj);
                }
            }
        }
    }

    unsafe impl Send for XdpProgram {}
    unsafe impl Sync for XdpProgram {}
}

#[cfg(not(feature = "libbpf"))]
mod stub_impl {
    use super::*;

    /// XDP program handle (stub implementation without libbpf)
    ///
    /// This is a stub that returns errors when libbpf feature is disabled.
    /// Enable the `libbpf` feature to use actual XDP functionality.
    #[derive(Debug)]
    pub struct XdpProgram;

    impl XdpProgram {
        /// Load XDP program - stub implementation
        pub fn load(_path: &str) -> Result<Self, XdpError> {
            Err(XdpError::FeatureNotEnabled)
        }

        /// Attach XDP program - stub implementation
        pub fn attach(&self, _ifname: &str, _flags: XdpFlags) -> Result<(), XdpError> {
            Err(XdpError::FeatureNotEnabled)
        }

        /// Detach XDP program - stub implementation
        pub fn detach(&self, _ifname: &str) -> Result<(), XdpError> {
            Err(XdpError::FeatureNotEnabled)
        }

        /// Get xsks_map FD - stub implementation
        pub fn xsks_map_fd(&self) -> i32 {
            -1
        }

        /// Get stats_map FD - stub implementation
        pub fn stats_map_fd(&self) -> i32 {
            -1
        }

        /// Read statistics - stub implementation
        pub fn read_stats(&self) -> Result<XdpStats, XdpError> {
            Err(XdpError::FeatureNotEnabled)
        }
    }
}

// Re-export the appropriate implementation
#[cfg(feature = "libbpf")]
pub use libbpf_impl::*;

#[cfg(not(feature = "libbpf"))]
pub use stub_impl::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xdp_flags() {
        assert_eq!(XdpFlags::SKB_MODE.bits(), 1 << 1);
        assert_eq!(XdpFlags::DRV_MODE.bits(), 1 << 2);
        assert_eq!(XdpFlags::HW_MODE.bits(), 1 << 3);
    }

    #[test]
    fn test_xdp_stats_default() {
        let stats = XdpStats::default();
        assert_eq!(stats.rx_packets, 0);
        assert_eq!(stats.rx_bytes, 0);
        assert_eq!(stats.dropped, 0);
        assert_eq!(stats.redirected, 0);
    }

    #[test]
    fn test_xdp_stats_display() {
        let stats = XdpStats {
            rx_packets: 1000,
            rx_bytes: 1500000,
            dropped: 10,
            redirected: 990,
        };

        let display = format!("{}", stats);
        assert!(display.contains("1000"));
        assert!(display.contains("1500000"));
        assert!(display.contains("10"));
        assert!(display.contains("990"));
    }

    #[test]
    fn test_port_constants() {
        assert_eq!(WRAITH_PORT_MIN, 40000);
        assert_eq!(WRAITH_PORT_MAX, 50000);
    }

    #[test]
    #[cfg(not(feature = "libbpf"))]
    fn test_stub_returns_error() {
        let result = XdpProgram::load("test.o");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), XdpError::FeatureNotEnabled));
    }
}
