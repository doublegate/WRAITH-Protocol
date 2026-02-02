//! Polymorphic wire format encoding for the WRAITH v2 protocol.
//!
//! Uses session-derived keys to permute field positions and XOR-mask header
//! bytes, making the wire format appear random to passive observers. Each
//! session produces a unique encoding layout.
//!
//! ## Algorithm
//!
//! 1. Derive a format key and XOR mask from the session secret using BLAKE3.
//! 2. Compute field positions via Fisher-Yates shuffle seeded by the format key.
//! 3. Encode header fields at shuffled byte offsets.
//! 4. XOR the entire 24-byte header with the mask.
//!
//! Decoding reverses: XOR to unmask, then read fields from derived positions.

use super::header_v2::FrameHeaderV2;
use super::types_v2::{FRAME_HEADER_V2_SIZE, FlagsV2, FrameTypeV2};
use crate::error::FrameError;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Number of header fields that are permuted.
const FIELD_COUNT: usize = 7;

/// Field sizes in bytes, indexed by field ID.
/// Order: Version(1), FrameType(1), Flags(2), Sequence(8), Length(4), StreamID(4), Reserved(4)
const FIELD_SIZES: [usize; FIELD_COUNT] = [1, 1, 2, 8, 4, 4, 4];

/// Total bytes accounted for by all fields (must equal FRAME_HEADER_V2_SIZE).
const TOTAL_FIELD_BYTES: usize = 1 + 1 + 2 + 8 + 4 + 4 + 4; // = 24

/// Polymorphic wire format derived from a session secret.
///
/// Each instance defines a unique mapping of header fields to byte positions,
/// plus an XOR mask applied to the encoded header.
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct PolymorphicFormat {
    /// BLAKE3-derived format key (32 bytes).
    format_key: [u8; 32],
    /// Byte offset for each field (indexed by field ID).
    field_offsets: [usize; FIELD_COUNT],
    /// XOR mask applied to the entire header (first 24 bytes used).
    xor_mask: [u8; 32],
}

impl PolymorphicFormat {
    /// Derive a polymorphic format from a 32-byte session secret.
    ///
    /// The session secret is typically `SessionKeysV2::format_key` derived
    /// during the v2 handshake via [`wraith_crypto::kdf::derive_session_keys_v2`].
    #[must_use]
    pub fn derive(session_secret: &[u8; 32]) -> Self {
        // Derive format key for field position computation
        let format_key: [u8; 32] =
            *blake3::keyed_hash(b"wraith-v2-polymorphic-format-key", session_secret).as_bytes();

        // Derive XOR mask
        let xor_mask: [u8; 32] =
            *blake3::keyed_hash(b"wraith-v2-polymorphic-xor-mask--", session_secret).as_bytes();

        // Compute field positions via Fisher-Yates shuffle
        let field_offsets = Self::derive_positions(&format_key);

        Self {
            format_key,
            field_offsets,
            xor_mask,
        }
    }

    /// Derive field byte offsets using Fisher-Yates shuffle.
    ///
    /// Generates a permutation of the field order, then computes byte offsets
    /// based on the permuted order and field sizes.
    fn derive_positions(format_key: &[u8; 32]) -> [usize; FIELD_COUNT] {
        // Start with identity permutation
        let mut perm: [usize; FIELD_COUNT] = [0, 1, 2, 3, 4, 5, 6];

        // Fisher-Yates shuffle using bytes from the format key as randomness
        for i in (1..FIELD_COUNT).rev() {
            // Use a byte from the format key to determine swap index
            let rand_byte = format_key[i % 32] as usize;
            let j = rand_byte % (i + 1);
            perm.swap(i, j);
        }

        // Convert permutation to byte offsets
        let mut offsets = [0usize; FIELD_COUNT];
        let mut pos = 0;
        for &field_id in &perm {
            offsets[field_id] = pos;
            pos += FIELD_SIZES[field_id];
        }

        debug_assert_eq!(pos, TOTAL_FIELD_BYTES);
        offsets
    }

    /// Encode a v2 frame header using the polymorphic format.
    ///
    /// Fields are placed at derived byte positions, then XOR-masked.
    #[must_use]
    pub fn encode_header(&self, header: &FrameHeaderV2) -> [u8; FRAME_HEADER_V2_SIZE] {
        let mut buf = [0u8; FRAME_HEADER_V2_SIZE];

        // Write each field at its derived offset
        self.write_field(&mut buf, 0, &[header.version]);
        self.write_field(&mut buf, 1, &[header.frame_type.into()]);
        self.write_field(&mut buf, 2, &header.flags.bits().to_le_bytes());
        self.write_field(&mut buf, 3, &header.sequence.to_le_bytes());
        self.write_field(&mut buf, 4, &header.length.to_le_bytes());
        self.write_field(&mut buf, 5, &header.stream_id.to_le_bytes());
        self.write_field(&mut buf, 6, &header.reserved.to_le_bytes());

        // Apply XOR mask
        for (i, byte) in buf.iter_mut().enumerate() {
            *byte ^= self.xor_mask[i % 32];
        }

        buf
    }

    /// Decode a polymorphic-encoded header back to a `FrameHeaderV2`.
    ///
    /// # Errors
    ///
    /// Returns `FrameError::TooShort` if the buffer is smaller than 24 bytes.
    /// Returns `FrameError::InvalidFrameType` if the decoded type byte is invalid.
    pub fn decode_header(&self, data: &[u8]) -> Result<FrameHeaderV2, FrameError> {
        if data.len() < FRAME_HEADER_V2_SIZE {
            return Err(FrameError::TooShort {
                expected: FRAME_HEADER_V2_SIZE,
                actual: data.len(),
            });
        }

        // Remove XOR mask
        let mut buf = [0u8; FRAME_HEADER_V2_SIZE];
        for i in 0..FRAME_HEADER_V2_SIZE {
            buf[i] = data[i] ^ self.xor_mask[i % 32];
        }

        // Read each field from its derived offset
        let version = buf[self.field_offsets[0]];

        let frame_type_byte = buf[self.field_offsets[1]];
        let frame_type = FrameTypeV2::try_from(frame_type_byte)?;

        let flags_offset = self.field_offsets[2];
        let flags = FlagsV2::from_bits(u16::from_le_bytes([
            buf[flags_offset],
            buf[flags_offset + 1],
        ]));

        let seq_offset = self.field_offsets[3];
        let sequence = u64::from_le_bytes([
            buf[seq_offset],
            buf[seq_offset + 1],
            buf[seq_offset + 2],
            buf[seq_offset + 3],
            buf[seq_offset + 4],
            buf[seq_offset + 5],
            buf[seq_offset + 6],
            buf[seq_offset + 7],
        ]);

        let len_offset = self.field_offsets[4];
        let length = u32::from_le_bytes([
            buf[len_offset],
            buf[len_offset + 1],
            buf[len_offset + 2],
            buf[len_offset + 3],
        ]);

        let sid_offset = self.field_offsets[5];
        let stream_id = u32::from_le_bytes([
            buf[sid_offset],
            buf[sid_offset + 1],
            buf[sid_offset + 2],
            buf[sid_offset + 3],
        ]);

        let res_offset = self.field_offsets[6];
        let reserved = u32::from_le_bytes([
            buf[res_offset],
            buf[res_offset + 1],
            buf[res_offset + 2],
            buf[res_offset + 3],
        ]);

        Ok(FrameHeaderV2 {
            version,
            frame_type,
            flags,
            sequence,
            length,
            stream_id,
            reserved,
        })
    }

    /// Write a field's bytes at its derived offset.
    fn write_field(&self, buf: &mut [u8], field_id: usize, data: &[u8]) {
        let offset = self.field_offsets[field_id];
        buf[offset..offset + data.len()].copy_from_slice(data);
    }

    /// Get the field offsets (for debugging/testing).
    #[must_use]
    pub fn field_offsets(&self) -> &[usize; FIELD_COUNT] {
        &self.field_offsets
    }
}

#[cfg(test)]
mod tests {
    use super::super::types_v2::PROTOCOL_VERSION_V2;
    use super::*;

    fn test_secret() -> [u8; 32] {
        [0x42u8; 32]
    }

    fn test_header() -> FrameHeaderV2 {
        FrameHeaderV2 {
            version: PROTOCOL_VERSION_V2,
            frame_type: FrameTypeV2::StreamData,
            flags: FlagsV2::empty().with(FlagsV2::SYN).with(FlagsV2::ECN),
            sequence: 0x0123_4567_89AB_CDEF,
            length: 0xDEAD_BEEF,
            stream_id: 0xCAFE_BABE,
            reserved: 0x1234_5678,
        }
    }

    #[test]
    fn test_derive_deterministic() {
        let secret = test_secret();
        let fmt1 = PolymorphicFormat::derive(&secret);
        let fmt2 = PolymorphicFormat::derive(&secret);
        assert_eq!(fmt1.field_offsets, fmt2.field_offsets);
        assert_eq!(fmt1.xor_mask, fmt2.xor_mask);
    }

    #[test]
    fn test_different_secrets_different_formats() {
        let fmt1 = PolymorphicFormat::derive(&[0x01u8; 32]);
        let fmt2 = PolymorphicFormat::derive(&[0x02u8; 32]);
        // Very likely to differ (6! = 720 permutations plus different XOR masks)
        assert_ne!(fmt1.xor_mask, fmt2.xor_mask);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let fmt = PolymorphicFormat::derive(&test_secret());
        let header = test_header();

        let encoded = fmt.encode_header(&header);
        let decoded = fmt.decode_header(&encoded).unwrap();

        assert_eq!(header, decoded);
    }

    #[test]
    fn test_encode_decode_all_frame_types() {
        let fmt = PolymorphicFormat::derive(&test_secret());
        let types = [
            FrameTypeV2::Data,
            FrameTypeV2::DataFin,
            FrameTypeV2::Ack,
            FrameTypeV2::Ping,
            FrameTypeV2::Rekey,
            FrameTypeV2::StreamOpen,
            FrameTypeV2::PathChallenge,
            FrameTypeV2::Close,
            FrameTypeV2::Padding,
        ];

        for ft in types {
            let h = FrameHeaderV2::new(ft);
            let encoded = fmt.encode_header(&h);
            let decoded = fmt.decode_header(&encoded).unwrap();
            assert_eq!(decoded.frame_type, ft);
        }
    }

    #[test]
    fn test_encoded_looks_random() {
        let fmt = PolymorphicFormat::derive(&test_secret());
        let header = test_header();
        let encoded = fmt.encode_header(&header);

        // The encoded bytes should not match plaintext encoding
        let plaintext = header.encode();
        assert_ne!(encoded, plaintext);

        // Check that the encoded data has reasonable entropy (not all same byte)
        let unique_bytes: std::collections::HashSet<u8> = encoded.iter().copied().collect();
        assert!(unique_bytes.len() > 4, "Encoded data should have entropy");
    }

    #[test]
    fn test_xor_mask_reversibility() {
        let fmt = PolymorphicFormat::derive(&test_secret());
        let header = test_header();

        // Encode twice should produce identical output (deterministic)
        let enc1 = fmt.encode_header(&header);
        let enc2 = fmt.encode_header(&header);
        assert_eq!(enc1, enc2);
    }

    #[test]
    fn test_field_offsets_valid() {
        let fmt = PolymorphicFormat::derive(&test_secret());
        let offsets = fmt.field_offsets();

        // All offsets should be within the header
        for (i, &offset) in offsets.iter().enumerate() {
            assert!(
                offset + FIELD_SIZES[i] <= FRAME_HEADER_V2_SIZE,
                "Field {i} at offset {offset} with size {} exceeds header",
                FIELD_SIZES[i]
            );
        }

        // Fields should not overlap: collect all used byte positions
        let mut used = [false; FRAME_HEADER_V2_SIZE];
        for (i, &offset) in offsets.iter().enumerate() {
            for j in 0..FIELD_SIZES[i] {
                assert!(
                    !used[offset + j],
                    "Byte {} used by multiple fields",
                    offset + j
                );
                used[offset + j] = true;
            }
        }

        // All 24 bytes should be used
        assert!(used.iter().all(|&b| b), "Not all header bytes used");
    }

    #[test]
    fn test_decode_too_short() {
        let fmt = PolymorphicFormat::derive(&test_secret());
        let buf = [0u8; 23];
        assert!(matches!(
            fmt.decode_header(&buf),
            Err(FrameError::TooShort { .. })
        ));
    }

    #[test]
    fn test_different_sessions_different_encodings() {
        let fmt1 = PolymorphicFormat::derive(&[0xAA; 32]);
        let fmt2 = PolymorphicFormat::derive(&[0xBB; 32]);
        let header = test_header();

        let enc1 = fmt1.encode_header(&header);
        let enc2 = fmt2.encode_header(&header);

        // Same header, different sessions -> different wire bytes
        assert_ne!(enc1, enc2);
    }

    #[test]
    fn test_cross_session_decode_fails() {
        let fmt1 = PolymorphicFormat::derive(&[0xAA; 32]);
        let fmt2 = PolymorphicFormat::derive(&[0xBB; 32]);
        let header = test_header();

        let encoded = fmt1.encode_header(&header);

        // Decoding with wrong session format should produce garbage (or error)
        let result = fmt2.decode_header(&encoded);
        // It may succeed but produce wrong data, or fail on invalid frame type
        match result {
            Ok(decoded) => assert_ne!(
                decoded, header,
                "Wrong format should not produce correct header"
            ),
            Err(_) => {} // Also acceptable
        }
    }

    #[test]
    fn test_boundary_values() {
        let fmt = PolymorphicFormat::derive(&test_secret());

        let h = FrameHeaderV2 {
            version: PROTOCOL_VERSION_V2,
            frame_type: FrameTypeV2::Data,
            flags: FlagsV2::from_bits(0xFFFF),
            sequence: u64::MAX,
            length: u32::MAX,
            stream_id: u32::MAX,
            reserved: u32::MAX,
        };

        let encoded = fmt.encode_header(&h);
        let decoded = fmt.decode_header(&encoded).unwrap();
        assert_eq!(h, decoded);
    }

    #[test]
    fn test_zero_values() {
        let fmt = PolymorphicFormat::derive(&test_secret());

        let h = FrameHeaderV2 {
            version: 0,
            frame_type: FrameTypeV2::Data,
            flags: FlagsV2::empty(),
            sequence: 0,
            length: 0,
            stream_id: 0,
            reserved: 0,
        };

        let encoded = fmt.encode_header(&h);
        let decoded = fmt.decode_header(&encoded).unwrap();
        assert_eq!(h, decoded);
    }

    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn prop_polymorphic_roundtrip(
                seq in any::<u64>(),
                length in any::<u32>(),
                stream_id in any::<u32>(),
                reserved in any::<u32>(),
                flags_bits in any::<u16>(),
                secret in prop::array::uniform32(any::<u8>()),
            ) {
                let fmt = PolymorphicFormat::derive(&secret);
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

                let encoded = fmt.encode_header(&h);
                let decoded = fmt.decode_header(&encoded).unwrap();
                prop_assert_eq!(h, decoded);
            }

            #[test]
            fn prop_field_offsets_no_overlap(
                secret in prop::array::uniform32(any::<u8>()),
            ) {
                let fmt = PolymorphicFormat::derive(&secret);
                let offsets = fmt.field_offsets();
                let mut used = [false; FRAME_HEADER_V2_SIZE];
                for (i, &offset) in offsets.iter().enumerate() {
                    for j in 0..FIELD_SIZES[i] {
                        prop_assert!(!used[offset + j]);
                        used[offset + j] = true;
                    }
                }
                prop_assert!(used.iter().all(|&b| b));
            }
        }
    }
}
