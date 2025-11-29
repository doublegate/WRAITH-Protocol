# WRAITH-Recon Implementation Details

**Document Version:** 1.5.0 (The "Superset" - Complete Technical Spec)
**Last Updated:** 2025-11-29

---

## 1. Capture Engine (Passive)

### 1.1 AF_XDP Integration (Kernel Bypass)
The passive engine uses `wraith-transport`'s AF_XDP backend to read packets directly from the NIC's RX queue, bypassing the OS TCP/IP stack for performance and invisibility.

**Socket Configuration:**
```rust
// Pseudo-code for Socket Creation
let config = UmemConfig {
    frame_size: 2048,
    fill_queue_size: 4096,
    comp_queue_size: 4096,
    frame_headroom: 0,
    ..Default::default()
};

let (umem, fill_q, comp_q) = Umem::new(config)?;
let (tx_q, rx_q) = Socket::new(iface, queue_id, umem)?;
```

### 1.2 Fallback Mechanism (libpcap)
While AF_XDP is preferred, it requires Kernel 5.18+. For older systems or non-Linux platforms, a fallback is implemented.

```rust
pub enum CaptureSource {
    Xdp(XdpSocket),
    Pcap(pcap::Capture<pcap::Active>),
}

impl CaptureSource {
    pub fn next_packet(&mut self) -> Option<&[u8]> {
        match self {
            Self::Xdp(sock) => sock.receive_frame(),
            Self::Pcap(cap) => cap.next().ok().map(|p| p.data),
        }
    }
}
```

### 1.3 BPF Filter Logic
**eBPF Program Structure (C-like pseudo-code):**
```c
SEC("xdp")
int xdp_filter(struct xdp_md *ctx) {
    void *data_end = (void *)(long)ctx->data_end;
    void *data = (void *)(long)ctx->data;
    struct ethhdr *eth = data;
    
    // 1. Parse Ethernet
    if (eth + 1 > data_end) return XDP_DROP;
    if (eth->h_proto != htons(ETH_P_IP)) return XDP_PASS;
    
    // 2. Parse IP
    struct iphdr *ip = data + sizeof(*eth);
    if (ip + 1 > data_end) return XDP_DROP;
    
    // 3. Governance Check (Allowed Map)
    u32 *val = bpf_map_lookup_elem(&allowed_cidrs, &ip->daddr);
    if (!val) return XDP_PASS; // Pass non-target traffic to OS
    
    // 4. Redirect to AF_XDP Socket
    return bpf_redirect_map(&xsks_map, ctx->rx_queue_index, 0);
}
```

### 1.4 User-Space Loader
Loads the BPF object file using `libbpf-rs`.

```rust
// src/xdp/loader.rs
pub fn attach_xdp(iface: &str) -> Result<BpfLink> {
    let skel = XdpFilterSkelBuilder::default().open()?.load()?;
    let link = skel.progs().xdp_filter().attach_xdp(iface_index)?;
    Ok(link)
}
```

---

## 2. Active Stealth Engine (Active)

### 2.1 Stateless Scanner
The active scanner tracks connection state in a compact Hash Map (`ConnectionTable`), bypassing the kernel's TCP stack entirely to avoid `RST` packets from the OS and reduce resource usage.

```rust
// src/recon/active.rs
pub struct StealthScanner {
    transport: Transport,
    obfuscator: Obfuscator,
    state: ConnectionTable, // (SrcIP, SrcPort, DstIP, DstPort) -> State
}

impl StealthScanner {
    pub async fn scan_target(&mut self, target_ip: &str, ports: &[u16]) {
        let profile = TimingProfile::Pareto { min_ms: 10, alpha: 1.2 };

        for port in ports {
            // Jitter calculation
            let delay = self.obfuscator.calculate_delay(&profile);
            sleep(delay).await;

            let packet = self.construct_syn_probe(target_ip, *port);
            self.transport.send(packet).await.unwrap();
        }
    }
}
```

### 2.2 Jitter Algorithms (Mathematical Specification)

The `Obfuscator` uses statistical distributions to generate Inter-Arrival Times (IAT).

**1. Pareto Distribution (Long Tails):**
Used to mimic "bursty" user traffic (e.g., web browsing).
$$ IAT = \frac{X_m}{U^{1/\alpha}} $$
*   $X_m$: Minimum delay (e.g., 10ms).
*   $U$: Random uniform variable $(0, 1]$.
*   $\alpha$: Shape parameter (1.1 - 1.5).

**2. Normal Distribution (Regular with Jitter):**
Used for "heartbeat" mimicry (e.g., NTP, beacons).
$$ IAT = \mu + (\sigma \cdot Z) $$
*   $\mu$: Mean interval.
*   $\sigma$: Standard deviation.
*   $Z$: Box-Muller transform of random variables.

---

## 3. Mimicry Engine (Exfiltration)

### 3.1 DNS Tunneling Implementation
*   **Encoding:** Base32 (RFC 4648) to be case-insensitive safe.
*   **Structure:** `[ChunkID][Data].[SessionID].[Domain]`

```rust
pub struct DnsMimic {
    domain: String,
    session_id: u16,
}

impl DnsMimic {
    pub fn encode(&self, data: &[u8]) -> Vec<u8> {
        let encoded = base32::encode(data);
        // Split into labels < 63 chars per DNS spec
        let labels: Vec<String> = encoded.chars()
            .collect::<Vec<char>>()
            .chunks(60)
            .map(|c| c.iter().collect())
            .collect();
        // Construct Packet...
    }
}
```

---

## 4. Asset Data Structures

### 4.1 Asset Graph (In-Memory)
Uses a graph structure to model relationships (e.g., "Host A talks to Host B on Port 443").

```rust
use petgraph::graph::{Graph, NodeIndex};

pub struct AssetDB {
    graph: Graph<HostNode, ConnectionEdge>,
    ip_index: HashMap<IpAddr, NodeIndex>,
}

pub struct HostNode {
    pub ip: IpAddr,
    pub mac: Option<MacAddr>,
    pub os_fingerprint: Option<OsFamily>,
    pub open_ports: HashSet<u16>,
    pub tags: Vec<String>,
}

pub struct ConnectionEdge {
    pub port: u16,
    pub proto: Protocol,
    pub last_seen: u64,
}
```

---

## 5. Governance & Safety (The "Kill Switch")

### 5.1 Runtime Scope Check
Every packet generation function calls `Governance::check(dst_ip)` before writing to the TX ring.

```rust
// src/governance/enforcement.rs
use ipnet::IpNet;

pub struct SafetyController {
    allowed_cidrs: Vec<IpNet>,
    excluded_ips: Vec<std::net::IpAddr>,
    kill_switch_active: AtomicBool,
}

impl SafetyController {
    pub fn validate_target(&self, ip: std::net::IpAddr) -> Result<(), String> {
        if self.kill_switch_active.load(Ordering::SeqCst) {
            return Err("Kill switch is ACTIVE".to_string());
        }
        if self.excluded_ips.contains(&ip) {
            return Err("Target Blacklisted".to_string());
        }
        if !self.allowed_cidrs.iter().any(|net| net.contains(&ip)) {
            return Err("Target Out of Scope".to_string());
        }
        Ok(())
    }
}
```

---

## 6. Build & Deployment

### Compilation
WRAITH-Recon requires `libpcap` headers for the fallback capture mode.

```bash
# Install dependencies (Debian/Ubuntu)
sudo apt-get install libpcap-dev clang llvm

# Build
cargo build -p wraith-cli --bin wraith-recon --release
```

### Capabilities
The binary requires raw socket capabilities to function without root.

```bash
sudo setcap cap_net_raw,cap_net_admin,cap_sys_resource=eip target/release/wraith-recon
```