//! v1/v2 wire format compatibility layer.
//!
//! Provides format detection, negotiation, and a unified encoding/decoding
//! interface that handles both v1 and v2 frame headers transparently.

use super::header_v2::FrameHeaderV2;
use super::types_v2::{FRAME_HEADER_V2_SIZE, FlagsV2, FrameTypeV2, PROTOCOL_VERSION_V2};
use super::{FrameFlags, FrameHeader, FrameType};
use crate::FRAME_HEADER_SIZE;
use crate::error::FrameError;

/// Wire format version in use for a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireFormat {
    /// v1 format: 28-byte header, big-endian, 8-byte nonce prefix.
    V1,
    /// v2 format: 24-byte header, little-endian, expanded fields.
    V2,
    /// v2 with polymorphic encoding: field positions and XOR mask derived from session.
    V2Polymorphic,
}

impl WireFormat {
    /// Get the header size for this wire format.
    #[must_use]
    pub const fn header_size(self) -> usize {
        match self {
            Self::V1 => FRAME_HEADER_SIZE,
            Self::V2 | Self::V2Polymorphic => FRAME_HEADER_V2_SIZE,
        }
    }

    /// Check if this is a v2 variant.
    #[must_use]
    pub const fn is_v2(self) -> bool {
        matches!(self, Self::V2 | Self::V2Polymorphic)
    }
}

/// Format negotiation preference.
///
/// Used during handshake to agree on the wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatNegotiation {
    /// Preferred format (highest priority).
    pub preferred: WireFormat,
    /// Whether v1 format is acceptable as fallback.
    pub allow_v1: bool,
    /// Whether plain v2 (non-polymorphic) is acceptable.
    pub allow_v2_plain: bool,
}

impl Default for FormatNegotiation {
    fn default() -> Self {
        Self {
            preferred: WireFormat::V2Polymorphic,
            allow_v1: true,
            allow_v2_plain: true,
        }
    }
}

impl FormatNegotiation {
    /// Create a v2-only negotiation (no v1 fallback).
    #[must_use]
    pub fn v2_only() -> Self {
        Self {
            preferred: WireFormat::V2Polymorphic,
            allow_v1: false,
            allow_v2_plain: true,
        }
    }

    /// Create a v1-only negotiation (compatibility mode).
    #[must_use]
    pub fn v1_only() -> Self {
        Self {
            preferred: WireFormat::V1,
            allow_v1: true,
            allow_v2_plain: false,
        }
    }

    /// Negotiate the best common format between local and remote preferences.
    ///
    /// Returns `None` if no common format exists.
    #[must_use]
    pub fn negotiate(&self, remote: &FormatNegotiation) -> Option<WireFormat> {
        // Try preferred formats first (highest to lowest)
        let candidates = [WireFormat::V2Polymorphic, WireFormat::V2, WireFormat::V1];

        candidates
            .iter()
            .copied()
            .find(|&fmt| self.supports(fmt) && remote.supports(fmt))
    }

    /// Check if this negotiation supports a given format.
    #[must_use]
    pub fn supports(&self, fmt: WireFormat) -> bool {
        match fmt {
            WireFormat::V1 => self.allow_v1,
            WireFormat::V2 => self.allow_v2_plain || self.preferred == WireFormat::V2,
            WireFormat::V2Polymorphic => {
                self.preferred == WireFormat::V2Polymorphic || self.allow_v2_plain
            }
        }
    }
}

/// Detect the wire format version from a packet's first bytes.
///
/// Heuristic detection based on byte patterns:
/// - v2 starts with version byte 0x20
/// - v1 starts with an 8-byte nonce (usually non-zero random), followed by
///   a frame type byte in range 0x01-0x0F
///
/// For polymorphic v2, detection is not possible from bytes alone (requires
/// the session format key). This function returns `V2` for any v2 header.
///
/// Returns `None` if the format cannot be determined.
#[must_use]
pub fn detect_format(data: &[u8]) -> Option<WireFormat> {
    if data.len() < 2 {
        return None;
    }

    // Check for v2: first byte is the version byte 0x20
    if data[0] == PROTOCOL_VERSION_V2 && data.len() >= FRAME_HEADER_V2_SIZE {
        // Verify the second byte is a valid v2 frame type
        if FrameTypeV2::is_valid_byte(data[1]) {
            return Some(WireFormat::V2);
        }
    }

    // Check for v1: byte 8 is the frame type (0x01-0x0F)
    if data.len() >= FRAME_HEADER_SIZE {
        let frame_type_byte = data[8];
        if (0x01..=0x0F).contains(&frame_type_byte) {
            return Some(WireFormat::V1);
        }
    }

    None
}

/// Encode a v1 frame header into a 28-byte buffer.
///
/// Layout: Nonce(8B) + FrameType(1B) + Flags(1B) + StreamID(2B) + Sequence(4B)
///         + Offset(8B) + PayloadLen(2B) + Reserved(2B)
pub fn encode_v1_header(header: &FrameHeader, nonce: &[u8; 8], buf: &mut [u8]) {
    buf[..8].copy_from_slice(nonce);
    buf[8] = header.frame_type as u8;
    buf[9] = header.flags.as_u8();
    buf[10..12].copy_from_slice(&header.stream_id.to_be_bytes());
    buf[12..16].copy_from_slice(&header.sequence.to_be_bytes());
    buf[16..24].copy_from_slice(&header.offset.to_be_bytes());
    buf[24..26].copy_from_slice(&header.payload_len.to_be_bytes());
    buf[26..28].copy_from_slice(&[0u8; 2]); // Reserved
}

/// Decode a v1 frame header from a byte buffer.
///
/// # Errors
///
/// Returns errors for invalid frame types.
pub fn decode_v1_header(buf: &[u8]) -> Result<(FrameHeader, [u8; 8]), FrameError> {
    if buf.len() < FRAME_HEADER_SIZE {
        return Err(FrameError::TooShort {
            expected: FRAME_HEADER_SIZE,
            actual: buf.len(),
        });
    }

    let mut nonce = [0u8; 8];
    nonce.copy_from_slice(&buf[..8]);

    let frame_type = FrameType::try_from(buf[8])?;
    let flags = FrameFlags(buf[9]);
    let stream_id = u16::from_be_bytes([buf[10], buf[11]]);
    let sequence = u32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]);
    let offset = u64::from_be_bytes([
        buf[16], buf[17], buf[18], buf[19], buf[20], buf[21], buf[22], buf[23],
    ]);
    let payload_len = u16::from_be_bytes([buf[24], buf[25]]);

    Ok((
        FrameHeader {
            frame_type,
            flags,
            stream_id,
            sequence,
            offset,
            payload_len,
        },
        nonce,
    ))
}

/// Convert a v1 `FrameHeader` to a v2 `FrameHeaderV2`.
///
/// Maps v1 fields to v2 equivalents, expanding field widths.
#[must_use]
pub fn v1_header_to_v2(v1: &FrameHeader) -> FrameHeaderV2 {
    FrameHeaderV2 {
        version: PROTOCOL_VERSION_V2,
        frame_type: FrameTypeV2::from(v1.frame_type),
        flags: FlagsV2::from(v1.flags),
        sequence: u64::from(v1.sequence),
        length: u32::from(v1.payload_len),
        stream_id: u32::from(v1.stream_id),
        reserved: 0,
    }
}

/// Convert a v2 `FrameHeaderV2` to a v1 `FrameHeader`.
///
/// Truncates fields that don't fit in v1's smaller widths.
/// Returns `None` if the v2 frame type has no v1 equivalent.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn v2_header_to_v1(v2: &FrameHeaderV2) -> Option<FrameHeader> {
    let frame_type = match v2.frame_type {
        FrameTypeV2::Data | FrameTypeV2::DataFin => FrameType::Data,
        FrameTypeV2::Ack | FrameTypeV2::AckEcn => FrameType::Ack,
        FrameTypeV2::Ping => FrameType::Ping,
        FrameTypeV2::Pong => FrameType::Pong,
        FrameTypeV2::Rekey | FrameTypeV2::RekeyAck => FrameType::Rekey,
        FrameTypeV2::StreamOpen => FrameType::StreamOpen,
        FrameTypeV2::StreamClose => FrameType::StreamClose,
        FrameTypeV2::StreamReset => FrameType::StreamReset,
        FrameTypeV2::WindowUpdate | FrameTypeV2::StreamWindow => FrameType::WindowUpdate,
        FrameTypeV2::GoAway => FrameType::GoAway,
        FrameTypeV2::PathChallenge => FrameType::PathChallenge,
        FrameTypeV2::PathResponse => FrameType::PathResponse,
        FrameTypeV2::Close | FrameTypeV2::CloseAck => FrameType::Close,
        FrameTypeV2::Padding | FrameTypeV2::PaddingRandom => FrameType::Pad,
        // Types with no v1 equivalent
        _ => return None,
    };

    Some(FrameHeader {
        frame_type,
        flags: FrameFlags(v2.flags.bits() as u8),
        stream_id: v2.stream_id as u16,
        sequence: v2.sequence as u32,
        offset: 0, // v2 doesn't have offset in header
        payload_len: v2.length as u16,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wire_format_header_size() {
        assert_eq!(WireFormat::V1.header_size(), 28);
        assert_eq!(WireFormat::V2.header_size(), 24);
        assert_eq!(WireFormat::V2Polymorphic.header_size(), 24);
    }

    #[test]
    fn test_wire_format_is_v2() {
        assert!(!WireFormat::V1.is_v2());
        assert!(WireFormat::V2.is_v2());
        assert!(WireFormat::V2Polymorphic.is_v2());
    }

    #[test]
    fn test_detect_format_v1() {
        // Build a v1 frame: 8 bytes nonce + frame type at byte 8
        let mut data = vec![0u8; 28];
        data[8] = 0x01; // Data frame type
        assert_eq!(detect_format(&data), Some(WireFormat::V1));
    }

    #[test]
    fn test_detect_format_v2() {
        let mut data = vec![0u8; 24];
        data[0] = PROTOCOL_VERSION_V2; // v2 version byte
        data[1] = 0x00; // Data frame type (valid v2)
        assert_eq!(detect_format(&data), Some(WireFormat::V2));
    }

    #[test]
    fn test_detect_format_too_short() {
        assert_eq!(detect_format(&[]), None);
        assert_eq!(detect_format(&[0x20]), None);
    }

    #[test]
    fn test_detect_format_ambiguous() {
        // Data that doesn't clearly match either format
        let data = vec![0u8; 28];
        // Nonce all zeros, frame type byte 0x00 (reserved in v1)
        assert_eq!(detect_format(&data), None);
    }

    #[test]
    fn test_v1_header_encode_decode_roundtrip() {
        let header = FrameHeader {
            frame_type: FrameType::Data,
            flags: FrameFlags::new().with_syn(),
            stream_id: 42,
            sequence: 1000,
            offset: 8192,
            payload_len: 1400,
        };
        let nonce = [1, 2, 3, 4, 5, 6, 7, 8];

        let mut buf = [0u8; 28];
        encode_v1_header(&header, &nonce, &mut buf);
        let (decoded, decoded_nonce) = decode_v1_header(&buf).unwrap();

        assert_eq!(decoded.frame_type, header.frame_type);
        assert_eq!(decoded.flags.as_u8(), header.flags.as_u8());
        assert_eq!(decoded.stream_id, header.stream_id);
        assert_eq!(decoded.sequence, header.sequence);
        assert_eq!(decoded.offset, header.offset);
        assert_eq!(decoded.payload_len, header.payload_len);
        assert_eq!(decoded_nonce, nonce);
    }

    #[test]
    fn test_v1_header_decode_too_short() {
        let buf = [0u8; 10];
        assert!(matches!(
            decode_v1_header(&buf),
            Err(FrameError::TooShort { .. })
        ));
    }

    #[test]
    fn test_v1_to_v2_conversion() {
        let v1 = FrameHeader {
            frame_type: FrameType::Data,
            flags: FrameFlags::new().with_syn().with_fin(),
            stream_id: 42,
            sequence: 1000,
            offset: 8192,
            payload_len: 1400,
        };

        let v2 = v1_header_to_v2(&v1);
        assert_eq!(v2.version, PROTOCOL_VERSION_V2);
        assert_eq!(v2.frame_type, FrameTypeV2::Data);
        assert!(v2.flags.is_syn());
        assert!(v2.flags.is_fin());
        assert_eq!(v2.sequence, 1000);
        assert_eq!(v2.length, 1400);
        assert_eq!(v2.stream_id, 42);
    }

    #[test]
    fn test_v2_to_v1_conversion() {
        let v2 = FrameHeaderV2 {
            version: PROTOCOL_VERSION_V2,
            frame_type: FrameTypeV2::Data,
            flags: FlagsV2::empty().with(FlagsV2::SYN),
            sequence: 1000,
            length: 1400,
            stream_id: 42,
            reserved: 0,
        };

        let v1 = v2_header_to_v1(&v2).unwrap();
        assert_eq!(v1.frame_type, FrameType::Data);
        assert!(v1.flags.is_syn());
        assert_eq!(v1.sequence, 1000);
        assert_eq!(v1.payload_len, 1400);
        assert_eq!(v1.stream_id, 42);
    }

    #[test]
    fn test_v2_to_v1_no_equivalent() {
        let v2 = FrameHeaderV2 {
            frame_type: FrameTypeV2::Datagram,
            ..FrameHeaderV2::new(FrameTypeV2::Datagram)
        };
        assert!(v2_header_to_v1(&v2).is_none());
    }

    #[test]
    fn test_format_negotiation_default() {
        let local = FormatNegotiation::default();
        let remote = FormatNegotiation::default();
        assert_eq!(local.negotiate(&remote), Some(WireFormat::V2Polymorphic));
    }

    #[test]
    fn test_format_negotiation_v1_only() {
        let local = FormatNegotiation::v1_only();
        let remote = FormatNegotiation::default();
        assert_eq!(local.negotiate(&remote), Some(WireFormat::V1));
    }

    #[test]
    fn test_format_negotiation_v2_only_vs_v1_only() {
        let local = FormatNegotiation::v2_only();
        let remote = FormatNegotiation::v1_only();
        // v2-only doesn't support V1, v1-only doesn't support V2
        assert_eq!(local.negotiate(&remote), None);
    }

    #[test]
    fn test_format_negotiation_v2_only() {
        let local = FormatNegotiation::v2_only();
        let remote = FormatNegotiation::v2_only();
        assert_eq!(local.negotiate(&remote), Some(WireFormat::V2Polymorphic));
    }

    #[test]
    fn test_all_v1_types_convert_to_v2() {
        let v1_types = [
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

        for ft in v1_types {
            let v1 = FrameHeader {
                frame_type: ft,
                flags: FrameFlags::new(),
                stream_id: 16,
                sequence: 1,
                offset: 0,
                payload_len: 0,
            };
            let v2 = v1_header_to_v2(&v1);
            // Should be a valid v2 header
            let encoded = v2.encode();
            assert!(FrameHeaderV2::decode(&encoded).is_ok());
        }
    }

    #[test]
    fn test_v2_header_via_compat_encode_decode() {
        // Test the full path: create v2 header, encode, detect format, decode
        let h = FrameHeaderV2::new(FrameTypeV2::Ack);
        let encoded = h.encode();

        let detected = detect_format(&encoded);
        assert_eq!(detected, Some(WireFormat::V2));

        let decoded = FrameHeaderV2::decode(&encoded).unwrap();
        assert_eq!(decoded.frame_type, FrameTypeV2::Ack);
    }
}
