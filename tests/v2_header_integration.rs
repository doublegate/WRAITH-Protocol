//! Integration tests for v2 FrameHeaderV2 encode/decode.

use wraith_core::{FRAME_HEADER_V2_SIZE, FlagsV2, FrameHeaderV2, FrameTypeV2, PROTOCOL_VERSION_V2};

/// All valid FrameTypeV2 variants.
const ALL_FRAME_TYPES: &[FrameTypeV2] = &[
    FrameTypeV2::Data,
    FrameTypeV2::DataFin,
    FrameTypeV2::Datagram,
    FrameTypeV2::Ack,
    FrameTypeV2::AckEcn,
    FrameTypeV2::Ping,
    FrameTypeV2::Pong,
    FrameTypeV2::WindowUpdate,
    FrameTypeV2::GoAway,
    FrameTypeV2::QosUpdate,
    FrameTypeV2::Timestamp,
    FrameTypeV2::Rekey,
    FrameTypeV2::RekeyAck,
    FrameTypeV2::FecRepair,
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
    FrameTypeV2::GroupJoin,
    FrameTypeV2::GroupLeave,
    FrameTypeV2::GroupRekey,
    FrameTypeV2::Padding,
    FrameTypeV2::PaddingRandom,
];

#[test]
fn test_all_frame_types_encode_decode_roundtrip() {
    for &ft in ALL_FRAME_TYPES {
        let h = FrameHeaderV2 {
            version: PROTOCOL_VERSION_V2,
            frame_type: ft,
            flags: FlagsV2::from_bits(0x00FF),
            sequence: 0x123456789ABCDEF0,
            length: 0xDEADBEEF,
            stream_id: 0xCAFEBABE,
            reserved: 0x12345678,
        };
        let encoded = h.encode();
        let decoded = FrameHeaderV2::decode(&encoded).expect("decode should succeed");
        assert_eq!(h, decoded, "Round-trip failed for {ft:?}");
    }
}

#[test]
fn test_all_individual_flags() {
    let individual_flags = [
        FlagsV2::SYN,
        FlagsV2::FIN,
        FlagsV2::ACK,
        FlagsV2::PRI,
        FlagsV2::CMP,
        FlagsV2::ECN,
        FlagsV2::RTX,
        FlagsV2::EXT,
    ];

    for &flag in &individual_flags {
        let h = FrameHeaderV2 {
            flags: FlagsV2::empty().with(flag),
            ..FrameHeaderV2::new(FrameTypeV2::Data)
        };
        let decoded = FrameHeaderV2::decode(&h.encode()).unwrap();
        assert!(
            decoded.flags.contains(flag),
            "Flag 0x{flag:04X} lost in round-trip"
        );
    }
}

#[test]
fn test_all_flags_combined() {
    let all = FlagsV2::from_bits(0xFFFF);
    let h = FrameHeaderV2 {
        flags: all,
        ..FrameHeaderV2::new(FrameTypeV2::Data)
    };
    let decoded = FrameHeaderV2::decode(&h.encode()).unwrap();
    assert_eq!(decoded.flags.bits(), 0xFFFF);
}

#[test]
fn test_boundary_values() {
    let cases: &[(u64, u32, u32, u32)] = &[
        (0, 0, 0, 0),
        (1, 1, 1, 1),
        (u64::MAX, u32::MAX, u32::MAX, u32::MAX),
        (u32::MAX as u64, u16::MAX as u32, u16::MAX as u32, 0),
        (u32::MAX as u64 + 1, 0, 0, 0),
    ];

    for &(seq, length, stream_id, reserved) in cases {
        let h = FrameHeaderV2 {
            version: PROTOCOL_VERSION_V2,
            frame_type: FrameTypeV2::Data,
            flags: FlagsV2::empty(),
            sequence: seq,
            length,
            stream_id,
            reserved,
        };
        let decoded = FrameHeaderV2::decode(&h.encode()).unwrap();
        assert_eq!(decoded.sequence, seq);
        assert_eq!(decoded.length, length);
        assert_eq!(decoded.stream_id, stream_id);
        assert_eq!(decoded.reserved, reserved);
    }
}

#[test]
fn test_zero_header() {
    let h = FrameHeaderV2 {
        version: 0,
        frame_type: FrameTypeV2::Data,
        flags: FlagsV2::empty(),
        sequence: 0,
        length: 0,
        stream_id: 0,
        reserved: 0,
    };
    let decoded = FrameHeaderV2::decode(&h.encode()).unwrap();
    assert_eq!(decoded, h);
}

#[test]
fn test_encode_into_with_larger_buffer() {
    let h = FrameHeaderV2 {
        version: PROTOCOL_VERSION_V2,
        frame_type: FrameTypeV2::StreamData,
        flags: FlagsV2::empty().with(FlagsV2::SYN).with(FlagsV2::ECN),
        sequence: 999,
        length: 4096,
        stream_id: 7,
        reserved: 0,
    };

    let mut buf = [0xFFu8; 64];
    h.encode_into(&mut buf);

    // Decode from the first 24 bytes
    let decoded = FrameHeaderV2::decode(&buf).unwrap();
    assert_eq!(decoded, h);

    // Bytes after 24 should be untouched
    for &b in &buf[FRAME_HEADER_V2_SIZE..] {
        assert_eq!(b, 0xFF);
    }
}

#[test]
fn test_decode_rejects_short_buffers() {
    for len in 0..FRAME_HEADER_V2_SIZE {
        let buf = vec![0u8; len];
        assert!(FrameHeaderV2::decode(&buf).is_err());
    }
}

#[test]
fn test_decode_accepts_exact_and_larger_buffers() {
    let h = FrameHeaderV2::new(FrameTypeV2::Ping);
    let encoded = h.encode();

    // Exact size
    assert!(FrameHeaderV2::decode(&encoded).is_ok());

    // Larger buffer
    let mut larger = vec![0u8; 128];
    larger[..FRAME_HEADER_V2_SIZE].copy_from_slice(&encoded);
    assert!(FrameHeaderV2::decode(&larger).is_ok());
}

#[test]
fn test_invalid_frame_type_bytes() {
    let h = FrameHeaderV2::new(FrameTypeV2::Data);
    let mut buf = h.encode();

    let invalid_bytes: &[u8] = &[0x03, 0x0F, 0x18, 0x23, 0x36, 0x43, 0x52, 0x70, 0x80, 0xFF];
    for &b in invalid_bytes {
        buf[1] = b;
        assert!(
            FrameHeaderV2::decode(&buf).is_err(),
            "Should reject frame type 0x{b:02X}"
        );
    }
}

#[test]
fn test_version_check() {
    let h = FrameHeaderV2::new(FrameTypeV2::Data);
    assert!(h.is_v2());

    let h_v1 = FrameHeaderV2 {
        version: 0x10,
        ..FrameHeaderV2::new(FrameTypeV2::Data)
    };
    assert!(!h_v1.is_v2());
}

#[test]
fn test_frame_type_categories() {
    assert!(FrameTypeV2::Data.is_data());
    assert!(FrameTypeV2::Ack.is_control());
    assert!(FrameTypeV2::Rekey.is_crypto());
    assert!(FrameTypeV2::StreamOpen.is_stream());
    assert!(FrameTypeV2::PathChallenge.is_path());
    assert!(FrameTypeV2::Close.is_session());
    assert!(FrameTypeV2::Padding.is_obfuscation());
}

mod proptests {
    use proptest::prelude::*;
    use wraith_core::{FlagsV2, FrameHeaderV2, FrameTypeV2, PROTOCOL_VERSION_V2};

    proptest! {
        #[test]
        fn prop_encode_decode_roundtrip(
            seq in any::<u64>(),
            length in any::<u32>(),
            stream_id in any::<u32>(),
            reserved in any::<u32>(),
            flags_bits in any::<u16>(),
            ft_idx in 0..22usize,
        ) {
            let valid_types: &[FrameTypeV2] = &[
                FrameTypeV2::Data, FrameTypeV2::DataFin, FrameTypeV2::Datagram,
                FrameTypeV2::Ack, FrameTypeV2::AckEcn, FrameTypeV2::Ping,
                FrameTypeV2::Pong, FrameTypeV2::Rekey, FrameTypeV2::RekeyAck,
                FrameTypeV2::StreamOpen, FrameTypeV2::StreamData, FrameTypeV2::StreamClose,
                FrameTypeV2::StreamReset, FrameTypeV2::StreamWindow, FrameTypeV2::Priority,
                FrameTypeV2::PathChallenge, FrameTypeV2::PathResponse, FrameTypeV2::PathMigrate,
                FrameTypeV2::Close, FrameTypeV2::CloseAck, FrameTypeV2::Padding,
                FrameTypeV2::PaddingRandom,
            ];
            let ft = valid_types[ft_idx % valid_types.len()];

            let h = FrameHeaderV2 {
                version: PROTOCOL_VERSION_V2,
                frame_type: ft,
                flags: FlagsV2::from_bits(flags_bits),
                sequence: seq,
                length,
                stream_id,
                reserved,
            };
            let decoded = FrameHeaderV2::decode(&h.encode()).unwrap();
            prop_assert_eq!(h, decoded);
        }
    }
}
