//! v2 Frame types and flags definitions.
//!
//! Extended frame type categories and expanded flag bitfields for the
//! WRAITH v2 wire format. Frame types are organized into functional
//! categories with reserved ranges for future expansion.

use crate::error::FrameError;

/// v2 protocol version constant (0x20 = version 2.0).
pub const PROTOCOL_VERSION_V2: u8 = 0x20;

/// v2 frame header size in bytes.
pub const FRAME_HEADER_V2_SIZE: usize = 24;

/// v2 connection ID size in bytes (128 bits).
pub const CONNECTION_ID_V2_SIZE: usize = 16;

/// v2 Frame types organized by functional category.
///
/// Categories:
/// - Data frames (0x00-0x0F): Payload delivery
/// - Control frames (0x10-0x1F): Session management
/// - Crypto frames (0x20-0x2F): Key management
/// - Stream frames (0x30-0x3F): Stream lifecycle
/// - Path frames (0x40-0x4F): Multi-path and migration
/// - Session frames (0x50-0x5F): Session lifecycle
/// - Obfuscation frames (0xF0-0xFF): Traffic analysis resistance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FrameTypeV2 {
    // === Data frames (0x00-0x0F) ===
    /// Standard data payload
    Data = 0x00,
    /// Final data frame in stream
    DataFin = 0x01,
    /// Unreliable datagram (no retransmission)
    Datagram = 0x02,

    // === Control frames (0x10-0x1F) ===
    /// Selective acknowledgment
    Ack = 0x10,
    /// Acknowledgment with ECN feedback
    AckEcn = 0x11,
    /// Keepalive / RTT measurement
    Ping = 0x12,
    /// Response to Ping
    Pong = 0x13,
    /// Flow control window update
    WindowUpdate = 0x14,
    /// Graceful shutdown notification
    GoAway = 0x15,
    /// QoS parameter update
    QosUpdate = 0x16,
    /// Timestamp synchronization
    Timestamp = 0x17,

    // === Crypto frames (0x20-0x2F) ===
    /// Forward secrecy ratchet
    Rekey = 0x20,
    /// Ratchet acknowledgment
    RekeyAck = 0x21,
    /// Forward Error Correction repair data
    FecRepair = 0x22,

    // === Stream frames (0x30-0x3F) ===
    /// Open a new stream
    StreamOpen = 0x30,
    /// Stream data payload
    StreamData = 0x31,
    /// Close stream gracefully
    StreamClose = 0x32,
    /// Abort stream with error
    StreamReset = 0x33,
    /// Stream-level flow control
    StreamWindow = 0x34,
    /// Stream priority update
    Priority = 0x35,

    // === Path frames (0x40-0x4F) ===
    /// Path validation challenge
    PathChallenge = 0x40,
    /// Path validation response
    PathResponse = 0x41,
    /// Connection migration request
    PathMigrate = 0x42,

    // === Session frames (0x50-0x5F) ===
    /// Session close
    Close = 0x50,
    /// Session close acknowledgment
    CloseAck = 0x51,

    // === Group frames (0x60-0x6F) ===
    /// Join group
    GroupJoin = 0x60,
    /// Leave group
    GroupLeave = 0x61,
    /// Group rekey
    GroupRekey = 0x62,

    // === Obfuscation frames (0xF0-0xFF) ===
    /// Fixed-pattern padding
    Padding = 0xF0,
    /// Random-content padding
    PaddingRandom = 0xF1,
}

/// Lookup table for v2 frame type validation.
/// Maps byte value to validity: 0 = invalid, non-zero = valid.
static FRAME_TYPE_V2_TABLE: [u8; 256] = {
    let mut table = [0u8; 256];

    // Data frames
    table[0x00] = 1; // Data
    table[0x01] = 1; // DataFin
    table[0x02] = 1; // Datagram

    // Control frames
    table[0x10] = 1; // Ack
    table[0x11] = 1; // AckEcn
    table[0x12] = 1; // Ping
    table[0x13] = 1; // Pong
    table[0x14] = 1; // WindowUpdate
    table[0x15] = 1; // GoAway
    table[0x16] = 1; // QosUpdate
    table[0x17] = 1; // Timestamp

    // Crypto frames
    table[0x20] = 1; // Rekey
    table[0x21] = 1; // RekeyAck
    table[0x22] = 1; // FecRepair

    // Stream frames
    table[0x30] = 1; // StreamOpen
    table[0x31] = 1; // StreamData
    table[0x32] = 1; // StreamClose
    table[0x33] = 1; // StreamReset
    table[0x34] = 1; // StreamWindow
    table[0x35] = 1; // Priority

    // Path frames
    table[0x40] = 1; // PathChallenge
    table[0x41] = 1; // PathResponse
    table[0x42] = 1; // PathMigrate

    // Session frames
    table[0x50] = 1; // Close
    table[0x51] = 1; // CloseAck

    // Group frames
    table[0x60] = 1; // GroupJoin
    table[0x61] = 1; // GroupLeave
    table[0x62] = 1; // GroupRekey

    // Obfuscation frames
    table[0xF0] = 1; // Padding
    table[0xF1] = 1; // PaddingRandom

    table
};

impl FrameTypeV2 {
    /// Check if this is a data frame (0x00-0x0F).
    #[must_use]
    pub fn is_data(self) -> bool {
        (self as u8) < 0x10
    }

    /// Check if this is a control frame (0x10-0x1F).
    #[must_use]
    pub fn is_control(self) -> bool {
        let b = self as u8;
        (0x10..0x20).contains(&b)
    }

    /// Check if this is a crypto frame (0x20-0x2F).
    #[must_use]
    pub fn is_crypto(self) -> bool {
        let b = self as u8;
        (0x20..0x30).contains(&b)
    }

    /// Check if this is a stream frame (0x30-0x3F).
    #[must_use]
    pub fn is_stream(self) -> bool {
        let b = self as u8;
        (0x30..0x40).contains(&b)
    }

    /// Check if this is a path frame (0x40-0x4F).
    #[must_use]
    pub fn is_path(self) -> bool {
        let b = self as u8;
        (0x40..0x50).contains(&b)
    }

    /// Check if this is a session frame (0x50-0x5F).
    #[must_use]
    pub fn is_session(self) -> bool {
        let b = self as u8;
        (0x50..0x60).contains(&b)
    }

    /// Check if this is an obfuscation frame (0xF0-0xFF).
    #[must_use]
    pub fn is_obfuscation(self) -> bool {
        (self as u8) >= 0xF0
    }

    /// Get the category name of this frame type.
    #[must_use]
    pub fn category(&self) -> &'static str {
        if self.is_data() {
            "data"
        } else if self.is_control() {
            "control"
        } else if self.is_crypto() {
            "crypto"
        } else if self.is_stream() {
            "stream"
        } else if self.is_path() {
            "path"
        } else if self.is_session() {
            "session"
        } else {
            "obfuscation"
        }
    }

    /// Validate whether a byte is a known v2 frame type.
    #[must_use]
    pub fn is_valid_byte(value: u8) -> bool {
        FRAME_TYPE_V2_TABLE[value as usize] != 0
    }
}

impl TryFrom<u8> for FrameTypeV2 {
    type Error = FrameError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if FRAME_TYPE_V2_TABLE[value as usize] != 0 {
            // SAFETY: All values in the table that are non-zero correspond to valid
            // FrameTypeV2 discriminants, which are explicitly defined in the enum.
            Ok(unsafe { core::mem::transmute::<u8, FrameTypeV2>(value) })
        } else {
            Err(FrameError::InvalidFrameType(value))
        }
    }
}

impl From<FrameTypeV2> for u8 {
    fn from(ft: FrameTypeV2) -> u8 {
        ft as u8
    }
}

/// v2 frame flags (16-bit).
///
/// Expanded bitfield with additional flags for ECN support,
/// retransmission marking, extension headers, and compression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FlagsV2(u16);

impl FlagsV2 {
    /// Stream synchronization / initiation
    pub const SYN: u16 = 0b0000_0000_0000_0001;
    /// Final frame in stream
    pub const FIN: u16 = 0b0000_0000_0000_0010;
    /// Acknowledgment data present
    pub const ACK: u16 = 0b0000_0000_0000_0100;
    /// Priority frame (expedited processing)
    pub const PRI: u16 = 0b0000_0000_0000_1000;
    /// Payload is compressed
    pub const CMP: u16 = 0b0000_0000_0001_0000;
    /// Explicit Congestion Notification
    pub const ECN: u16 = 0b0000_0000_0010_0000;
    /// Retransmitted frame
    pub const RTX: u16 = 0b0000_0000_0100_0000;
    /// Extension header(s) present
    pub const EXT: u16 = 0b0000_0000_1000_0000;

    /// Create empty flags.
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create from raw bits.
    #[must_use]
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    /// Get raw bits.
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Check if a specific flag is set.
    #[must_use]
    pub const fn contains(self, flag: u16) -> bool {
        self.0 & flag != 0
    }

    /// Set a flag.
    #[must_use]
    pub const fn with(self, flag: u16) -> Self {
        Self(self.0 | flag)
    }

    /// Clear a flag.
    #[must_use]
    pub const fn without(self, flag: u16) -> Self {
        Self(self.0 & !flag)
    }

    /// Check SYN flag.
    #[must_use]
    pub const fn is_syn(self) -> bool {
        self.contains(Self::SYN)
    }

    /// Check FIN flag.
    #[must_use]
    pub const fn is_fin(self) -> bool {
        self.contains(Self::FIN)
    }

    /// Check ACK flag.
    #[must_use]
    pub const fn is_ack(self) -> bool {
        self.contains(Self::ACK)
    }

    /// Check PRI flag.
    #[must_use]
    pub const fn is_pri(self) -> bool {
        self.contains(Self::PRI)
    }

    /// Check CMP (compressed) flag.
    #[must_use]
    pub const fn is_compressed(self) -> bool {
        self.contains(Self::CMP)
    }

    /// Check ECN flag.
    #[must_use]
    pub const fn is_ecn(self) -> bool {
        self.contains(Self::ECN)
    }

    /// Check RTX (retransmission) flag.
    #[must_use]
    pub const fn is_retransmit(self) -> bool {
        self.contains(Self::RTX)
    }

    /// Check EXT (extension) flag.
    #[must_use]
    pub const fn has_extensions(self) -> bool {
        self.contains(Self::EXT)
    }
}

/// Convert v1 `FrameType` to v2 `FrameTypeV2`.
///
/// Maps the v1 frame type numbering to the equivalent v2 categorized types.
impl From<super::FrameType> for FrameTypeV2 {
    fn from(v1: super::FrameType) -> Self {
        match v1 {
            super::FrameType::Reserved => FrameTypeV2::Padding, // map reserved to padding
            super::FrameType::Data => FrameTypeV2::Data,
            super::FrameType::Ack => FrameTypeV2::Ack,
            super::FrameType::Control => FrameTypeV2::GoAway, // closest mapping
            super::FrameType::Rekey => FrameTypeV2::Rekey,
            super::FrameType::Ping => FrameTypeV2::Ping,
            super::FrameType::Pong => FrameTypeV2::Pong,
            super::FrameType::Close => FrameTypeV2::Close,
            super::FrameType::Pad => FrameTypeV2::Padding,
            super::FrameType::StreamOpen => FrameTypeV2::StreamOpen,
            super::FrameType::StreamClose => FrameTypeV2::StreamClose,
            super::FrameType::StreamReset => FrameTypeV2::StreamReset,
            super::FrameType::WindowUpdate => FrameTypeV2::WindowUpdate,
            super::FrameType::GoAway => FrameTypeV2::GoAway,
            super::FrameType::PathChallenge => FrameTypeV2::PathChallenge,
            super::FrameType::PathResponse => FrameTypeV2::PathResponse,
        }
    }
}

/// Convert v1 `FrameFlags` to v2 `FlagsV2`.
///
/// The lower 5 bits of v1 flags map directly to the same positions in v2.
impl From<super::FrameFlags> for FlagsV2 {
    fn from(v1: super::FrameFlags) -> Self {
        Self(u16::from(v1.as_u8()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_type_v2_data_category() {
        assert!(FrameTypeV2::Data.is_data());
        assert!(FrameTypeV2::DataFin.is_data());
        assert!(FrameTypeV2::Datagram.is_data());
        assert!(!FrameTypeV2::Ack.is_data());
    }

    #[test]
    fn test_frame_type_v2_control_category() {
        assert!(FrameTypeV2::Ack.is_control());
        assert!(FrameTypeV2::AckEcn.is_control());
        assert!(FrameTypeV2::Ping.is_control());
        assert!(FrameTypeV2::Pong.is_control());
        assert!(FrameTypeV2::WindowUpdate.is_control());
        assert!(FrameTypeV2::GoAway.is_control());
        assert!(FrameTypeV2::QosUpdate.is_control());
        assert!(FrameTypeV2::Timestamp.is_control());
        assert!(!FrameTypeV2::Data.is_control());
    }

    #[test]
    fn test_frame_type_v2_crypto_category() {
        assert!(FrameTypeV2::Rekey.is_crypto());
        assert!(FrameTypeV2::RekeyAck.is_crypto());
        assert!(FrameTypeV2::FecRepair.is_crypto());
    }

    #[test]
    fn test_frame_type_v2_stream_category() {
        assert!(FrameTypeV2::StreamOpen.is_stream());
        assert!(FrameTypeV2::StreamData.is_stream());
        assert!(FrameTypeV2::StreamClose.is_stream());
        assert!(FrameTypeV2::StreamReset.is_stream());
        assert!(FrameTypeV2::StreamWindow.is_stream());
        assert!(FrameTypeV2::Priority.is_stream());
    }

    #[test]
    fn test_frame_type_v2_path_category() {
        assert!(FrameTypeV2::PathChallenge.is_path());
        assert!(FrameTypeV2::PathResponse.is_path());
        assert!(FrameTypeV2::PathMigrate.is_path());
    }

    #[test]
    fn test_frame_type_v2_session_category() {
        assert!(FrameTypeV2::Close.is_session());
        assert!(FrameTypeV2::CloseAck.is_session());
    }

    #[test]
    fn test_frame_type_v2_obfuscation_category() {
        assert!(FrameTypeV2::Padding.is_obfuscation());
        assert!(FrameTypeV2::PaddingRandom.is_obfuscation());
    }

    #[test]
    fn test_frame_type_v2_category_names() {
        assert_eq!(FrameTypeV2::Data.category(), "data");
        assert_eq!(FrameTypeV2::Ack.category(), "control");
        assert_eq!(FrameTypeV2::Rekey.category(), "crypto");
        assert_eq!(FrameTypeV2::StreamOpen.category(), "stream");
        assert_eq!(FrameTypeV2::PathChallenge.category(), "path");
        assert_eq!(FrameTypeV2::Close.category(), "session");
        assert_eq!(FrameTypeV2::Padding.category(), "obfuscation");
    }

    #[test]
    fn test_frame_type_v2_try_from_valid() {
        let valid_bytes: &[u8] = &[
            0x00, 0x01, 0x02, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x20, 0x21, 0x22,
            0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x40, 0x41, 0x42, 0x50, 0x51, 0x60, 0x61, 0x62,
            0xF0, 0xF1,
        ];
        for &b in valid_bytes {
            assert!(
                FrameTypeV2::try_from(b).is_ok(),
                "Expected 0x{b:02X} to be valid"
            );
        }
    }

    #[test]
    fn test_frame_type_v2_try_from_invalid() {
        let invalid_bytes: &[u8] = &[
            0x03, 0x0F, 0x18, 0x23, 0x36, 0x43, 0x52, 0x70, 0x80, 0xEF, 0xFF,
        ];
        for &b in invalid_bytes {
            assert!(
                FrameTypeV2::try_from(b).is_err(),
                "Expected 0x{b:02X} to be invalid"
            );
        }
    }

    #[test]
    fn test_frame_type_v2_roundtrip() {
        let types = [
            FrameTypeV2::Data,
            FrameTypeV2::DataFin,
            FrameTypeV2::Ack,
            FrameTypeV2::Rekey,
            FrameTypeV2::StreamOpen,
            FrameTypeV2::PathChallenge,
            FrameTypeV2::Close,
            FrameTypeV2::Padding,
        ];
        for ft in types {
            let byte: u8 = ft.into();
            let recovered = FrameTypeV2::try_from(byte).unwrap();
            assert_eq!(ft, recovered);
        }
    }

    #[test]
    fn test_frame_type_v2_is_valid_byte() {
        assert!(FrameTypeV2::is_valid_byte(0x00));
        assert!(FrameTypeV2::is_valid_byte(0xF0));
        assert!(!FrameTypeV2::is_valid_byte(0xFF));
        assert!(!FrameTypeV2::is_valid_byte(0x80));
    }

    #[test]
    fn test_flags_v2_empty() {
        let f = FlagsV2::empty();
        assert_eq!(f.bits(), 0);
        assert!(!f.is_syn());
        assert!(!f.is_fin());
        assert!(!f.is_ecn());
    }

    #[test]
    fn test_flags_v2_set_and_check() {
        let f = FlagsV2::empty()
            .with(FlagsV2::SYN)
            .with(FlagsV2::ECN)
            .with(FlagsV2::CMP);
        assert!(f.is_syn());
        assert!(f.is_ecn());
        assert!(f.is_compressed());
        assert!(!f.is_fin());
        assert!(!f.is_retransmit());
    }

    #[test]
    fn test_flags_v2_clear() {
        let f = FlagsV2::from_bits(0xFFFF);
        let f = f.without(FlagsV2::SYN);
        assert!(!f.is_syn());
        assert!(f.is_fin());
    }

    #[test]
    fn test_flags_v2_roundtrip() {
        for bits in [0u16, 0x00FF, 0xFF00, 0xFFFF, 0x0055, 0x00AA] {
            let f = FlagsV2::from_bits(bits);
            assert_eq!(f.bits(), bits);
        }
    }

    #[test]
    fn test_flags_v2_individual_flags() {
        let flags_and_checks: &[(u16, fn(FlagsV2) -> bool)] = &[
            (FlagsV2::SYN, FlagsV2::is_syn),
            (FlagsV2::FIN, FlagsV2::is_fin),
            (FlagsV2::ACK, FlagsV2::is_ack),
            (FlagsV2::PRI, FlagsV2::is_pri),
            (FlagsV2::CMP, FlagsV2::is_compressed),
            (FlagsV2::ECN, FlagsV2::is_ecn),
            (FlagsV2::RTX, FlagsV2::is_retransmit),
            (FlagsV2::EXT, FlagsV2::has_extensions),
        ];
        for &(flag, check) in flags_and_checks {
            let f = FlagsV2::empty().with(flag);
            assert!(check(f), "Flag 0x{flag:04X} should be set");
        }
    }

    #[test]
    fn test_v1_to_v2_frame_type_conversion() {
        use super::super::FrameType;
        assert_eq!(FrameTypeV2::from(FrameType::Data), FrameTypeV2::Data);
        assert_eq!(FrameTypeV2::from(FrameType::Ack), FrameTypeV2::Ack);
        assert_eq!(FrameTypeV2::from(FrameType::Rekey), FrameTypeV2::Rekey);
        assert_eq!(FrameTypeV2::from(FrameType::Ping), FrameTypeV2::Ping);
        assert_eq!(FrameTypeV2::from(FrameType::Close), FrameTypeV2::Close);
        assert_eq!(
            FrameTypeV2::from(FrameType::StreamOpen),
            FrameTypeV2::StreamOpen
        );
        assert_eq!(
            FrameTypeV2::from(FrameType::PathChallenge),
            FrameTypeV2::PathChallenge
        );
    }

    #[test]
    fn test_v1_to_v2_flags_conversion() {
        use super::super::FrameFlags;
        let v1 = FrameFlags::new().with_syn().with_fin();
        let v2: FlagsV2 = v1.into();
        assert!(v2.is_syn());
        assert!(v2.is_fin());
        assert!(!v2.is_ecn());
    }
}
