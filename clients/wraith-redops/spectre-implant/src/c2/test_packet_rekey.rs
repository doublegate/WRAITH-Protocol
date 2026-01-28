#[cfg(test)]
mod tests {
    use super::super::packet::{WraithFrame, FRAME_REKEY_DH};
    use alloc::vec::Vec;

    #[test]
    fn test_frame_rekey_dh_constant() {
        // This should fail to compile initially because FRAME_REKEY_DH is not defined
        let payload = Vec::from([0u8; 32]); // Dummy 32-byte public key
        let frame = WraithFrame::new(FRAME_REKEY_DH, payload);
        
        assert_eq!(frame.frame_type, FRAME_REKEY_DH);
        assert_eq!(FRAME_REKEY_DH, 0x06); // Expecting 0x06 as the new type
    }
}
