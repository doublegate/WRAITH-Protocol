# WRAITH-Recon Client - Sprint Planning (Granular)

**Client Name:** WRAITH-Recon
**Tier:** 3 (Advanced)
**Timeline:** 12 weeks (3 sprints x 4 weeks)
**Total Story Points:** 180
**Protocol Alignment:** Synchronized with core protocol development (Phases 1-5)
**Governance:** [Security Testing Parameters](../../ref-docs/WRAITH-Security-Testing-Parameters-v1.0.md)

---

## WRAITH Protocol Stack Dependencies

| Crate | Purpose | Integration Phase |
|-------|---------|-------------------|
| wraith-core | Frame construction, session management | Phase 1 (Weeks 40-43) |
| wraith-crypto | Noise_XX handshake, AEAD encryption, Elligator2 | Phase 2 (Weeks 44-47) |
| wraith-transport | AF_XDP, io_uring, UDP/TCP transports | Phase 1-2 (Weeks 40-47) |
| wraith-obfuscation | Padding, timing jitter, protocol mimicry | Phase 2 (Weeks 44-47) |
| wraith-discovery | DHT integration, relay coordination | Phase 3 (Weeks 48-51) |
| wraith-files | Chunking, BLAKE3 integrity, compression | Phase 3 (Weeks 48-51) |

## Protocol Milestones

- [x] Core frame encoding complete (wraith-core v0.1.0)
- [x] Basic UDP transport functional (wraith-transport v0.1.0)
- [ ] Noise_XX handshake implementation (wraith-crypto v0.2.0)
- [ ] AF_XDP kernel bypass (wraith-transport v0.2.0)
- [ ] Protocol mimicry profiles (wraith-obfuscation v0.1.0)
- [ ] Multi-path exfiltration (wraith-files v0.1.0)

---

## User Stories

### RECON-001: Rules of Engagement Enforcement

**As a** security testing operator,
**I want** cryptographically-signed Rules of Engagement (RoE) enforcement,
**So that** all reconnaissance activities are constrained to authorized scope.

**Story Points:** 21
**Priority:** P0 (Critical)
**Sprint:** Phase 1, Week 40-41

#### Acceptance Criteria

1. RoE loaded from signed JSON with Ed25519 verification
2. Target validation occurs before ANY network activity
3. Kill switch activates within 1ms of HALT signal
4. Audit log captures all scope violations with timestamps
5. System refuses to start without valid RoE signature

#### Implementation

```rust
//! RECON-001: Rules of Engagement Enforcement
//! Location: wraith-recon/src/governance/roe.rs

use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, VerifyingKey, Verifier};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use thiserror::Error;

/// Rules of Engagement configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesOfEngagement {
    /// Unique operation identifier
    pub operation_id: String,

    /// Authorized target networks (CIDR notation)
    pub authorized_targets: Vec<IpNet>,

    /// Explicitly excluded targets (takes precedence)
    pub excluded_targets: Vec<IpNet>,

    /// Authorized ports for scanning
    pub authorized_ports: HashSet<u16>,

    /// Maximum scan rate (packets per second)
    pub max_scan_rate: u32,

    /// Operation valid from timestamp
    pub valid_from: DateTime<Utc>,

    /// Operation expires at timestamp
    pub valid_until: DateTime<Utc>,

    /// Emergency contact information
    pub emergency_contact: String,

    /// Ed25519 signature over serialized RoE
    #[serde(with = "signature_serde")]
    pub signature: Signature,
}

mod signature_serde {
    use ed25519_dalek::Signature;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(sig: &Signature, s: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        hex::encode(sig.to_bytes()).serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Signature, D::Error>
    where D: Deserializer<'de> {
        let hex_str = String::deserialize(d)?;
        let bytes = hex::decode(&hex_str).map_err(serde::de::Error::custom)?;
        let arr: [u8; 64] = bytes.try_into().map_err(|_| {
            serde::de::Error::custom("invalid signature length")
        })?;
        Ok(Signature::from_bytes(&arr))
    }
}

#[derive(Debug, Error)]
pub enum RoeError {
    #[error("RoE signature verification failed")]
    InvalidSignature,

    #[error("RoE has expired at {0}")]
    Expired(DateTime<Utc>),

    #[error("RoE not yet valid until {0}")]
    NotYetValid(DateTime<Utc>),

    #[error("Target {0} is not in authorized scope")]
    TargetOutOfScope(IpAddr),

    #[error("Target {0} is explicitly excluded")]
    TargetExcluded(IpAddr),

    #[error("Port {0} is not authorized")]
    PortNotAuthorized(u16),

    #[error("Kill switch activated")]
    KillSwitchActive,
}

/// Safety controller enforcing RoE constraints
pub struct SafetyController {
    roe: RulesOfEngagement,
    kill_switch: Arc<AtomicBool>,
    verifying_key: VerifyingKey,
    violations: Arc<parking_lot::Mutex<Vec<Violation>>>,
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub timestamp: DateTime<Utc>,
    pub target: IpAddr,
    pub reason: String,
}

impl SafetyController {
    /// Create new safety controller with verified RoE
    pub fn new(
        roe: RulesOfEngagement,
        verifying_key: VerifyingKey,
    ) -> Result<Self, RoeError> {
        // Verify signature over RoE (excluding signature field)
        let mut roe_for_verify = roe.clone();
        roe_for_verify.signature = Signature::from_bytes(&[0u8; 64]);
        let roe_bytes = serde_json::to_vec(&roe_for_verify)
            .expect("RoE serialization failed");

        verifying_key
            .verify(&roe_bytes, &roe.signature)
            .map_err(|_| RoeError::InvalidSignature)?;

        // Validate temporal constraints
        let now = Utc::now();
        if now < roe.valid_from {
            return Err(RoeError::NotYetValid(roe.valid_from));
        }
        if now > roe.valid_until {
            return Err(RoeError::Expired(roe.valid_until));
        }

        Ok(Self {
            roe,
            kill_switch: Arc::new(AtomicBool::new(false)),
            verifying_key,
            violations: Arc::new(parking_lot::Mutex::new(Vec::new())),
        })
    }

    /// Check if target is within authorized scope
    pub fn check_target(&self, target: IpAddr, port: u16) -> Result<(), RoeError> {
        // Kill switch takes absolute precedence
        if self.kill_switch.load(Ordering::SeqCst) {
            return Err(RoeError::KillSwitchActive);
        }

        // Check exclusion list first (takes precedence)
        for excluded in &self.roe.excluded_targets {
            if excluded.contains(&target) {
                self.record_violation(target, "Target in exclusion list");
                return Err(RoeError::TargetExcluded(target));
            }
        }

        // Check authorized targets
        let in_scope = self.roe.authorized_targets
            .iter()
            .any(|net| net.contains(&target));

        if !in_scope {
            self.record_violation(target, "Target not in authorized scope");
            return Err(RoeError::TargetOutOfScope(target));
        }

        // Check authorized ports
        if !self.roe.authorized_ports.is_empty()
            && !self.roe.authorized_ports.contains(&port)
        {
            return Err(RoeError::PortNotAuthorized(port));
        }

        Ok(())
    }

    /// Activate emergency kill switch
    pub fn activate_kill_switch(&self) {
        self.kill_switch.store(true, Ordering::SeqCst);
        tracing::error!("KILL SWITCH ACTIVATED - All operations halted");
    }

    /// Get kill switch handle for external monitoring
    pub fn kill_switch_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.kill_switch)
    }

    fn record_violation(&self, target: IpAddr, reason: &str) {
        let violation = Violation {
            timestamp: Utc::now(),
            target,
            reason: reason.to_string(),
        };
        self.violations.lock().push(violation.clone());
        tracing::warn!(
            target = %target,
            reason = %reason,
            "RoE violation recorded"
        );
    }

    /// Export violations for audit
    pub fn export_violations(&self) -> Vec<Violation> {
        self.violations.lock().clone()
    }
}

/// UDP listener for HALT broadcast signals
pub struct HaltListener {
    kill_switch: Arc<AtomicBool>,
    verifying_key: VerifyingKey,
}

impl HaltListener {
    pub fn new(
        kill_switch: Arc<AtomicBool>,
        verifying_key: VerifyingKey,
    ) -> Self {
        Self { kill_switch, verifying_key }
    }

    /// Start listening for HALT signals on designated port
    pub async fn listen(&self, port: u16) -> std::io::Result<()> {
        use tokio::net::UdpSocket;

        let socket = UdpSocket::bind(("0.0.0.0", port)).await?;
        let mut buf = [0u8; 128];

        loop {
            let (len, _addr) = socket.recv_from(&mut buf).await?;
            if len >= 72 {
                // HALT packet: 8-byte magic + 64-byte signature
                let magic = &buf[0..8];
                if magic == b"WRAITHLT" {
                    let sig_bytes: [u8; 64] = buf[8..72].try_into().unwrap();
                    let signature = Signature::from_bytes(&sig_bytes);

                    if self.verifying_key.verify(magic, &signature).is_ok() {
                        self.kill_switch.store(true, Ordering::SeqCst);
                        tracing::error!("HALT signal received - Kill switch activated");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn create_test_roe() -> (RulesOfEngagement, VerifyingKey) {
        let signing_key = SigningKey::from_bytes(&[1u8; 32]);
        let verifying_key = signing_key.verifying_key();

        let mut roe = RulesOfEngagement {
            operation_id: "TEST-001".to_string(),
            authorized_targets: vec!["192.168.1.0/24".parse().unwrap()],
            excluded_targets: vec!["192.168.1.1/32".parse().unwrap()],
            authorized_ports: [22, 80, 443].into_iter().collect(),
            max_scan_rate: 1000,
            valid_from: Utc::now() - chrono::Duration::hours(1),
            valid_until: Utc::now() + chrono::Duration::hours(1),
            emergency_contact: "security@example.com".to_string(),
            signature: Signature::from_bytes(&[0u8; 64]),
        };

        let roe_bytes = serde_json::to_vec(&roe).unwrap();
        roe.signature = signing_key.sign(&roe_bytes);

        (roe, verifying_key)
    }

    #[test]
    fn test_valid_target() {
        let (roe, key) = create_test_roe();
        let controller = SafetyController::new(roe, key).unwrap();

        // Valid target and port
        assert!(controller.check_target("192.168.1.100".parse().unwrap(), 80).is_ok());
    }

    #[test]
    fn test_excluded_target() {
        let (roe, key) = create_test_roe();
        let controller = SafetyController::new(roe, key).unwrap();

        // Explicitly excluded
        let result = controller.check_target("192.168.1.1".parse().unwrap(), 80);
        assert!(matches!(result, Err(RoeError::TargetExcluded(_))));
    }

    #[test]
    fn test_out_of_scope() {
        let (roe, key) = create_test_roe();
        let controller = SafetyController::new(roe, key).unwrap();

        // Not in authorized range
        let result = controller.check_target("10.0.0.1".parse().unwrap(), 80);
        assert!(matches!(result, Err(RoeError::TargetOutOfScope(_))));
    }

    #[test]
    fn test_kill_switch() {
        let (roe, key) = create_test_roe();
        let controller = SafetyController::new(roe, key).unwrap();

        controller.activate_kill_switch();

        let result = controller.check_target("192.168.1.100".parse().unwrap(), 80);
        assert!(matches!(result, Err(RoeError::KillSwitchActive)));
    }
}
```

#### Governance Checkpoint

- [ ] RoE schema reviewed by legal/compliance
- [ ] Signature verification tested with production keys
- [ ] Kill switch latency verified < 1ms
- [ ] Violation logging meets audit requirements

---

### RECON-002: AF_XDP High-Performance Packet Capture

**As a** network analyst,
**I want** AF_XDP-based packet capture at wire speed,
**So that** I can perform passive reconnaissance without packet loss.

**Story Points:** 34
**Priority:** P0 (Critical)
**Sprint:** Phase 1, Week 41-43

#### Acceptance Criteria

1. Capture rate exceeds 1M packets/second on single core
2. Zero-copy path from NIC to userspace
3. eBPF filter enforces RoE at kernel level
4. Memory-mapped ring buffers prevent allocation overhead
5. Graceful fallback to AF_PACKET on unsupported systems

#### Implementation

```rust
//! RECON-002: AF_XDP High-Performance Packet Capture
//! Location: wraith-recon/src/capture/afxdp.rs

use std::ffi::CString;
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr::NonNull;
use std::sync::Arc;

use libbpf_rs::{MapFlags, Object, ObjectBuilder};
use thiserror::Error;

/// AF_XDP socket configuration
#[derive(Debug, Clone)]
pub struct XdpConfig {
    /// Network interface name
    pub interface: String,

    /// Queue ID to attach to
    pub queue_id: u32,

    /// Number of frames in UMEM
    pub frame_count: u32,

    /// Size of each frame (must be power of 2)
    pub frame_size: u32,

    /// Fill ring size
    pub fill_size: u32,

    /// Completion ring size
    pub comp_size: u32,

    /// RX ring size
    pub rx_size: u32,

    /// TX ring size
    pub tx_size: u32,

    /// Use zero-copy mode if available
    pub zero_copy: bool,
}

impl Default for XdpConfig {
    fn default() -> Self {
        Self {
            interface: "eth0".to_string(),
            queue_id: 0,
            frame_count: 4096,
            frame_size: 4096,
            fill_size: 2048,
            comp_size: 2048,
            rx_size: 2048,
            tx_size: 2048,
            zero_copy: true,
        }
    }
}

#[derive(Debug, Error)]
pub enum XdpError {
    #[error("Failed to load BPF program: {0}")]
    BpfLoad(String),

    #[error("Failed to create UMEM: {0}")]
    UmemCreate(String),

    #[error("Failed to create XDP socket: {0}")]
    SocketCreate(String),

    #[error("Interface {0} not found")]
    InterfaceNotFound(String),

    #[error("AF_XDP not supported on this system")]
    NotSupported,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// UMEM (User-space Memory) region for zero-copy
pub struct Umem {
    /// Memory-mapped region
    area: NonNull<u8>,

    /// Total size of the region
    size: usize,

    /// Frame size
    frame_size: u32,

    /// Number of frames
    frame_count: u32,
}

impl Umem {
    /// Allocate page-aligned UMEM region
    pub fn new(frame_count: u32, frame_size: u32) -> Result<Self, XdpError> {
        let size = (frame_count as usize) * (frame_size as usize);

        // Allocate page-aligned memory
        let area = unsafe {
            let ptr = libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_HUGETLB,
                -1,
                0,
            );

            // Fall back to regular pages if huge pages unavailable
            let ptr = if ptr == libc::MAP_FAILED {
                libc::mmap(
                    std::ptr::null_mut(),
                    size,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                )
            } else {
                ptr
            };

            if ptr == libc::MAP_FAILED {
                return Err(XdpError::UmemCreate("mmap failed".to_string()));
            }

            NonNull::new(ptr as *mut u8).unwrap()
        };

        Ok(Self {
            area,
            size,
            frame_size,
            frame_count,
        })
    }

    /// Get frame at index
    pub fn frame(&self, idx: u32) -> &[u8] {
        assert!(idx < self.frame_count);
        let offset = (idx as usize) * (self.frame_size as usize);
        unsafe {
            std::slice::from_raw_parts(
                self.area.as_ptr().add(offset),
                self.frame_size as usize,
            )
        }
    }

    /// Get mutable frame at index
    pub fn frame_mut(&mut self, idx: u32) -> &mut [u8] {
        assert!(idx < self.frame_count);
        let offset = (idx as usize) * (self.frame_size as usize);
        unsafe {
            std::slice::from_raw_parts_mut(
                self.area.as_ptr().add(offset),
                self.frame_size as usize,
            )
        }
    }

    /// Get raw pointer for registration
    pub fn as_ptr(&self) -> *mut u8 {
        self.area.as_ptr()
    }

    /// Get total size
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for Umem {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.area.as_ptr() as *mut libc::c_void, self.size);
        }
    }
}

/// Fill ring producer (kernel -> userspace buffer management)
pub struct FillQueue {
    ring: NonNull<u64>,
    mask: u32,
    size: u32,
    producer: *mut u32,
    consumer: *const u32,
}

impl FillQueue {
    /// Reserve entries in the fill queue
    pub fn reserve(&mut self, count: u32) -> Option<u32> {
        let prod = unsafe { (*self.producer) };
        let cons = unsafe { (*self.consumer) };
        let free = self.size - (prod - cons);

        if free < count {
            return None;
        }

        Some(prod)
    }

    /// Submit frame addresses to fill queue
    pub fn submit(&mut self, start: u32, addrs: &[u64]) {
        for (i, &addr) in addrs.iter().enumerate() {
            let idx = (start + i as u32) & self.mask;
            unsafe {
                *self.ring.as_ptr().add(idx as usize) = addr;
            }
        }

        // Memory barrier
        std::sync::atomic::fence(std::sync::atomic::Ordering::Release);

        unsafe {
            *self.producer = start + addrs.len() as u32;
        }
    }
}

/// Completion ring consumer (kernel signals TX completion)
pub struct CompQueue {
    ring: NonNull<u64>,
    mask: u32,
    producer: *const u32,
    consumer: *mut u32,
}

impl CompQueue {
    /// Consume completed frame addresses
    pub fn consume(&mut self, max: u32) -> Vec<u64> {
        let prod = unsafe { (*self.producer) };
        let cons = unsafe { (*self.consumer) };
        let available = prod - cons;
        let count = available.min(max);

        if count == 0 {
            return Vec::new();
        }

        // Memory barrier
        std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);

        let mut addrs = Vec::with_capacity(count as usize);
        for i in 0..count {
            let idx = (cons + i) & self.mask;
            let addr = unsafe { *self.ring.as_ptr().add(idx as usize) };
            addrs.push(addr);
        }

        unsafe {
            *self.consumer = cons + count;
        }

        addrs
    }
}

/// RX ring for receiving packets
pub struct RxRing {
    ring: NonNull<XdpDesc>,
    mask: u32,
    producer: *const u32,
    consumer: *mut u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct XdpDesc {
    pub addr: u64,
    pub len: u32,
    pub options: u32,
}

impl RxRing {
    /// Receive packets from the RX ring
    pub fn receive(&mut self, max: u32) -> Vec<XdpDesc> {
        let prod = unsafe { (*self.producer) };
        let cons = unsafe { (*self.consumer) };
        let available = prod - cons;
        let count = available.min(max);

        if count == 0 {
            return Vec::new();
        }

        std::sync::atomic::fence(std::sync::atomic::Ordering::Acquire);

        let mut descs = Vec::with_capacity(count as usize);
        for i in 0..count {
            let idx = (cons + i) & self.mask;
            let desc = unsafe { *self.ring.as_ptr().add(idx as usize) };
            descs.push(desc);
        }

        unsafe {
            *self.consumer = cons + count;
        }

        descs
    }
}

/// AF_XDP socket for high-performance packet capture
pub struct XdpSocket {
    fd: RawFd,
    umem: Arc<Umem>,
    fill: FillQueue,
    comp: CompQueue,
    rx: RxRing,
    config: XdpConfig,
}

impl XdpSocket {
    /// Create new AF_XDP socket
    pub fn new(config: XdpConfig) -> Result<Self, XdpError> {
        // Check kernel support
        let kernel_version = Self::check_kernel_support()?;
        tracing::info!(
            kernel = %kernel_version,
            interface = %config.interface,
            "Initializing AF_XDP socket"
        );

        // Create UMEM
        let umem = Arc::new(Umem::new(config.frame_count, config.frame_size)?);

        // Create socket (simplified - actual implementation uses libbpf)
        let fd = unsafe {
            libc::socket(libc::AF_XDP, libc::SOCK_RAW, 0)
        };

        if fd < 0 {
            return Err(XdpError::SocketCreate("socket() failed".to_string()));
        }

        // Note: Full implementation would register UMEM, create rings,
        // load BPF program, and attach to interface

        todo!("Complete AF_XDP socket initialization")
    }

    fn check_kernel_support() -> Result<String, XdpError> {
        let uname = nix::sys::utsname::uname()
            .map_err(|_| XdpError::NotSupported)?;

        let release = uname.release().to_string_lossy();
        let parts: Vec<u32> = release
            .split('.')
            .take(2)
            .filter_map(|s| s.parse().ok())
            .collect();

        if parts.len() >= 2 && (parts[0] > 5 || (parts[0] == 5 && parts[1] >= 4)) {
            Ok(release.to_string())
        } else {
            Err(XdpError::NotSupported)
        }
    }

    /// Receive packets (zero-copy)
    pub fn receive(&mut self) -> Vec<PacketRef> {
        let descs = self.rx.receive(64);

        descs.iter().map(|desc| {
            let frame_idx = (desc.addr / self.config.frame_size as u64) as u32;
            let offset = (desc.addr % self.config.frame_size as u64) as usize;
            let data = &self.umem.frame(frame_idx)[offset..offset + desc.len as usize];

            PacketRef {
                data,
                timestamp: std::time::Instant::now(),
            }
        }).collect()
    }
}

impl Drop for XdpSocket {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.fd);
        }
    }
}

/// Zero-copy packet reference
pub struct PacketRef<'a> {
    pub data: &'a [u8],
    pub timestamp: std::time::Instant,
}

/// eBPF program for kernel-level RoE enforcement
pub struct BpfFilter {
    obj: Object,
}

impl BpfFilter {
    /// Load BPF program from object file
    pub fn load(path: &str) -> Result<Self, XdpError> {
        let obj = ObjectBuilder::default()
            .open_file(path)
            .map_err(|e| XdpError::BpfLoad(e.to_string()))?
            .load()
            .map_err(|e| XdpError::BpfLoad(e.to_string()))?;

        Ok(Self { obj })
    }

    /// Update authorized CIDR map
    pub fn update_authorized_cidrs(&mut self, cidrs: &[ipnet::IpNet]) -> Result<(), XdpError> {
        let map = self.obj.map("authorized_cidrs")
            .ok_or_else(|| XdpError::BpfLoad("Map not found".to_string()))?;

        for (i, cidr) in cidrs.iter().enumerate() {
            let key = (i as u32).to_ne_bytes();
            let value = match cidr {
                ipnet::IpNet::V4(v4) => {
                    let mut buf = [0u8; 8];
                    buf[0..4].copy_from_slice(&v4.network().octets());
                    buf[4] = v4.prefix_len();
                    buf
                }
                ipnet::IpNet::V6(_) => {
                    // IPv6 handling
                    continue;
                }
            };

            map.update(&key, &value, MapFlags::ANY)
                .map_err(|e| XdpError::BpfLoad(e.to_string()))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_umem_allocation() {
        let umem = Umem::new(1024, 4096).unwrap();
        assert_eq!(umem.size(), 1024 * 4096);
    }

    #[test]
    fn test_frame_access() {
        let mut umem = Umem::new(16, 4096).unwrap();

        // Write to frame
        let frame = umem.frame_mut(0);
        frame[0..4].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);

        // Read back
        let frame = umem.frame(0);
        assert_eq!(&frame[0..4], &[0xDE, 0xAD, 0xBE, 0xEF]);
    }
}
```

#### Governance Checkpoint

- [ ] eBPF program reviewed for safety
- [ ] Kernel-level filtering matches RoE constraints
- [ ] Memory bounds verified (no buffer overflows)
- [ ] Performance benchmarks documented

---

### RECON-003: Passive Network Topology Discovery

**As a** network analyst,
**I want** passive topology discovery from captured traffic,
**So that** I can map network infrastructure without active probing.

**Story Points:** 21
**Priority:** P1 (High)
**Sprint:** Phase 1, Week 42-43

#### Acceptance Criteria

1. Build asset graph from observed traffic patterns
2. Extract OS fingerprints from TCP/IP characteristics
3. Identify network boundaries and routing paths
4. Detect services from protocol signatures
5. Export topology in standard formats (GraphML, JSON)

#### Implementation

```rust
//! RECON-003: Passive Network Topology Discovery
//! Location: wraith-recon/src/analysis/topology.rs

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

/// Discovered network asset
#[derive(Debug, Clone)]
pub struct Asset {
    /// IP address
    pub addr: IpAddr,

    /// Discovered hostnames (rDNS, mDNS, NetBIOS)
    pub hostnames: Vec<String>,

    /// Detected operating system
    pub os_fingerprint: Option<OsFingerprint>,

    /// Open ports with service info
    pub services: HashMap<u16, ServiceInfo>,

    /// First seen timestamp
    pub first_seen: Instant,

    /// Last seen timestamp
    pub last_seen: Instant,

    /// Total bytes observed
    pub bytes_total: u64,

    /// Total packets observed
    pub packets_total: u64,
}

/// TCP/IP stack fingerprint for OS detection
#[derive(Debug, Clone)]
pub struct OsFingerprint {
    /// Initial TTL (64 = Linux, 128 = Windows, 255 = Cisco)
    pub ttl: u8,

    /// TCP window size
    pub window_size: u16,

    /// TCP options (MSS, Scale, SACK, Timestamp)
    pub tcp_options: TcpOptions,

    /// DF (Don't Fragment) bit behavior
    pub df_set: bool,

    /// Detected OS family
    pub os_family: OsFamily,

    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct TcpOptions {
    pub mss: Option<u16>,
    pub window_scale: Option<u8>,
    pub sack_permitted: bool,
    pub timestamp: bool,
    pub nop_count: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OsFamily {
    Linux,
    Windows,
    MacOS,
    BSD,
    Cisco,
    Solaris,
    Unknown,
}

/// Network service information
#[derive(Debug, Clone)]
pub struct ServiceInfo {
    pub port: u16,
    pub protocol: Protocol,
    pub service_name: Option<String>,
    pub version: Option<String>,
    pub banner: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Protocol {
    Tcp,
    Udp,
    Sctp,
}

/// Connection between two assets
#[derive(Debug, Clone)]
pub struct Connection {
    /// Source port
    pub src_port: u16,

    /// Destination port
    pub dst_port: u16,

    /// Protocol
    pub protocol: Protocol,

    /// Total bytes transferred
    pub bytes: u64,

    /// Total packets
    pub packets: u64,

    /// Connection state
    pub state: ConnectionState,

    /// First packet timestamp
    pub first_seen: Instant,

    /// Last packet timestamp
    pub last_seen: Instant,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    SynSent,
    SynReceived,
    Established,
    FinWait,
    Closed,
    Reset,
}

/// Network topology graph
pub struct TopologyGraph {
    graph: DiGraph<Asset, Connection>,
    addr_index: HashMap<IpAddr, NodeIndex>,
}

impl TopologyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            addr_index: HashMap::new(),
        }
    }

    /// Add or update asset from observed packet
    pub fn observe_packet(&mut self, packet: &ParsedPacket) {
        let now = Instant::now();

        // Update source asset
        let src_idx = self.get_or_create_asset(packet.src_addr, now);
        {
            let src = &mut self.graph[src_idx];
            src.last_seen = now;
            src.bytes_total += packet.len as u64;
            src.packets_total += 1;

            // Update OS fingerprint if TCP SYN
            if packet.is_syn() {
                src.os_fingerprint = Some(self.fingerprint_os(packet));
            }
        }

        // Update destination asset
        let dst_idx = self.get_or_create_asset(packet.dst_addr, now);
        {
            let dst = &mut self.graph[dst_idx];
            dst.last_seen = now;

            // Record service if destination port is well-known
            if packet.dst_port < 1024 {
                dst.services.entry(packet.dst_port).or_insert_with(|| {
                    ServiceInfo {
                        port: packet.dst_port,
                        protocol: packet.protocol.clone(),
                        service_name: well_known_service(packet.dst_port),
                        version: None,
                        banner: None,
                    }
                });
            }
        }

        // Update or create edge (connection)
        self.update_connection(src_idx, dst_idx, packet, now);
    }

    fn get_or_create_asset(&mut self, addr: IpAddr, now: Instant) -> NodeIndex {
        if let Some(&idx) = self.addr_index.get(&addr) {
            return idx;
        }

        let asset = Asset {
            addr,
            hostnames: Vec::new(),
            os_fingerprint: None,
            services: HashMap::new(),
            first_seen: now,
            last_seen: now,
            bytes_total: 0,
            packets_total: 0,
        };

        let idx = self.graph.add_node(asset);
        self.addr_index.insert(addr, idx);
        idx
    }

    fn update_connection(
        &mut self,
        src: NodeIndex,
        dst: NodeIndex,
        packet: &ParsedPacket,
        now: Instant,
    ) {
        // Find existing edge or create new one
        let edge = self.graph.edges_connecting(src, dst)
            .find(|e| {
                let conn = e.weight();
                conn.src_port == packet.src_port
                    && conn.dst_port == packet.dst_port
                    && conn.protocol == packet.protocol
            });

        if let Some(edge) = edge {
            let edge_idx = edge.id();
            let conn = &mut self.graph[edge_idx];
            conn.bytes += packet.len as u64;
            conn.packets += 1;
            conn.last_seen = now;
            conn.state = self.infer_state(packet, &conn.state);
        } else {
            let conn = Connection {
                src_port: packet.src_port,
                dst_port: packet.dst_port,
                protocol: packet.protocol.clone(),
                bytes: packet.len as u64,
                packets: 1,
                state: if packet.is_syn() {
                    ConnectionState::SynSent
                } else {
                    ConnectionState::Established
                },
                first_seen: now,
                last_seen: now,
            };
            self.graph.add_edge(src, dst, conn);
        }
    }

    fn fingerprint_os(&self, packet: &ParsedPacket) -> OsFingerprint {
        let ttl = packet.ttl;
        let window = packet.tcp_window.unwrap_or(0);
        let options = packet.tcp_options.clone().unwrap_or_default();

        // Determine OS family from TTL
        let os_family = match ttl {
            1..=64 => OsFamily::Linux,
            65..=128 => OsFamily::Windows,
            129..=255 => OsFamily::Cisco,
            _ => OsFamily::Unknown,
        };

        // Calculate confidence based on multiple factors
        let confidence = self.calculate_confidence(ttl, window, &options, &os_family);

        OsFingerprint {
            ttl,
            window_size: window,
            tcp_options: options,
            df_set: packet.df_flag,
            os_family,
            confidence,
        }
    }

    fn calculate_confidence(
        &self,
        ttl: u8,
        window: u16,
        options: &TcpOptions,
        family: &OsFamily,
    ) -> f32 {
        let mut score = 0.0;

        // TTL match (weight: 0.3)
        score += match (family, ttl) {
            (OsFamily::Linux, 64) => 0.3,
            (OsFamily::Windows, 128) => 0.3,
            (OsFamily::Cisco, 255) => 0.3,
            _ => 0.1,
        };

        // Window size patterns (weight: 0.3)
        score += match (family, window) {
            (OsFamily::Linux, 5840) => 0.3,
            (OsFamily::Linux, 14600) => 0.3,
            (OsFamily::Windows, 8192) => 0.3,
            (OsFamily::Windows, 65535) => 0.3,
            _ => 0.1,
        };

        // TCP options patterns (weight: 0.4)
        if options.timestamp && options.sack_permitted {
            score += 0.2; // Modern stack
        }
        if options.window_scale.is_some() {
            score += 0.1;
        }
        if options.mss.is_some() {
            score += 0.1;
        }

        score.min(1.0)
    }

    fn infer_state(&self, packet: &ParsedPacket, current: &ConnectionState) -> ConnectionState {
        match (current, packet.is_syn(), packet.is_fin(), packet.is_rst()) {
            (_, true, false, false) => ConnectionState::SynSent,
            (ConnectionState::SynSent, false, false, false) => ConnectionState::Established,
            (_, false, true, false) => ConnectionState::FinWait,
            (ConnectionState::FinWait, false, true, false) => ConnectionState::Closed,
            (_, false, false, true) => ConnectionState::Reset,
            _ => current.clone(),
        }
    }

    /// Export topology as GraphML
    pub fn export_graphml(&self) -> String {
        let mut output = String::new();
        output.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        output.push_str("\n<graphml xmlns=\"http://graphml.graphdrawing.org/xmlns\">\n");
        output.push_str("  <graph id=\"G\" edgedefault=\"directed\">\n");

        // Export nodes
        for idx in self.graph.node_indices() {
            let asset = &self.graph[idx];
            output.push_str(&format!(
                "    <node id=\"{}\">\n      <data key=\"ip\">{}</data>\n",
                idx.index(),
                asset.addr
            ));
            if let Some(ref fp) = asset.os_fingerprint {
                output.push_str(&format!(
                    "      <data key=\"os\">{:?}</data>\n",
                    fp.os_family
                ));
            }
            output.push_str("    </node>\n");
        }

        // Export edges
        for edge in self.graph.edge_references() {
            let conn = edge.weight();
            output.push_str(&format!(
                "    <edge source=\"{}\" target=\"{}\">\n      <data key=\"port\">{}</data>\n      <data key=\"bytes\">{}</data>\n    </edge>\n",
                edge.source().index(),
                edge.target().index(),
                conn.dst_port,
                conn.bytes
            ));
        }

        output.push_str("  </graph>\n</graphml>\n");
        output
    }

    /// Get all discovered assets
    pub fn assets(&self) -> impl Iterator<Item = &Asset> {
        self.graph.node_weights()
    }

    /// Get asset count
    pub fn asset_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get connection count
    pub fn connection_count(&self) -> usize {
        self.graph.edge_count()
    }
}

/// Parsed packet for topology analysis
#[derive(Debug, Clone)]
pub struct ParsedPacket {
    pub src_addr: IpAddr,
    pub dst_addr: IpAddr,
    pub src_port: u16,
    pub dst_port: u16,
    pub protocol: Protocol,
    pub len: u16,
    pub ttl: u8,
    pub df_flag: bool,
    pub tcp_flags: u8,
    pub tcp_window: Option<u16>,
    pub tcp_options: Option<TcpOptions>,
}

impl ParsedPacket {
    pub fn is_syn(&self) -> bool {
        self.tcp_flags & 0x02 != 0 && self.tcp_flags & 0x10 == 0
    }

    pub fn is_fin(&self) -> bool {
        self.tcp_flags & 0x01 != 0
    }

    pub fn is_rst(&self) -> bool {
        self.tcp_flags & 0x04 != 0
    }
}

impl Default for TcpOptions {
    fn default() -> Self {
        Self {
            mss: None,
            window_scale: None,
            sack_permitted: false,
            timestamp: false,
            nop_count: 0,
        }
    }
}

fn well_known_service(port: u16) -> Option<String> {
    match port {
        21 => Some("ftp".to_string()),
        22 => Some("ssh".to_string()),
        23 => Some("telnet".to_string()),
        25 => Some("smtp".to_string()),
        53 => Some("dns".to_string()),
        80 => Some("http".to_string()),
        110 => Some("pop3".to_string()),
        143 => Some("imap".to_string()),
        443 => Some("https".to_string()),
        445 => Some("smb".to_string()),
        3306 => Some("mysql".to_string()),
        3389 => Some("rdp".to_string()),
        5432 => Some("postgresql".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_topology_build() {
        let mut graph = TopologyGraph::new();

        let packet = ParsedPacket {
            src_addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            dst_addr: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            src_port: 54321,
            dst_port: 80,
            protocol: Protocol::Tcp,
            len: 64,
            ttl: 64,
            df_flag: true,
            tcp_flags: 0x02, // SYN
            tcp_window: Some(65535),
            tcp_options: Some(TcpOptions::default()),
        };

        graph.observe_packet(&packet);

        assert_eq!(graph.asset_count(), 2);
        assert_eq!(graph.connection_count(), 1);
    }

    #[test]
    fn test_os_fingerprint() {
        let graph = TopologyGraph::new();

        let packet = ParsedPacket {
            src_addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            dst_addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)),
            src_port: 12345,
            dst_port: 22,
            protocol: Protocol::Tcp,
            len: 60,
            ttl: 64,
            df_flag: true,
            tcp_flags: 0x02,
            tcp_window: Some(14600),
            tcp_options: Some(TcpOptions {
                mss: Some(1460),
                window_scale: Some(7),
                sack_permitted: true,
                timestamp: true,
                nop_count: 1,
            }),
        };

        let fp = graph.fingerprint_os(&packet);
        assert_eq!(fp.os_family, OsFamily::Linux);
        assert!(fp.confidence > 0.5);
    }
}
```

---

### RECON-004: Protocol Mimicry Engine

**As a** penetration tester,
**I want** traffic that mimics legitimate protocols,
**So that** reconnaissance evades detection systems.

**Story Points:** 34
**Priority:** P1 (High)
**Sprint:** Phase 2, Week 44-47

#### Acceptance Criteria

1. Generate valid DNS queries with Base32 payload encoding
2. Produce TLS Client Hello matching Chrome/Firefox fingerprints
3. Embed data in ICMP echo request padding
4. All generated traffic passes Wireshark protocol dissectors
5. JA3/JA3S fingerprints match legitimate browsers

#### Implementation

```rust
//! RECON-004: Protocol Mimicry Engine
//! Location: wraith-recon/src/mimicry/mod.rs

use rand::Rng;
use std::net::Ipv4Addr;

/// Protocol mimicry profile
#[derive(Debug, Clone)]
pub enum MimicryProfile {
    /// DNS queries with Base32-encoded payloads
    Dns(DnsProfile),

    /// TLS Client Hello matching browser fingerprint
    Tls(TlsProfile),

    /// ICMP echo with steganographic payload
    Icmp(IcmpProfile),

    /// HTTP/2 with multiplexed data streams
    Http2(Http2Profile),

    /// WebSocket with fragmented messages
    WebSocket(WebSocketProfile),
}

/// DNS mimicry profile
#[derive(Debug, Clone)]
pub struct DnsProfile {
    /// Subdomain label format (affects encoding density)
    pub label_format: DnsLabelFormat,

    /// Maximum query name length
    pub max_query_len: usize,

    /// Use EDNS0 extensions
    pub use_edns0: bool,

    /// Authoritative nameserver domain
    pub ns_domain: String,
}

#[derive(Debug, Clone)]
pub enum DnsLabelFormat {
    /// Standard Base32 (RFC 4648)
    Base32,
    /// Base32-Hex variant
    Base32Hex,
    /// Hexadecimal encoding
    Hex,
}

/// TLS fingerprint profile
#[derive(Debug, Clone)]
pub struct TlsProfile {
    /// Target browser to mimic
    pub browser: BrowserFingerprint,

    /// TLS version (1.2 or 1.3)
    pub tls_version: TlsVersion,

    /// ALPN protocols
    pub alpn: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum BrowserFingerprint {
    Chrome120,
    Chrome121,
    Firefox120,
    Firefox121,
    Safari17,
    Edge120,
}

#[derive(Debug, Clone, Copy)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

/// ICMP steganography profile
#[derive(Debug, Clone)]
pub struct IcmpProfile {
    /// Payload size (8-1472 bytes)
    pub payload_size: usize,

    /// Identifier field strategy
    pub id_strategy: IcmpIdStrategy,

    /// Sequence number strategy
    pub seq_strategy: IcmpSeqStrategy,
}

#[derive(Debug, Clone)]
pub enum IcmpIdStrategy {
    /// Random ID per session
    RandomPerSession,
    /// Incremental from random base
    Incremental,
    /// Encode data in ID field
    DataCarrier,
}

#[derive(Debug, Clone)]
pub enum IcmpSeqStrategy {
    /// Standard incrementing
    Incrementing,
    /// Randomized sequence
    Random,
}

/// HTTP/2 profile
#[derive(Debug, Clone)]
pub struct Http2Profile {
    /// Initial window size
    pub initial_window_size: u32,
    /// Max concurrent streams
    pub max_concurrent_streams: u32,
    /// Header table size
    pub header_table_size: u32,
}

/// WebSocket profile
#[derive(Debug, Clone)]
pub struct WebSocketProfile {
    /// Fragmentation threshold
    pub fragment_threshold: usize,
    /// Masking key generation
    pub masking: WebSocketMasking,
}

#[derive(Debug, Clone)]
pub enum WebSocketMasking {
    Random,
    Predictable(u32),
}

/// DNS packet builder
pub struct DnsBuilder {
    profile: DnsProfile,
    transaction_id: u16,
}

impl DnsBuilder {
    pub fn new(profile: DnsProfile) -> Self {
        Self {
            profile,
            transaction_id: rand::thread_rng().gen(),
        }
    }

    /// Encode data as DNS query
    pub fn encode_query(&mut self, data: &[u8]) -> Vec<u8> {
        let mut packet = Vec::with_capacity(512);

        // DNS Header (12 bytes)
        packet.extend_from_slice(&self.transaction_id.to_be_bytes());
        packet.extend_from_slice(&[
            0x01, 0x00, // Flags: Standard query, recursion desired
            0x00, 0x01, // Questions: 1
            0x00, 0x00, // Answer RRs: 0
            0x00, 0x00, // Authority RRs: 0
            0x00, 0x00, // Additional RRs: 0 (will update for EDNS0)
        ]);

        // Encode data as subdomain labels
        let encoded = match self.profile.label_format {
            DnsLabelFormat::Base32 => base32_encode(data),
            DnsLabelFormat::Base32Hex => base32hex_encode(data),
            DnsLabelFormat::Hex => hex::encode(data),
        };

        // Split into labels (max 63 chars each)
        let labels: Vec<&str> = encoded
            .as_bytes()
            .chunks(63)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect();

        // Write labels
        for label in labels {
            packet.push(label.len() as u8);
            packet.extend_from_slice(label.as_bytes());
        }

        // Append authoritative domain
        for label in self.profile.ns_domain.split('.') {
            packet.push(label.len() as u8);
            packet.extend_from_slice(label.as_bytes());
        }

        packet.push(0x00); // End of name

        // Query type and class
        packet.extend_from_slice(&[
            0x00, 0x10, // Type: TXT
            0x00, 0x01, // Class: IN
        ]);

        // Add EDNS0 if configured
        if self.profile.use_edns0 {
            // Update additional RRs count
            packet[11] = 0x01;

            // OPT record
            packet.push(0x00); // Name (root)
            packet.extend_from_slice(&[
                0x00, 0x29, // Type: OPT
                0x10, 0x00, // UDP payload size: 4096
                0x00,       // Extended RCODE
                0x00,       // EDNS version
                0x00, 0x00, // Flags
                0x00, 0x00, // Data length
            ]);
        }

        self.transaction_id = self.transaction_id.wrapping_add(1);
        packet
    }

    /// Decode data from DNS response
    pub fn decode_response(&self, packet: &[u8]) -> Option<Vec<u8>> {
        if packet.len() < 12 {
            return None;
        }

        // Skip header, parse answer section
        let mut pos = 12;

        // Skip question section
        while pos < packet.len() && packet[pos] != 0 {
            let len = packet[pos] as usize;
            pos += len + 1;
        }
        pos += 5; // Skip null byte, type, class

        // Parse answer
        if pos >= packet.len() {
            return None;
        }

        // Skip name (may be compressed)
        if packet[pos] & 0xC0 == 0xC0 {
            pos += 2;
        } else {
            while pos < packet.len() && packet[pos] != 0 {
                pos += packet[pos] as usize + 1;
            }
            pos += 1;
        }

        // Skip type, class, TTL
        pos += 8;

        // Read data length
        if pos + 2 > packet.len() {
            return None;
        }
        let data_len = u16::from_be_bytes([packet[pos], packet[pos + 1]]) as usize;
        pos += 2;

        // Extract TXT record data
        if pos + data_len > packet.len() {
            return None;
        }

        let txt_len = packet[pos] as usize;
        let txt_data = &packet[pos + 1..pos + 1 + txt_len];

        // Decode from Base32
        match self.profile.label_format {
            DnsLabelFormat::Base32 => base32_decode(txt_data),
            DnsLabelFormat::Base32Hex => base32hex_decode(txt_data),
            DnsLabelFormat::Hex => hex::decode(txt_data).ok(),
        }
    }
}

/// TLS Client Hello builder
pub struct TlsClientHelloBuilder {
    profile: TlsProfile,
}

impl TlsClientHelloBuilder {
    pub fn new(profile: TlsProfile) -> Self {
        Self { profile }
    }

    /// Generate TLS Client Hello matching browser fingerprint
    pub fn build(&self, sni: &str) -> Vec<u8> {
        let mut hello = Vec::with_capacity(512);

        // TLS Record Layer
        hello.push(0x16); // Content type: Handshake
        hello.extend_from_slice(&[0x03, 0x01]); // Version: TLS 1.0 (compat)

        // Length placeholder
        let length_pos = hello.len();
        hello.extend_from_slice(&[0x00, 0x00]);

        // Handshake header
        hello.push(0x01); // Client Hello
        let handshake_len_pos = hello.len();
        hello.extend_from_slice(&[0x00, 0x00, 0x00]); // Length placeholder

        // Client version
        match self.profile.tls_version {
            TlsVersion::Tls12 => hello.extend_from_slice(&[0x03, 0x03]),
            TlsVersion::Tls13 => hello.extend_from_slice(&[0x03, 0x03]), // Same in Hello
        }

        // Random (32 bytes)
        let random: [u8; 32] = rand::thread_rng().gen();
        hello.extend_from_slice(&random);

        // Session ID
        hello.push(32);
        let session_id: [u8; 32] = rand::thread_rng().gen();
        hello.extend_from_slice(&session_id);

        // Cipher suites (browser-specific)
        let ciphers = self.get_cipher_suites();
        hello.extend_from_slice(&(ciphers.len() as u16 * 2).to_be_bytes());
        for cipher in &ciphers {
            hello.extend_from_slice(&cipher.to_be_bytes());
        }

        // Compression methods
        hello.push(1);
        hello.push(0); // null compression

        // Extensions
        let extensions = self.build_extensions(sni);
        hello.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
        hello.extend_from_slice(&extensions);

        // Fix up lengths
        let total_len = hello.len() - 5;
        hello[length_pos] = (total_len >> 8) as u8;
        hello[length_pos + 1] = total_len as u8;

        let handshake_len = hello.len() - handshake_len_pos - 3;
        hello[handshake_len_pos] = (handshake_len >> 16) as u8;
        hello[handshake_len_pos + 1] = (handshake_len >> 8) as u8;
        hello[handshake_len_pos + 2] = handshake_len as u8;

        hello
    }

    fn get_cipher_suites(&self) -> Vec<u16> {
        match self.profile.browser {
            BrowserFingerprint::Chrome120 | BrowserFingerprint::Chrome121 => vec![
                0x1301, // TLS_AES_128_GCM_SHA256
                0x1302, // TLS_AES_256_GCM_SHA384
                0x1303, // TLS_CHACHA20_POLY1305_SHA256
                0xc02c, // TLS_ECDHE_ECDSA_WITH_AES_256_GCM_SHA384
                0xc02b, // TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
                0xc030, // TLS_ECDHE_RSA_WITH_AES_256_GCM_SHA384
                0xc02f, // TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
            ],
            BrowserFingerprint::Firefox120 | BrowserFingerprint::Firefox121 => vec![
                0x1301,
                0x1303,
                0x1302,
                0xc02b,
                0xc02f,
                0xc02c,
                0xc030,
                0xcca9, // TLS_ECDHE_ECDSA_WITH_CHACHA20_POLY1305_SHA256
                0xcca8, // TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256
            ],
            _ => vec![0x1301, 0x1302, 0x1303, 0xc02c, 0xc02b],
        }
    }

    fn build_extensions(&self, sni: &str) -> Vec<u8> {
        let mut ext = Vec::new();

        // SNI extension
        ext.extend_from_slice(&[0x00, 0x00]); // Type: SNI
        let sni_len = sni.len() + 5;
        ext.extend_from_slice(&(sni_len as u16).to_be_bytes());
        ext.extend_from_slice(&((sni_len - 2) as u16).to_be_bytes());
        ext.push(0x00); // Host name type
        ext.extend_from_slice(&(sni.len() as u16).to_be_bytes());
        ext.extend_from_slice(sni.as_bytes());

        // Supported versions (TLS 1.3)
        ext.extend_from_slice(&[0x00, 0x2b]); // Type
        ext.extend_from_slice(&[0x00, 0x03]); // Length
        ext.push(0x02); // Versions length
        ext.extend_from_slice(&[0x03, 0x04]); // TLS 1.3

        // Supported groups
        ext.extend_from_slice(&[0x00, 0x0a]); // Type
        ext.extend_from_slice(&[0x00, 0x08]); // Length
        ext.extend_from_slice(&[0x00, 0x06]); // List length
        ext.extend_from_slice(&[0x00, 0x1d]); // x25519
        ext.extend_from_slice(&[0x00, 0x17]); // secp256r1
        ext.extend_from_slice(&[0x00, 0x18]); // secp384r1

        // Key share (x25519)
        ext.extend_from_slice(&[0x00, 0x33]); // Type
        ext.extend_from_slice(&[0x00, 0x26]); // Length
        ext.extend_from_slice(&[0x00, 0x24]); // List length
        ext.extend_from_slice(&[0x00, 0x1d]); // x25519
        ext.extend_from_slice(&[0x00, 0x20]); // Key length
        let key_share: [u8; 32] = rand::thread_rng().gen();
        ext.extend_from_slice(&key_share);

        // ALPN
        if !self.profile.alpn.is_empty() {
            ext.extend_from_slice(&[0x00, 0x10]); // Type
            let alpn_data: Vec<u8> = self.profile.alpn.iter()
                .flat_map(|p| {
                    let mut v = vec![p.len() as u8];
                    v.extend_from_slice(p.as_bytes());
                    v
                })
                .collect();
            ext.extend_from_slice(&((alpn_data.len() + 2) as u16).to_be_bytes());
            ext.extend_from_slice(&(alpn_data.len() as u16).to_be_bytes());
            ext.extend_from_slice(&alpn_data);
        }

        ext
    }

    /// Calculate JA3 fingerprint
    pub fn calculate_ja3(&self, hello: &[u8]) -> String {
        // JA3 = SSLVersion,Ciphers,Extensions,EllipticCurves,EllipticCurvePointFormats
        // Simplified implementation
        let ciphers = self.get_cipher_suites();
        let cipher_str: String = ciphers.iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("-");

        let ja3_string = format!("771,{},0-10-11-13-43-45-51,29-23-24,0", cipher_str);

        // MD5 hash
        format!("{:x}", md5::compute(ja3_string.as_bytes()))
    }
}

/// ICMP steganography builder
pub struct IcmpBuilder {
    profile: IcmpProfile,
    identifier: u16,
    sequence: u16,
}

impl IcmpBuilder {
    pub fn new(profile: IcmpProfile) -> Self {
        let identifier = match profile.id_strategy {
            IcmpIdStrategy::RandomPerSession => rand::thread_rng().gen(),
            IcmpIdStrategy::Incremental => rand::thread_rng().gen(),
            IcmpIdStrategy::DataCarrier => 0,
        };

        Self {
            profile,
            identifier,
            sequence: 0,
        }
    }

    /// Build ICMP echo request with embedded payload
    pub fn build_echo_request(&mut self, payload: &[u8]) -> Vec<u8> {
        let mut packet = Vec::with_capacity(8 + self.profile.payload_size);

        // ICMP header
        packet.push(8); // Type: Echo Request
        packet.push(0); // Code
        packet.extend_from_slice(&[0x00, 0x00]); // Checksum placeholder

        // Identifier
        let id = match self.profile.id_strategy {
            IcmpIdStrategy::DataCarrier if payload.len() >= 2 => {
                u16::from_be_bytes([payload[0], payload[1]])
            }
            _ => self.identifier,
        };
        packet.extend_from_slice(&id.to_be_bytes());

        // Sequence
        match self.profile.seq_strategy {
            IcmpSeqStrategy::Incrementing => {
                packet.extend_from_slice(&self.sequence.to_be_bytes());
                self.sequence = self.sequence.wrapping_add(1);
            }
            IcmpSeqStrategy::Random => {
                let seq: u16 = rand::thread_rng().gen();
                packet.extend_from_slice(&seq.to_be_bytes());
            }
        }

        // Payload (data embedded in padding area)
        let data_start = match self.profile.id_strategy {
            IcmpIdStrategy::DataCarrier => 2,
            _ => 0,
        };

        // Add actual payload
        packet.extend_from_slice(&payload[data_start..]);

        // Pad to required size
        while packet.len() < 8 + self.profile.payload_size {
            packet.push(rand::thread_rng().gen());
        }

        // Calculate checksum
        let checksum = Self::calculate_checksum(&packet);
        packet[2] = (checksum >> 8) as u8;
        packet[3] = checksum as u8;

        packet
    }

    fn calculate_checksum(data: &[u8]) -> u16 {
        let mut sum: u32 = 0;

        for chunk in data.chunks(2) {
            let word = if chunk.len() == 2 {
                u16::from_be_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_be_bytes([chunk[0], 0])
            };
            sum = sum.wrapping_add(word as u32);
        }

        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !sum as u16
    }

    /// Extract payload from ICMP echo reply
    pub fn extract_payload(&self, packet: &[u8]) -> Option<Vec<u8>> {
        if packet.len() < 8 {
            return None;
        }

        // Verify type (0 = echo reply)
        if packet[0] != 0 {
            return None;
        }

        let mut payload = Vec::new();

        // Extract from identifier if data carrier mode
        if matches!(self.profile.id_strategy, IcmpIdStrategy::DataCarrier) {
            payload.extend_from_slice(&packet[4..6]);
        }

        // Extract from payload area
        payload.extend_from_slice(&packet[8..]);

        Some(payload)
    }
}

// Base32 encoding utilities
fn base32_encode(data: &[u8]) -> String {
    data_encoding::BASE32_NOPAD.encode(data).to_lowercase()
}

fn base32_decode(data: &[u8]) -> Option<Vec<u8>> {
    let s = std::str::from_utf8(data).ok()?.to_uppercase();
    data_encoding::BASE32_NOPAD.decode(s.as_bytes()).ok()
}

fn base32hex_encode(data: &[u8]) -> String {
    data_encoding::BASE32HEX_NOPAD.encode(data).to_lowercase()
}

fn base32hex_decode(data: &[u8]) -> Option<Vec<u8>> {
    let s = std::str::from_utf8(data).ok()?.to_uppercase();
    data_encoding::BASE32HEX_NOPAD.decode(s.as_bytes()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_encode_decode() {
        let profile = DnsProfile {
            label_format: DnsLabelFormat::Base32,
            max_query_len: 253,
            use_edns0: true,
            ns_domain: "example.com".to_string(),
        };

        let mut builder = DnsBuilder::new(profile);
        let data = b"Hello, WRAITH!";
        let packet = builder.encode_query(data);

        // Verify packet structure
        assert!(packet.len() > 12);
        assert_eq!(packet[2] & 0x80, 0); // QR bit = 0 (query)
    }

    #[test]
    fn test_tls_client_hello() {
        let profile = TlsProfile {
            browser: BrowserFingerprint::Chrome120,
            tls_version: TlsVersion::Tls13,
            alpn: vec!["h2".to_string(), "http/1.1".to_string()],
        };

        let builder = TlsClientHelloBuilder::new(profile);
        let hello = builder.build("example.com");

        // Verify record layer
        assert_eq!(hello[0], 0x16); // Handshake
        assert_eq!(hello[5], 0x01); // Client Hello
    }

    #[test]
    fn test_icmp_checksum() {
        let profile = IcmpProfile {
            payload_size: 56,
            id_strategy: IcmpIdStrategy::RandomPerSession,
            seq_strategy: IcmpSeqStrategy::Incrementing,
        };

        let mut builder = IcmpBuilder::new(profile);
        let packet = builder.build_echo_request(b"test data");

        // Verify checksum
        let checksum = IcmpBuilder::calculate_checksum(&packet);
        assert_eq!(checksum, 0xFFFF); // Correct checksum yields 0xFFFF
    }
}
```

---

### RECON-005: Multi-Path Data Exfiltration

**As a** data loss prevention tester,
**I want** multi-path exfiltration simulation,
**So that** I can validate DLP detection capabilities.

**Story Points:** 21
**Priority:** P1 (High)
**Sprint:** Phase 3, Week 48-49

#### Acceptance Criteria

1. Split data across multiple covert channels simultaneously
2. Implement chunking with BLAKE3 integrity verification
3. Rate limiting per channel respects RoE constraints
4. Automatic failover when channel is blocked
5. End-to-end encryption with perfect forward secrecy

#### Implementation Details

Located in `wraith-recon/src/exfil/` with modules:
- `splitter.rs` - Multi-path data distribution
- `scheduler.rs` - Transmission scheduling
- `integrity.rs` - BLAKE3 chunk verification
- `channels/` - Channel implementations (DNS, ICMP, HTTPS)

---

### RECON-006: TUI Dashboard

**As an** operator,
**I want** a terminal-based dashboard,
**So that** I can monitor reconnaissance in real-time.

**Story Points:** 21
**Priority:** P2 (Medium)
**Sprint:** Phase 3, Week 49-50

#### Acceptance Criteria

1. Real-time asset discovery visualization
2. Network topology graph rendering
3. Exfiltration progress tracking
4. Kill switch and RoE status display
5. Keyboard shortcuts for common operations

---

### RECON-007: Reporting and Audit Export

**As a** security auditor,
**I want** comprehensive audit exports,
**So that** I can document testing activities.

**Story Points:** 13
**Priority:** P2 (Medium)
**Sprint:** Phase 3, Week 50-51

#### Acceptance Criteria

1. Export findings in JSON, CSV, and HTML formats
2. Include complete audit trail with timestamps
3. PCAPNG capture export for forensic analysis
4. GraphML topology export for visualization tools
5. Compliance report templates (PCI-DSS, HIPAA)

---

## Sprint Summary

| Sprint | Weeks | Story Points | Key Deliverables |
|--------|-------|--------------|------------------|
| Phase 1 | 40-43 | 60 | RoE enforcement, AF_XDP capture, passive analysis |
| Phase 2 | 44-47 | 60 | Active probing, protocol mimicry, detection evasion |
| Phase 3 | 48-51 | 60 | Exfiltration, TUI dashboard, reporting |

## Governance Gates

### Phase 1 Exit Criteria
- [ ] RoE signature verification passes all test cases
- [ ] Kill switch latency verified < 1ms
- [ ] AF_XDP capture achieves > 1M pps
- [ ] No out-of-scope packets transmitted

### Phase 2 Exit Criteria
- [ ] All mimicry profiles pass Wireshark validation
- [ ] JA3 fingerprints match target browsers
- [ ] Timing jitter follows Pareto distribution
- [ ] IDS/IPS evasion metrics documented

### Phase 3 Exit Criteria
- [ ] End-to-end exfiltration completes with integrity verification
- [ ] All audit exports validate against schemas
- [ ] User manual complete with examples
- [ ] Integration tests pass in isolated lab environment

---

## Risk Register

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| RoE bypass vulnerability | Critical | Low | Multiple validation layers, fuzzing |
| AF_XDP kernel compatibility | High | Medium | Fallback to AF_PACKET |
| Protocol mimicry detection | Medium | Medium | Regular fingerprint updates |
| Performance degradation | Medium | Low | Continuous benchmarking |

---

## Dependencies

### External Libraries
- `libbpf-rs` - eBPF loading and management
- `petgraph` - Graph data structures
- `crossterm` - Terminal UI
- `pcap-file` - PCAPNG writing
- `ed25519-dalek` - Signature verification
- `data-encoding` - Base32 encoding

### WRAITH Core Integration
- `wraith-crypto` - AEAD encryption, key derivation
- `wraith-obfuscation` - Padding, timing profiles
- `wraith-transport` - UDP socket abstraction
