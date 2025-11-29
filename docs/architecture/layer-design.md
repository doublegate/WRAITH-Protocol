# WRAITH Protocol Layer Design

**Document Version:** 1.0.0
**Last Updated:** 2025-11-28
**Status:** Architecture Specification

---

## Overview

This document provides detailed specifications for each layer of the WRAITH protocol stack. Each layer has clearly defined responsibilities, interfaces, and failure modes.

---

## Layer 1: Network Layer

### Responsibilities

- Raw packet transmission and reception
- Protocol encapsulation (UDP/raw sockets/covert channels)
- MTU discovery and fragmentation avoidance
- Interface selection and binding

### Supported Transports

#### 1.1 UDP (Primary)

**Port Strategy:**
- Dynamic port allocation (ephemeral ports)
- Configurable listening port (default: random)
- Port hopping for enhanced privacy

**Characteristics:**
- Connection-less, no kernel state
- Low overhead (8-byte header)
- Firewall-friendly (common protocol)
- NAT traversal via hole punching

**Implementation:**
```rust
pub struct UdpTransport {
    socket: socket2::Socket,
    local_addr: SocketAddr,
    send_buffer_size: usize,  // SO_SNDBUF
    recv_buffer_size: usize,  // SO_RCVBUF
}

impl UdpTransport {
    pub async fn send_to(&self, data: &[u8], addr: SocketAddr) -> io::Result<usize>;
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)>;
}
```

#### 1.2 Raw Sockets (Advanced)

**Requirements:**
- CAP_NET_RAW capability or root privileges
- Custom IP/UDP headers
- Evasion of OS network stack

**Use Cases:**
- Custom TTL/TOS manipulation
- IP fragmentation control
- Protocol spoofing (ethical use only)

**Characteristics:**
- Bypass kernel UDP processing
- Full control over packet structure
- Higher CPU overhead (manual checksums)

#### 1.3 ICMP Covert Channel

**Technique:**
- Encode data in ICMP Echo Request/Reply
- Payload embedded in ping packet data field
- Appears as network diagnostics traffic

**Bandwidth:**
- ~50-200 bytes per packet
- Rate limited to avoid suspicion (1-10 pps)
- Suitable for control channel only

**Structure:**
```
ICMP Echo Request:
├─ Type: 8
├─ Code: 0
├─ Checksum: (calculated)
├─ Identifier: Random
├─ Sequence: Incrementing
└─ Data: Encrypted WRAITH frame
```

#### 1.4 DNS-over-HTTPS Tunnel

**Mechanism:**
- Base32-encode payload into subdomain
- Query TXT records from controlled nameserver
- Response contains encoded reply

**Query Format:**
```
<base32-payload>.tunnel.example.com TXT
```

**Bandwidth:**
- 100-500 bytes per query
- 10-50 queries/second typical
- Total: 1-25 KB/s sustained

**Advantages:**
- Works when UDP blocked
- Difficult to distinguish from legitimate DNS
- HTTPS-encrypted DNS queries (DoH)

**Disadvantages:**
- High latency (100-500ms per RTT)
- Low bandwidth
- Requires cooperative DNS server

---

## Layer 2: Kernel Acceleration Layer

### Responsibilities

- Zero-copy packet delivery to/from userspace
- Kernel bypass for hot path
- NUMA-aware memory allocation
- Batch packet processing

### 2.1 XDP/eBPF Programs

**Purpose:** Filter and redirect packets at NIC driver level before kernel stack.

**XDP Actions:**
- `XDP_DROP`: Discard packet (for non-protocol traffic)
- `XDP_PASS`: Pass to kernel stack (for unknown connections)
- `XDP_REDIRECT`: Send to AF_XDP socket (for known connections)
- `XDP_TX`: Bounce back to NIC (for reflection attacks defense)

**Connection Map:**
```c
// eBPF map: CID → Queue Index
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(key_size, 8);    // Connection ID
    __uint(value_size, 4);  // Queue index
    __uint(max_entries, 65536);
} connection_map SEC(".maps");
```

**Packet Processing:**
```c
SEC("xdp")
int wraith_filter(struct xdp_md *ctx) {
    // Parse Ethernet → IP → UDP → CID
    __u8 *cid = extract_cid(ctx);

    // Lookup connection
    __u32 *queue_idx = bpf_map_lookup_elem(&connection_map, cid);
    if (queue_idx) {
        return bpf_redirect_map(&xsk_map, *queue_idx, 0);
    }

    // Special handling for handshake CID (0xFFFFFFFFFFFFFFFF)
    if (is_handshake_cid(cid)) {
        return bpf_redirect_map(&xsk_map, 0, 0);  // Queue 0
    }

    return XDP_PASS;  // Unknown, let kernel handle
}
```

**Performance:**
- 26M packets/sec drop rate (single core)
- 24M packets/sec redirect rate (single core)
- <100ns processing time per packet
- Verified by kernel (memory-safe)

### 2.2 AF_XDP Sockets

**Architecture:**
```
┌───────────────────────────────────────────────────────────┐
│                          UMEM                             │
│     (Shared memory for DMA, 64MB typical)                 │
│  ┌────────┬────────┬────────┬────────┬────────┬────────┐  │
│  │ Frame 0│ Frame 1│ Frame 2│   ...  │Frame N │        │  │
│  └────────┴────────┴────────┴────────┴────────┴────────┘  │
└───────────────────────────────────────────────────────────┘
         ▲                                           │
         │                                           │
    ┌────┴────┐                                 ┌────▼────┐
    │  Fill   │                                 │  Comp   │
    │  Ring   │                                 │  Ring   │
    │(RX bufs)│                                 │(TX done)│
    └─────────┘                                 └─────────┘
         ▲                                           ▲
         │                                           │
    ┌────┴────┐                                 ┌────┴────┐
    │   RX    │                                 │   TX    │
    │  Ring   │                                 │  Ring   │
    │(packets)│                                 │(to send)│
    └─────────┘                                 └─────────┘
```

**Ring Buffer Operations:**

*Fill Ring* (Userspace → Kernel):
- Provides empty buffers for incoming packets
- Userspace produces, kernel consumes
- Pre-allocated at startup

*RX Ring* (Kernel → Userspace):
- Delivers received packets
- Kernel produces, userspace consumes
- Zero-copy (points into UMEM)

*TX Ring* (Userspace → Kernel):
- Submits packets for transmission
- Userspace produces, kernel consumes
- Zero-copy (points into UMEM)

*Completion Ring* (Kernel → Userspace):
- Notifies TX completion
- Kernel produces, userspace consumes
- Returns frame indices to free list

**Zero-Copy Semantics:**
- Packets never copied between kernel/userspace
- DMA directly into UMEM
- Userspace processes in-place
- Huge pages reduce TLB pressure

**Batching:**
```rust
// Batch receive up to 64 packets
let mut batch = Vec::with_capacity(64);
socket.recv_batch(&mut batch, 64)?;

for packet in batch {
    process_packet(packet);
}

// Batch transmit
let outgoing: Vec<&[u8]> = /* ... */;
socket.send_batch(&outgoing)?;
```

### 2.3 io_uring for File I/O

**Purpose:** Async file operations with zero-copy and batching.

**Operations:**
- `IORING_OP_READ`: Read file chunk
- `IORING_OP_WRITE`: Write file chunk
- `IORING_OP_FSYNC`: Flush to disk
- `IORING_OP_SEND_ZC`: Zero-copy send (kernel 6.0+)
- `IORING_OP_RECV_MULTISHOT`: Continuous receive

**Submission Queue (SQ):**
```rust
// Build SQE (Submission Queue Entry)
let read_sqe = opcode::Read::new(
    types::Fd(file.as_raw_fd()),
    buffer_ptr,
    chunk_size as u32,
)
.offset(file_offset)
.build()
.user_data(user_data_id);

// Submit to ring
ring.submission().push(&read_sqe)?;
ring.submit()?;
```

**Completion Queue (CQ):**
```rust
// Wait for completion
let cqe = ring.completion().next().unwrap();
let result = cqe.result();
let user_data = cqe.user_data();

// Handle result
match callbacks.remove(&user_data) {
    Some(callback) => callback(result),
    None => warn!("Orphaned completion"),
}
```

**Registered Buffers:**
- Pre-register buffer pool with kernel
- Reduces syscall overhead
- Faster I/O submission

**SQE Linking:**
```rust
// Chain dependent operations
let read_sqe = opcode::Read::new(...)
    .build()
    .flags(IoringFlag::IO_LINK);  // Link to next

let write_sqe = opcode::Write::new(...)
    .build();  // Executes only if read succeeds

ring.submission().push(&read_sqe)?;
ring.submission().push(&write_sqe)?;
```

---

## Layer 3: Obfuscation Layer

### Responsibilities

- Traffic indistinguishability
- Censorship resistance
- Timing obfuscation
- Protocol mimicry

### 3.1 Elligator2 Key Encoding

**Purpose:** Make Curve25519 public keys indistinguishable from random.

**Algorithm:**
1. Generate X25519 keypair
2. Convert Montgomery point to Edwards form
3. Add random low-order component (8 cosets)
4. Attempt Elligator2 inverse map
5. If encodable, randomize high bit
6. If not encodable, regenerate (50% success rate)

**Decoding:**
```rust
fn decode_representative(repr: &[u8; 32]) -> PublicKey {
    let mut clean = *repr;
    clean[31] &= 0x7F;  // Clear high bit

    let edwards = elligator2_forward(&clean);
    let montgomery = edwards.to_montgomery();

    PublicKey::from(montgomery)
}
```

**Security:**
- Representatives uniformly distributed
- No distinguisher with <2^128 queries
- Twofold advantage: hide keys AND valid ciphertext

### 3.2 Packet Padding

**Padding Classes:**
| Class | Size | Use Case |
|-------|------|----------|
| Tiny | 64 B | ACKs, control frames |
| Small | 256 B | Handshakes, small metadata |
| Medium | 512 B | Small chunks |
| Large | 1024 B | Typical data |
| MTU | 1472 B | Maximum efficiency |
| Jumbo | 8960 B | High-throughput LANs |

**Selection Algorithm:**
```rust
pub enum PaddingMode {
    Performance,  // Minimal padding (next size class)
    Privacy,      // Random selection from valid classes
    Stealth,      // Match target protocol distribution
}

fn select_padding(payload_len: usize, mode: PaddingMode) -> usize {
    match mode {
        PaddingMode::Performance => {
            CLASSES.iter()
                .find(|&&s| s >= payload_len + OVERHEAD)
                .copied()
                .unwrap_or(JUMBO_SIZE)
        }
        PaddingMode::Privacy => {
            let valid: Vec<_> = CLASSES.iter()
                .filter(|&&s| s >= payload_len + OVERHEAD)
                .copied()
                .collect();
            valid[random() % valid.len()]
        }
        PaddingMode::Stealth => {
            https_packet_size_distribution()
        }
    }
}
```

**Padding Content:**
```rust
// Deterministically random padding from session key
let padding_key = HKDF(session_key, "padding", 32);
let padding_stream = ChaCha20::new(padding_key, packet_nonce);
let padding = padding_stream.generate(padding_len);
```

### 3.3 Timing Obfuscation

**Inter-Packet Delay:**
```rust
fn calculate_delay(mode: TimingMode, last_send: Instant) -> Duration {
    match mode {
        TimingMode::LowLatency => Duration::ZERO,

        TimingMode::Moderate => {
            // Exponential distribution (mean 5ms)
            let u = random::<f64>();
            let delay_ms = -5.0 * u.ln();
            Duration::from_secs_f64(delay_ms / 1000.0)
        }

        TimingMode::HighPrivacy => {
            // Match HTTPS timing distribution
            sample_https_timing()
        }
    }
}
```

**Burst Shaping:**
```rust
struct BurstShaper {
    target_rate: u64,      // bytes/sec
    window_size: Duration, // 100ms
    sent_in_window: u64,
    window_start: Instant,
}

impl BurstShaper {
    fn should_queue(&mut self, packet_size: usize) -> bool {
        // Reset window if expired
        if self.window_start.elapsed() >= self.window_size {
            self.sent_in_window = 0;
            self.window_start = Instant::now();
        }

        let threshold = (self.target_rate * self.window_size.as_millis() as u64) / 1000;

        if self.sent_in_window + packet_size as u64 > threshold * 3 / 2 {
            return true;  // Queue excess
        }

        self.sent_in_window += packet_size as u64;
        false
    }
}
```

### 3.4 Cover Traffic

**PAD Frame Generation:**
```rust
struct CoverTrafficGenerator {
    min_rate_pps: u32,     // Minimum packets/sec (default: 10)
    max_idle: Duration,    // Maximum gap (default: 100ms)
    last_send: Instant,
}

impl CoverTrafficGenerator {
    fn should_generate(&self, has_real_data: bool) -> bool {
        if has_real_data {
            return false;  // Real data has priority
        }

        // Mandatory after idle timeout
        if self.last_send.elapsed() > self.max_idle {
            return true;
        }

        // Probabilistic based on rate
        let elapsed = self.last_send.elapsed().as_secs_f64();
        let p = elapsed * self.min_rate_pps as f64;
        random::<f64>() < p
    }

    fn generate_pad_frame(&self) -> Frame {
        let size = random_range(64, 256);
        Frame {
            frame_type: FrameType::PAD,
            payload: random_bytes(size),
            ..Default::default()
        }
    }
}
```

### 3.5 Protocol Mimicry

#### HTTPS/TLS Wrapper

**TLS Record Format:**
```
TLS Application Data Record:
├─ Content Type: 0x17 (Application Data)
├─ Legacy Version: 0x0303 (TLS 1.2)
├─ Length: u16 (record length)
└─ Encrypted WRAITH Frame
```

**Handshake Simulation:**
- Initial packets mimic TLS ClientHello/ServerHello
- Random SNI (server name indication)
- Valid cipher suites
- TLS session ID

**Implementation:**
```rust
fn wrap_as_tls(frame: &[u8]) -> Vec<u8> {
    let mut record = Vec::new();
    record.push(0x17);  // Application Data
    record.extend_from_slice(&[0x03, 0x03]);  // TLS 1.2
    record.extend_from_slice(&(frame.len() as u16).to_be_bytes());
    record.extend_from_slice(frame);
    record
}
```

#### WebSocket Wrapper

**Frame Format:**
```
WebSocket Binary Frame:
├─ FIN=1, RSV=000, Opcode=0x2 (binary)
├─ MASK=1, Payload Length (1-9 bytes)
├─ Masking Key (4 bytes, random)
└─ Masked WRAITH Frame (XOR with key)
```

**Masking:**
```rust
fn mask_websocket(payload: &[u8], mask: [u8; 4]) -> Vec<u8> {
    payload.iter()
        .enumerate()
        .map(|(i, &b)| b ^ mask[i % 4])
        .collect()
}
```

#### DNS-over-HTTPS Covert Channel

**Query Construction:**
```rust
fn encode_doh_query(payload: &[u8]) -> String {
    let encoded = base32::encode(payload);
    format!("{}.tunnel.example.com", encoded)
}

fn send_doh_query(domain: &str) -> reqwest::Result<Vec<u8>> {
    let client = reqwest::Client::new();
    let dns_message = build_dns_query(domain, RecordType::TXT);

    client.post("https://1.1.1.1/dns-query")
        .header("Content-Type", "application/dns-message")
        .body(dns_message)
        .send()?
        .bytes()
}
```

---

## Layer 4: Cryptographic Transport

### Responsibilities

- Authenticated encryption
- Key establishment and ratcheting
- Replay protection
- Forward secrecy

### 4.1 Noise_XX Handshake

**Pattern:**
```
-> e
<- e, ee, s, es
-> s, se
```

**Phase 1 (Initiator → Responder):**
```rust
struct Phase1Message {
    cid: [u8; 8],           // 0xFFFFFFFFFFFFFFFF
    version: u32,            // Protocol version
    timestamp: u64,          // Unix µs (anti-replay)
    ephemeral_key: [u8; 32], // Elligator2-encoded
    padding: [u8; 28],       // Random
    mac: [u8; 16],           // BLAKE3 MAC
}
// Total: 96 bytes
```

**Phase 2 (Responder → Initiator):**
```rust
struct Phase2Message {
    cid: [u8; 8],           // Derived from ee
    ephemeral_key: [u8; 32],// Elligator2-encoded
    encrypted_payload: [    // XChaCha20-Poly1305
        static_key: [u8; 32],
        timestamp_echo: u64,
        cipher_suite: u32,
        padding: [u8; 12],
        auth_tag: [u8; 16],
    ],
    mac: [u8; 16],
}
// Total: 128 bytes
```

**Phase 3 (Initiator → Responder):**
```rust
struct Phase3Message {
    cid: [u8; 8],
    encrypted_payload: [
        static_key: [u8; 32],
        session_params: u64,
        auth_tag: [u8; 16],
    ],
    mac: [u8; 16],
}
// Total: 80 bytes
```

**Key Derivation:**
```rust
// Input keying material
let ikm = [
    DH(ie, re),  // 32 bytes
    DH(ie, rs),  // 32 bytes
    DH(is, re),  // 32 bytes
    DH(is, rs),  // 32 bytes
]; // Total: 128 bytes

// Extract
let prk = HKDF_Extract(salt: "wraith-v1", ikm);

// Expand
let initiator_tx_key = HKDF_Expand(prk, "i2r-data", 32);
let responder_tx_key = HKDF_Expand(prk, "r2i-data", 32);
let initiator_nonce_salt = HKDF_Expand(prk, "i2r-nonce", 4);
let responder_nonce_salt = HKDF_Expand(prk, "r2i-nonce", 4);
let connection_id = HKDF_Expand(prk, "conn-id", 8);
```

### 4.2 AEAD Encryption

**XChaCha20-Poly1305:**
- **Nonce:** 192 bits (never reused, no counter limit)
- **Key:** 256 bits (derived from handshake)
- **Tag:** 128 bits (authentication)

**Nonce Construction:**
```
Full Nonce (192 bits):
├─ Zero Padding: [0; 16]
└─ Protocol Nonce: [session_salt: 4B | packet_counter: 4B]
```

**Encryption:**
```rust
fn encrypt_frame(frame: &[u8], key: &[u8; 32], nonce: &[u8; 24]) -> Vec<u8> {
    let cipher = XChaCha20Poly1305::new(key.into());
    cipher.encrypt(nonce.into(), frame)
        .expect("Encryption failed")
}
```

**Counter Overflow Protection:**
```rust
if packet_counter >= u32::MAX - (1 << 20) {
    // 1M packets headroom
    initiate_rekey();
}
```

### 4.3 Key Ratcheting

**Symmetric Ratchet (Every Packet):**
```rust
// After sending/receiving
chain_key_next = BLAKE3(chain_key_current || 0x01);
message_key = BLAKE3(chain_key_current || 0x02);

// Use message_key for AEAD, then zeroize
encrypt_with(message_key);
zeroize(message_key);
zeroize(chain_key_current);
```

**DH Ratchet (Time/Volume Triggered):**
```rust
fn should_rekey(&self) -> bool {
    self.packets_since_rekey >= 1_000_000
        || self.last_rekey.elapsed() >= Duration::from_secs(120)
}

fn perform_rekey(&mut self) -> Result<Vec<u8>> {
    // Generate new ephemeral key
    let new_ephemeral = EphemeralSecret::new(OsRng);
    let new_public = PublicKey::from(&new_ephemeral);

    // Perform DH with peer's ephemeral
    let new_dh = new_ephemeral.diffie_hellman(&self.peer_ephemeral);

    // Derive new chain key
    let new_chain = HKDF(
        self.chain_key || new_dh.as_bytes(),
        "rekey",
        32
    );

    // Update state
    zeroize(&mut self.chain_key);
    self.chain_key = new_chain;
    self.local_ephemeral = new_ephemeral;
    self.packets_since_rekey = 0;
    self.last_rekey = Instant::now();

    // Build REKEY frame
    Ok(build_rekey_frame(new_public))
}
```

**REKEY Frame:**
```rust
struct RekeyFrame {
    new_ephemeral: [u8; 32],  // Elligator2-encoded
    ratchet_sequence: u32,     // Monotonic counter
    auth_tag: [u8; 16],        // BLAKE3-HMAC
}
```

---

## Layer 5: Session Layer

### Responsibilities

- Connection lifecycle management
- Stream multiplexing
- Flow control
- Congestion control
- Loss detection and recovery

### 5.1 Session States

```
State Machine:
CLOSED
  ↓ connect() / accept()
HANDSHAKING (3-phase Noise_XX)
  ↓ handshake complete
ESTABLISHED
  ↓ REKEY trigger
REKEYING
  ↓ rekey complete
ESTABLISHED
  ↓ CLOSE sent/received
DRAINING (allow pending data)
  ↓ timeout
CLOSED
```

**State Transitions:**
```rust
pub enum SessionState {
    Closed,
    Handshaking(HandshakePhase),
    Established,
    Rekeying,
    Draining,
    Migrating,  // Path validation in progress
}

pub enum HandshakePhase {
    InitSent,      // Initiator sent Phase 1
    RespSent,      // Responder sent Phase 2
    InitComplete,  // Initiator sent Phase 3
}
```

### 5.2 Stream Multiplexing

**Stream ID Allocation:**
| Range | Initiator | Direction | Type |
|-------|-----------|-----------|------|
| 0x0000 | N/A | Session-level | Control |
| 0x0001-0x3FFF | Client | C→S | Normal |
| 0x4000-0x7FFF | Server | S→C | Normal |
| 0x8000-0xBFFF | Client | C→S | Expedited |
| 0xC000-0xFFFF | Server | S→C | Expedited |

**Stream Object:**
```rust
pub struct Stream {
    stream_id: u16,
    state: StreamState,

    // Transmit state
    send_window: u64,      // Flow control credit
    send_buffer: VecDeque<Chunk>,
    next_seq: u32,

    // Receive state
    recv_window: u64,
    recv_buffer: BTreeMap<u64, Vec<u8>>,  // offset → data
    largest_received: u32,

    // Statistics
    bytes_sent: u64,
    bytes_received: u64,
}

pub enum StreamState {
    Idle,
    Open,
    HalfClosedLocal,   // FIN sent
    HalfClosedRemote,  // FIN received
    Closed,
}
```

**Stream Lifecycle:**
```rust
// Open stream
let stream_id = session.open_stream()?;

// Send data
session.stream_write(stream_id, &data, offset)?;

// Close stream
session.stream_close(stream_id)?;
```

### 5.3 Flow Control

**Window Updates:**
```rust
// Receiver advertises credit
let window_update = Frame {
    frame_type: FrameType::WindowUpdate,
    stream_id,
    payload: [
        max_stream_data: u64,  // Per-stream window
        max_data: u64,          // Connection-level window
    ],
};
```

**Sender Enforcement:**
```rust
fn can_send(&self, stream_id: u16, size: usize) -> bool {
    let stream = &self.streams[stream_id];

    // Check stream-level window
    if stream.bytes_sent + size as u64 > stream.send_window {
        return false;
    }

    // Check connection-level window
    if self.total_bytes_sent + size as u64 > self.max_data {
        return false;
    }

    true
}
```

**Auto-Tuning:**
```rust
// Increase window based on receive rate
if receiver_buffer_occupancy < 0.5 {
    new_window = current_window * 2;
} else {
    new_window = current_window + mss;
}
new_window = new_window.min(MAX_WINDOW);
```

### 5.4 Congestion Control (BBRv2)

**BBR State Machine:**
```
STARTUP (exponential growth)
  ↓ bandwidth plateau
DRAIN (reduce in-flight to BDP)
  ↓ in-flight ≤ BDP
PROBE_BW (steady state, 8-phase cycle)
  ↓ every 10 seconds
PROBE_RTT (measure min RTT)
  ↓ RTT measured
PROBE_BW (resume)
```

**Key Metrics:**
```rust
pub struct BbrState {
    btl_bw: u64,           // Bottleneck bandwidth (bytes/sec)
    min_rtt: Duration,     // Minimum RTT observed
    pacing_gain: f64,      // Multiplier for pacing rate
    cwnd_gain: f64,        // Multiplier for congestion window
    round_count: u64,      // Round-trip counter
    cycle_index: usize,    // PROBE_BW phase (0-7)
}
```

**Bandwidth Estimation:**
```rust
fn update_bandwidth(&mut self, ack: &AckInfo) {
    let delivery_rate = ack.bytes_acked as f64 / ack.ack_elapsed.as_secs_f64();

    // Max filter over 10 RTTs
    if delivery_rate > self.btl_bw as f64 {
        self.btl_bw = delivery_rate as u64;
        self.bw_filter_expires = now + self.min_rtt * 10;
    }

    // Expire old samples
    if now > self.bw_filter_expires {
        self.btl_bw = (self.btl_bw as f64 * 0.9) as u64;  // Decay
    }
}
```

**Pacing:**
```rust
fn pacing_rate(&self) -> u64 {
    (self.btl_bw as f64 * self.pacing_gain) as u64
}

fn schedule_next_send(&self, packet_size: usize) -> Instant {
    let rate = self.pacing_rate();
    let interval = packet_size as f64 / rate as f64;
    self.last_send + Duration::from_secs_f64(interval)
}
```

**PROBE_BW Phases:**
```rust
const PROBE_BW_GAINS: [f64; 8] = [
    1.25,  // Phase 0: Probe UP
    0.75,  // Phase 1: Drain
    1.0,   // Phase 2-5: Cruise
    1.0,
    1.0,
    1.0,
    1.0,   // Phase 6: Refill
    1.0,   // Phase 7: Cruise
];

fn advance_probe_bw_phase(&mut self) {
    self.cycle_index = (self.cycle_index + 1) % 8;
    self.pacing_gain = PROBE_BW_GAINS[self.cycle_index];
}
```

### 5.5 Loss Detection

**Mechanisms:**
1. **Time Threshold:** Packet lost if not ACKed within 1.5× smoothed RTT
2. **Packet Threshold:** Lost if 3+ later packets ACKed (reordering tolerance)
3. **PTO (Probe Timeout):** Send probe if no ACK within timeout

**RTT Estimation:**
```rust
fn update_rtt(&mut self, measured_rtt: Duration) {
    if self.smoothed_rtt == Duration::ZERO {
        // First measurement
        self.smoothed_rtt = measured_rtt;
        self.rtt_var = measured_rtt / 2;
    } else {
        // EWMA with alpha = 1/8
        let delta = (measured_rtt.as_millis() as i64
                    - self.smoothed_rtt.as_millis() as i64).abs() as u64;
        self.rtt_var = self.rtt_var * 3 / 4 + Duration::from_millis(delta / 4);
        self.smoothed_rtt = self.smoothed_rtt * 7 / 8 + measured_rtt / 8;
    }
}

fn pto_duration(&self) -> Duration {
    self.smoothed_rtt
        + max(4 * self.rtt_var, Duration::from_millis(1))
        + self.max_ack_delay
}
```

**Loss Detection:**
```rust
fn detect_losses(&self, largest_acked: u32) -> Vec<u32> {
    let time_threshold = self.smoothed_rtt * 3 / 2;
    let packet_threshold = 3;

    let mut lost = Vec::new();

    for (&seq, packet) in &self.unacked {
        // Time-based
        if packet.sent_time.elapsed() > time_threshold {
            lost.push(seq);
            continue;
        }

        // Packet-based (reordering tolerance)
        if largest_acked >= seq + packet_threshold {
            lost.push(seq);
        }
    }

    lost
}
```

---

## Layer 6: Application Layer

### Responsibilities

- File chunking and reassembly
- Integrity verification
- Transfer management
- Progress reporting

### 6.1 File Chunking

**Chunk Size:**
- Default: 256 KiB (262,144 bytes)
- Rationale: Balance between overhead and parallelism
- Configurable based on file size

**Chunker:**
```rust
pub struct FileChunker {
    file: File,
    chunk_size: usize,
    total_size: u64,
    current_offset: u64,
}

impl FileChunker {
    pub fn next_chunk(&mut self) -> io::Result<Option<Chunk>> {
        if self.current_offset >= self.total_size {
            return Ok(None);
        }

        let remaining = self.total_size - self.current_offset;
        let size = remaining.min(self.chunk_size as u64) as usize;

        let mut buffer = vec![0u8; size];
        self.file.read_exact(&mut buffer)?;

        let hash = BLAKE3::hash(&buffer);

        let chunk = Chunk {
            offset: self.current_offset,
            data: buffer,
            hash: hash.as_bytes()[..16].try_into().unwrap(),
        };

        self.current_offset += size as u64;

        Ok(Some(chunk))
    }
}
```

### 6.2 BLAKE3 Tree Hashing

**Per-Chunk Hash:**
```rust
let chunk_hash = BLAKE3::hash(&chunk_data);
let truncated = &chunk_hash.as_bytes()[..16];  // First 128 bits
```

**Root Hash (Merkle Tree):**
```rust
pub struct FileHasher {
    hasher: blake3::Hasher,
}

impl FileHasher {
    pub fn update_chunk(&mut self, chunk: &[u8]) {
        self.hasher.update(chunk);
    }

    pub fn finalize(self) -> [u8; 32] {
        self.hasher.finalize().into()
    }
}
```

**Verification:**
```rust
fn verify_chunk(chunk: &Chunk) -> bool {
    let computed = BLAKE3::hash(&chunk.data);
    let truncated = &computed.as_bytes()[..16];
    truncated == chunk.hash
}
```

### 6.3 Transfer State Machine

```
IDLE
  ↓ initiate_transfer()
REQUESTING (send file metadata)
  ↓ peer accepts
TRANSFERRING (send chunks)
  ↓ all chunks sent
COMPLETING (wait for final ACKs)
  ↓ all ACKed
COMPLETED
```

**State Object:**
```rust
pub struct Transfer {
    transfer_id: Uuid,
    file_path: PathBuf,
    file_size: u64,
    chunk_size: usize,

    // Progress tracking
    chunks_sent: BitVec,
    chunks_acked: BitVec,
    bytes_sent: u64,
    bytes_acked: u64,

    // Timestamps
    started_at: Instant,
    completed_at: Option<Instant>,

    // Statistics
    throughput_samples: VecDeque<(Instant, u64)>,
    rtt_samples: VecDeque<Duration>,
}

impl Transfer {
    pub fn progress(&self) -> f64 {
        self.bytes_acked as f64 / self.file_size as f64
    }

    pub fn estimated_completion(&self) -> Duration {
        let elapsed = self.started_at.elapsed();
        let progress = self.progress();

        if progress > 0.01 {
            Duration::from_secs_f64(elapsed.as_secs_f64() / progress)
        } else {
            Duration::from_secs(u64::MAX)  // Unknown
        }
    }

    pub fn current_throughput(&self) -> u64 {
        // Bytes/sec over last 5 seconds
        let cutoff = Instant::now() - Duration::from_secs(5);
        let recent: u64 = self.throughput_samples.iter()
            .filter(|(ts, _)| *ts > cutoff)
            .map(|(_, bytes)| bytes)
            .sum();

        recent / 5
    }
}
```

---

## Inter-Layer Communication

### API Boundaries

**Application ↔ Session:**
```rust
pub trait SessionApi {
    fn open_stream(&mut self) -> Result<StreamId>;
    fn stream_write(&mut self, id: StreamId, data: &[u8], offset: u64) -> Result<()>;
    fn stream_close(&mut self, id: StreamId) -> Result<()>;
}
```

**Session ↔ Crypto:**
```rust
pub trait CryptoApi {
    fn encrypt_frame(&mut self, frame: &[u8]) -> Result<Vec<u8>>;
    fn decrypt_frame(&mut self, ciphertext: &[u8]) -> Result<Vec<u8>>;
    fn needs_rekey(&self) -> bool;
}
```

**Crypto ↔ Obfuscation:**
```rust
pub trait ObfuscationApi {
    fn apply_padding(&self, ciphertext: &[u8], mode: PaddingMode) -> Vec<u8>;
    fn generate_cover_traffic(&self) -> Option<Vec<u8>>;
}
```

**Obfuscation ↔ Transport:**
```rust
pub trait TransportApi {
    fn send_packet(&mut self, data: &[u8], addr: SocketAddr) -> Result<()>;
    fn recv_packet(&mut self) -> Result<(Vec<u8>, SocketAddr)>;
}
```

---

## Error Handling

### Error Propagation

Each layer defines error types that map to higher-level errors:

```rust
pub enum LayerError {
    Network(NetworkError),
    Kernel(KernelError),
    Obfuscation(ObfuscationError),
    Crypto(CryptoError),
    Session(SessionError),
    Application(ApplicationError),
}
```

### Recovery Strategies

| Error Type | Layer | Recovery |
|------------|-------|----------|
| Packet loss | Session | Retransmission |
| Decryption failure | Crypto | Drop packet, request retransmit |
| Connection timeout | Session | Attempt reconnect |
| File I/O error | Application | Pause transfer, notify user |
| UMEM exhaustion | Kernel | Apply backpressure |
| XDP program load failure | Kernel | Fallback to UDP sockets |

---

## Performance Considerations

### Batching

All layers support batch operations:
- Network: Send/recv up to 64 packets
- Kernel: io_uring batch submission
- Crypto: Vectorized SIMD operations
- Session: Batch ACK generation

### Memory Management

- **Zero-Copy:** Minimize data copying (UMEM, registered buffers)
- **NUMA-Aware:** Allocate memory on local node
- **Huge Pages:** Reduce TLB misses
- **Object Pooling:** Reuse packet buffers

### CPU Optimization

- **Thread-per-Core:** No locks in hot path
- **CPU Pinning:** Prevent context switching
- **SIMD:** ChaCha20 vectorized on x86_64
- **Cache Awareness:** Pack hot data structures

---

## Conclusion

This layered architecture provides clear separation of concerns, enabling:
- Independent testing and optimization
- Flexible deployment (with/without kernel bypass)
- Protocol evolution without breaking compatibility
- Clear security boundaries

Each layer can be implemented and verified independently, facilitating parallel development and rigorous testing.

---

**See Also:**
- [Protocol Overview](protocol-overview.md)
- [Security Model](security-model.md)
- [Performance Architecture](performance-architecture.md)
