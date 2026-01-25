use alloc::vec::Vec;

pub const FRAME_TYPE_DATA: u8 = 0x01;
pub const FRAME_TYPE_CONTROL: u8 = 0x03;
pub const FRAME_TYPE_REKEY: u8 = 0x04;

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
}
