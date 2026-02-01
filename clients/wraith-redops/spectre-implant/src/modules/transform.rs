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
        if let Ok(mut decoded) = general_purpose::STANDARD.decode(data) {
            let sd = SensitiveData::new(&decoded);
            decoded.zeroize();
            sd
        } else {
            SensitiveData::new(b"Base64 decode failed")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_base64() {
        let t = Transform;
        let encoded = b"SGVsbG8=";
        let sensitive = t.decode_base64(encoded);
        let guard = sensitive.unlock().expect("Unlock failed");
        assert_eq!(&*guard, b"Hello");
    }

    #[test]
    fn test_decode_base64_invalid() {
        let t = Transform;
        let encoded = b"InvalidBase64!!!!";
        let sensitive = t.decode_base64(encoded);
        let guard = sensitive.unlock().expect("Unlock failed");
        assert_eq!(&*guard, b"Base64 decode failed");
    }
}
