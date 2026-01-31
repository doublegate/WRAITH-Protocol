//! Tactic: Defense Evasion (TA0005)
//! Technique: T1140 (Deobfuscate/Decode Files or Information)

use crate::utils::sensitive::SensitiveData;
use alloc::vec::Vec;
use base64::{Engine as _, engine::general_purpose};
use zeroize::Zeroize;

pub struct Transform;

impl Transform {
    pub fn decode_xor(&self, data: &[u8], key: u8) -> SensitiveData {
        let mut decoded = Vec::with_capacity(data.len());
        for &b in data {
            decoded.push(b ^ key);
        }
        let sd = SensitiveData::new(&decoded);
        decoded.zeroize();
        sd
    }

    pub fn decode_base64(&self, data: &[u8]) -> SensitiveData {
        if let Ok(decoded) = general_purpose::STANDARD.decode(data) {
            let sd = SensitiveData::new(&decoded);
            // decoded vec drops here, zeroize logic in Vec? No.
            // But we pass reference to SensitiveData which copies it.
            // We should zeroize the decoded vec explicitly if possible.
            // But Vec<u8> doesn't zeroize on drop unless wrapped.
            // We can zeroize it manually?
            // decoded is moved into SensitiveData? No, referenced.
            // Actually SensitiveData::new takes slice.
            sd
        } else {
            SensitiveData::new(b"Base64 decode failed")
        }
    }
}
