//! Full pipeline integration tests for the v2 wire format.
//!
//! Tests cross-module scenarios: CID generation -> header creation ->
//! polymorphic encoding -> format detection -> decoding -> verification.

use wraith_core::{
    ConnectionIdV2, FlagsV2, FrameHeaderV2, FrameTypeV2, PROTOCOL_VERSION_V2, PolymorphicFormat,
    WireFormat, detect_format,
};
use wraith_crypto::kdf::derive_session_keys_v2;

#[test]
fn test_full_pipeline_generate_encode_detect_decode() {
    let cid = ConnectionIdV2::generate();
    assert!(cid.is_valid());

    let header = FrameHeaderV2 {
        version: PROTOCOL_VERSION_V2,
        frame_type: FrameTypeV2::Data,
        flags: FlagsV2::empty().with(FlagsV2::SYN),
        sequence: 1,
        length: 1024,
        stream_id: cid.to_bytes()[0] as u32, // use part of CID as stream ID
        reserved: 0,
    };

    // Plain encoding path
    let encoded = header.encode();
    let detected = detect_format(&encoded);
    assert_eq!(detected, Some(WireFormat::V2));

    let decoded = FrameHeaderV2::decode(&encoded).unwrap();
    assert_eq!(header, decoded);
}

#[test]
fn test_full_pipeline_polymorphic_encode_decode() {
    let secret = [0x42u8; 32];
    let fmt = PolymorphicFormat::derive(&secret);
    let cid = ConnectionIdV2::generate();

    let header = FrameHeaderV2 {
        version: PROTOCOL_VERSION_V2,
        frame_type: FrameTypeV2::StreamData,
        flags: FlagsV2::empty().with(FlagsV2::ECN),
        sequence: 100,
        length: 2048,
        stream_id: 5,
        reserved: 0,
    };

    let wire_bytes = fmt.encode_header(&header);

    // Polymorphic encoding should NOT be detectable as v2 (XOR mask changes version byte)
    // (depends on the mask, but generally won't match the v2 version byte)

    // Decode with same format
    let decoded = fmt.decode_header(&wire_bytes).unwrap();
    assert_eq!(header, decoded);

    // CID should still be valid independently
    assert!(cid.is_valid());
}

#[test]
fn test_alice_bob_shared_secret_pipeline() {
    // Simulate Alice and Bob establishing a session
    let shared_secret = [0xDE; 32];
    let transcript = [0xAD; 32];

    // Both derive the same session keys
    let alice_keys = derive_session_keys_v2(&shared_secret, &transcript);
    let bob_keys = derive_session_keys_v2(&shared_secret, &transcript);

    // Both derive polymorphic format from the format key
    let alice_fmt = PolymorphicFormat::derive(&alice_keys.format_key);
    let bob_fmt = PolymorphicFormat::derive(&bob_keys.format_key);

    // Both generate their own CIDs
    let alice_cid = ConnectionIdV2::generate();
    let bob_cid = ConnectionIdV2::generate();
    assert_ne!(alice_cid, bob_cid);

    // Alice sends a message to Bob
    let msg_header = FrameHeaderV2 {
        version: PROTOCOL_VERSION_V2,
        frame_type: FrameTypeV2::Data,
        flags: FlagsV2::empty().with(FlagsV2::SYN),
        sequence: 0,
        length: 512,
        stream_id: 1,
        reserved: 0,
    };

    let wire = alice_fmt.encode_header(&msg_header);
    let decoded = bob_fmt
        .decode_header(&wire)
        .expect("Bob should decode Alice's header");
    assert_eq!(msg_header, decoded);

    // Bob sends an ACK back
    let ack_header = FrameHeaderV2 {
        version: PROTOCOL_VERSION_V2,
        frame_type: FrameTypeV2::Ack,
        flags: FlagsV2::empty().with(FlagsV2::ACK),
        sequence: 0,
        length: 0,
        stream_id: 1,
        reserved: 0,
    };

    let wire = bob_fmt.encode_header(&ack_header);
    let decoded = alice_fmt
        .decode_header(&wire)
        .expect("Alice should decode Bob's ACK");
    assert_eq!(ack_header, decoded);
}

#[test]
fn test_multi_message_pipeline() {
    let fmt = PolymorphicFormat::derive(&[0x99; 32]);

    let mut headers = Vec::new();
    let mut wire_messages = Vec::new();

    // Encode 50 messages
    for seq in 0..50u64 {
        let h = FrameHeaderV2 {
            version: PROTOCOL_VERSION_V2,
            frame_type: if seq % 5 == 0 {
                FrameTypeV2::Ack
            } else {
                FrameTypeV2::Data
            },
            flags: FlagsV2::from_bits(seq as u16 & 0xFF),
            sequence: seq,
            length: (seq as u32) * 100,
            stream_id: (seq as u32) % 8,
            reserved: 0,
        };
        wire_messages.push(fmt.encode_header(&h));
        headers.push(h);
    }

    // Decode all and verify
    for (i, wire) in wire_messages.iter().enumerate() {
        let decoded = fmt.decode_header(wire).unwrap();
        assert_eq!(headers[i], decoded, "Mismatch at message {i}");
    }
}

#[test]
fn test_cid_rotation_with_header_pipeline() {
    let base_cid = ConnectionIdV2::generate();
    let fmt = PolymorphicFormat::derive(&[0x77; 32]);

    for seq in 0..20u64 {
        let rotated_cid = base_cid.rotate(seq);
        assert!(rotated_cid.is_valid() || rotated_cid == ConnectionIdV2::INVALID);

        let h = FrameHeaderV2 {
            version: PROTOCOL_VERSION_V2,
            frame_type: FrameTypeV2::Data,
            flags: FlagsV2::empty(),
            sequence: seq,
            length: 256,
            stream_id: 1,
            reserved: 0,
        };

        let wire = fmt.encode_header(&h);
        let decoded = fmt.decode_header(&wire).unwrap();
        assert_eq!(h, decoded);
    }
}

#[test]
fn test_v1_migration_in_full_pipeline() {
    // Simulate a v1 client migrating to v2
    let v1_cid =
        wraith_core::ConnectionId::from_bytes([0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04]);
    let v2_cid = ConnectionIdV2::from_v1(v1_cid);
    assert!(v2_cid.is_migrated_v1());

    // Use v1 header, convert to v2, encode polymorphic, decode
    let v1_header = wraith_core::frame::FrameHeader {
        frame_type: wraith_core::FrameType::Data,
        flags: wraith_core::FrameFlags::new().with_syn(),
        stream_id: 10,
        sequence: 500,
        offset: 0,
        payload_len: 1400,
    };

    let v2_header = wraith_core::frame::compat::v1_header_to_v2(&v1_header);
    assert!(v2_header.is_v2());

    let fmt = PolymorphicFormat::derive(&[0xCC; 32]);
    let wire = fmt.encode_header(&v2_header);
    let decoded = fmt.decode_header(&wire).unwrap();
    assert_eq!(v2_header, decoded);

    // Convert back to v1 and verify key fields
    let recovered_v1 = wraith_core::frame::compat::v2_header_to_v1(&decoded).unwrap();
    assert_eq!(recovered_v1.frame_type, v1_header.frame_type);
    assert_eq!(recovered_v1.stream_id, v1_header.stream_id);
    assert_eq!(recovered_v1.sequence, v1_header.sequence);
    assert_eq!(recovered_v1.payload_len, v1_header.payload_len);
}

#[test]
fn test_different_sessions_produce_different_wire_bytes() {
    let header = FrameHeaderV2 {
        version: PROTOCOL_VERSION_V2,
        frame_type: FrameTypeV2::Data,
        flags: FlagsV2::empty(),
        sequence: 42,
        length: 1000,
        stream_id: 1,
        reserved: 0,
    };

    let mut wire_outputs = std::collections::HashSet::new();
    for i in 0..20u8 {
        let keys = derive_session_keys_v2(&[i; 32], &[0xFF; 32]);
        let fmt = PolymorphicFormat::derive(&keys.format_key);
        let wire = fmt.encode_header(&header);
        wire_outputs.insert(wire);
    }

    // All 20 sessions should produce unique wire encodings
    assert_eq!(wire_outputs.len(), 20, "Expected 20 unique wire encodings");
}

mod proptests {
    use proptest::prelude::*;
    use wraith_core::{
        FlagsV2, FrameHeaderV2, FrameTypeV2, PROTOCOL_VERSION_V2, PolymorphicFormat,
    };
    use wraith_crypto::kdf::derive_session_keys_v2;

    proptest! {
        #[test]
        fn prop_full_pipeline_roundtrip(
            secret in prop::array::uniform32(any::<u8>()),
            transcript in prop::array::uniform32(any::<u8>()),
            seq in any::<u64>(),
            length in any::<u32>(),
        ) {
            let keys = derive_session_keys_v2(&secret, &transcript);
            let fmt = PolymorphicFormat::derive(&keys.format_key);

            let valid_types: &[FrameTypeV2] = &[
                FrameTypeV2::Data, FrameTypeV2::Ack, FrameTypeV2::Ping,
                FrameTypeV2::StreamOpen, FrameTypeV2::Close, FrameTypeV2::Padding,
            ];
            let ft = valid_types[seq as usize % valid_types.len()];

            let h = FrameHeaderV2 {
                version: PROTOCOL_VERSION_V2,
                frame_type: ft,
                flags: FlagsV2::empty(),
                sequence: seq,
                length,
                stream_id: 0,
                reserved: 0,
            };

            let wire = fmt.encode_header(&h);
            let decoded = fmt.decode_header(&wire).unwrap();
            prop_assert_eq!(h, decoded);
        }
    }
}
