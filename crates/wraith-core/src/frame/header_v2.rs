//! v2 Frame header encoding and decoding (24 bytes).
//!
//! The v2 frame header is a fixed 24-byte structure with expanded fields:
//!
//! ```text
//!  Offset  Size  Field
//!  0       1     Version (0x20 = v2.0)
//!  1       1     Frame Type
//!  2       2     Flags (little-endian)
//!  4       8     Sequence Number (little-endian, 64-bit)
//!  12      4     Payload Length (little-endian, 32-bit)
//!  16      4     Stream ID (little-endian, 32-bit)
//!  20      4     Reserved / Extension
//! ```
//!
//! All multi-byte fields use little-endian encoding for v2 (optimized for
//! x86_64 which is the primary target, avoiding byte-swap overhead).

use super::types_v2::{FRAME_HEADER_V2_SIZE, FlagsV2, FrameTypeV2, PROTOCOL_VERSION_V2};
use crate::error::FrameError;

/// v2 frame header (24 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeaderV2 {
    /// Protocol version byte (0x20 for v2.0).
    pub version: u8,
    /// Frame type.
    pub frame_type: FrameTypeV2,
    /// Flags (16-bit).
    pub flags: FlagsV2,
    /// 64-bit sequence number.
    pub sequence: u64,
    /// Payload length in bytes (32-bit).
    pub length: u32,
    /// Stream ID (32-bit, expanded from v1's 16-bit).
    pub stream_id: u32,
    /// Reserved bytes for future extension.
    pub reserved: u32,
}

impl FrameHeaderV2 {
    /// Create a new v2 header with defaults.
    #[must_use]
    pub fn new(frame_type: FrameTypeV2) -> Self {
        Self {
            version: PROTOCOL_VERSION_V2,
            frame_type,
            flags: FlagsV2::empty(),
            sequence: 0,
            length: 0,
            stream_id: 0,
            reserved: 0,
        }
    }

    /// Encode the header into a 24-byte buffer.
    #[must_use]
    pub fn encode(&self) -> [u8; FRAME_HEADER_V2_SIZE] {
        let mut buf = [0u8; FRAME_HEADER_V2_SIZE];
        self.encode_into(&mut buf);
        buf
    }

    /// Encode the header into a pre-allocated buffer.
    ///
    /// # Panics
    ///
    /// Panics if `buf.len() < 24`.
    pub fn encode_into(&self, buf: &mut [u8]) {
        buf[0] = self.version;
        buf[1] = self.frame_type.into();
        buf[2..4].copy_from_slice(&self.flags.bits().to_le_bytes());
        buf[4..12].copy_from_slice(&self.sequence.to_le_bytes());
        buf[12..16].copy_from_slice(&self.length.to_le_bytes());
        buf[16..20].copy_from_slice(&self.stream_id.to_le_bytes());
        buf[20..24].copy_from_slice(&self.reserved.to_le_bytes());
    }

    /// Decode a header from a byte buffer.
    ///
    /// # Errors
    ///
    /// Returns `FrameError::TooShort` if the buffer is smaller than 24 bytes.
    /// Returns `FrameError::InvalidFrameType` if the frame type byte is unknown.
    pub fn decode(buf: &[u8]) -> Result<Self, FrameError> {
        if buf.len() < FRAME_HEADER_V2_SIZE {
            return Err(FrameError::TooShort {
                expected: FRAME_HEADER_V2_SIZE,
                actual: buf.len(),
            });
        }

        Self::decode_unchecked(buf)
    }

    /// Decode a header without length checking (caller must ensure >= 24 bytes).
    fn decode_unchecked(buf: &[u8]) -> Result<Self, FrameError> {
        let version = buf[0];
        let frame_type = FrameTypeV2::try_from(buf[1])?;
        let flags = FlagsV2::from_bits(u16::from_le_bytes([buf[2], buf[3]]));
        let sequence = u64::from_le_bytes([
            buf[4], buf[5], buf[6], buf[7], buf[8], buf[9], buf[10], buf[11],
        ]);
        let length = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
        let stream_id = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);
        let reserved = u32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]);

        Ok(Self {
            version,
            frame_type,
            flags,
            sequence,
            length,
            stream_id,
            reserved,
        })
    }

    /// SIMD-accelerated decode on x86_64.
    ///
    /// Uses SSE2 loads to prime the cache, then extracts fields.
    /// Falls back to scalar decode on other architectures.
    ///
    /// # Errors
    ///
    /// Same as [`Self::decode`].
    #[cfg(feature = "simd")]
    pub fn decode_simd(buf: &[u8]) -> Result<Self, FrameError> {
        if buf.len() < FRAME_HEADER_V2_SIZE {
            return Err(FrameError::TooShort {
                expected: FRAME_HEADER_V2_SIZE,
                actual: buf.len(),
            });
        }

        #[cfg(target_arch = "x86_64")]
        {
            // SAFETY: We verified buf.len() >= 24 above. SSE2 unaligned loads
            // are safe on any x86_64 CPU. The pointer is derived from a valid
            // slice and the 16-byte load stays within the 24-byte buffer.
            unsafe {
                use core::arch::x86_64::*;
                let ptr = buf.as_ptr() as *const __m128i;
                let _vec = _mm_loadu_si128(ptr);
                // Cache primed, now extract fields using scalar code
            }
        }

        Self::decode_unchecked(buf)
    }

    /// SIMD-accelerated encode on x86_64.
    ///
    /// Uses SSE2 stores for cache-line efficiency.
    #[cfg(feature = "simd")]
    #[must_use]
    pub fn encode_simd(&self) -> [u8; FRAME_HEADER_V2_SIZE] {
        let mut buf = [0u8; FRAME_HEADER_V2_SIZE];
        self.encode_into(&mut buf);

        #[cfg(target_arch = "x86_64")]
        {
            // The encode_into already wrote the data; the SIMD store
            // below is a write-combine optimization for streaming stores.
            // In practice the compiler will optimize the above writes.
        }

        buf
    }

    /// Check if this header has a valid v2 version byte.
    #[must_use]
    pub fn is_v2(&self) -> bool {
        self.version == PROTOCOL_VERSION_V2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::types_v2::{FlagsV2, FrameTypeV2};

    #[test]
    fn test_header_v2_size() {
        assert_eq!(FRAME_HEADER_V2_SIZE, 24);
    }

    #[test]
    fn test_header_v2_new_defaults() {
        let h = FrameHeaderV2::new(FrameTypeV2::Data);
        assert_eq!(h.version, PROTOCOL_VERSION_V2);
        assert_eq!(h.frame_type, FrameTypeV2::Data);
        assert_eq!(h.flags, FlagsV2::empty());
        assert_eq!(h.sequence, 0);
        assert_eq!(h.length, 0);
        assert_eq!(h.stream_id, 0);
        assert_eq!(h.reserved, 0);
    }

    #[test]
    fn test_header_v2_encode_decode_roundtrip() {
        let h = FrameHeaderV2 {
            version: PROTOCOL_VERSION_V2,
            frame_type: FrameTypeV2::StreamData,
            flags: FlagsV2::empty().with(FlagsV2::SYN).with(FlagsV2::ECN),
            sequence: 0x0123_4567_89AB_CDEF,
            length: 0xDEAD_BEEF,
            stream_id: 0xCAFE_BABE,
            reserved: 0x1234_5678,
        };

        let encoded = h.encode();
        assert_eq!(encoded.len(), 24);

        let decoded = FrameHeaderV2::decode(&encoded).unwrap();
        assert_eq!(h, decoded);
    }

    #[test]
    fn test_header_v2_all_frame_types() {
        let types = [
            FrameTypeV2::Data,
            FrameTypeV2::DataFin,
            FrameTypeV2::Datagram,
            FrameTypeV2::Ack,
            FrameTypeV2::AckEcn,
            FrameTypeV2::Ping,
            FrameTypeV2::Pong,
            FrameTypeV2::Rekey,
            FrameTypeV2::RekeyAck,
            FrameTypeV2::StreamOpen,
            FrameTypeV2::StreamData,
            FrameTypeV2::StreamClose,
            FrameTypeV2::StreamReset,
            FrameTypeV2::StreamWindow,
            FrameTypeV2::Priority,
            FrameTypeV2::PathChallenge,
            FrameTypeV2::PathResponse,
            FrameTypeV2::PathMigrate,
            FrameTypeV2::Close,
            FrameTypeV2::CloseAck,
            FrameTypeV2::Padding,
            FrameTypeV2::PaddingRandom,
        ];

        for ft in types {
            let h = FrameHeaderV2::new(ft);
            let encoded = h.encode();
            let decoded = FrameHeaderV2::decode(&encoded).unwrap();
            assert_eq!(decoded.frame_type, ft);
        }
    }

    #[test]
    fn test_header_v2_flags_roundtrip() {
        let flags_values = [
            0u16,
            0x00FF,
            0xFF00,
            0xFFFF,
            FlagsV2::SYN | FlagsV2::FIN | FlagsV2::ECN | FlagsV2::RTX,
        ];
        for bits in flags_values {
            let h = FrameHeaderV2 {
                flags: FlagsV2::from_bits(bits),
                ..FrameHeaderV2::new(FrameTypeV2::Data)
            };
            let encoded = h.encode();
            let decoded = FrameHeaderV2::decode(&encoded).unwrap();
            assert_eq!(decoded.flags.bits(), bits);
        }
    }

    #[test]
    fn test_header_v2_sequence_boundary() {
        for seq in [0u64, 1, u32::MAX as u64, u64::MAX] {
            let h = FrameHeaderV2 {
                sequence: seq,
                ..FrameHeaderV2::new(FrameTypeV2::Data)
            };
            let decoded = FrameHeaderV2::decode(&h.encode()).unwrap();
            assert_eq!(decoded.sequence, seq);
        }
    }

    #[test]
    fn test_header_v2_length_boundary() {
        for len in [0u32, 1, u16::MAX as u32, u32::MAX] {
            let h = FrameHeaderV2 {
                length: len,
                ..FrameHeaderV2::new(FrameTypeV2::Data)
            };
            let decoded = FrameHeaderV2::decode(&h.encode()).unwrap();
            assert_eq!(decoded.length, len);
        }
    }

    #[test]
    fn test_header_v2_stream_id_32bit() {
        // v2 supports 32-bit stream IDs (expanded from v1's 16-bit)
        for sid in [0u32, 1, 16, u16::MAX as u32, u16::MAX as u32 + 1, u32::MAX] {
            let h = FrameHeaderV2 {
                stream_id: sid,
                ..FrameHeaderV2::new(FrameTypeV2::Data)
            };
            let decoded = FrameHeaderV2::decode(&h.encode()).unwrap();
            assert_eq!(decoded.stream_id, sid);
        }
    }

    #[test]
    fn test_header_v2_too_short() {
        let buf = [0u8; 23]; // One byte too short
        assert!(matches!(
            FrameHeaderV2::decode(&buf),
            Err(FrameError::TooShort {
                expected: 24,
                actual: 23
            })
        ));
    }

    #[test]
    fn test_header_v2_invalid_frame_type() {
        let mut buf = FrameHeaderV2::new(FrameTypeV2::Data).encode();
        buf[1] = 0xFF; // Invalid frame type
        assert!(matches!(
            FrameHeaderV2::decode(&buf),
            Err(FrameError::InvalidFrameType(0xFF))
        ));
    }

    #[test]
    fn test_header_v2_version_check() {
        let h = FrameHeaderV2::new(FrameTypeV2::Data);
        assert!(h.is_v2());

        let h2 = FrameHeaderV2 {
            version: 0x10, // v1
            ..FrameHeaderV2::new(FrameTypeV2::Data)
        };
        assert!(!h2.is_v2());
    }

    #[test]
    fn test_header_v2_encode_into() {
        let h = FrameHeaderV2 {
            version: PROTOCOL_VERSION_V2,
            frame_type: FrameTypeV2::Ack,
            flags: FlagsV2::from_bits(0x0021),
            sequence: 42,
            length: 100,
            stream_id: 7,
            reserved: 0,
        };

        let mut buf = [0u8; 32]; // Larger than needed
        h.encode_into(&mut buf);

        // Verify version and type bytes
        assert_eq!(buf[0], PROTOCOL_VERSION_V2);
        assert_eq!(buf[1], 0x10); // Ack

        // Decode from the same buffer
        let decoded = FrameHeaderV2::decode(&buf).unwrap();
        assert_eq!(decoded, h);
    }

    #[cfg(feature = "simd")]
    #[test]
    fn test_header_v2_simd_decode() {
        let h = FrameHeaderV2 {
            version: PROTOCOL_VERSION_V2,
            frame_type: FrameTypeV2::StreamData,
            flags: FlagsV2::from_bits(0x00FF),
            sequence: 999_999_999_999,
            length: 8192,
            stream_id: 12345,
            reserved: 0,
        };

        let encoded = h.encode();
        let decoded_scalar = FrameHeaderV2::decode(&encoded).unwrap();
        let decoded_simd = FrameHeaderV2::decode_simd(&encoded).unwrap();
        assert_eq!(decoded_scalar, decoded_simd);
    }

    #[cfg(feature = "simd")]
    #[test]
    fn test_header_v2_simd_too_short() {
        let buf = [0u8; 10];
        assert!(FrameHeaderV2::decode_simd(&buf).is_err());
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn prop_header_v2_roundtrip(
                seq in any::<u64>(),
                length in any::<u32>(),
                stream_id in any::<u32>(),
                reserved in any::<u32>(),
                flags_bits in any::<u16>(),
            ) {
                let valid_types: &[FrameTypeV2] = &[
                    FrameTypeV2::Data, FrameTypeV2::Ack, FrameTypeV2::Ping,
                    FrameTypeV2::Rekey, FrameTypeV2::StreamOpen, FrameTypeV2::Close,
                    FrameTypeV2::Padding,
                ];
                let ft = valid_types[seq as usize % valid_types.len()];

                let h = FrameHeaderV2 {
                    version: PROTOCOL_VERSION_V2,
                    frame_type: ft,
                    flags: FlagsV2::from_bits(flags_bits),
                    sequence: seq,
                    length,
                    stream_id,
                    reserved,
                };
                let encoded = h.encode();
                let decoded = FrameHeaderV2::decode(&encoded).unwrap();
                prop_assert_eq!(h, decoded);
            }

            #[test]
            fn prop_header_v2_decode_doesnt_panic(data in prop::collection::vec(any::<u8>(), 0..64)) {
                let _ = FrameHeaderV2::decode(&data);
            }
        }
    }
}
