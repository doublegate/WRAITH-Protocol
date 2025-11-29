# Phase 4: Obfuscation & Stealth Sprint Planning

**Duration:** Weeks 21-24 (3-4 weeks)
**Total Story Points:** 76
**Risk Level:** Medium (effectiveness difficult to validate)

---

## Phase Overview

**Goal:** Implement comprehensive traffic obfuscation including packet padding, timing obfuscation, cover traffic generation, and protocol mimicry to defeat deep packet inspection and traffic analysis.

### Success Criteria

- [ ] Padding overhead: <20% (privacy mode)
- [ ] TLS mimicry passes DPI inspection tools
- [ ] Cover traffic maintains configurable baseline rate
- [ ] Timing obfuscation defeats correlation attacks
- [ ] Performance impact <10% (privacy mode)
- [ ] Configurable obfuscation levels (none, low, medium, high, paranoid)
- [ ] Protocol mimicry indistinguishable from legitimate traffic

### Dependencies

- Phase 2 complete (crypto layer, especially Elligator2)
- Phase 3 complete (transport layer)
- Traffic analysis tools for validation (Wireshark, Zeek, Suricata)

### Deliverables

1. Packet padding engine (6 size classes)
2. Timing obfuscation (jitter, delays)
3. Cover traffic generator
4. TLS 1.3 record wrapper
5. WebSocket frame wrapper
6. DNS-over-HTTPS tunnel
7. Padding mode selection (adaptive)
8. Statistical traffic analysis resistance
9. Obfuscation benchmarks

---

## Sprint Breakdown

### Sprint 4.1: Packet Padding (Weeks 21-22)

**Duration:** 1.5 weeks
**Story Points:** 21

**4.1.1: Padding Engine** (13 SP)

Implement packet padding to obscure message sizes.

```rust
// wraith-obfuscation/src/padding.rs

use rand::Rng;
use rand_distr::{Distribution, Geometric};

/// Packet padding modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddingMode {
    /// No padding
    None,
    /// Round to power of 2
    PowerOfTwo,
    /// Fixed size classes (128, 512, 1024, 4096, 8192, 16384)
    SizeClasses,
    /// Constant rate padding (always max size)
    ConstantRate,
    /// Statistical padding (random from distribution)
    Statistical,
}

/// Padding size classes (bytes)
const SIZE_CLASSES: &[usize] = &[128, 512, 1024, 4096, 8192, 16384];

pub struct PaddingEngine {
    mode: PaddingMode,
    rng: rand::rngs::ThreadRng,
}

impl PaddingEngine {
    pub fn new(mode: PaddingMode) -> Self {
        Self {
            mode,
            rng: rand::thread_rng(),
        }
    }

    /// Calculate padded size for given plaintext length
    pub fn padded_size(&mut self, plaintext_len: usize) -> usize {
        match self.mode {
            PaddingMode::None => plaintext_len,

            PaddingMode::PowerOfTwo => {
                // Round up to next power of 2
                let next_pow2 = plaintext_len.next_power_of_two();
                next_pow2.max(128) // Minimum 128 bytes
            }

            PaddingMode::SizeClasses => {
                // Find smallest size class that fits
                SIZE_CLASSES.iter()
                    .find(|&&size| size >= plaintext_len)
                    .copied()
                    .unwrap_or(*SIZE_CLASSES.last().unwrap())
            }

            PaddingMode::ConstantRate => {
                // Always pad to maximum size
                *SIZE_CLASSES.last().unwrap()
            }

            PaddingMode::Statistical => {
                // Use geometric distribution for realistic padding
                let mean = 1.5; // Average padding multiplier
                let p = 1.0 / mean;
                let geo = Geometric::new(p).unwrap();

                let extra_padding = geo.sample(&mut self.rng) as usize * 128;
                let padded = plaintext_len + extra_padding;

                // Clamp to reasonable bounds
                padded.min(16384).max(128)
            }
        }
    }

    /// Add padding to buffer
    pub fn pad(&mut self, buffer: &mut Vec<u8>, target_size: usize) {
        if buffer.len() >= target_size {
            return;
        }

        let padding_len = target_size - buffer.len();

        // Add padding bytes (random data)
        let mut padding = vec![0u8; padding_len];
        self.rng.fill(&mut padding[..]);

        buffer.extend_from_slice(&padding);
    }

    /// Remove padding from buffer (returns original length)
    pub fn unpad(&self, buffer: &[u8], original_len: usize) -> &[u8] {
        &buffer[..original_len.min(buffer.len())]
    }

    /// Calculate overhead percentage
    pub fn overhead(&self, plaintext_len: usize) -> f64 {
        let padded = self.padded_size_const(plaintext_len);
        ((padded - plaintext_len) as f64 / plaintext_len as f64) * 100.0
    }

    fn padded_size_const(&self, plaintext_len: usize) -> usize {
        match self.mode {
            PaddingMode::None => plaintext_len,
            PaddingMode::PowerOfTwo => plaintext_len.next_power_of_two().max(128),
            PaddingMode::SizeClasses => {
                SIZE_CLASSES.iter()
                    .find(|&&size| size >= plaintext_len)
                    .copied()
                    .unwrap_or(*SIZE_CLASSES.last().unwrap())
            }
            PaddingMode::ConstantRate => *SIZE_CLASSES.last().unwrap(),
            PaddingMode::Statistical => {
                // Average case for statistical mode
                (plaintext_len as f64 * 1.5) as usize
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_padding() {
        let mut engine = PaddingEngine::new(PaddingMode::None);
        assert_eq!(engine.padded_size(100), 100);
        assert_eq!(engine.padded_size(1000), 1000);
    }

    #[test]
    fn test_power_of_two() {
        let mut engine = PaddingEngine::new(PaddingMode::PowerOfTwo);
        assert_eq!(engine.padded_size(100), 128);
        assert_eq!(engine.padded_size(129), 256);
        assert_eq!(engine.padded_size(1000), 1024);
    }

    #[test]
    fn test_size_classes() {
        let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
        assert_eq!(engine.padded_size(100), 128);
        assert_eq!(engine.padded_size(500), 512);
        assert_eq!(engine.padded_size(1000), 1024);
        assert_eq!(engine.padded_size(5000), 8192);
    }

    #[test]
    fn test_constant_rate() {
        let mut engine = PaddingEngine::new(PaddingMode::ConstantRate);
        assert_eq!(engine.padded_size(100), 16384);
        assert_eq!(engine.padded_size(8000), 16384);
    }

    #[test]
    fn test_padding_overhead() {
        let engine = PaddingEngine::new(PaddingMode::SizeClasses);

        // 100 bytes -> 128 bytes = 28% overhead
        let overhead = engine.overhead(100);
        assert!((overhead - 28.0).abs() < 1.0);

        // 500 bytes -> 512 bytes = 2.4% overhead
        let overhead = engine.overhead(500);
        assert!((overhead - 2.4).abs() < 1.0);
    }

    #[test]
    fn test_pad_unpad_roundtrip() {
        let mut engine = PaddingEngine::new(PaddingMode::SizeClasses);
        let original = b"hello world";
        let original_len = original.len();

        let mut buffer = original.to_vec();
        let target_size = engine.padded_size(original_len);

        engine.pad(&mut buffer, target_size);
        assert_eq!(buffer.len(), target_size);

        let unpadded = engine.unpad(&buffer, original_len);
        assert_eq!(unpadded, original);
    }
}
```

**Acceptance Criteria:**
- [ ] All padding modes implemented
- [ ] Overhead <20% for size classes mode
- [ ] Random padding data (not zeros)
- [ ] Deterministic size selection (same input → same size)
- [ ] Statistical mode provides realistic distribution

---

**4.1.2: Adaptive Padding Selection** (8 SP)

Automatically select padding mode based on threat model and performance requirements.

```rust
// wraith-obfuscation/src/adaptive.rs

use crate::padding::{PaddingMode, PaddingEngine};

#[derive(Debug, Clone, Copy)]
pub enum ThreatLevel {
    /// No obfuscation needed
    Low,
    /// Light obfuscation for casual observers
    Medium,
    /// Strong obfuscation for capable adversaries
    High,
    /// Maximum obfuscation regardless of cost
    Paranoid,
}

#[derive(Debug, Clone)]
pub struct ObfuscationProfile {
    pub padding_mode: PaddingMode,
    pub timing_jitter: bool,
    pub cover_traffic: bool,
    pub protocol_mimicry: Option<MimicryMode>,
}

#[derive(Debug, Clone, Copy)]
pub enum MimicryMode {
    Tls,
    WebSocket,
    DnsOverHttps,
}

impl ObfuscationProfile {
    /// Select profile based on threat level
    pub fn from_threat_level(level: ThreatLevel) -> Self {
        match level {
            ThreatLevel::Low => Self {
                padding_mode: PaddingMode::None,
                timing_jitter: false,
                cover_traffic: false,
                protocol_mimicry: None,
            },

            ThreatLevel::Medium => Self {
                padding_mode: PaddingMode::SizeClasses,
                timing_jitter: true,
                cover_traffic: false,
                protocol_mimicry: None,
            },

            ThreatLevel::High => Self {
                padding_mode: PaddingMode::Statistical,
                timing_jitter: true,
                cover_traffic: true,
                protocol_mimicry: Some(MimicryMode::Tls),
            },

            ThreatLevel::Paranoid => Self {
                padding_mode: PaddingMode::ConstantRate,
                timing_jitter: true,
                cover_traffic: true,
                protocol_mimicry: Some(MimicryMode::Tls),
            },
        }
    }

    /// Estimate performance overhead
    pub fn estimated_overhead(&self) -> f64 {
        let mut overhead = 0.0;

        // Padding overhead
        overhead += match self.padding_mode {
            PaddingMode::None => 0.0,
            PaddingMode::PowerOfTwo => 15.0,
            PaddingMode::SizeClasses => 10.0,
            PaddingMode::ConstantRate => 50.0,
            PaddingMode::Statistical => 20.0,
        };

        // Timing overhead
        if self.timing_jitter {
            overhead += 5.0;
        }

        // Cover traffic overhead
        if self.cover_traffic {
            overhead += 25.0;
        }

        // Protocol mimicry overhead
        if self.protocol_mimicry.is_some() {
            overhead += 8.0;
        }

        overhead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_level_profiles() {
        let low = ObfuscationProfile::from_threat_level(ThreatLevel::Low);
        assert_eq!(low.padding_mode, PaddingMode::None);
        assert!(!low.timing_jitter);

        let high = ObfuscationProfile::from_threat_level(ThreatLevel::High);
        assert!(high.timing_jitter);
        assert!(high.cover_traffic);
        assert!(high.protocol_mimicry.is_some());
    }

    #[test]
    fn test_overhead_estimation() {
        let low = ObfuscationProfile::from_threat_level(ThreatLevel::Low);
        assert_eq!(low.estimated_overhead(), 0.0);

        let paranoid = ObfuscationProfile::from_threat_level(ThreatLevel::Paranoid);
        assert!(paranoid.estimated_overhead() > 50.0);
    }
}
```

**Acceptance Criteria:**
- [ ] Threat level mapping works
- [ ] Overhead estimation accurate
- [ ] Profiles configurable
- [ ] Easy to add new threat levels

---

### Sprint 4.2: Timing Obfuscation (Week 22)

**Duration:** 1 week
**Story Points:** 13

**4.2.1: Timing Jitter** (8 SP)

```rust
// wraith-obfuscation/src/timing.rs

use rand::Rng;
use rand_distr::{Distribution, Normal, Exp};
use std::time::Duration;

pub struct TimingObfuscator {
    mode: TimingMode,
    rng: rand::rngs::ThreadRng,
}

#[derive(Debug, Clone, Copy)]
pub enum TimingMode {
    /// No timing obfuscation
    None,
    /// Fixed delay
    Fixed(Duration),
    /// Uniform random delay
    Uniform { min: Duration, max: Duration },
    /// Normal distribution
    Normal { mean: Duration, stddev: Duration },
    /// Exponential distribution
    Exponential { mean: Duration },
}

impl TimingObfuscator {
    pub fn new(mode: TimingMode) -> Self {
        Self {
            mode,
            rng: rand::thread_rng(),
        }
    }

    /// Calculate delay before sending next packet
    pub fn next_delay(&mut self) -> Duration {
        match self.mode {
            TimingMode::None => Duration::from_micros(0),

            TimingMode::Fixed(delay) => delay,

            TimingMode::Uniform { min, max } => {
                let min_us = min.as_micros() as u64;
                let max_us = max.as_micros() as u64;
                let delay_us = self.rng.gen_range(min_us..=max_us);
                Duration::from_micros(delay_us)
            }

            TimingMode::Normal { mean, stddev } => {
                let mean_us = mean.as_micros() as f64;
                let stddev_us = stddev.as_micros() as f64;

                let normal = Normal::new(mean_us, stddev_us).unwrap();
                let delay_us = normal.sample(&mut self.rng).max(0.0) as u64;

                Duration::from_micros(delay_us)
            }

            TimingMode::Exponential { mean } => {
                let mean_us = mean.as_micros() as f64;
                let lambda = 1.0 / mean_us;

                let exp = Exp::new(lambda).unwrap();
                let delay_us = exp.sample(&mut self.rng) as u64;

                Duration::from_micros(delay_us)
            }
        }
    }

    /// Sleep for obfuscated delay
    pub fn sleep(&mut self) {
        let delay = self.next_delay();
        if delay > Duration::from_micros(0) {
            std::thread::sleep(delay);
        }
    }

    /// Async sleep
    #[cfg(feature = "async")]
    pub async fn sleep_async(&mut self) {
        let delay = self.next_delay();
        if delay > Duration::from_micros(0) {
            tokio::time::sleep(delay).await;
        }
    }
}

/// Traffic shaping to mimic specific patterns
pub struct TrafficShaper {
    target_rate: f64, // packets per second
    last_send: std::time::Instant,
}

impl TrafficShaper {
    pub fn new(packets_per_second: f64) -> Self {
        Self {
            target_rate: packets_per_second,
            last_send: std::time::Instant::now(),
        }
    }

    /// Wait until next packet should be sent
    pub fn wait_for_next(&mut self) {
        let interval = Duration::from_secs_f64(1.0 / self.target_rate);
        let elapsed = self.last_send.elapsed();

        if elapsed < interval {
            std::thread::sleep(interval - elapsed);
        }

        self.last_send = std::time::Instant::now();
    }

    /// Set new target rate
    pub fn set_rate(&mut self, packets_per_second: f64) {
        self.target_rate = packets_per_second;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_delay() {
        let mut obfuscator = TimingObfuscator::new(
            TimingMode::Fixed(Duration::from_millis(10))
        );

        let delay = obfuscator.next_delay();
        assert_eq!(delay, Duration::from_millis(10));
    }

    #[test]
    fn test_uniform_delay() {
        let mut obfuscator = TimingObfuscator::new(
            TimingMode::Uniform {
                min: Duration::from_millis(5),
                max: Duration::from_millis(15),
            }
        );

        for _ in 0..100 {
            let delay = obfuscator.next_delay();
            assert!(delay >= Duration::from_millis(5));
            assert!(delay <= Duration::from_millis(15));
        }
    }

    #[test]
    fn test_traffic_shaper() {
        let mut shaper = TrafficShaper::new(100.0); // 100 pps

        let start = std::time::Instant::now();

        for _ in 0..10 {
            shaper.wait_for_next();
        }

        let elapsed = start.elapsed();

        // Should take ~100ms for 10 packets at 100 pps
        assert!(elapsed >= Duration::from_millis(90));
        assert!(elapsed <= Duration::from_millis(150));
    }
}
```

**Acceptance Criteria:**
- [ ] All timing modes work
- [ ] Distributions statistically correct
- [ ] Traffic shaping maintains rate
- [ ] Async support (optional feature)
- [ ] Performance impact <5%

---

**4.2.2: Cover Traffic Generator** (5 SP)

```rust
// wraith-obfuscation/src/cover.rs

use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct CoverTrafficGenerator {
    baseline_rate: f64, // packets per second
    packet_size: usize,
    running: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl CoverTrafficGenerator {
    pub fn new(baseline_rate: f64, packet_size: usize) -> Self {
        Self {
            baseline_rate,
            packet_size,
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }

    /// Start generating cover traffic
    pub fn start<F>(&mut self, mut send_fn: F)
    where
        F: FnMut(Vec<u8>) + Send + 'static,
    {
        if self.running.load(Ordering::Acquire) {
            return; // Already running
        }

        self.running.store(true, Ordering::Release);
        let running = self.running.clone();
        let packet_size = self.packet_size;
        let interval = Duration::from_secs_f64(1.0 / self.baseline_rate);

        let handle = std::thread::spawn(move || {
            let mut rng = rand::thread_rng();

            while running.load(Ordering::Acquire) {
                // Generate random cover packet
                let mut packet = vec![0u8; packet_size];
                rand::Rng::fill(&mut rng, &mut packet[..]);

                send_fn(packet);

                std::thread::sleep(interval);
            }
        });

        self.handle = Some(handle);
    }

    /// Stop generating cover traffic
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Release);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    /// Set new baseline rate
    pub fn set_rate(&mut self, packets_per_second: f64) {
        self.baseline_rate = packets_per_second;
        // Restart if running
        if self.running.load(Ordering::Acquire) {
            self.stop();
            // Caller needs to call start() again with send_fn
        }
    }
}

impl Drop for CoverTrafficGenerator {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn test_cover_traffic_generation() {
        let packets_sent = Arc::new(Mutex::new(Vec::new()));
        let packets_clone = packets_sent.clone();

        let mut generator = CoverTrafficGenerator::new(10.0, 512);

        generator.start(move |packet| {
            packets_clone.lock().unwrap().push(packet);
        });

        std::thread::sleep(Duration::from_millis(500));
        generator.stop();

        let sent = packets_sent.lock().unwrap();
        // Should have sent ~5 packets (10 pps * 0.5 sec)
        assert!(sent.len() >= 3 && sent.len() <= 7);
        assert_eq!(sent[0].len(), 512);
    }
}
```

**Acceptance Criteria:**
- [ ] Cover traffic generates at baseline rate
- [ ] Packet sizes configurable
- [ ] Start/stop works correctly
- [ ] Thread-safe
- [ ] Graceful shutdown

---

### Sprint 4.3: Protocol Mimicry (Week 23-24)

**Duration:** 1.5 weeks
**Story Points:** 34

**4.3.1: TLS 1.3 Record Wrapper** (13 SP)

```rust
// wraith-obfuscation/src/tls_mimicry.rs

/// TLS 1.3 record layer mimicry
/// Wraps WRAITH packets to look like TLS application data

const TLS_CONTENT_TYPE_APPLICATION_DATA: u8 = 23;
const TLS_VERSION_1_2: u16 = 0x0303; // Legacy version in TLS 1.3 records

pub struct TlsRecordWrapper {
    sequence_number: u64,
}

impl TlsRecordWrapper {
    pub fn new() -> Self {
        Self {
            sequence_number: 0,
        }
    }

    /// Wrap payload in TLS record
    pub fn wrap(&mut self, payload: &[u8]) -> Vec<u8> {
        let mut record = Vec::with_capacity(5 + payload.len());

        // TLS Record Header (5 bytes)
        record.push(TLS_CONTENT_TYPE_APPLICATION_DATA); // Content Type
        record.extend_from_slice(&TLS_VERSION_1_2.to_be_bytes()); // Legacy version
        record.extend_from_slice(&(payload.len() as u16).to_be_bytes()); // Length

        // Payload (encrypted in real TLS, our already-encrypted data)
        record.extend_from_slice(payload);

        self.sequence_number += 1;

        record
    }

    /// Unwrap TLS record to get payload
    pub fn unwrap(&self, record: &[u8]) -> Result<Vec<u8>, TlsError> {
        if record.len() < 5 {
            return Err(TlsError::TooShort);
        }

        // Parse TLS record header
        let content_type = record[0];
        let version = u16::from_be_bytes([record[1], record[2]]);
        let length = u16::from_be_bytes([record[3], record[4]]) as usize;

        if content_type != TLS_CONTENT_TYPE_APPLICATION_DATA {
            return Err(TlsError::InvalidContentType);
        }

        if record.len() < 5 + length {
            return Err(TlsError::IncompleteRecord);
        }

        // Extract payload
        let payload = record[5..5 + length].to_vec();

        Ok(payload)
    }
}

/// Full TLS session mimicry (including handshake simulation)
pub struct TlsSessionMimicry {
    handshake_complete: bool,
    wrapper: TlsRecordWrapper,
}

impl TlsSessionMimicry {
    pub fn new() -> Self {
        Self {
            handshake_complete: false,
            wrapper: TlsRecordWrapper::new(),
        }
    }

    /// Generate fake TLS handshake messages
    pub fn generate_handshake(&mut self) -> Vec<Vec<u8>> {
        let mut messages = Vec::new();

        // ClientHello
        messages.push(self.fake_client_hello());

        // ServerHello + Certificate + ... + Finished
        messages.push(self.fake_server_hello());

        // ClientFinished
        messages.push(self.fake_client_finished());

        self.handshake_complete = true;

        messages
    }

    fn fake_client_hello(&self) -> Vec<u8> {
        // Simplified ClientHello structure
        let mut hello = Vec::new();

        // Record header
        hello.push(22); // Handshake content type
        hello.extend_from_slice(&TLS_VERSION_1_2.to_be_bytes());

        // Handshake header
        hello.push(1); // ClientHello type
        // Length will be filled later

        // Random (32 bytes)
        let mut rng = rand::thread_rng();
        let mut random = [0u8; 32];
        rand::Rng::fill(&mut rng, &mut random[..]);
        hello.extend_from_slice(&random);

        // Session ID (empty)
        hello.push(0);

        // Cipher suites (simplified)
        hello.extend_from_slice(&[0x00, 0x02]); // Length: 2
        hello.extend_from_slice(&[0x13, 0x01]); // TLS_AES_128_GCM_SHA256

        // Compression methods
        hello.push(1); // Length
        hello.push(0); // No compression

        // TODO: Add extensions (SNI, supported_versions, etc.)

        hello
    }

    fn fake_server_hello(&self) -> Vec<u8> {
        // Simplified ServerHello + other handshake messages
        vec![0x16, 0x03, 0x03, 0x00, 0x5a] // Placeholder
    }

    fn fake_client_finished(&self) -> Vec<u8> {
        // Simplified Finished message
        vec![0x16, 0x03, 0x03, 0x00, 0x35] // Placeholder
    }

    /// Wrap application data (after handshake)
    pub fn wrap_application_data(&mut self, data: &[u8]) -> Result<Vec<u8>, TlsError> {
        if !self.handshake_complete {
            return Err(TlsError::HandshakeNotComplete);
        }

        Ok(self.wrapper.wrap(data))
    }

    /// Unwrap application data
    pub fn unwrap_application_data(&self, record: &[u8]) -> Result<Vec<u8>, TlsError> {
        if !self.handshake_complete {
            return Err(TlsError::HandshakeNotComplete);
        }

        self.wrapper.unwrap(record)
    }
}

#[derive(Debug)]
pub enum TlsError {
    TooShort,
    InvalidContentType,
    IncompleteRecord,
    HandshakeNotComplete,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_record_wrap_unwrap() {
        let mut wrapper = TlsRecordWrapper::new();
        let payload = b"hello world";

        let record = wrapper.wrap(payload);

        // Check record structure
        assert_eq!(record[0], TLS_CONTENT_TYPE_APPLICATION_DATA);
        assert_eq!(record.len(), 5 + payload.len());

        let unwrapped = wrapper.unwrap(&record).unwrap();
        assert_eq!(unwrapped, payload);
    }

    #[test]
    fn test_tls_handshake_generation() {
        let mut session = TlsSessionMimicry::new();

        let handshake_msgs = session.generate_handshake();
        assert_eq!(handshake_msgs.len(), 3); // ClientHello, ServerHello, Finished

        assert!(session.handshake_complete);
    }

    #[test]
    fn test_application_data_wrapping() {
        let mut session = TlsSessionMimicry::new();

        // Should fail before handshake
        assert!(session.wrap_application_data(b"test").is_err());

        session.generate_handshake();

        // Should succeed after handshake
        let wrapped = session.wrap_application_data(b"test").unwrap();
        let unwrapped = session.unwrap_application_data(&wrapped).unwrap();
        assert_eq!(unwrapped, b"test");
    }
}
```

**Acceptance Criteria:**
- [ ] TLS record structure correct
- [ ] Wrap/unwrap round-trips
- [ ] Handshake messages realistic
- [ ] DPI tools accept as valid TLS
- [ ] Wireshark shows as TLS 1.3

---

**4.3.2: WebSocket Frame Wrapper** (8 SP)

```rust
// wraith-obfuscation/src/websocket_mimicry.rs

use rand::Rng;

const WEBSOCKET_OPCODE_BINARY: u8 = 0x02;
const WEBSOCKET_FIN_BIT: u8 = 0x80;

pub struct WebSocketFrameWrapper {
    client_to_server: bool, // Clients must mask frames
}

impl WebSocketFrameWrapper {
    pub fn new(client_to_server: bool) -> Self {
        Self { client_to_server }
    }

    /// Wrap payload in WebSocket frame
    pub fn wrap(&self, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();

        // Byte 1: FIN + RSV + OPCODE
        frame.push(WEBSOCKET_FIN_BIT | WEBSOCKET_OPCODE_BINARY);

        // Byte 2: MASK + Payload length
        let mask_bit = if self.client_to_server { 0x80 } else { 0x00 };

        if payload.len() < 126 {
            frame.push(mask_bit | payload.len() as u8);
        } else if payload.len() < 65536 {
            frame.push(mask_bit | 126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        } else {
            frame.push(mask_bit | 127);
            frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }

        // Masking key (if client)
        let masking_key = if self.client_to_server {
            let mut rng = rand::thread_rng();
            let key = [
                rng.gen::<u8>(),
                rng.gen::<u8>(),
                rng.gen::<u8>(),
                rng.gen::<u8>(),
            ];
            frame.extend_from_slice(&key);
            Some(key)
        } else {
            None
        };

        // Payload (masked if client)
        if let Some(key) = masking_key {
            let masked: Vec<u8> = payload.iter()
                .enumerate()
                .map(|(i, &byte)| byte ^ key[i % 4])
                .collect();
            frame.extend_from_slice(&masked);
        } else {
            frame.extend_from_slice(payload);
        }

        frame
    }

    /// Unwrap WebSocket frame to get payload
    pub fn unwrap(&self, frame: &[u8]) -> Result<Vec<u8>, WsError> {
        if frame.len() < 2 {
            return Err(WsError::TooShort);
        }

        let _fin = (frame[0] & 0x80) != 0;
        let opcode = frame[0] & 0x0F;

        if opcode != WEBSOCKET_OPCODE_BINARY {
            return Err(WsError::InvalidOpcode);
        }

        let masked = (frame[1] & 0x80) != 0;
        let mut payload_len = (frame[1] & 0x7F) as usize;
        let mut offset = 2;

        // Extended payload length
        if payload_len == 126 {
            if frame.len() < 4 {
                return Err(WsError::TooShort);
            }
            payload_len = u16::from_be_bytes([frame[2], frame[3]]) as usize;
            offset = 4;
        } else if payload_len == 127 {
            if frame.len() < 10 {
                return Err(WsError::TooShort);
            }
            payload_len = u64::from_be_bytes([
                frame[2], frame[3], frame[4], frame[5],
                frame[6], frame[7], frame[8], frame[9],
            ]) as usize;
            offset = 10;
        }

        // Masking key
        let masking_key = if masked {
            if frame.len() < offset + 4 {
                return Err(WsError::TooShort);
            }
            let key = [frame[offset], frame[offset + 1], frame[offset + 2], frame[offset + 3]];
            offset += 4;
            Some(key)
        } else {
            None
        };

        // Payload
        if frame.len() < offset + payload_len {
            return Err(WsError::IncompleteFrame);
        }

        let payload = if let Some(key) = masking_key {
            frame[offset..offset + payload_len]
                .iter()
                .enumerate()
                .map(|(i, &byte)| byte ^ key[i % 4])
                .collect()
        } else {
            frame[offset..offset + payload_len].to_vec()
        };

        Ok(payload)
    }
}

#[derive(Debug)]
pub enum WsError {
    TooShort,
    InvalidOpcode,
    IncompleteFrame,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_wrap_unwrap_server() {
        let wrapper = WebSocketFrameWrapper::new(false); // Server doesn't mask
        let payload = b"hello";

        let frame = wrapper.wrap(payload);
        let unwrapped = wrapper.unwrap(&frame).unwrap();

        assert_eq!(unwrapped, payload);
    }

    #[test]
    fn test_websocket_wrap_unwrap_client() {
        let wrapper = WebSocketFrameWrapper::new(true); // Client masks
        let payload = b"hello";

        let frame = wrapper.wrap(payload);

        // Frame should be masked
        assert_ne!(&frame[6..11], payload); // Masked payload different from original

        let unwrapped = wrapper.unwrap(&frame).unwrap();
        assert_eq!(unwrapped, payload);
    }

    #[test]
    fn test_websocket_extended_length() {
        let wrapper = WebSocketFrameWrapper::new(false);
        let payload = vec![0u8; 300]; // > 125 bytes

        let frame = wrapper.wrap(&payload);
        let unwrapped = wrapper.unwrap(&frame).unwrap();

        assert_eq!(unwrapped, payload);
    }
}
```

**Acceptance Criteria:**
- [ ] WebSocket frame structure correct
- [ ] Masking works (client frames)
- [ ] Extended length encoding works
- [ ] DPI recognizes as WebSocket
- [ ] Wireshark parses correctly

---

**4.3.3: DNS-over-HTTPS Tunneling** (13 SP)

```rust
// wraith-obfuscation/src/doh_tunnel.rs

use base64::Engine;

/// DNS-over-HTTPS tunneling for WRAITH traffic
pub struct DohTunnel {
    resolver_url: String,
}

impl DohTunnel {
    pub fn new(resolver_url: String) -> Self {
        Self { resolver_url }
    }

    /// Encode payload as fake DNS query
    pub fn encode_query(&self, payload: &[u8]) -> String {
        // Encode payload as base64url (DNS wireformat simulation)
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);

        // Format as DNS query parameter
        format!("{}?dns={}", self.resolver_url, encoded)
    }

    /// Decode DNS response to get payload
    pub fn decode_response(&self, response: &[u8]) -> Result<Vec<u8>, DohError> {
        // Parse fake DNS response
        // In real DoH, this would be DNS wireformat
        // For WRAITH, we just base64-decode the payload

        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(response)
            .map_err(|_| DohError::DecodeFailed)
    }

    /// Create fake DNS query packet
    pub fn create_dns_query(&self, domain: &str, payload: &[u8]) -> Vec<u8> {
        let mut query = Vec::new();

        // DNS header (12 bytes)
        query.extend_from_slice(&[0x00, 0x01]); // Transaction ID
        query.extend_from_slice(&[0x01, 0x00]); // Flags (standard query)
        query.extend_from_slice(&[0x00, 0x01]); // Questions: 1
        query.extend_from_slice(&[0x00, 0x00]); // Answers: 0
        query.extend_from_slice(&[0x00, 0x00]); // Authority: 0
        query.extend_from_slice(&[0x00, 0x01]); // Additional: 1 (EDNS)

        // Question section
        for label in domain.split('.') {
            query.push(label.len() as u8);
            query.extend_from_slice(label.as_bytes());
        }
        query.push(0); // End of name

        query.extend_from_slice(&[0x00, 0x10]); // Type: TXT
        query.extend_from_slice(&[0x00, 0x01]); // Class: IN

        // EDNS OPT record (used to carry payload)
        query.push(0); // Name: root
        query.extend_from_slice(&[0x00, 0x29]); // Type: OPT
        query.extend_from_slice(&[0x10, 0x00]); // UDP payload size: 4096
        query.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Extended RCODE and flags

        // Encode payload in EDNS option
        let payload_len = payload.len() as u16;
        query.extend_from_slice(&payload_len.to_be_bytes());
        query.extend_from_slice(payload);

        query
    }

    /// Parse fake DNS response
    pub fn parse_dns_response(&self, response: &[u8]) -> Result<Vec<u8>, DohError> {
        if response.len() < 12 {
            return Err(DohError::InvalidResponse);
        }

        // Skip DNS header
        let mut offset = 12;

        // Skip question section (simplified parsing)
        while offset < response.len() && response[offset] != 0 {
            offset += 1 + response[offset] as usize;
        }
        offset += 1; // Skip null terminator
        offset += 4; // Skip type + class

        // Parse answer section (simplified - assumes EDNS)
        // In practice, would need proper DNS parsing

        // Extract payload from EDNS option
        if offset + 10 < response.len() {
            let payload_len = u16::from_be_bytes([
                response[offset + 8],
                response[offset + 9],
            ]) as usize;

            if offset + 10 + payload_len <= response.len() {
                return Ok(response[offset + 10..offset + 10 + payload_len].to_vec());
            }
        }

        Err(DohError::InvalidResponse)
    }
}

#[derive(Debug)]
pub enum DohError {
    DecodeFailed,
    InvalidResponse,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doh_encode_decode() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
        let payload = b"secret data";

        let query_url = tunnel.encode_query(payload);
        assert!(query_url.contains("dns="));

        // Extract base64 part
        let encoded = query_url.split("dns=").nth(1).unwrap();
        let decoded = tunnel.decode_response(encoded.as_bytes()).unwrap();

        assert_eq!(decoded, payload);
    }

    #[test]
    fn test_dns_query_creation() {
        let tunnel = DohTunnel::new("https://dns.example.com/dns-query".to_string());
        let payload = b"test";

        let query = tunnel.create_dns_query("wraith.example.com", payload);

        // Should have DNS header
        assert_eq!(query.len() > 12, true);

        // Parse it back
        let parsed = tunnel.parse_dns_response(&query).unwrap();
        assert_eq!(parsed, payload);
    }
}
```

**Acceptance Criteria:**
- [ ] DNS query/response encoding works
- [ ] EDNS0 payload carrier functional
- [ ] Base64url encoding correct
- [ ] DPI recognizes as DNS
- [ ] Wireshark shows as DoH traffic

---

### Sprint 4.4: Testing & Validation (Week 24)

**Duration:** 1 week
**Story Points:** 8

**4.4.1: DPI Evasion Testing** (5 SP)

```bash
# Test script for DPI evasion validation

#!/bin/bash

echo "Testing WRAITH obfuscation against DPI tools..."

# Test 1: Wireshark dissector
echo "1. Testing TLS mimicry with Wireshark..."
wireshark -r wraith_tls.pcap -Y "tls" | grep "Application Data"

# Test 2: Zeek (Bro) IDS
echo "2. Testing against Zeek IDS..."
zeek -r wraith_traffic.pcap

# Test 3: Suricata IDS
echo "3. Testing against Suricata..."
suricata -r wraith_traffic.pcap -l ./logs/

# Test 4: nDPI (Deep Packet Inspection)
echo "4. Testing against nDPI..."
ndpiReader -i wraith_traffic.pcap

# Test 5: Statistical analysis
echo "5. Performing statistical traffic analysis..."
python3 traffic_analysis.py wraith_traffic.pcap

echo "DPI evasion testing complete."
```

**Acceptance Criteria:**
- [ ] Wireshark identifies traffic as TLS/WebSocket/DNS
- [ ] Zeek doesn't flag traffic as suspicious
- [ ] Suricata generates no alerts
- [ ] nDPI classifies as expected protocol
- [ ] Statistical tests show realistic patterns

---

**4.4.2: Performance Benchmarks** (3 SP)

```rust
// wraith-obfuscation/benches/obfuscation.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use wraith_obfuscation::*;

fn bench_padding(c: &mut Criterion) {
    let mut group = c.benchmark_group("padding");

    for size in [128, 512, 1024, 4096] {
        let data = vec![0u8; size];
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(format!("size_classes_{}", size), &data, |b, data| {
            let mut engine = padding::PaddingEngine::new(padding::PaddingMode::SizeClasses);
            b.iter(|| {
                let mut buf = data.clone();
                let target = engine.padded_size(data.len());
                engine.pad(&mut buf, target);
                black_box(buf);
            });
        });
    }

    group.finish();
}

fn bench_tls_wrap(c: &mut Criterion) {
    let mut wrapper = tls_mimicry::TlsRecordWrapper::new();
    let payload = vec![0u8; 1024];

    c.bench_function("tls_wrap", |b| {
        b.iter(|| {
            let wrapped = wrapper.wrap(black_box(&payload));
            black_box(wrapped);
        });
    });
}

criterion_group!(benches, bench_padding, bench_tls_wrap);
criterion_main!(benches);
```

**Acceptance Criteria:**
- [ ] Padding overhead measured
- [ ] Mimicry overhead <10%
- [ ] Timing impact quantified
- [ ] Benchmark results documented

---

## Definition of Done (Phase 4)

### Code Quality
- [ ] All code passes `cargo clippy`
- [ ] Code formatted with `rustfmt`
- [ ] Public APIs documented
- [ ] Test coverage >80%

### Functionality
- [ ] Packet padding works (6 modes)
- [ ] Timing obfuscation functional
- [ ] Cover traffic generator works
- [ ] TLS/WebSocket/DoH mimicry implemented
- [ ] Adaptive profile selection

### Effectiveness
- [ ] DPI tools classify as expected protocol
- [ ] Statistical analysis shows realistic patterns
- [ ] No obvious WRAITH signatures in captures
- [ ] Padding overhead <20% (privacy mode)

### Performance
- [ ] Obfuscation overhead <10% (medium threat level)
- [ ] Latency increase <5ms
- [ ] Throughput reduction <15%

### Testing
- [ ] Unit tests for all modules
- [ ] DPI evasion testing complete
- [ ] Statistical validation
- [ ] Performance benchmarks

### Documentation
- [ ] Obfuscation modes documented
- [ ] DPI evasion guide
- [ ] Configuration examples
- [ ] Performance impact documented

---

## Risk Mitigation

### Effectiveness Validation
**Risk**: Difficult to prove obfuscation works against all DPI
**Mitigation**: Test against multiple DPI tools, document limitations

### Performance Impact
**Risk**: Obfuscation reduces throughput significantly
**Mitigation**: Configurable levels, benchmark all modes

### False Positives
**Risk**: Legitimate DPI might still flag traffic
**Mitigation**: Improve mimicry realism, stay updated on DPI techniques

---

## Phase 4 Completion Checklist

- [ ] Sprint 4.1: Packet padding engine
- [ ] Sprint 4.2: Timing obfuscation & cover traffic
- [ ] Sprint 4.3: Protocol mimicry (TLS, WebSocket, DoH)
- [ ] Sprint 4.4: DPI testing & benchmarks
- [ ] All acceptance criteria met
- [ ] DPI evasion validated
- [ ] Documentation complete

**Estimated Completion:** Week 24 (end of Phase 4)
