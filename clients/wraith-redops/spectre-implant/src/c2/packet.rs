use alloc::vec::Vec;

pub const FRAME_TYPE_DATA: u8 = 0x01;
pub const FRAME_TYPE_CONTROL: u8 = 0x03;
pub const FRAME_TYPE_REKEY: u8 = 0x04;
pub const FRAME_TYPE_MESH_RELAY: u8 = 0x05;
pub const FRAME_REKEY_DH: u8 = 0x06;
pub const FRAME_PQ_KEX: u8 = 0x07;

pub struct WraithFrame {
    pub nonce: u64,
    pub frame_type: u8,
    pub flags: u8,
    pub stream_id: u16,
    pub sequence: u32,
    pub offset: u64,
    pub payload: Vec<u8>,
}

impl WraithFrame {
    pub fn new(frame_type: u8, payload: Vec<u8>) -> Self {
        Self {
            nonce: 0, // Should be set by session
            frame_type,
            flags: 0,
            stream_id: 0,
            sequence: 0,
            offset: 0,
            payload,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(28 + self.payload.len());
        buf.extend_from_slice(&self.nonce.to_be_bytes());
        buf.push(self.frame_type);
        buf.push(self.flags);
        buf.extend_from_slice(&self.stream_id.to_be_bytes());
        buf.extend_from_slice(&self.sequence.to_be_bytes());
        buf.extend_from_slice(&self.offset.to_be_bytes());
        buf.extend_from_slice(&(self.payload.len() as u16).to_be_bytes());
        buf.extend_from_slice(&[0u8; 2]); // Reserved
        buf.extend_from_slice(&self.payload);
        buf
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 28 {
            return None;
        }

        let nonce = u64::from_be_bytes(data[0..8].try_into().ok()?);
        let frame_type = data[8];
        let flags = data[9];
        let stream_id = u16::from_be_bytes(data[10..12].try_into().ok()?);
        let sequence = u32::from_be_bytes(data[12..16].try_into().ok()?);
        let offset = u64::from_be_bytes(data[16..24].try_into().ok()?);
        let payload_len = u16::from_be_bytes(data[24..26].try_into().ok()?) as usize;

        if data.len() < 28 + payload_len {
            return None;
        }

        let payload = data[28..28 + payload_len].to_vec();

        Some(Self {
            nonce,
            frame_type,
            flags,
            stream_id,
            sequence,
            offset,
            payload,
        })
    }
}
