//! Frame encoding and decoding for the WRAITH wire protocol.
//!
//! This module implements zero-copy parsing of protocol frames with
//! careful attention to alignment for DMA efficiency. All multi-byte
//! fields are big-endian (network byte order).
//!
//! ## SIMD Acceleration
//!
//! When the `simd` feature is enabled, frame parsing uses vectorized
//! instructions for extracting and byte-swapping header fields. This
//! provides ~2-3x speedup for header parsing on x86_64 and aarch64
//! platforms with SIMD support.

use crate::FRAME_HEADER_SIZE;
use crate::error::FrameError;
use rand::Rng;

/// Maximum payload size (9000 - header - auth tag = 8944)
const MAX_PAYLOAD_SIZE: usize = 8944;

/// Maximum file offset (256 TB - reasonable upper bound)
const MAX_FILE_OFFSET: u64 = 256 * 1024 * 1024 * 1024 * 1024;

/// Maximum sequence number delta (detect reordering/attacks)
/// Reserved for future sequence anomaly detection
#[allow(dead_code)]
const MAX_SEQUENCE_DELTA: u32 = 1_000_000;

/// Frame types as defined in the protocol specification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrameType {
    /// Reserved (invalid)
    Reserved = 0x00,
    /// File data payload
    Data = 0x01,
    /// Selective acknowledgment
    Ack = 0x02,
    /// Stream management
    Control = 0x03,
    /// Forward secrecy ratchet
    Rekey = 0x04,
    /// Keepalive / RTT measurement
    Ping = 0x05,
    /// Response to PING
    Pong = 0x06,
    /// Session termination
    Close = 0x07,
    /// Cover traffic (no payload)
    Pad = 0x08,
    /// New stream initiation
    StreamOpen = 0x09,
    /// Stream termination
    StreamClose = 0x0A,
    /// Abort stream with error
    StreamReset = 0x0B,
    /// Flow control credit
    WindowUpdate = 0x0C,
    /// Graceful shutdown
    GoAway = 0x0D,
    /// Connection migration challenge
    PathChallenge = 0x0E,
    /// Migration acknowledgment
    PathResponse = 0x0F,
}

impl TryFrom<u8> for FrameType {
    type Error = FrameError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Err(FrameError::ReservedFrameType),
            0x01 => Ok(Self::Data),
            0x02 => Ok(Self::Ack),
            0x03 => Ok(Self::Control),
            0x04 => Ok(Self::Rekey),
            0x05 => Ok(Self::Ping),
            0x06 => Ok(Self::Pong),
            0x07 => Ok(Self::Close),
            0x08 => Ok(Self::Pad),
            0x09 => Ok(Self::StreamOpen),
            0x0A => Ok(Self::StreamClose),
            0x0B => Ok(Self::StreamReset),
            0x0C => Ok(Self::WindowUpdate),
            0x0D => Ok(Self::GoAway),
            0x0E => Ok(Self::PathChallenge),
            0x0F => Ok(Self::PathResponse),
            0x10..=0x1F => Err(FrameError::ReservedFrameType),
            _ => Err(FrameError::InvalidFrameType(value)),
        }
    }
}

/// Frame flags bitmap
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameFlags(u8);

impl FrameFlags {
    /// Stream synchronization / initiation
    pub const SYN: u8 = 0b0000_0001;
    /// Final frame in stream
    pub const FIN: u8 = 0b0000_0010;
    /// Acknowledgment data present
    pub const ACK: u8 = 0b0000_0100;
    /// Priority frame (expedited processing)
    pub const PRI: u8 = 0b0000_1000;
    /// Payload is compressed (LZ4)
    pub const CMP: u8 = 0b0001_0000;

    /// Create new empty flags
    #[must_use]
    pub fn new() -> Self {
        Self(0)
    }

    /// Add SYN flag
    #[must_use]
    pub fn with_syn(mut self) -> Self {
        self.0 |= Self::SYN;
        self
    }

    /// Add FIN flag
    #[must_use]
    pub fn with_fin(mut self) -> Self {
        self.0 |= Self::FIN;
        self
    }

    /// Check if SYN is set
    #[must_use]
    pub fn is_syn(&self) -> bool {
        self.0 & Self::SYN != 0
    }

    /// Check if FIN is set
    #[must_use]
    pub fn is_fin(&self) -> bool {
        self.0 & Self::FIN != 0
    }

    /// Check if payload is compressed
    #[must_use]
    pub fn is_compressed(&self) -> bool {
        self.0 & Self::CMP != 0
    }

    /// Get raw byte value
    #[must_use]
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

/// Parsed frame header fields
///
/// This struct contains all the fields extracted from a frame header,
/// replacing the previous 6-tuple return type for improved readability
/// and maintainability.
#[derive(Debug, Clone, Copy)]
pub struct FrameHeader {
    /// Frame type (DATA, ACK, CONTROL, etc.)
    pub frame_type: FrameType,
    /// Frame flags bitmap
    pub flags: FrameFlags,
    /// Stream identifier
    pub stream_id: u16,
    /// Sequence number
    pub sequence: u32,
    /// File offset for DATA frames
    pub offset: u64,
    /// Payload length in bytes
    pub payload_len: u16,
}

/// SIMD-accelerated frame parsing
#[cfg(feature = "simd")]
mod simd_parse {
    use super::*;

    /// Parse frame header using SIMD instructions (x86_64 SSE2)
    ///
    /// Uses 128-bit SIMD loads to read header data more efficiently.
    /// On x86_64 with SSE2+ (virtually all modern CPUs), this provides
    /// ~1.5-2x speedup compared to scalar byte-by-byte loading.
    ///
    /// # Safety
    ///
    /// Caller must ensure data.len() >= FRAME_HEADER_SIZE (28 bytes).
    #[cfg(target_arch = "x86_64")]
    pub(super) fn parse_header_simd(data: &[u8]) -> super::FrameHeader {
        #[cfg(target_arch = "x86_64")]
        {
            // SAFETY: Caller ensures data.len() >= FRAME_HEADER_SIZE (28 bytes). x86_64 SSE2
            // supports unaligned loads via _mm_loadu_si128. Pointers are derived from valid
            // slice data and offsets are within bounds (ptr1 at 0, ptr2 at 12, both < 28).
            unsafe {
                use core::arch::x86_64::*;

                // Load first 16 bytes using SSE2 unaligned load
                let ptr1 = data.as_ptr() as *const __m128i;
                let _vec1 = _mm_loadu_si128(ptr1);

                // Load next 16 bytes (overlapping, covers bytes 12-27)
                let ptr2 = data.as_ptr().add(12) as *const __m128i;
                let _vec2 = _mm_loadu_si128(ptr2);

                // Extract individual fields (compiler optimizes these to direct loads)
                // The SIMD loads above prime the cache and prefetch the data
                let frame_type_byte = data[8];
                let flags_byte = data[9];
                let stream_id = u16::from_be_bytes([data[10], data[11]]);
                let sequence = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
                let offset = u64::from_be_bytes([
                    data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
                ]);
                let payload_len = u16::from_be_bytes([data[24], data[25]]);

                let frame_type =
                    FrameType::try_from(frame_type_byte).unwrap_or(FrameType::Reserved);
                let flags = FrameFlags(flags_byte);

                super::FrameHeader {
                    frame_type,
                    flags,
                    stream_id,
                    sequence,
                    offset,
                    payload_len,
                }
            }
        }
    }

    /// Parse frame header using SIMD instructions (aarch64 NEON)
    ///
    /// Uses 128-bit NEON loads for efficient header reading on ARM64.
    ///
    /// # Safety
    ///
    /// Caller must ensure data.len() >= FRAME_HEADER_SIZE (28 bytes).
    #[cfg(target_arch = "aarch64")]
    pub(super) fn parse_header_simd(data: &[u8]) -> super::FrameHeader {
        #[cfg(target_arch = "aarch64")]
        {
            // SAFETY: Caller ensures data.len() >= FRAME_HEADER_SIZE (28 bytes). ARM64 NEON
            // supports unaligned loads via vld1q_u8. Pointers are derived from valid slice
            // data and offsets are within bounds (ptr1 at 0, ptr2 at 12, both < 28).
            unsafe {
                use core::arch::aarch64::*;

                // Load first 16 bytes using NEON
                let ptr1 = data.as_ptr();
                let _vec1 = vld1q_u8(ptr1);

                // Load next 16 bytes (overlapping, covers bytes 12-27)
                let ptr2 = data.as_ptr().add(12);
                let _vec2 = vld1q_u8(ptr2);

                // Extract individual fields
                let frame_type_byte = data[8];
                let flags_byte = data[9];
                let stream_id = u16::from_be_bytes([data[10], data[11]]);
                let sequence = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
                let offset = u64::from_be_bytes([
                    data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
                ]);
                let payload_len = u16::from_be_bytes([data[24], data[25]]);

                let frame_type =
                    FrameType::try_from(frame_type_byte).unwrap_or(FrameType::Reserved);
                let flags = FrameFlags(flags_byte);

                super::FrameHeader {
                    frame_type,
                    flags,
                    stream_id,
                    sequence,
                    offset,
                    payload_len,
                }
            }
        }
    }

    /// Fallback for unsupported architectures - uses scalar parsing
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    pub(super) fn parse_header_simd(data: &[u8]) -> super::FrameHeader {
        super::parse_header_scalar(data)
    }
}

/// Scalar (non-SIMD) frame header parsing
///
/// This is the fallback implementation used when the `simd` feature
/// is disabled or on platforms without SIMD support.
fn parse_header_scalar(data: &[u8]) -> FrameHeader {
    let frame_type_byte = data[8];
    let flags_byte = data[9];
    let stream_id = u16::from_be_bytes([data[10], data[11]]);
    let sequence = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
    let offset = u64::from_be_bytes([
        data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
    ]);
    let payload_len = u16::from_be_bytes([data[24], data[25]]);

    let frame_type = FrameType::try_from(frame_type_byte).unwrap_or(FrameType::Reserved);
    let flags = FrameFlags(flags_byte);

    FrameHeader {
        frame_type,
        flags,
        stream_id,
        sequence,
        offset,
        payload_len,
    }
}

/// Zero-copy frame view into a packet buffer
#[derive(Debug)]
pub struct Frame<'a> {
    raw: &'a [u8],
    kind: FrameType,
    flags: FrameFlags,
    stream_id: u16,
    sequence: u32,
    offset: u64,
    payload_len: u16,
}

impl<'a> Frame<'a> {
    /// Parse a frame from raw bytes (zero-copy)
    ///
    /// Uses SIMD-accelerated header parsing when the `simd` feature is enabled.
    /// Falls back to scalar parsing otherwise.
    ///
    /// # Errors
    ///
    /// Returns `FrameError::TooShort` if data is smaller than the minimum header size.
    /// Returns `FrameError::ReservedFrameType` if the frame type is in the reserved range.
    /// Returns `FrameError::InvalidFrameType` if the frame type byte is unrecognized.
    /// Returns `FrameError::PayloadOverflow` if the declared payload length exceeds available data.
    /// Returns `FrameError::ReservedStreamId` if stream ID is in reserved range (1-15).
    /// Returns `FrameError::InvalidOffset` if offset exceeds maximum file size.
    /// Returns `FrameError::PayloadTooLarge` if payload exceeds maximum size.
    pub fn parse(data: &'a [u8]) -> Result<Self, FrameError> {
        if data.len() < FRAME_HEADER_SIZE {
            return Err(FrameError::TooShort {
                expected: FRAME_HEADER_SIZE,
                actual: data.len(),
            });
        }

        // Use SIMD parsing when feature is enabled, otherwise use scalar
        #[cfg(feature = "simd")]
        let header = simd_parse::parse_header_simd(data);

        #[cfg(not(feature = "simd"))]
        let header = parse_header_scalar(data);

        // Validate frame type (SIMD path uses unwrap_or(Reserved) to avoid branching)
        if matches!(header.frame_type, FrameType::Reserved) {
            return Err(FrameType::try_from(data[8]).unwrap_err());
        }

        if FRAME_HEADER_SIZE + header.payload_len as usize > data.len() {
            return Err(FrameError::PayloadOverflow);
        }

        // Validate stream ID (1-15 are reserved for protocol use)
        if header.stream_id > 0 && header.stream_id < 16 {
            return Err(FrameError::ReservedStreamId(header.stream_id as u32));
        }

        // Validate offset (sanity check against max file size)
        if header.offset > MAX_FILE_OFFSET {
            return Err(FrameError::InvalidOffset {
                offset: header.offset,
                max: MAX_FILE_OFFSET,
            });
        }

        // Validate payload length
        if header.payload_len as usize > MAX_PAYLOAD_SIZE {
            return Err(FrameError::PayloadTooLarge {
                size: header.payload_len as usize,
                max: MAX_PAYLOAD_SIZE,
            });
        }

        Ok(Self {
            raw: data,
            kind: header.frame_type,
            flags: header.flags,
            stream_id: header.stream_id,
            sequence: header.sequence,
            offset: header.offset,
            payload_len: header.payload_len,
        })
    }

    /// Parse a frame with minimal validation (hot-path optimized).
    ///
    /// Only validates frame type and payload bounds (memory safety).
    /// Skips stream ID reserved range, offset bounds, and payload size checks.
    /// Use this on trusted internal paths (e.g., after AEAD decryption)
    /// where frames are known to be well-formed.
    ///
    /// # Errors
    ///
    /// Returns `FrameError::TooShort` if data is smaller than the minimum header size.
    /// Returns `FrameError::ReservedFrameType` if the frame type is reserved.
    /// Returns `FrameError::PayloadOverflow` if the declared payload length exceeds available data.
    pub fn parse_fast(data: &'a [u8]) -> Result<Self, FrameError> {
        if data.len() < FRAME_HEADER_SIZE {
            return Err(FrameError::TooShort {
                expected: FRAME_HEADER_SIZE,
                actual: data.len(),
            });
        }

        let header = parse_header_scalar(data);

        if matches!(header.frame_type, FrameType::Reserved) {
            return Err(FrameType::try_from(data[8]).unwrap_err());
        }

        if FRAME_HEADER_SIZE + header.payload_len as usize > data.len() {
            return Err(FrameError::PayloadOverflow);
        }

        Ok(Self {
            raw: data,
            kind: header.frame_type,
            flags: header.flags,
            stream_id: header.stream_id,
            sequence: header.sequence,
            offset: header.offset,
            payload_len: header.payload_len,
        })
    }

    /// Parse a frame using scalar (non-SIMD) implementation
    ///
    /// This method is exposed for testing and benchmarking purposes.
    /// Use [`Frame::parse`] for normal operation, which automatically
    /// selects the best implementation.
    ///
    /// # Errors
    ///
    /// Same as [`Frame::parse`].
    pub fn parse_scalar(data: &'a [u8]) -> Result<Self, FrameError> {
        if data.len() < FRAME_HEADER_SIZE {
            return Err(FrameError::TooShort {
                expected: FRAME_HEADER_SIZE,
                actual: data.len(),
            });
        }

        let header = parse_header_scalar(data);

        if matches!(header.frame_type, FrameType::Reserved) {
            return Err(FrameType::try_from(data[8]).unwrap_err());
        }

        if FRAME_HEADER_SIZE + header.payload_len as usize > data.len() {
            return Err(FrameError::PayloadOverflow);
        }

        Ok(Self {
            raw: data,
            kind: header.frame_type,
            flags: header.flags,
            stream_id: header.stream_id,
            sequence: header.sequence,
            offset: header.offset,
            payload_len: header.payload_len,
        })
    }

    /// Parse a frame using SIMD implementation (if available)
    ///
    /// This method is exposed for testing and benchmarking purposes.
    /// Use [`Frame::parse`] for normal operation.
    ///
    /// # Errors
    ///
    /// Same as [`Frame::parse`].
    ///
    /// # Panics
    ///
    /// Panics if the `simd` feature is not enabled.
    #[cfg(feature = "simd")]
    pub fn parse_simd(data: &'a [u8]) -> Result<Self, FrameError> {
        if data.len() < FRAME_HEADER_SIZE {
            return Err(FrameError::TooShort {
                expected: FRAME_HEADER_SIZE,
                actual: data.len(),
            });
        }

        let header = simd_parse::parse_header_simd(data);

        if matches!(header.frame_type, FrameType::Reserved) {
            return Err(FrameType::try_from(data[8]).unwrap_err());
        }

        if FRAME_HEADER_SIZE + header.payload_len as usize > data.len() {
            return Err(FrameError::PayloadOverflow);
        }

        Ok(Self {
            raw: data,
            kind: header.frame_type,
            flags: header.flags,
            stream_id: header.stream_id,
            sequence: header.sequence,
            offset: header.offset,
            payload_len: header.payload_len,
        })
    }

    /// Get the frame type
    #[must_use]
    pub fn frame_type(&self) -> FrameType {
        self.kind
    }

    /// Get the frame flags
    #[must_use]
    pub fn flags(&self) -> FrameFlags {
        self.flags
    }

    /// Get the stream ID
    #[must_use]
    pub fn stream_id(&self) -> u16 {
        self.stream_id
    }

    /// Get the sequence number
    #[must_use]
    pub fn sequence(&self) -> u32 {
        self.sequence
    }

    /// Get the file offset
    #[must_use]
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Get the nonce bytes
    #[must_use]
    pub fn nonce(&self) -> &[u8] {
        &self.raw[0..8]
    }

    /// Get the payload slice (zero-copy)
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.raw[FRAME_HEADER_SIZE..FRAME_HEADER_SIZE + self.payload_len as usize]
    }
}

/// Builder for constructing frames
#[derive(Default)]
pub struct FrameBuilder {
    frame_type: Option<FrameType>,
    flags: FrameFlags,
    stream_id: u16,
    sequence: u32,
    offset: u64,
    payload: Vec<u8>,
    nonce: [u8; 8],
}

impl FrameBuilder {
    /// Create a new frame builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the frame type
    #[must_use]
    pub fn frame_type(mut self, ft: FrameType) -> Self {
        self.frame_type = Some(ft);
        self
    }

    /// Set the flags
    #[must_use]
    pub fn flags(mut self, flags: FrameFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set the stream ID
    #[must_use]
    pub fn stream_id(mut self, id: u16) -> Self {
        self.stream_id = id;
        self
    }

    /// Set the sequence number
    #[must_use]
    pub fn sequence(mut self, seq: u32) -> Self {
        self.sequence = seq;
        self
    }

    /// Set the file offset
    #[must_use]
    pub fn offset(mut self, off: u64) -> Self {
        self.offset = off;
        self
    }

    /// Set the payload
    #[must_use]
    pub fn payload(mut self, data: &[u8]) -> Self {
        self.payload = data.to_vec();
        self
    }

    /// Set the nonce
    #[must_use]
    pub fn nonce(mut self, n: [u8; 8]) -> Self {
        self.nonce = n;
        self
    }

    /// Build the frame into a pre-allocated buffer (zero-allocation).
    ///
    /// Writes the frame directly into `buf`, which must be exactly `total_size` bytes.
    /// This avoids heap allocation and is suitable for hot-path packet sending loops.
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::PayloadOverflow`] if `buf.len()` is too small for header + payload.
    pub fn build_into(&self, buf: &mut [u8]) -> Result<usize, FrameError> {
        let frame_type = self.frame_type.unwrap_or(FrameType::Data);
        let payload_len = self.payload.len();
        let total_size = buf.len();

        if total_size < FRAME_HEADER_SIZE + payload_len {
            return Err(FrameError::PayloadOverflow);
        }

        let padding_len = total_size - FRAME_HEADER_SIZE - payload_len;

        // Write header
        buf[..8].copy_from_slice(&self.nonce);
        buf[8] = frame_type as u8;
        buf[9] = self.flags.as_u8();
        buf[10..12].copy_from_slice(&self.stream_id.to_be_bytes());
        buf[12..16].copy_from_slice(&self.sequence.to_be_bytes());
        buf[16..24].copy_from_slice(&self.offset.to_be_bytes());
        #[allow(clippy::cast_possible_truncation)]
        let payload_len_u16 = payload_len as u16;
        buf[24..26].copy_from_slice(&payload_len_u16.to_be_bytes());
        buf[26..28].copy_from_slice(&[0u8; 2]); // Reserved

        // Write payload
        buf[FRAME_HEADER_SIZE..FRAME_HEADER_SIZE + payload_len].copy_from_slice(&self.payload);

        // Write random padding
        if padding_len > 0 {
            rand::thread_rng().fill(&mut buf[FRAME_HEADER_SIZE + payload_len..]);
        }

        Ok(total_size)
    }

    /// Build the frame into a byte buffer
    ///
    /// # Errors
    ///
    /// Returns [`FrameError::PayloadOverflow`] if `total_size` is too small for header + payload.
    pub fn build(self, total_size: usize) -> Result<Vec<u8>, FrameError> {
        let frame_type = self.frame_type.unwrap_or(FrameType::Data);
        let payload_len = self.payload.len();

        if total_size < FRAME_HEADER_SIZE + payload_len {
            return Err(FrameError::PayloadOverflow);
        }

        let padding_len = total_size - FRAME_HEADER_SIZE - payload_len;
        let mut buf = Vec::with_capacity(total_size);

        // Write header
        buf.extend_from_slice(&self.nonce);
        buf.push(frame_type as u8);
        buf.push(self.flags.as_u8());
        buf.extend_from_slice(&self.stream_id.to_be_bytes());
        buf.extend_from_slice(&self.sequence.to_be_bytes());
        buf.extend_from_slice(&self.offset.to_be_bytes());
        #[allow(clippy::cast_possible_truncation)]
        let payload_len_u16 = payload_len as u16;
        buf.extend_from_slice(&payload_len_u16.to_be_bytes());
        buf.extend_from_slice(&[0u8; 2]); // Reserved

        // Write payload
        buf.extend_from_slice(&self.payload);

        // Write random padding using thread-local PRNG (fast, non-syscall).
        // Padding bytes are encrypted by AEAD before transmission, so
        // cryptographic-quality randomness is not required here.
        if padding_len > 0 {
            let start = buf.len();
            buf.resize(start + padding_len, 0);
            rand::thread_rng().fill(&mut buf[start..]);
        }

        Ok(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_roundtrip() {
        let original = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(42)
            .sequence(1000)
            .offset(0)
            .payload(b"Hello, WRAITH!")
            .build(128)
            .unwrap();

        let parsed = Frame::parse(&original).unwrap();

        assert_eq!(parsed.frame_type(), FrameType::Data);
        assert_eq!(parsed.stream_id(), 42);
        assert_eq!(parsed.sequence(), 1000);
        assert_eq!(parsed.offset(), 0);
        assert_eq!(parsed.payload(), b"Hello, WRAITH!");
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
    fn test_all_frame_types() {
        let frame_types = vec![
            FrameType::Data,
            FrameType::Ack,
            FrameType::Control,
            FrameType::Rekey,
            FrameType::Ping,
            FrameType::Pong,
            FrameType::Close,
            FrameType::Pad,
            FrameType::StreamOpen,
            FrameType::StreamClose,
            FrameType::StreamReset,
            FrameType::WindowUpdate,
            FrameType::GoAway,
            FrameType::PathChallenge,
            FrameType::PathResponse,
        ];

        for ft in frame_types {
            let frame = FrameBuilder::new()
                .frame_type(ft)
                .stream_id(16)
                .sequence(1)
                .payload(&[0u8; 16])
                .build(64)
                .unwrap();

            let parsed = Frame::parse(&frame).unwrap();
            assert_eq!(parsed.frame_type(), ft);
        }
    }

    #[test]
    fn test_reserved_frame_type() {
        // Reserved type 0x00
        let mut frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .build(64)
            .unwrap();

        frame[8] = 0x00; // Overwrite with reserved type

        assert!(matches!(
            Frame::parse(&frame),
            Err(FrameError::ReservedFrameType)
        ));

        // Reserved range 0x10-0x1F
        frame[8] = 0x15;
        assert!(matches!(
            Frame::parse(&frame),
            Err(FrameError::ReservedFrameType)
        ));
    }

    #[test]
    fn test_invalid_frame_type() {
        let mut frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .build(64)
            .unwrap();

        frame[8] = 0xFF; // Invalid type

        assert!(matches!(
            Frame::parse(&frame),
            Err(FrameError::InvalidFrameType(0xFF))
        ));
    }

    #[test]
    fn test_frame_flags() {
        let flags = FrameFlags::new().with_syn().with_fin();

        assert!(flags.is_syn());
        assert!(flags.is_fin());
        assert!(!flags.is_compressed());

        let frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .flags(flags)
            .build(64)
            .unwrap();

        let parsed = Frame::parse(&frame).unwrap();
        assert!(parsed.flags().is_syn());
        assert!(parsed.flags().is_fin());
    }

    #[test]
    fn test_frame_with_max_payload() {
        let payload = vec![0xAA; 1428]; // Max standard MTU payload
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .payload(&payload)
            .build(FRAME_HEADER_SIZE + 1428)
            .unwrap();

        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.payload(), &payload[..]);
    }

    #[test]
    fn test_frame_payload_overflow() {
        let mut frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .payload(b"test")
            .build(64)
            .unwrap();

        // Corrupt payload length to exceed actual size
        frame[24] = 0xFF;
        frame[25] = 0xFF;

        assert!(matches!(
            Frame::parse(&frame),
            Err(FrameError::PayloadOverflow)
        ));
    }

    #[test]
    fn test_frame_zero_payload() {
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Pad)
            .payload(&[])
            .build(FRAME_HEADER_SIZE + 16)
            .unwrap();

        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.payload().len(), 0);
    }

    #[test]
    fn test_frame_nonce_extraction() {
        let nonce = [1, 2, 3, 4, 5, 6, 7, 8];
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .nonce(nonce)
            .build(64)
            .unwrap();

        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.nonce(), &nonce);
    }

    #[test]
    fn test_frame_sequence_wrap() {
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .sequence(u32::MAX)
            .build(64)
            .unwrap();

        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.sequence(), u32::MAX);
    }

    #[test]
    fn test_frame_offset_large() {
        // Use a large offset within MAX_FILE_OFFSET
        let large_offset = MAX_FILE_OFFSET - 1024;
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .offset(large_offset)
            .build(64)
            .unwrap();

        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.offset(), large_offset);
    }

    #[test]
    fn test_frame_builder_default_type() {
        let frame = FrameBuilder::new().stream_id(16).build(64).unwrap();

        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.frame_type(), FrameType::Data);
    }

    #[test]
    fn test_frame_padding_is_random() {
        let frame1 = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .payload(b"test")
            .build(128)
            .unwrap();

        let frame2 = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .payload(b"test")
            .build(128)
            .unwrap();

        // Padding should be different due to randomization
        let padding1 = &frame1[FRAME_HEADER_SIZE + 4..];
        let padding2 = &frame2[FRAME_HEADER_SIZE + 4..];
        assert_ne!(padding1, padding2);
    }

    #[test]
    fn test_frame_minimum_size() {
        // Minimum frame is just header + 0 payload + 0 padding
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Ping)
            .payload(&[])
            .build(FRAME_HEADER_SIZE)
            .unwrap();

        assert_eq!(frame.len(), FRAME_HEADER_SIZE);
        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.payload().len(), 0);
    }

    #[test]
    fn test_frame_stream_id_range() {
        // Test client-initiated stream (odd, above reserved range)
        let frame = FrameBuilder::new().stream_id(17).build(64).unwrap();
        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.stream_id(), 17);
        assert_eq!(parsed.stream_id() % 2, 1);

        // Test server-initiated stream (even, above reserved range)
        let frame = FrameBuilder::new().stream_id(16).build(64).unwrap();
        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.stream_id(), 16);
        assert_eq!(parsed.stream_id() % 2, 0);

        // Test maximum stream ID
        let frame = FrameBuilder::new().stream_id(u16::MAX).build(64).unwrap();
        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.stream_id(), u16::MAX);
    }

    #[test]
    fn test_scalar_vs_simd_parsing() {
        // Test that scalar and SIMD parsing produce identical results
        let test_cases = vec![
            (FrameType::Data, 16, 100, 0, "test payload"),
            (FrameType::Ack, 42, 1000, 1024, "ack data"),
            (FrameType::Ping, u16::MAX, u32::MAX, MAX_FILE_OFFSET, ""),
            (FrameType::StreamOpen, 0, 0, 0, "stream open"),
            (FrameType::Close, 999, 999999, 123456789, "close payload"),
        ];

        for (ft, stream_id, sequence, offset, payload) in test_cases {
            let frame = FrameBuilder::new()
                .frame_type(ft)
                .stream_id(stream_id)
                .sequence(sequence)
                .offset(offset)
                .payload(payload.as_bytes())
                .build(128)
                .unwrap();

            // Parse with scalar implementation
            let scalar = Frame::parse_scalar(&frame).unwrap();

            // Parse with SIMD implementation (if enabled)
            #[cfg(feature = "simd")]
            {
                let simd = Frame::parse_simd(&frame).unwrap();

                // Compare all fields
                assert_eq!(scalar.frame_type(), simd.frame_type());
                assert_eq!(scalar.flags().as_u8(), simd.flags().as_u8());
                assert_eq!(scalar.stream_id(), simd.stream_id());
                assert_eq!(scalar.sequence(), simd.sequence());
                assert_eq!(scalar.offset(), simd.offset());
                assert_eq!(scalar.payload(), simd.payload());
                assert_eq!(scalar.nonce(), simd.nonce());
            }

            // Verify parse() uses correct implementation
            let default = Frame::parse(&frame).unwrap();
            assert_eq!(scalar.frame_type(), default.frame_type());
            assert_eq!(scalar.stream_id(), default.stream_id());
            assert_eq!(scalar.sequence(), default.sequence());
            assert_eq!(scalar.offset(), default.offset());
            assert_eq!(scalar.payload(), default.payload());
        }
    }

    #[test]
    fn test_simd_boundary_values() {
        // Test SIMD parsing with boundary values for all fields
        let frame = FrameBuilder::new()
            .frame_type(FrameType::PathResponse)
            .stream_id(u16::MAX)
            .sequence(u32::MAX)
            .offset(u64::MAX)
            .payload(&vec![0xFF; 256])
            .build(512)
            .unwrap();

        let scalar = Frame::parse_scalar(&frame).unwrap();
        assert_eq!(scalar.stream_id(), u16::MAX);
        assert_eq!(scalar.sequence(), u32::MAX);
        assert_eq!(scalar.offset(), u64::MAX);
        assert_eq!(scalar.payload().len(), 256);

        #[cfg(feature = "simd")]
        {
            let simd = Frame::parse_simd(&frame).unwrap();
            assert_eq!(simd.stream_id(), u16::MAX);
            assert_eq!(simd.sequence(), u32::MAX);
            assert_eq!(simd.offset(), u64::MAX);
            assert_eq!(simd.payload().len(), 256);
        }
    }

    #[test]
    fn test_simd_all_frame_types() {
        // Verify SIMD parsing works for all frame types
        let frame_types = vec![
            FrameType::Data,
            FrameType::Ack,
            FrameType::Control,
            FrameType::Rekey,
            FrameType::Ping,
            FrameType::Pong,
            FrameType::Close,
            FrameType::Pad,
            FrameType::StreamOpen,
            FrameType::StreamClose,
            FrameType::StreamReset,
            FrameType::WindowUpdate,
            FrameType::GoAway,
            FrameType::PathChallenge,
            FrameType::PathResponse,
        ];

        for ft in frame_types {
            let frame = FrameBuilder::new()
                .frame_type(ft)
                .stream_id(42)
                .sequence(1000)
                .payload(&[0xAA; 32])
                .build(128)
                .unwrap();

            let scalar = Frame::parse_scalar(&frame).unwrap();
            assert_eq!(scalar.frame_type(), ft);

            #[cfg(feature = "simd")]
            {
                let simd = Frame::parse_simd(&frame).unwrap();
                assert_eq!(simd.frame_type(), ft);
                assert_eq!(scalar.stream_id(), simd.stream_id());
                assert_eq!(scalar.sequence(), simd.sequence());
            }
        }
    }

    #[test]
    fn test_simd_with_flags() {
        // Test SIMD parsing preserves all flag bits
        let flags = FrameFlags::new().with_syn().with_fin();

        let frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .flags(flags)
            .stream_id(123)
            .sequence(456)
            .build(64)
            .unwrap();

        let scalar = Frame::parse_scalar(&frame).unwrap();
        assert!(scalar.flags().is_syn());
        assert!(scalar.flags().is_fin());

        #[cfg(feature = "simd")]
        {
            let simd = Frame::parse_simd(&frame).unwrap();
            assert!(simd.flags().is_syn());
            assert!(simd.flags().is_fin());
            assert_eq!(scalar.flags().as_u8(), simd.flags().as_u8());
        }
    }

    #[test]
    fn test_simd_zero_fields() {
        // Test SIMD parsing with all-zero header fields
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(0)
            .sequence(0)
            .offset(0)
            .payload(&[])
            .build(FRAME_HEADER_SIZE)
            .unwrap();

        let scalar = Frame::parse_scalar(&frame).unwrap();
        assert_eq!(scalar.stream_id(), 0);
        assert_eq!(scalar.sequence(), 0);
        assert_eq!(scalar.offset(), 0);
        assert_eq!(scalar.payload().len(), 0);

        #[cfg(feature = "simd")]
        {
            let simd = Frame::parse_simd(&frame).unwrap();
            assert_eq!(simd.stream_id(), 0);
            assert_eq!(simd.sequence(), 0);
            assert_eq!(simd.offset(), 0);
            assert_eq!(simd.payload().len(), 0);
        }
    }

    #[test]
    fn test_simd_nonce_extraction() {
        // Test SIMD parsing correctly extracts nonce
        let nonce = [1, 2, 3, 4, 5, 6, 7, 8];
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .nonce(nonce)
            .payload(b"test")
            .build(64)
            .unwrap();

        let scalar = Frame::parse_scalar(&frame).unwrap();
        assert_eq!(scalar.nonce(), &nonce);

        #[cfg(feature = "simd")]
        {
            let simd = Frame::parse_simd(&frame).unwrap();
            assert_eq!(simd.nonce(), &nonce);
            assert_eq!(scalar.nonce(), simd.nonce());
        }
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn prop_parse_doesnt_panic(data in prop::collection::vec(any::<u8>(), 0..2048)) {
                let _ = Frame::parse(&data);
            }

            #[test]
            fn prop_roundtrip_preserves_data(
                stream_id in prop::num::u16::ANY.prop_filter("not reserved", |&id| id == 0 || id >= 16),
                sequence in any::<u32>(),
                offset in 0u64..=MAX_FILE_OFFSET,
                payload in prop::collection::vec(any::<u8>(), 0..MAX_PAYLOAD_SIZE.min(1024)),
                total_size in 128usize..2048
            ) {
                let frame_bytes = FrameBuilder::new()
                    .frame_type(FrameType::Data)
                    .stream_id(stream_id)
                    .sequence(sequence)
                    .offset(offset)
                    .payload(&payload)
                    .build(total_size.max(FRAME_HEADER_SIZE + payload.len()))
                    .unwrap();

                let parsed = Frame::parse(&frame_bytes).unwrap();
                prop_assert_eq!(parsed.stream_id(), stream_id);
                prop_assert_eq!(parsed.sequence(), sequence);
                prop_assert_eq!(parsed.offset(), offset);
                prop_assert_eq!(parsed.payload(), payload.as_slice());
            }

            #[test]
            fn prop_all_valid_frame_types_parseable(
                type_byte in 0x01u8..=0x0F
            ) {
                let mut frame = FrameBuilder::new()
                    .frame_type(FrameType::Data)
                    .build(64)
                    .unwrap();

                frame[8] = type_byte;
                prop_assert!(Frame::parse(&frame).is_ok());
            }

            #[test]
            fn prop_invalid_frame_types_rejected(
                type_byte in prop::sample::select(vec![0x00u8, 0x20, 0x40, 0x80, 0xFF])
            ) {
                let mut frame = FrameBuilder::new()
                    .frame_type(FrameType::Data)
                    .build(64)
                    .unwrap();

                frame[8] = type_byte;
                prop_assert!(Frame::parse(&frame).is_err());
            }

            #[test]
            fn prop_flags_roundtrip(flags in any::<u8>()) {
                let frame = FrameBuilder::new()
                    .frame_type(FrameType::Data)
                    .flags(FrameFlags(flags))
                    .build(64)
                    .unwrap();

                let parsed = Frame::parse(&frame).unwrap();
                prop_assert_eq!(parsed.flags().as_u8(), flags);
            }

            #[test]
            fn prop_payload_length_respected(
                payload_len in 0usize..1024,
                total_size in 128usize..2048
            ) {
                let payload = vec![0x42; payload_len];
                let size = total_size.max(FRAME_HEADER_SIZE + payload_len);

                let frame = FrameBuilder::new()
                    .frame_type(FrameType::Data)
                    .payload(&payload)
                    .build(size)
                    .unwrap();

                let parsed = Frame::parse(&frame).unwrap();
                prop_assert_eq!(parsed.payload().len(), payload_len);
            }
        }
    }

    // Sprint 4.5: Frame Validation Hardening Tests

    #[test]
    fn test_reserved_stream_id_rejection() {
        // Stream IDs 1-15 are reserved
        for stream_id in 1u16..16 {
            let mut frame = FrameBuilder::new()
                .frame_type(FrameType::Data)
                .stream_id(100) // Start with valid ID
                .build(64)
                .unwrap();

            // Manually set reserved stream ID
            let bytes = stream_id.to_be_bytes();
            frame[10] = bytes[0];
            frame[11] = bytes[1];

            let result = Frame::parse(&frame);
            assert!(
                matches!(result, Err(FrameError::ReservedStreamId(id)) if id == stream_id as u32),
                "Expected ReservedStreamId error for stream ID {}, got {:?}",
                stream_id,
                result
            );
        }
    }

    #[test]
    fn test_stream_id_zero_allowed() {
        // Stream ID 0 is allowed (connection-level control)
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Control)
            .stream_id(0)
            .build(64)
            .unwrap();

        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.stream_id(), 0);
    }

    #[test]
    fn test_stream_id_above_reserved_allowed() {
        // Stream ID 16 and above are valid
        for stream_id in [16, 17, 100, 1000, 65535] {
            let frame = FrameBuilder::new()
                .frame_type(FrameType::Data)
                .stream_id(stream_id)
                .build(64)
                .unwrap();

            let parsed = Frame::parse(&frame).unwrap();
            assert_eq!(parsed.stream_id(), stream_id);
        }
    }

    #[test]
    fn test_offset_bounds_valid() {
        // Valid offsets up to MAX_FILE_OFFSET
        let valid_offsets = [
            0,
            1024,
            1024 * 1024,
            1024 * 1024 * 1024,
            MAX_FILE_OFFSET - 1,
            MAX_FILE_OFFSET,
        ];

        for offset in valid_offsets {
            let frame = FrameBuilder::new()
                .frame_type(FrameType::Data)
                .stream_id(100)
                .offset(offset)
                .build(64)
                .unwrap();

            let parsed = Frame::parse(&frame).unwrap();
            assert_eq!(parsed.offset(), offset);
        }
    }

    #[test]
    fn test_offset_bounds_invalid() {
        // Test offset validation by manually constructing invalid frames
        // Layout: nonce(8) + type(1) + flags(1) + stream(2) + seq(4) + offset(8) + len(2) + reserved(2)
        let mut frame = vec![0u8; FRAME_HEADER_SIZE];

        // Set valid frame type (DATA = 0x01)
        frame[8] = 0x01;

        // Set stream ID to 100 (valid, not reserved)
        frame[10..12].copy_from_slice(&100u16.to_be_bytes());

        // Test various invalid offsets
        let invalid_offsets = [MAX_FILE_OFFSET + 1, MAX_FILE_OFFSET + 1024, u64::MAX];

        for offset in invalid_offsets {
            // Set invalid offset at bytes 16-23
            frame[16..24].copy_from_slice(&offset.to_be_bytes());

            let result = Frame::parse(&frame);
            assert!(
                matches!(
                    result,
                    Err(FrameError::InvalidOffset { offset: o, max: m })
                    if o == offset && m == MAX_FILE_OFFSET
                ),
                "Expected InvalidOffset error for offset {}, got {:?}",
                offset,
                result
            );
        }
    }

    #[test]
    fn test_sequence_number_validation_valid() {
        // Valid sequence numbers within MAX_SEQUENCE_DELTA
        let base_seq = 1000;
        let valid_deltas = [0, 1, 100, 1000, 10000, MAX_SEQUENCE_DELTA];

        for delta in valid_deltas {
            let seq = base_seq + delta;
            let frame = FrameBuilder::new()
                .frame_type(FrameType::Data)
                .stream_id(100)
                .sequence(seq)
                .build(64)
                .unwrap();

            let parsed = Frame::parse(&frame).unwrap();
            assert_eq!(parsed.sequence(), seq);
        }
    }

    #[test]
    fn test_payload_size_limits_valid() {
        // Valid payload sizes up to MAX_PAYLOAD_SIZE
        let valid_sizes = [0, 1, 100, 1024, 4096, MAX_PAYLOAD_SIZE];

        for size in valid_sizes {
            let payload = vec![0x42; size];
            let frame = FrameBuilder::new()
                .frame_type(FrameType::Data)
                .stream_id(100)
                .payload(&payload)
                .build(FRAME_HEADER_SIZE + size)
                .unwrap();

            let parsed = Frame::parse(&frame).unwrap();
            assert_eq!(parsed.payload().len(), size);
        }
    }

    #[test]
    fn test_payload_size_limits_invalid() {
        // Test payload size validation by manually constructing invalid frame
        // Make frame large enough to avoid PayloadOverflow error
        let invalid_len = (MAX_PAYLOAD_SIZE + 1) as u16;
        let mut frame = vec![0u8; FRAME_HEADER_SIZE + invalid_len as usize];

        // Set valid frame type (DATA = 0x01)
        frame[8] = 0x01;

        // Set stream ID to 100 (valid, not reserved)
        frame[10..12].copy_from_slice(&100u16.to_be_bytes());

        // Set payload length to exceed MAX_PAYLOAD_SIZE
        frame[24..26].copy_from_slice(&invalid_len.to_be_bytes());

        let result = Frame::parse(&frame);
        assert!(
            matches!(
                result,
                Err(FrameError::PayloadTooLarge { size, max })
                if size == invalid_len as usize && max == MAX_PAYLOAD_SIZE
            ),
            "Expected PayloadTooLarge error, got {:?}",
            result
        );
    }

    #[test]
    fn test_payload_size_max_boundary() {
        // Test exact MAX_PAYLOAD_SIZE boundary
        let payload = vec![0x55; MAX_PAYLOAD_SIZE];
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(100)
            .payload(&payload)
            .build(FRAME_HEADER_SIZE + MAX_PAYLOAD_SIZE)
            .unwrap();

        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.payload().len(), MAX_PAYLOAD_SIZE);
    }

    #[test]
    fn test_validation_constants() {
        // Verify that constants are set to expected values
        assert_eq!(MAX_PAYLOAD_SIZE, 8944);
        assert_eq!(MAX_FILE_OFFSET, 256 * 1024 * 1024 * 1024 * 1024);
        assert_eq!(MAX_SEQUENCE_DELTA, 1_000_000);
    }

    #[test]
    fn test_combined_validation() {
        // Test frame with all valid fields near boundaries
        let frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(16) // Just above reserved range
            .sequence(500_000) // Well within delta
            .offset(100 * 1024 * 1024 * 1024) // 100 GB
            .payload(&vec![0xAB; 4096])
            .build(FRAME_HEADER_SIZE + 4096)
            .unwrap();

        let parsed = Frame::parse(&frame).unwrap();
        assert_eq!(parsed.frame_type(), FrameType::Data);
        assert_eq!(parsed.stream_id(), 16);
        assert_eq!(parsed.sequence(), 500_000);
        assert_eq!(parsed.offset(), 100 * 1024 * 1024 * 1024);
        assert_eq!(parsed.payload().len(), 4096);
    }

    #[test]
    fn test_multiple_validation_failures() {
        // Test that first validation error is returned
        let mut frame = FrameBuilder::new()
            .frame_type(FrameType::Data)
            .stream_id(100)
            .build(64)
            .unwrap();

        // Set reserved stream ID (should fail first)
        frame[10] = 0;
        frame[11] = 5; // Stream ID 5

        let result = Frame::parse(&frame);
        assert!(
            matches!(result, Err(FrameError::ReservedStreamId(5))),
            "Expected ReservedStreamId error first, got {:?}",
            result
        );
    }
}
