//! Integration tests for v1/v2 wire format compatibility layer.

use wraith_core::frame::FrameHeader;
use wraith_core::frame::compat::{
    FormatNegotiation, WireFormat, decode_v1_header, detect_format, encode_v1_header,
    v1_header_to_v2, v2_header_to_v1,
};
use wraith_core::{
    FlagsV2, FrameFlags, FrameHeaderV2, FrameType, FrameTypeV2, PROTOCOL_VERSION_V2,
};

#[test]
fn test_v1_to_v2_to_v1_roundtrip() {
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

    let recovered = v2_header_to_v1(&v2).expect("Should convert back to v1");
    assert_eq!(recovered.frame_type, v1.frame_type);
    assert_eq!(recovered.stream_id, v1.stream_id);
    assert_eq!(recovered.sequence, v1.sequence);
    assert_eq!(recovered.payload_len, v1.payload_len);
    // Note: flags lower bits should match, offset is lost in v2
}

#[test]
fn test_v1_header_encode_decode_roundtrip() {
    let header = FrameHeader {
        frame_type: FrameType::Ack,
        flags: FrameFlags::new(),
        stream_id: 100,
        sequence: 50000,
        offset: 0,
        payload_len: 0,
    };
    let nonce = [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE];

    let mut buf = [0u8; 28];
    encode_v1_header(&header, &nonce, &mut buf);
    let (decoded, decoded_nonce) = decode_v1_header(&buf).unwrap();

    assert_eq!(decoded.frame_type, header.frame_type);
    assert_eq!(decoded.stream_id, header.stream_id);
    assert_eq!(decoded.sequence, header.sequence);
    assert_eq!(decoded.payload_len, header.payload_len);
    assert_eq!(decoded_nonce, nonce);
}

#[test]
fn test_detect_format_v1_packet() {
    let mut data = vec![0u8; 28];
    // v1: byte 8 is frame type in range 0x01-0x0F
    data[8] = 0x01; // Data
    assert_eq!(detect_format(&data), Some(WireFormat::V1));
}

#[test]
fn test_detect_format_v2_packet() {
    let h = FrameHeaderV2::new(FrameTypeV2::Data);
    let encoded = h.encode();
    assert_eq!(detect_format(&encoded), Some(WireFormat::V2));
}

#[test]
fn test_detect_format_actual_v2_encoded_headers() {
    let types = [
        FrameTypeV2::Data,
        FrameTypeV2::Ack,
        FrameTypeV2::Ping,
        FrameTypeV2::StreamOpen,
        FrameTypeV2::Close,
        FrameTypeV2::Padding,
    ];
    for ft in types {
        let h = FrameHeaderV2::new(ft);
        let encoded = h.encode();
        let detected = detect_format(&encoded);
        assert_eq!(
            detected,
            Some(WireFormat::V2),
            "Failed to detect v2 for {ft:?}"
        );
    }
}

#[test]
fn test_detect_format_too_short() {
    assert_eq!(detect_format(&[]), None);
    assert_eq!(detect_format(&[0x20]), None);
    assert_eq!(detect_format(&[0x20, 0x00]), None); // Only 2 bytes, need >= 24
}

#[test]
fn test_detect_format_ambiguous_data() {
    let data = vec![0u8; 28]; // All zeros
    assert_eq!(detect_format(&data), None);
}

#[test]
fn test_negotiation_both_default() {
    let local = FormatNegotiation::default();
    let remote = FormatNegotiation::default();
    assert_eq!(local.negotiate(&remote), Some(WireFormat::V2Polymorphic));
}

#[test]
fn test_negotiation_v1_only_vs_default() {
    let local = FormatNegotiation::v1_only();
    let remote = FormatNegotiation::default();
    assert_eq!(local.negotiate(&remote), Some(WireFormat::V1));
}

#[test]
fn test_negotiation_v2_only_vs_v1_only_fails() {
    let local = FormatNegotiation::v2_only();
    let remote = FormatNegotiation::v1_only();
    assert_eq!(local.negotiate(&remote), None);
}

#[test]
fn test_negotiation_v2_only_both() {
    let local = FormatNegotiation::v2_only();
    let remote = FormatNegotiation::v2_only();
    assert_eq!(local.negotiate(&remote), Some(WireFormat::V2Polymorphic));
}

#[test]
fn test_negotiation_symmetry() {
    let configs = [
        FormatNegotiation::default(),
        FormatNegotiation::v1_only(),
        FormatNegotiation::v2_only(),
    ];
    for a in &configs {
        for b in &configs {
            assert_eq!(
                a.negotiate(b),
                b.negotiate(a),
                "Negotiation should be symmetric"
            );
        }
    }
}

#[test]
fn test_wire_format_header_sizes() {
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
fn test_all_v1_types_have_v2_equivalent() {
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
            stream_id: 1,
            sequence: 1,
            offset: 0,
            payload_len: 0,
        };
        let v2 = v1_header_to_v2(&v1);
        assert!(v2.is_v2(), "Converted header should be v2 for {ft:?}");
        let encoded = v2.encode();
        assert!(FrameHeaderV2::decode(&encoded).is_ok());
    }
}

#[test]
fn test_v2_types_without_v1_equivalent() {
    let v2_only_types = [
        FrameTypeV2::Datagram,
        FrameTypeV2::QosUpdate,
        FrameTypeV2::Timestamp,
        FrameTypeV2::FecRepair,
        FrameTypeV2::StreamData,
        FrameTypeV2::Priority,
        FrameTypeV2::PathMigrate,
        FrameTypeV2::GroupJoin,
        FrameTypeV2::GroupLeave,
        FrameTypeV2::GroupRekey,
    ];

    for ft in v2_only_types {
        let v2 = FrameHeaderV2::new(ft);
        let result = v2_header_to_v1(&v2);
        assert!(result.is_none(), "{ft:?} should have no v1 equivalent");
    }
}

#[test]
fn test_v2_to_v1_truncation() {
    // v2 fields wider than v1 get truncated
    let v2 = FrameHeaderV2 {
        version: PROTOCOL_VERSION_V2,
        frame_type: FrameTypeV2::Data,
        flags: FlagsV2::from_bits(0x0103), // upper bits lost in v1 (only 8-bit)
        sequence: 0x1_0000_0001,           // > u32::MAX, truncates
        length: 0x1_0001,                  // > u16::MAX, truncates
        stream_id: 0x1_0001,               // > u16::MAX, truncates
        reserved: 0,
    };

    let v1 = v2_header_to_v1(&v2).unwrap();
    assert_eq!(v1.sequence, 1); // truncated to lower 32 bits
    assert_eq!(v1.payload_len, 1); // truncated to lower 16 bits
    assert_eq!(v1.stream_id, 1); // truncated to lower 16 bits
}
