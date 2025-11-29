# Phase 1: Foundation Sprint Planning

**Duration:** 4-6 weeks
**Story Points:** 89
**Status:** Not Started
**Risk Level:** Low

---

## Phase Goals

Establish the foundational protocol types, frame encoding/decoding, session management, and testing infrastructure. This phase creates the core abstractions that all subsequent phases will build upon.

### Success Criteria
- [ ] Frame parsing benchmarks: >1M frames/sec
- [ ] Zero-copy frame parsing validated
- [ ] All frame types (15 types) encodable/decodable
- [ ] Session state transitions validated
- [ ] Stream multiplexing functional
- [ ] Test coverage >80%
- [ ] CI/CD pipeline operational

---

## Sprint 1.1: Project Setup & Core Types (Week 1)

**Story Points:** 13

### Tasks

#### Task 1.1.1: Project Structure (SP: 3)
**Description:** Set up Rust workspace with all crates.

**Implementation:**
```bash
# Create workspace structure
mkdir -p crates/{wraith-core,wraith-crypto,wraith-transport,wraith-obfuscation,wraith-discovery,wraith-files,wraith-cli,wraith-xdp}
mkdir -p {xtask/src,tests,benches,.github/workflows}
```

**Deliverables:**
- [ ] Workspace Cargo.toml with members
- [ ] Per-crate Cargo.toml files
- [ ] README.md for each crate
- [ ] LICENSE files (MIT/Apache-2.0 dual)
- [ ] .gitignore configured

**Acceptance Criteria:**
- `cargo build --workspace` succeeds
- `cargo test --workspace` succeeds
- All crates have consistent versioning (0.1.0)

**Estimated Time:** 4-6 hours

---

#### Task 1.1.2: Dependency Configuration (SP: 2)
**Description:** Configure workspace dependencies.

**Key Dependencies:**
```toml
[workspace.dependencies]
# Async
tokio = { version = "1.35", features = ["full"] }

# Crypto (placeholders for Phase 2)
chacha20poly1305 = "0.10"
x25519-dalek = "2.0"
blake3 = "1.5"
rand = "0.8"
zeroize = { version = "1.7", features = ["derive"] }

# Serialization
bincode = "1.3"
serde = { version = "1.0", features = ["derive"] }

# CLI
clap = { version = "4.4", features = ["derive"] }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Testing
proptest = "1.4"
criterion = "0.5"
```

**Deliverables:**
- [ ] Workspace dependencies configured
- [ ] Consistent feature flags across workspace
- [ ] Dev dependencies for testing

**Acceptance Criteria:**
- All dependencies resolve
- `cargo check --workspace` succeeds
- No duplicate dependencies (check with `cargo tree`)

**Estimated Time:** 2-3 hours

---

#### Task 1.1.3: Error Type Hierarchy (SP: 3)
**Description:** Define protocol-wide error types.

**Implementation:**
```rust
// wraith-core/src/error.rs

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("frame parsing error: {0}")]
    Frame(#[from] FrameError),

    #[error("session error: {0}")]
    Session(#[from] SessionError),

    #[error("stream error: {0}")]
    Stream(#[from] StreamError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("timeout")]
    Timeout,

    #[error("connection closed")]
    ConnectionClosed,
}

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("frame too short: expected at least {expected}, got {actual}")]
    TooShort { expected: usize, actual: usize },

    #[error("invalid frame type: 0x{0:02X}")]
    InvalidFrameType(u8),

    #[error("payload length exceeds packet size")]
    PayloadOverflow,

    #[error("invalid padding")]
    InvalidPadding,
}

// ... SessionError, StreamError ...
```

**Deliverables:**
- [ ] ProtocolError enum
- [ ] FrameError enum
- [ ] SessionError enum
- [ ] StreamError enum
- [ ] Error conversion implementations

**Acceptance Criteria:**
- All errors implement `std::error::Error`
- Error messages are descriptive
- Error conversions work via `?` operator
- Unit tests for error construction

**Estimated Time:** 4-5 hours

---

#### Task 1.1.4: Protocol Constants (SP: 2)
**Description:** Define protocol constants and limits.

**Implementation:**
```rust
// wraith-core/src/constants.rs

pub mod frame {
    /// Fixed frame header size in bytes
    pub const HEADER_SIZE: usize = 28;

    /// AEAD authentication tag size
    pub const AUTH_TAG_SIZE: usize = 16;

    /// Connection ID size
    pub const CONNECTION_ID_SIZE: usize = 8;

    /// Maximum payload for standard MTU
    pub const MAX_PAYLOAD_STANDARD: usize = 1428;

    /// Maximum payload for jumbo frames
    pub const MAX_PAYLOAD_JUMBO: usize = 8928;
}

pub mod session {
    use std::time::Duration;

    /// Maximum concurrent streams per session
    pub const MAX_STREAMS: u16 = 16384;

    /// Initial flow control window
    pub const INITIAL_WINDOW: u64 = 1_048_576; // 1 MiB

    /// Maximum flow control window
    pub const MAX_WINDOW: u64 = 16_777_216; // 16 MiB

    /// Idle timeout before connection close
    pub const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

    /// Handshake timeout
    pub const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);

    /// Rekey interval for forward secrecy
    pub const REKEY_INTERVAL: Duration = Duration::from_secs(120);

    /// Maximum packets before mandatory rekey
    pub const REKEY_PACKET_LIMIT: u64 = 1_000_000;
}

pub mod limits {
    /// Maximum packet reordering before loss detection
    pub const REORDER_THRESHOLD: u32 = 3;

    /// Maximum consecutive PTOs before connection close
    pub const MAX_PTO_COUNT: u32 = 5;

    /// Default file chunk size
    pub const DEFAULT_CHUNK_SIZE: usize = 262144; // 256 KiB
}
```

**Deliverables:**
- [ ] Frame constants module
- [ ] Session constants module
- [ ] Limits module
- [ ] Timing constants module
- [ ] Size constants module

**Acceptance Criteria:**
- All constants documented with rationale
- Constants grouped logically
- No magic numbers in code (use constants)

**Estimated Time:** 2-3 hours

---

#### Task 1.1.5: Logging Infrastructure (SP: 3)
**Description:** Set up structured logging with tracing.

**Implementation:**
```rust
// wraith-core/src/logging.rs

use tracing::{Level, Span};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init_logging(level: Option<Level>) {
    let filter = if let Some(level) = level {
        EnvFilter::default()
            .add_directive(level.into())
    } else {
        EnvFilter::from_default_env()
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}

// Usage throughout codebase:
use tracing::{debug, info, warn, error, trace, instrument};

#[instrument(skip(data))]
pub fn process_frame(data: &[u8]) -> Result<Frame> {
    trace!("Processing frame of {} bytes", data.len());
    // ...
}
```

**Deliverables:**
- [ ] Logging initialization function
- [ ] Environment variable configuration
- [ ] Log level filtering
- [ ] Structured fields for key events
- [ ] Logging best practices doc

**Acceptance Criteria:**
- Logging works in tests and binaries
- Log levels configurable via RUST_LOG
- Performance impact <1% (release builds)
- Structured data easily parseable

**Estimated Time:** 3-4 hours

---

### Sprint 1.1 Definition of Done
- [x] All tasks completed
- [x] Code compiles without warnings
- [x] Basic tests pass (`cargo test`)
- [x] Documentation builds (`cargo doc`)
- [x] CI pipeline running (lint, test, build)

---

## Sprint 1.2: Frame Encoding/Decoding (Week 2)

**Story Points:** 21

### Tasks

#### Task 1.2.1: Frame Type Enum (SP: 2)
**Description:** Define all protocol frame types.

**Implementation:**
```rust
// wraith-core/src/frame.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrameType {
    Reserved = 0x00,
    Data = 0x01,
    Ack = 0x02,
    Control = 0x03,
    Rekey = 0x04,
    Ping = 0x05,
    Pong = 0x06,
    Close = 0x07,
    Pad = 0x08,
    StreamOpen = 0x09,
    StreamClose = 0x0A,
    StreamReset = 0x0B,
    WindowUpdate = 0x0C,
    GoAway = 0x0D,
    PathChallenge = 0x0E,
    PathResponse = 0x0F,
}

impl TryFrom<u8> for FrameType {
    type Error = FrameError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Err(FrameError::ReservedFrameType),
            0x01 => Ok(Self::Data),
            0x02 => Ok(Self::Ack),
            // ... (all types)
            _ => Err(FrameError::InvalidFrameType(value)),
        }
    }
}
```

**Deliverables:**
- [ ] FrameType enum (16 variants)
- [ ] TryFrom<u8> implementation
- [ ] Display/Debug implementations
- [ ] Serde support (optional)

**Acceptance Criteria:**
- All 16 frame types defined
- Invalid types rejected
- Conversions tested

**Estimated Time:** 2 hours

---

#### Task 1.2.2: Frame Flags (SP: 2)
**Description:** Implement frame flags bitmap.

**Implementation:**
```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameFlags(u8);

impl FrameFlags {
    pub const SYN: u8 = 0b0000_0001;
    pub const FIN: u8 = 0b0000_0010;
    pub const ACK: u8 = 0b0000_0100;
    pub const PRI: u8 = 0b0000_1000;
    pub const CMP: u8 = 0b0001_0000;

    pub fn new() -> Self {
        Self(0)
    }

    pub fn with_syn(mut self) -> Self {
        self.0 |= Self::SYN;
        self
    }

    pub fn with_fin(mut self) -> Self {
        self.0 |= Self::FIN;
        self
    }

    pub fn is_syn(&self) -> bool {
        self.0 & Self::SYN != 0
    }

    pub fn is_fin(&self) -> bool {
        self.0 & Self::FIN != 0
    }

    pub fn is_compressed(&self) -> bool {
        self.0 & Self::CMP != 0
    }
}
```

**Deliverables:**
- [ ] FrameFlags struct
- [ ] Bit flag constants
- [ ] Builder methods (with_*)
- [ ] Query methods (is_*)
- [ ] Tests for all flag combinations

**Acceptance Criteria:**
- All flags work independently
- Flags can be combined
- Query methods accurate
- Zero-size when default

**Estimated Time:** 2-3 hours

---

#### Task 1.2.3: Frame Header Parsing (SP: 5)
**Description:** Zero-copy frame header parsing.

**Implementation:**
```rust
#[repr(C, packed)]
struct RawFrameHeader {
    nonce: [u8; 8],
    frame_type: u8,
    flags: u8,
    stream_id: [u8; 2],
    sequence: [u8; 4],
    offset: [u8; 8],
    payload_len: [u8; 2],
    reserved: [u8; 2],
}

pub struct Frame<'a> {
    raw: &'a [u8],
    header: FrameHeader,
}

impl<'a> Frame<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, FrameError> {
        if data.len() < HEADER_SIZE {
            return Err(FrameError::TooShort {
                expected: HEADER_SIZE,
                actual: data.len(),
            });
        }

        let header_bytes: &[u8; HEADER_SIZE] = data[..HEADER_SIZE]
            .try_into()
            .unwrap();

        let header = unsafe {
            std::ptr::read_unaligned(
                header_bytes.as_ptr() as *const RawFrameHeader
            )
        };

        // Validate frame type
        let frame_type = FrameType::try_from(header.frame_type)?;

        // Validate payload length
        let payload_len = u16::from_be_bytes(header.payload_len) as usize;
        if HEADER_SIZE + payload_len > data.len() {
            return Err(FrameError::PayloadOverflow);
        }

        Ok(Self { raw: data, header })
    }

    pub fn frame_type(&self) -> FrameType {
        FrameType::try_from(self.header.frame_type).unwrap()
    }

    pub fn nonce(&self) -> &[u8; 8] {
        &self.header.nonce
    }

    pub fn stream_id(&self) -> u16 {
        u16::from_be_bytes(self.header.stream_id)
    }

    pub fn sequence(&self) -> u32 {
        u32::from_be_bytes(self.header.sequence)
    }

    pub fn offset(&self) -> u64 {
        u64::from_be_bytes(self.header.offset)
    }

    pub fn payload(&self) -> &[u8] {
        let len = u16::from_be_bytes(self.header.payload_len) as usize;
        &self.raw[HEADER_SIZE..HEADER_SIZE + len]
    }
}
```

**Deliverables:**
- [ ] RawFrameHeader struct
- [ ] Frame struct
- [ ] parse() method (zero-copy)
- [ ] Accessor methods (frame_type, nonce, etc.)
- [ ] Validation logic

**Acceptance Criteria:**
- Zero-copy validated (no memcpy)
- All fields parseable
- Invalid frames rejected
- Benchmark: >1M frames/sec parsing

**Estimated Time:** 6-8 hours

---

#### Task 1.2.4: Frame Builder (SP: 5)
**Description:** Construct frames for transmission.

**Implementation:**
```rust
pub struct FrameBuilder {
    frame_type: FrameType,
    flags: FrameFlags,
    stream_id: u16,
    sequence: u32,
    offset: u64,
    payload: Vec<u8>,
    nonce: [u8; 8],
}

impl FrameBuilder {
    pub fn new() -> Self {
        Self {
            frame_type: FrameType::Data,
            flags: FrameFlags::new(),
            stream_id: 0,
            sequence: 0,
            offset: 0,
            payload: Vec::new(),
            nonce: [0u8; 8],
        }
    }

    pub fn frame_type(mut self, ft: FrameType) -> Self {
        self.frame_type = ft;
        self
    }

    pub fn flags(mut self, flags: FrameFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn stream_id(mut self, id: u16) -> Self {
        self.stream_id = id;
        self
    }

    pub fn sequence(mut self, seq: u32) -> Self {
        self.sequence = seq;
        self
    }

    pub fn payload(mut self, data: &[u8]) -> Self {
        self.payload = data.to_vec();
        self
    }

    pub fn build(self, padding_size: usize) -> Result<Vec<u8>, FrameError> {
        let payload_len = self.payload.len();
        let total_size = HEADER_SIZE + payload_len + padding_size;

        let mut buf = Vec::with_capacity(total_size);

        // Write header
        buf.extend_from_slice(&self.nonce);
        buf.push(self.frame_type as u8);
        buf.push(self.flags.0);
        buf.extend_from_slice(&self.stream_id.to_be_bytes());
        buf.extend_from_slice(&self.sequence.to_be_bytes());
        buf.extend_from_slice(&self.offset.to_be_bytes());
        buf.extend_from_slice(&(payload_len as u16).to_be_bytes());
        buf.extend_from_slice(&[0u8; 2]);  // Reserved

        // Write payload
        buf.extend_from_slice(&self.payload);

        // Write random padding
        let mut padding = vec![0u8; padding_size];
        getrandom::fill(&mut padding)
            .map_err(|_| FrameError::RandomFailure)?;
        buf.extend_from_slice(&padding);

        Ok(buf)
    }
}
```

**Deliverables:**
- [ ] FrameBuilder struct
- [ ] Builder methods (fluent API)
- [ ] build() method
- [ ] Padding generation
- [ ] Tests for all frame types

**Acceptance Criteria:**
- All frame types constructible
- Padding randomized
- Output parseable by Frame::parse()
- Benchmark: >500K frames/sec build

**Estimated Time:** 6-8 hours

---

#### Task 1.2.5: Frame Roundtrip Tests (SP: 3)
**Description:** Comprehensive frame encoding/decoding tests.

**Test Cases:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_frame_roundtrip() {
        let payload = b"Hello, WRAITH!";

        let frame_bytes = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(42)
            .sequence(1000)
            .offset(0)
            .payload(payload)
            .build(64)
            .unwrap();

        let parsed = Frame::parse(&frame_bytes).unwrap();

        assert_eq!(parsed.frame_type(), FrameType::Data);
        assert_eq!(parsed.stream_id(), 42);
        assert_eq!(parsed.sequence(), 1000);
        assert_eq!(parsed.payload(), payload);
    }

    #[test]
    fn test_all_frame_types() {
        // Test each FrameType can be built and parsed
    }

    #[test]
    fn test_frame_flags() {
        // Test flag combinations
    }

    #[test]
    fn test_frame_too_short() {
        let short = [0u8; 10];
        assert!(matches!(
            Frame::parse(&short),
            Err(FrameError::TooShort { .. })
        ));
    }

    #[test]
    fn test_invalid_frame_type() {
        // Test 0x00 and 0xFF rejected
    }

    #[proptest]
    fn prop_parse_doesnt_panic(bytes: Vec<u8>) {
        let _ = Frame::parse(&bytes);
    }
}
```

**Deliverables:**
- [ ] Roundtrip tests (all frame types)
- [ ] Flag combination tests
- [ ] Error case tests
- [ ] Property-based tests (fuzzing)
- [ ] Benchmark tests

**Acceptance Criteria:**
- All tests pass
- Property tests find no panics
- Benchmarks meet targets
- Test coverage >90% for frame module

**Estimated Time:** 4-5 hours

---

#### Task 1.2.6: Frame Benchmarks (SP: 2)
**Description:** Benchmark frame parsing and building performance.

**Implementation:**
```rust
// benches/frame_bench.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use wraith_core::frame::*;

fn bench_frame_parse(c: &mut Criterion) {
    let frame_bytes = FrameBuilder::new()
        .frame_type(FrameType::Data)
        .payload(b"test payload")
        .build(0)
        .unwrap();

    let mut group = c.benchmark_group("frame_parse");
    group.throughput(Throughput::Elements(1));

    group.bench_function("parse", |b| {
        b.iter(|| {
            let frame = Frame::parse(black_box(&frame_bytes)).unwrap();
            black_box(frame);
        });
    });

    group.finish();
}

fn bench_frame_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_build");
    group.throughput(Throughput::Elements(1));

    group.bench_function("build", |b| {
        b.iter(|| {
            let frame = FrameBuilder::new()
                .frame_type(FrameType::Data)
                .payload(b"test payload")
                .build(0)
                .unwrap();
            black_box(frame);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_frame_parse, bench_frame_build);
criterion_main!(benches);
```

**Deliverables:**
- [ ] Parse benchmark
- [ ] Build benchmark
- [ ] Throughput measurements
- [ ] Performance regression tests

**Acceptance Criteria:**
- Parse: >1M frames/sec
- Build: >500K frames/sec
- Benchmarks integrated in CI
- Performance tracked over time

**Estimated Time:** 3-4 hours

---

### Sprint 1.2 Definition of Done
- [x] All tasks completed
- [x] Frame module feature-complete
- [x] All tests passing (unit + property)
- [x] Benchmarks meet targets
- [x] Documentation complete

---

## Sprint 1.3: Session State Machine (Week 3)

**Story Points:** 18

### Tasks

#### Task 1.3.1: Session States (SP: 2)
**Description:** Define session state enumeration.

**Implementation:**
```rust
// wraith-core/src/session.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Closed,
    Handshaking(HandshakePhase),
    Established,
    Rekeying,
    Draining,
    Migrating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakePhase {
    InitSent,
    RespSent,
    InitComplete,
}
```

**Deliverables:**
- [ ] SessionState enum
- [ ] HandshakePhase enum
- [ ] State transition validation
- [ ] Display implementations

**Acceptance Criteria:**
- All valid states defined
- Invalid transitions prevented
- State documented with diagrams

**Estimated Time:** 2 hours

---

#### Task 1.3.2: Connection ID (SP: 2)
**Description:** Implement connection ID type.

**Implementation:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId([u8; 8]);

impl ConnectionId {
    pub const HANDSHAKE: Self = Self([0xFF; 8]);

    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 8] {
        &self.0
    }

    pub fn rotate(&self, seq: u32) -> Self {
        let mut rotated = self.0;
        let seq_bytes = seq.to_be_bytes();
        for i in 4..8 {
            rotated[i] ^= seq_bytes[i - 4];
        }
        Self(rotated)
    }
}
```

**Deliverables:**
- [ ] ConnectionId struct
- [ ] Constants (HANDSHAKE)
- [ ] Rotation method
- [ ] Tests for rotation

**Acceptance Criteria:**
- CID rotation reversible
- HANDSHAKE CID recognized
- Serialization works

**Estimated Time:** 2-3 hours

---

#### Task 1.3.3: Session Configuration (SP: 2)
**Description:** Session parameters and configuration.

**Implementation:**
```rust
#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub max_streams: u16,
    pub initial_window: u64,
    pub max_window: u64,
    pub idle_timeout: Duration,
    pub rekey_interval: Duration,
    pub rekey_packet_limit: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_streams: 16384,
            initial_window: 1024 * 1024,
            max_window: 16 * 1024 * 1024,
            idle_timeout: Duration::from_secs(30),
            rekey_interval: Duration::from_secs(120),
            rekey_packet_limit: 1_000_000,
        }
    }
}
```

**Deliverables:**
- [ ] SessionConfig struct
- [ ] Default implementation
- [ ] Configuration validation
- [ ] Builder pattern (optional)

**Acceptance Criteria:**
- All parameters documented
- Defaults reasonable
- Validation prevents invalid configs

**Estimated Time:** 2 hours

---

#### Task 1.3.4: Session Struct (SP: 5)
**Description:** Core Session data structure.

**Implementation:**
```rust
pub struct Session {
    state: SessionState,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    connection_id: ConnectionId,

    // Crypto (placeholder for Phase 2)
    keys: Option<SessionKeys>,

    // Streams
    streams: BTreeMap<u16, Stream>,
    next_stream_id: u16,

    // Send state
    send_queue: VecDeque<Frame>,
    next_seq: u32,
    unacked: BTreeMap<u32, SentPacket>,

    // Receive state
    largest_acked: u32,

    // Timing
    last_activity: Instant,
    last_rekey: Instant,
    packets_since_rekey: u64,

    // Configuration
    config: SessionConfig,
}

struct SentPacket {
    sent_time: Instant,
    size: usize,
    frame_type: FrameType,
    stream_id: Option<u16>,
}
```

**Deliverables:**
- [ ] Session struct
- [ ] Constructor (new_initiator, new_responder)
- [ ] Basic state accessors
- [ ] Tests for construction

**Acceptance Criteria:**
- Session constructible
- State initialized correctly
- Accessors work
- No memory leaks (valgrind/miri)

**Estimated Time:** 4-6 hours

---

#### Task 1.3.5: Session State Transitions (SP: 3)
**Description:** Implement state machine logic.

**Implementation:**
```rust
impl Session {
    pub fn can_transition(&self, to: SessionState) -> bool {
        use SessionState::*;
        use HandshakePhase::*;

        match (self.state, to) {
            (Closed, Handshaking(_)) => true,
            (Handshaking(InitSent), Handshaking(InitComplete)) => true,
            (Handshaking(RespSent), Established) => true,
            (Handshaking(InitComplete), Established) => true,
            (Established, Rekeying) => true,
            (Rekeying, Established) => true,
            (Established, Draining) => true,
            (Established, Migrating) => true,
            (Migrating, Established) => true,
            (_, Closed) => true,  // Can always close
            _ => false,
        }
    }

    pub fn transition_to(&mut self, state: SessionState) -> Result<()> {
        if !self.can_transition(state) {
            return Err(SessionError::InvalidTransition {
                from: self.state,
                to: state,
            });
        }

        tracing::info!(
            "Session state transition: {:?} → {:?}",
            self.state,
            state
        );

        self.state = state;
        Ok(())
    }
}
```

**Deliverables:**
- [ ] can_transition() method
- [ ] transition_to() method
- [ ] State machine tests
- [ ] State diagram documentation

**Acceptance Criteria:**
- All valid transitions allowed
- Invalid transitions rejected
- State logged on transition
- Tests cover all states

**Estimated Time:** 3-4 hours

---

#### Task 1.3.6: Session Tests (SP: 2)
**Description:** Comprehensive session tests.

**Test Cases:**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_session_creation() {
        let config = SessionConfig::default();
        let session = Session::new_initiator(
            "127.0.0.1:0".parse().unwrap(),
            "127.0.0.1:1234".parse().unwrap(),
            config,
        );

        assert_eq!(session.state(), SessionState::Closed);
        assert_eq!(session.next_stream_id(), 1);  // Odd for initiator
    }

    #[test]
    fn test_state_transitions() {
        // Test all valid transitions
    }

    #[test]
    fn test_invalid_transitions() {
        // Test invalid transitions rejected
    }

    #[test]
    fn test_stream_id_allocation() {
        // Test odd/even allocation
    }
}
```

**Deliverables:**
- [ ] Session creation tests
- [ ] State transition tests
- [ ] Stream ID allocation tests
- [ ] Configuration tests

**Acceptance Criteria:**
- All tests pass
- Test coverage >85%
- Edge cases handled

**Estimated Time:** 2-3 hours

---

### Sprint 1.3 Definition of Done
- [x] Session module complete
- [x] All state transitions validated
- [x] Tests passing
- [x] Documentation updated

---

## Sprint 1.4: Stream Multiplexing (Week 4)

**Story Points:** 16

### Tasks

#### Task 1.4.1: Stream States (SP: 2)
#### Task 1.4.2: Stream Struct (SP: 4)
#### Task 1.4.3: Stream Open/Close (SP: 3)
#### Task 1.4.4: Stream Data Buffering (SP: 4)
#### Task 1.4.5: Stream Tests (SP: 3)

*(Detailed task breakdown similar to above)*

---

## Sprint 1.5: CI/CD & Testing (Week 5)

**Story Points:** 13

### Tasks

#### Task 1.5.1: GitHub Actions Workflow (SP: 3)
#### Task 1.5.2: Test Coverage (SP: 2)
#### Task 1.5.3: Clippy & Rustfmt (SP: 2)
#### Task 1.5.4: Documentation CI (SP: 2)
#### Task 1.5.5: Benchmark CI (SP: 2)
#### Task 1.5.6: Security Audit (SP: 2)

*(Detailed task breakdown similar to above)*

---

## Sprint 1.6: Buffer Week & Documentation (Week 6)

**Story Points:** 8

This week is buffer time for:
- Completing delayed tasks from previous sprints
- Writing comprehensive documentation
- Addressing technical debt
- Performance profiling and optimization
- Code review and refactoring

---

## Phase 1 Completion Checklist

### Code Quality
- [ ] All modules compile without warnings
- [ ] Zero unsafe code (except where absolutely necessary)
- [ ] All public APIs documented with examples
- [ ] Internal code has clear comments
- [ ] No TODO/FIXME in production code

### Testing
- [ ] Unit test coverage >80%
- [ ] Integration tests for critical paths
- [ ] Property-based tests for parsing
- [ ] Benchmarks for performance-critical code
- [ ] Fuzz tests don't panic

### Performance
- [ ] Frame parsing: >1M frames/sec
- [ ] Frame building: >500K frames/sec
- [ ] Memory usage: <10 MB baseline
- [ ] No memory leaks (valgrind clean)
- [ ] CPU profiling shows no obvious hotspots

### CI/CD
- [ ] All GitHub Actions workflows passing
- [ ] Clippy lints enabled (deny warnings)
- [ ] Rustfmt enforced
- [ ] Documentation builds successfully
- [ ] Benchmarks tracked

### Documentation
- [ ] README.md complete with examples
- [ ] API documentation for all public items
- [ ] Architecture diagrams updated
- [ ] CHANGELOG.md started
- [ ] CONTRIBUTING.md written

---

## Risks & Mitigation

### Identified Risks

**Risk 1: Frame Parsing Performance**
- **Impact:** High (affects all subsequent phases)
- **Probability:** Low (standard implementation)
- **Mitigation:** Early benchmarking, optimization if needed
- **Contingency:** Accept 500K frames/sec if 1M not achievable

**Risk 2: Session State Machine Complexity**
- **Impact:** Medium (affects development velocity)
- **Probability:** Medium
- **Mitigation:** Clear state diagrams, comprehensive tests
- **Contingency:** Simplify state machine if too complex

**Risk 3: Testing Infrastructure Delays**
- **Impact:** Medium (slows future development)
- **Probability:** Low
- **Mitigation:** Start CI setup early (Sprint 1.5)
- **Contingency:** Manual testing temporarily

---

## Success Metrics

### Quantitative
- [ ] 89 story points completed
- [ ] >80% test coverage
- [ ] >1M frames/sec parsing (benchmark)
- [ ] Zero blocking bugs

### Qualitative
- [ ] Clean, maintainable code
- [ ] Clear architecture
- [ ] Solid foundation for Phase 2
- [ ] Team confidence high

---

## Next Phase Preview

**Phase 2 (Weeks 7-12): Cryptographic Layer**

Key tasks:
- Noise_XX handshake implementation
- Elligator2 encoding
- XChaCha20-Poly1305 AEAD
- Key ratcheting (symmetric + DH)
- Constant-time validation

Dependencies from Phase 1:
- Frame encoding (for handshake messages)
- Session state machine (handshake states)
- Error types (crypto errors)

---

**Last Updated:** 2025-11-28
**Phase Owner:** TBD
**Review Date:** End of Week 6
