//! Integration tests for v2 ConnectionIdV2 across modules.

use std::collections::HashSet;
use wraith_core::{ConnectionId, ConnectionIdV2};

#[test]
fn test_generation_uniqueness_1000() {
    let mut set = HashSet::new();
    for _ in 0..1000 {
        let cid = ConnectionIdV2::generate();
        assert!(set.insert(cid), "Duplicate CID in 1000 generations");
        assert!(cid.is_valid());
    }
}

#[test]
fn test_v1_to_v2_migration_roundtrip() {
    let v1_bytes: [[u8; 8]; 5] = [
        [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
        [0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0xF9, 0xF8],
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
        [0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE],
    ];

    for bytes in &v1_bytes {
        let v1 = ConnectionId::from_bytes(*bytes);
        let v2 = ConnectionIdV2::from_v1(v1);

        assert!(v2.is_migrated_v1(), "Should detect migrated v1 CID");
        assert!(v2.is_valid(), "Migrated CID should be valid");

        let recovered = v2.to_v1().expect("Should recover v1 CID");
        assert_eq!(recovered, v1, "Round-trip should preserve v1 CID");
    }
}

#[test]
fn test_rotation_determinism_and_uniqueness() {
    let base = ConnectionIdV2::from_bytes([
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
        0x10,
    ]);

    // Same sequence produces same rotation
    let r1 = base.rotate(42);
    let r2 = base.rotate(42);
    assert_eq!(r1, r2, "Same seq should produce same rotation");

    // Different sequences produce different rotations
    let mut rotated_set = HashSet::new();
    for seq in 0..100u64 {
        let rotated = base.rotate(seq);
        assert!(
            rotated_set.insert(rotated),
            "Rotation collision at seq {seq}"
        );
    }

    // Double rotation with same seq restores original (XOR self-inverse)
    let rotated = base.rotate(0xDEADBEEFCAFEBABE);
    let restored = rotated.rotate(0xDEADBEEFCAFEBABE);
    assert_eq!(base, restored);
}

#[test]
fn test_special_values_not_generated() {
    // Generate many CIDs and ensure none are special
    for _ in 0..10_000 {
        let cid = ConnectionIdV2::generate();
        assert!(!cid.is_special(), "Generated CID should not be special");
        assert_ne!(cid, ConnectionIdV2::INVALID);
    }
}

#[test]
fn test_special_values_properties() {
    assert!(!ConnectionIdV2::INVALID.is_valid());
    assert!(!ConnectionIdV2::INVALID.is_special());

    assert!(!ConnectionIdV2::HANDSHAKE.is_valid());
    assert!(ConnectionIdV2::HANDSHAKE.is_special());

    assert!(!ConnectionIdV2::VERSION_NEGOTIATION.is_valid());
    assert!(ConnectionIdV2::VERSION_NEGOTIATION.is_special());

    assert!(!ConnectionIdV2::STATELESS_RESET.is_valid());
    assert!(ConnectionIdV2::STATELESS_RESET.is_special());

    // All special values are distinct
    let specials = [
        ConnectionIdV2::INVALID,
        ConnectionIdV2::HANDSHAKE,
        ConnectionIdV2::VERSION_NEGOTIATION,
        ConnectionIdV2::STATELESS_RESET,
    ];
    for i in 0..specials.len() {
        for j in (i + 1)..specials.len() {
            assert_ne!(specials[i], specials[j]);
        }
    }
}

#[test]
fn test_serialization_roundtrip_buffer() {
    for _ in 0..100 {
        let cid = ConnectionIdV2::generate();
        let mut buf = [0u8; 32]; // larger than needed
        cid.write_to(&mut buf);
        let recovered = ConnectionIdV2::read_from(&buf);
        assert_eq!(cid, recovered);
    }
}

#[test]
fn test_from_bytes_to_bytes_identity() {
    let bytes = [
        0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD,
        0xEF,
    ];
    let cid = ConnectionIdV2::from_bytes(bytes);
    assert_eq!(cid.to_bytes(), bytes);
    assert_eq!(*cid.as_bytes(), bytes);
}

#[test]
fn test_display_format() {
    let cid = ConnectionIdV2::from_bytes([0u8; 16]);
    assert_eq!(format!("{cid}"), "00000000000000000000000000000000");

    let cid = ConnectionIdV2::from_bytes([0xFF; 16]);
    assert_eq!(format!("{cid}"), "ffffffffffffffffffffffffffffffff");
}

mod proptests {
    use proptest::prelude::*;
    use wraith_core::{ConnectionId, ConnectionIdV2};

    proptest! {
        #[test]
        fn prop_v1_migration_roundtrip(bytes in prop::array::uniform8(any::<u8>())) {
            // Skip all-zero v1 CID since from_v1 with all zeros has upper_nonzero=false
            if bytes == [0u8; 8] {
                return Ok(());
            }
            let v1 = ConnectionId::from_bytes(bytes);
            let v2 = ConnectionIdV2::from_v1(v1);
            prop_assert!(v2.is_migrated_v1());
            let recovered = v2.to_v1().unwrap();
            prop_assert_eq!(recovered.to_bytes(), v1.to_bytes());
        }

        #[test]
        fn prop_rotation_self_inverse(
            base in prop::array::uniform16(any::<u8>()),
            seq in any::<u64>(),
        ) {
            let cid = ConnectionIdV2::from_bytes(base);
            let double_rotated = cid.rotate(seq).rotate(seq);
            prop_assert_eq!(cid, double_rotated);
        }

        #[test]
        fn prop_write_read_roundtrip(bytes in prop::array::uniform16(any::<u8>())) {
            let cid = ConnectionIdV2::from_bytes(bytes);
            let mut buf = [0u8; 16];
            cid.write_to(&mut buf);
            let recovered = ConnectionIdV2::read_from(&buf);
            prop_assert_eq!(cid, recovered);
        }
    }
}
