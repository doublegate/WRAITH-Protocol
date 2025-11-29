//! Frame encoding and decoding for the WRAITH wire protocol.
//!
//! This module implements zero-copy parsing of protocol frames with
//! careful attention to alignment for DMA efficiency. All multi-byte
//! fields are big-endian (network byte order).

use crate::FRAME_HEADER_SIZE;
use crate::error::FrameError;

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
    pub fn new() -> Self {
        Self(0)
    }

    /// Add SYN flag
    pub fn with_syn(mut self) -> Self {
        self.0 |= Self::SYN;
        self
    }

    /// Add FIN flag
    pub fn with_fin(mut self) -> Self {
        self.0 |= Self::FIN;
        self
    }

    /// Check if SYN is set
    pub fn is_syn(&self) -> bool {
        self.0 & Self::SYN != 0
    }

    /// Check if FIN is set
    pub fn is_fin(&self) -> bool {
        self.0 & Self::FIN != 0
    }

    /// Check if payload is compressed
    pub fn is_compressed(&self) -> bool {
        self.0 & Self::CMP != 0
    }

    /// Get raw byte value
    pub fn as_u8(&self) -> u8 {
        self.0
    }
}

/// Zero-copy frame view into a packet buffer
#[derive(Debug)]
pub struct Frame<'a> {
    raw: &'a [u8],
    frame_type: FrameType,
    flags: FrameFlags,
    stream_id: u16,
    sequence: u32,
    offset: u64,
    payload_len: u16,
}

impl<'a> Frame<'a> {
    /// Parse a frame from raw bytes (zero-copy)
    pub fn parse(data: &'a [u8]) -> Result<Self, FrameError> {
        if data.len() < FRAME_HEADER_SIZE {
            return Err(FrameError::TooShort {
                expected: FRAME_HEADER_SIZE,
                actual: data.len(),
            });
        }

        let frame_type = FrameType::try_from(data[8])?;
        let flags = FrameFlags(data[9]);
        let stream_id = u16::from_be_bytes([data[10], data[11]]);
        let sequence = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
        let offset = u64::from_be_bytes([
            data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
        ]);
        let payload_len = u16::from_be_bytes([data[24], data[25]]);

        if FRAME_HEADER_SIZE + payload_len as usize > data.len() {
            return Err(FrameError::PayloadOverflow);
        }

        Ok(Self {
            raw: data,
            frame_type,
            flags,
            stream_id,
            sequence,
            offset,
            payload_len,
        })
    }

    /// Get the frame type
    pub fn frame_type(&self) -> FrameType {
        self.frame_type
    }

    /// Get the frame flags
    pub fn flags(&self) -> FrameFlags {
        self.flags
    }

    /// Get the stream ID
    pub fn stream_id(&self) -> u16 {
        self.stream_id
    }

    /// Get the sequence number
    pub fn sequence(&self) -> u32 {
        self.sequence
    }

    /// Get the file offset
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Get the nonce bytes
    pub fn nonce(&self) -> &[u8] {
        &self.raw[0..8]
    }

    /// Get the payload slice (zero-copy)
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the frame type
    pub fn frame_type(mut self, ft: FrameType) -> Self {
        self.frame_type = Some(ft);
        self
    }

    /// Set the flags
    pub fn flags(mut self, flags: FrameFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set the stream ID
    pub fn stream_id(mut self, id: u16) -> Self {
        self.stream_id = id;
        self
    }

    /// Set the sequence number
    pub fn sequence(mut self, seq: u32) -> Self {
        self.sequence = seq;
        self
    }

    /// Set the file offset
    pub fn offset(mut self, off: u64) -> Self {
        self.offset = off;
        self
    }

    /// Set the payload
    pub fn payload(mut self, data: &[u8]) -> Self {
        self.payload = data.to_vec();
        self
    }

    /// Set the nonce
    pub fn nonce(mut self, n: [u8; 8]) -> Self {
        self.nonce = n;
        self
    }

    /// Build the frame into a byte buffer
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
        buf.extend_from_slice(&(payload_len as u16).to_be_bytes());
        buf.extend_from_slice(&[0u8; 2]); // Reserved

        // Write payload
        buf.extend_from_slice(&self.payload);

        // Write random padding
        let mut padding = vec![0u8; padding_len];
        getrandom::fill(&mut padding).expect("CSPRNG failure");
        buf.extend_from_slice(&padding);

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
}
