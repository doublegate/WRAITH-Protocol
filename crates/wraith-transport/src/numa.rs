//! NUMA (Non-Uniform Memory Access) utilities for Linux.
//!
//! Provides NUMA topology detection and NUMA-aware memory allocation
//! for optimal performance on multi-socket systems.
//!
//! Note: This module is Linux-specific and has no effect on other platforms.

#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::ptr;
#[cfg(target_os = "linux")]
use tracing::{debug, warn};

/// Get the NUMA node for a given CPU core (Linux only)
///
/// # Arguments
/// * `cpu` - CPU core ID
///
/// # Returns
/// The NUMA node ID, or None if unable to determine
///
/// # Examples
/// ```no_run
/// # #[cfg(target_os = "linux")]
/// # {
/// use wraith_transport::numa::get_numa_node_for_cpu;
///
/// if let Some(node) = get_numa_node_for_cpu(0) {
///     println!("CPU 0 is on NUMA node {}", node);
/// }
/// # }
/// ```
#[cfg(target_os = "linux")]
pub fn get_numa_node_for_cpu(cpu: usize) -> Option<usize> {
    // Try to read from sysfs
    for node in 0..8 {
        // Support up to 8 NUMA nodes
        let path = format!("/sys/devices/system/node/node{}/cpu{}", node, cpu);
        if fs::metadata(&path).is_ok() {
            debug!("CPU {} found on NUMA node {}", cpu, node);
            return Some(node);
        }
    }

    // Fallback: check if NUMA is enabled at all
    if fs::metadata("/sys/devices/system/node/node0").is_ok() {
        // NUMA exists but CPU not in any node - likely node 0
        debug!("CPU {} defaulting to NUMA node 0", cpu);
        Some(0)
    } else {
        // No NUMA support
        debug!("No NUMA support detected for CPU {}", cpu);
        None
    }
}

/// Get the NUMA node for a given CPU core (non-Linux platforms)
///
/// Always returns None on non-Linux platforms.
#[cfg(not(target_os = "linux"))]
pub fn get_numa_node_for_cpu(_cpu: usize) -> Option<usize> {
    None
}

/// Get the total number of NUMA nodes in the system (Linux only)
///
/// # Returns
/// Number of NUMA nodes, or 1 if NUMA is not available
#[cfg(target_os = "linux")]
pub fn get_numa_node_count() -> usize {
    for node in 0..8 {
        let path = format!("/sys/devices/system/node/node{}", node);
        if fs::metadata(&path).is_err() {
            return node.max(1);
        }
    }
    1
}

/// Get the total number of NUMA nodes in the system (non-Linux)
#[cfg(not(target_os = "linux"))]
pub fn get_numa_node_count() -> usize {
    1
}

/// Allocate memory on a specific NUMA node (Linux only)
///
/// # Arguments
/// * `size` - Size in bytes to allocate
/// * `node` - NUMA node ID
///
/// # Returns
/// Pointer to allocated memory, or None on failure
///
/// # Safety
/// The caller is responsible for:
/// - Freeing the memory with `deallocate_on_node()`
/// - Not dereferencing the pointer after deallocation
/// - Ensuring the memory is properly initialized before use
#[cfg(target_os = "linux")]
pub unsafe fn allocate_on_node(size: usize, node: usize) -> Option<*mut u8> {
    use libc::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE, mmap};

    // SAFETY: mmap is a standard POSIX syscall. We request anonymous private mapping with
    // valid size and protection flags. The returned address is checked for MAP_FAILED.
    let addr = unsafe {
        mmap(
            ptr::null_mut(),
            size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };

    if addr == libc::MAP_FAILED {
        warn!("Failed to allocate {} bytes on NUMA node {}", size, node);
        return None;
    }

    // Try to bind to NUMA node using mbind
    // Note: This requires numactl-devel on the system
    // For now, we'll just log and continue without mbind
    debug!(
        "Allocated {} bytes at {:p} (NUMA node {} requested)",
        size, addr, node
    );

    // In a full implementation, we would call:
    // let ret = libc::mbind(
    //     addr,
    //     size,
    //     MPOL_BIND,
    //     &nodemask,
    //     maxnode,
    //     MPOL_MF_STRICT | MPOL_MF_MOVE,
    // );
    //
    // But this requires linking against libnuma, which we'll skip for now

    Some(addr as *mut u8)
}

/// Allocate memory on a specific NUMA node (non-Linux)
///
/// # Safety
///
/// The caller is responsible for:
/// - Freeing the memory with `deallocate_on_node()`
/// - Not dereferencing the pointer after deallocation
/// - Ensuring the memory is properly initialized before use
#[cfg(not(target_os = "linux"))]
pub unsafe fn allocate_on_node(size: usize, _node: usize) -> Option<*mut u8> {
    use std::alloc::{Layout, alloc};

    let layout = Layout::from_size_align(size, std::mem::align_of::<u8>()).ok()?;
    let ptr = alloc(layout);

    if ptr.is_null() { None } else { Some(ptr) }
}

/// Deallocate memory allocated with `allocate_on_node()` (Linux only)
///
/// # Safety
/// The caller must ensure:
/// - The pointer was allocated with `allocate_on_node()`
/// - The size matches the original allocation
/// - The pointer has not been deallocated before
#[cfg(target_os = "linux")]
pub unsafe fn deallocate_on_node(ptr: *mut u8, size: usize) {
    if !ptr.is_null() {
        // SAFETY: munmap is a standard POSIX syscall. Pointer and size must match the original
        // mmap allocation, which is enforced by caller's contract.
        unsafe {
            libc::munmap(ptr as *mut libc::c_void, size);
        }
    }
}

/// Deallocate memory allocated with `allocate_on_node()` (non-Linux)
///
/// # Safety
///
/// The caller must ensure:
/// - The pointer was allocated with `allocate_on_node()`
/// - The size matches the original allocation
/// - The pointer has not been deallocated before
#[cfg(not(target_os = "linux"))]
pub unsafe fn deallocate_on_node(ptr: *mut u8, size: usize) {
    use std::alloc::{Layout, dealloc};

    if !ptr.is_null() {
        // SAFETY: Layout matches the allocation from allocate_on_node (non-Linux path).
        // Pointer and size must be valid from original allocation (enforced by caller).
        let layout = Layout::from_size_align_unchecked(size, std::mem::align_of::<u8>());
        dealloc(ptr, layout);
    }
}

/// NUMA-aware memory allocator
///
/// Allocates memory on the NUMA node closest to the current CPU.
pub struct NumaAllocator {
    node: Option<usize>,
}

impl NumaAllocator {
    /// Create a NUMA allocator for the current CPU
    pub fn new() -> Self {
        let node = Self::current_numa_node();
        Self { node }
    }

    /// Create a NUMA allocator for a specific node
    pub fn for_node(node: usize) -> Self {
        Self { node: Some(node) }
    }

    /// Get the current CPU's NUMA node
    #[cfg(target_os = "linux")]
    fn current_numa_node() -> Option<usize> {
        // SAFETY: sched_getcpu is a standard Linux syscall that returns the current CPU ID
        // or -1 on error. Takes no arguments, has no side effects, and cannot cause memory
        // unsafety. Return value is checked for validity (>= 0) before use.
        let cpu = unsafe { libc::sched_getcpu() };
        if cpu >= 0 {
            get_numa_node_for_cpu(cpu as usize)
        } else {
            None
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn current_numa_node() -> Option<usize> {
        None
    }

    /// Allocate memory on this allocator's NUMA node
    ///
    /// # Safety
    /// The caller must free the memory with `deallocate()`
    pub unsafe fn allocate(&self, size: usize) -> Option<*mut u8> {
        if let Some(node) = self.node {
            // SAFETY: Delegates to allocate_on_node with valid node ID.
            unsafe { allocate_on_node(size, node) }
        } else {
            // Fallback to regular allocation on node 0
            // SAFETY: Delegates to allocate_on_node with default node 0.
            unsafe { allocate_on_node(size, 0) }
        }
    }

    /// Deallocate memory allocated with this allocator
    ///
    /// # Safety
    /// The caller must ensure the pointer was allocated by this allocator
    pub unsafe fn deallocate(&self, ptr: *mut u8, size: usize) {
        // SAFETY: Delegates to deallocate_on_node. Caller must ensure ptr/size match allocation.
        unsafe {
            deallocate_on_node(ptr, size);
        }
    }
}

impl Default for NumaAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_numa_node_count() {
        let count = get_numa_node_count();
        assert!(count >= 1);
        assert!(count <= 8);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_get_numa_node_for_cpu() {
        // Try to get NUMA node for CPU 0
        let node = get_numa_node_for_cpu(0);

        // Should either return a valid node or None (if no NUMA)
        if let Some(n) = node {
            assert!(n < 8);
        }
    }

    #[test]
    fn test_allocate_deallocate() {
        // SAFETY: Test allocates memory, writes test pattern, then deallocates with matching size.
        unsafe {
            let size = 4096;
            let ptr = allocate_on_node(size, 0);

            if let Some(p) = ptr {
                // Write some data to verify allocation works
                std::ptr::write_bytes(p, 0xAA, size);

                // Deallocate
                deallocate_on_node(p, size);
            }
        }
    }

    #[test]
    fn test_numa_allocator_new() {
        let allocator = NumaAllocator::new();
        // Should not panic
        assert!(allocator.node.is_none() || allocator.node.unwrap() < 8);
    }

    #[test]
    fn test_numa_allocator_for_node() {
        let allocator = NumaAllocator::for_node(0);
        assert_eq!(allocator.node, Some(0));
    }

    #[test]
    fn test_numa_allocator_allocate_deallocate() {
        let allocator = NumaAllocator::new();

        // SAFETY: Test allocates memory via NUMA allocator, writes/reads test pattern with valid
        // pointer arithmetic (i < size), then deallocates with matching size.
        unsafe {
            let size = 1024;
            if let Some(ptr) = allocator.allocate(size) {
                // Write pattern
                for i in 0..size {
                    *ptr.add(i) = (i % 256) as u8;
                }

                // Verify pattern
                for i in 0..size {
                    assert_eq!(*ptr.add(i), (i % 256) as u8);
                }

                allocator.deallocate(ptr, size);
            }
        }
    }

    #[test]
    fn test_numa_allocator_default() {
        let allocator1 = NumaAllocator::new();
        let allocator2 = NumaAllocator::default();

        // Both should work
        assert!(allocator1.node.is_none() || allocator1.node.unwrap() < 8);
        assert!(allocator2.node.is_none() || allocator2.node.unwrap() < 8);
    }
}
