//! Integration tests for v2 polymorphic wire format encoding.

use wraith_core::{FlagsV2, FrameHeaderV2, FrameTypeV2, PROTOCOL_VERSION_V2, PolymorphicFormat};
use wraith_crypto::kdf::derive_session_keys_v2;

fn make_header(ft: FrameTypeV2, seq: u64, length: u32, stream_id: u32) -> FrameHeaderV2 {
    FrameHeaderV2 {
        version: PROTOCOL_VERSION_V2,
        frame_type: ft,
        flags: FlagsV2::empty().with(FlagsV2::SYN),
        sequence: seq,
        length,
        stream_id,
        reserved: 0,
    }
}

#[test]
fn test_derivation_determinism() {
    let secret = [0x42u8; 32];
    let fmt1 = PolymorphicFormat::derive(&secret);
    let fmt2 = PolymorphicFormat::derive(&secret);
    assert_eq!(fmt1.field_offsets(), fmt2.field_offsets());
}

#[test]
fn test_different_secrets_different_formats() {
    let secrets: Vec<[u8; 32]> = (0..10u8).map(|i| [i; 32]).collect();
    let formats: Vec<_> = secrets.iter().map(PolymorphicFormat::derive).collect();

    // At least some should have different offsets (probabilistic but very high)
    let mut different_count = 0;
    for i in 0..formats.len() {
        for j in (i + 1)..formats.len() {
            if formats[i].field_offsets() != formats[j].field_offsets() {
                different_count += 1;
            }
        }
    }
    assert!(
        different_count > 0,
        "All formats identical -- extremely unlikely"
    );
}

#[test]
fn test_encode_decode_roundtrip_varied_headers() {
    let secret = [0xAB; 32];
    let fmt = PolymorphicFormat::derive(&secret);

    let headers = [
        make_header(FrameTypeV2::Data, 0, 0, 0),
        make_header(FrameTypeV2::Ack, u64::MAX, u32::MAX, u32::MAX),
        make_header(FrameTypeV2::StreamData, 42, 1500, 7),
        make_header(FrameTypeV2::Close, 999999, 0, 0),
        make_header(FrameTypeV2::Padding, 1, 1, 1),
    ];

    for h in &headers {
        let encoded = fmt.encode_header(h);
        let decoded = fmt.decode_header(&encoded).expect("decode should succeed");
        assert_eq!(*h, decoded, "Round-trip failed for {h:?}");
    }
}

#[test]
fn test_sequential_encode_decode_same_format() {
    let fmt = PolymorphicFormat::derive(&[0x55; 32]);

    for seq in 0..50u64 {
        let h = make_header(FrameTypeV2::StreamData, seq, (seq as u32) * 100, seq as u32);
        let encoded = fmt.encode_header(&h);
        let decoded = fmt.decode_header(&encoded).unwrap();
        assert_eq!(h, decoded, "Failed at seq {seq}");
    }
}

#[test]
fn test_encoded_differs_from_plaintext() {
    let fmt = PolymorphicFormat::derive(&[0x99; 32]);
    let h = make_header(FrameTypeV2::Data, 12345, 4096, 1);

    let polymorphic = fmt.encode_header(&h);
    let plaintext = h.encode();
    assert_ne!(
        polymorphic, plaintext,
        "Polymorphic should differ from plaintext"
    );
}

#[test]
fn test_wrong_format_cannot_decode_correctly() {
    let fmt1 = PolymorphicFormat::derive(&[0xAA; 32]);
    let fmt2 = PolymorphicFormat::derive(&[0xBB; 32]);
    let h = make_header(FrameTypeV2::Data, 42, 1000, 5);

    let encoded = fmt1.encode_header(&h);
    if let Ok(decoded) = fmt2.decode_header(&encoded) {
        assert_ne!(decoded, h, "Wrong format should not decode correctly");
    }
    // Err is also acceptable (invalid frame type byte after wrong XOR)
}

#[test]
fn test_crypto_integration_session_keys_to_polymorphic() {
    // Simulate Alice and Bob deriving the same format from shared secrets
    let shared_secret = [0x77u8; 32];
    let transcript = [0x88u8; 32];

    let keys_alice = derive_session_keys_v2(&shared_secret, &transcript);
    let keys_bob = derive_session_keys_v2(&shared_secret, &transcript);

    let fmt_alice = PolymorphicFormat::derive(&keys_alice.format_key);
    let fmt_bob = PolymorphicFormat::derive(&keys_bob.format_key);

    // Same format derived
    assert_eq!(fmt_alice.field_offsets(), fmt_bob.field_offsets());

    // Alice encodes, Bob decodes
    let header = make_header(FrameTypeV2::StreamData, 1, 512, 3);
    let wire_bytes = fmt_alice.encode_header(&header);
    let decoded = fmt_bob
        .decode_header(&wire_bytes)
        .expect("Bob should decode Alice's header");
    assert_eq!(header, decoded);
}

#[test]
fn test_crypto_integration_different_sessions_different_formats() {
    let keys1 = derive_session_keys_v2(&[0x01; 32], &[0x10; 32]);
    let keys2 = derive_session_keys_v2(&[0x02; 32], &[0x20; 32]);

    let fmt1 = PolymorphicFormat::derive(&keys1.format_key);
    let fmt2 = PolymorphicFormat::derive(&keys2.format_key);

    let h = make_header(FrameTypeV2::Data, 1, 100, 1);
    let enc1 = fmt1.encode_header(&h);
    let enc2 = fmt2.encode_header(&h);

    assert_ne!(
        enc1, enc2,
        "Different sessions should produce different wire bytes"
    );
}

#[test]
fn test_field_offsets_non_overlapping() {
    // Test with many different secrets
    let field_sizes: [usize; 7] = [1, 1, 2, 8, 4, 4, 4];
    for seed in 0..50u8 {
        let fmt = PolymorphicFormat::derive(&[seed; 32]);
        let offsets = fmt.field_offsets();

        let mut used = [false; 24];
        for (i, &offset) in offsets.iter().enumerate() {
            for j in 0..field_sizes[i] {
                assert!(
                    !used[offset + j],
                    "Overlap at byte {} for seed {seed}",
                    offset + j
                );
                used[offset + j] = true;
            }
        }
        assert!(
            used.iter().all(|&b| b),
            "Not all bytes used for seed {seed}"
        );
    }
}

mod proptests {
    use proptest::prelude::*;
    use wraith_core::{
        FlagsV2, FrameHeaderV2, FrameTypeV2, PROTOCOL_VERSION_V2, PolymorphicFormat,
    };

    proptest! {
        #[test]
        fn prop_polymorphic_roundtrip(
            secret in prop::array::uniform32(any::<u8>()),
            seq in any::<u64>(),
            length in any::<u32>(),
            stream_id in any::<u32>(),
            flags_bits in any::<u16>(),
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
                reserved: 0,
            };

            let encoded = fmt.encode_header(&h);
            let decoded = fmt.decode_header(&encoded).unwrap();
            prop_assert_eq!(h, decoded);
        }
    }
}
