//! WebSocket protocol mimicry.
//!
//! Wraps WRAITH packets in WebSocket binary frames to blend with
//! WebSocket traffic and evade DPI.

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

/// WebSocket opcode for binary frame
const WEBSOCKET_OPCODE_BINARY: u8 = 0x02;
/// WebSocket FIN bit
const WEBSOCKET_FIN_BIT: u8 = 0x80;

/// WebSocket frame wrapper
///
/// Wraps WRAITH packets in WebSocket binary frames with optional masking.
///
/// # Examples
///
/// ```
/// use wraith_obfuscation::websocket_mimicry::WebSocketFrameWrapper;
///
/// let wrapper = WebSocketFrameWrapper::new(false); // Server (no masking)
/// let payload = b"hello";
/// let frame = wrapper.wrap(payload);
///
/// let unwrapped = wrapper.unwrap(&frame).unwrap();
/// assert_eq!(unwrapped, payload);
/// ```
pub struct WebSocketFrameWrapper {
    client_to_server: bool, // Clients must mask frames
    rng: std::sync::Mutex<SmallRng>,
}

impl WebSocketFrameWrapper {
    /// Create a new WebSocket frame wrapper
    ///
    /// # Arguments
    ///
    /// * `client_to_server` - If true, frames will be masked (client mode)
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::websocket_mimicry::WebSocketFrameWrapper;
    ///
    /// let client_wrapper = WebSocketFrameWrapper::new(true);
    /// let server_wrapper = WebSocketFrameWrapper::new(false);
    /// ```
    #[must_use]
    pub fn new(client_to_server: bool) -> Self {
        Self {
            client_to_server,
            rng: std::sync::Mutex::new(
                SmallRng::from_rng(&mut rand::thread_rng()).expect("RNG seeding failed"),
            ),
        }
    }

    /// Wrap payload in WebSocket frame
    ///
    /// Creates a WebSocket binary frame containing the payload.
    /// Frames are masked if in client mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::websocket_mimicry::WebSocketFrameWrapper;
    ///
    /// let wrapper = WebSocketFrameWrapper::new(true);
    /// let frame = wrapper.wrap(b"test data");
    /// assert_eq!(frame[0] & 0x0F, 0x02); // Binary opcode
    /// ```
    pub fn wrap(&self, payload: &[u8]) -> Vec<u8> {
        // Pre-compute header and mask sizes for allocation
        let header_len = if payload.len() < 126 {
            2
        } else if payload.len() < 65536 {
            4
        } else {
            10
        };
        let mask_len = if self.client_to_server { 4 } else { 0 };
        let mut frame = Vec::with_capacity(header_len + mask_len + payload.len());

        // Byte 1: FIN + RSV + OPCODE
        frame.push(WEBSOCKET_FIN_BIT | WEBSOCKET_OPCODE_BINARY);

        // Byte 2: MASK + Payload length
        let mask_bit = if self.client_to_server { 0x80 } else { 0x00 };

        if payload.len() < 126 {
            frame.push(mask_bit | payload.len() as u8);
        } else if payload.len() < 65536 {
            frame.push(mask_bit | 126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        } else {
            frame.push(mask_bit | 127);
            frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }

        // Masking key (if client) -- use SmallRng for performance
        let masking_key = if self.client_to_server {
            let key: [u8; 4] = self.rng.lock().unwrap().r#gen();
            frame.extend_from_slice(&key);
            Some(key)
        } else {
            None
        };

        // Payload (masked if client) -- use 4-byte XOR for performance
        if let Some(key) = masking_key {
            let mask_u32 = u32::from_ne_bytes(key);
            let aligned_len = payload.len() & !3;
            let (chunks, remainder) = payload.split_at(aligned_len);

            for chunk in chunks.chunks_exact(4) {
                let val = u32::from_ne_bytes(chunk.try_into().unwrap()) ^ mask_u32;
                frame.extend_from_slice(&val.to_ne_bytes());
            }
            for (i, &byte) in remainder.iter().enumerate() {
                frame.push(byte ^ key[i]);
            }
        } else {
            frame.extend_from_slice(payload);
        }

        frame
    }

    /// Unwrap WebSocket frame to get payload
    ///
    /// Extracts the original payload from a WebSocket frame,
    /// handling masking if present.
    ///
    /// # Errors
    ///
    /// Returns `WsError` if the frame is malformed or incomplete.
    ///
    /// # Examples
    ///
    /// ```
    /// use wraith_obfuscation::websocket_mimicry::WebSocketFrameWrapper;
    ///
    /// let wrapper = WebSocketFrameWrapper::new(false);
    /// let original = b"test";
    /// let frame = wrapper.wrap(original);
    ///
    /// let unwrapped = wrapper.unwrap(&frame).unwrap();
    /// assert_eq!(unwrapped, original);
    /// ```
    pub fn unwrap(&self, frame: &[u8]) -> Result<Vec<u8>, WsError> {
        if frame.len() < 2 {
            return Err(WsError::TooShort);
        }

        let _fin = (frame[0] & 0x80) != 0;
        let opcode = frame[0] & 0x0F;

        if opcode != WEBSOCKET_OPCODE_BINARY {
            return Err(WsError::InvalidOpcode);
        }

        let masked = (frame[1] & 0x80) != 0;
        let mut payload_len = (frame[1] & 0x7F) as usize;
        let mut offset = 2;

        // Extended payload length
        if payload_len == 126 {
            if frame.len() < 4 {
                return Err(WsError::TooShort);
            }
            payload_len = u16::from_be_bytes([frame[2], frame[3]]) as usize;
            offset = 4;
        } else if payload_len == 127 {
            if frame.len() < 10 {
                return Err(WsError::TooShort);
            }
            payload_len = u64::from_be_bytes([
                frame[2], frame[3], frame[4], frame[5], frame[6], frame[7], frame[8], frame[9],
            ]) as usize;
            offset = 10;
        }

        // Masking key
        let masking_key = if masked {
            if frame.len() < offset + 4 {
                return Err(WsError::TooShort);
            }
            let key = [
                frame[offset],
                frame[offset + 1],
                frame[offset + 2],
                frame[offset + 3],
            ];
            offset += 4;
            Some(key)
        } else {
            None
        };

        // Payload
        if frame.len() < offset + payload_len {
            return Err(WsError::IncompleteFrame);
        }

        let payload = if let Some(key) = masking_key {
            frame[offset..offset + payload_len]
                .iter()
                .enumerate()
                .map(|(i, &byte)| byte ^ key[i % 4])
                .collect()
        } else {
            frame[offset..offset + payload_len].to_vec()
        };

        Ok(payload)
    }

    /// Check if this wrapper is in client mode
    #[must_use]
    pub fn is_client_mode(&self) -> bool {
        self.client_to_server
    }
}

impl Default for WebSocketFrameWrapper {
    fn default() -> Self {
        Self::new(false) // Default to server mode
    }
}

/// WebSocket error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsError {
    /// Frame too short to parse
    TooShort,
    /// Invalid opcode
    InvalidOpcode,
    /// Incomplete frame
    IncompleteFrame,
}

impl std::fmt::Display for WsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "WebSocket frame too short"),
            Self::InvalidOpcode => write!(f, "Invalid WebSocket opcode"),
            Self::IncompleteFrame => write!(f, "Incomplete WebSocket frame"),
        }
    }
}

impl std::error::Error for WsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_wrap_unwrap_server() {
        let wrapper = WebSocketFrameWrapper::new(false); // Server doesn't mask
        let payload = b"hello";

        let frame = wrapper.wrap(payload);
        let unwrapped = wrapper.unwrap(&frame).unwrap();

        assert_eq!(unwrapped, payload);
    }

    #[test]
    fn test_websocket_wrap_unwrap_client() {
        let wrapper = WebSocketFrameWrapper::new(true); // Client masks
        let payload = b"hello";

        let frame = wrapper.wrap(payload);

        // Frame should be masked (payload at offset 6 should differ)
        assert_ne!(&frame[6..11], payload);

        let unwrapped = wrapper.unwrap(&frame).unwrap();
        assert_eq!(unwrapped, payload);
    }

    #[test]
    fn test_websocket_frame_structure() {
        let wrapper = WebSocketFrameWrapper::new(false);
        let payload = b"test";

        let frame = wrapper.wrap(payload);

        // Check FIN bit and opcode
        assert_eq!(frame[0], WEBSOCKET_FIN_BIT | WEBSOCKET_OPCODE_BINARY);

        // Check payload length (small frame, < 126 bytes)
        assert_eq!(frame[1], payload.len() as u8);
    }

    #[test]
    fn test_websocket_extended_length_medium() {
        let wrapper = WebSocketFrameWrapper::new(false);
        let payload = vec![0u8; 300]; // > 125 bytes

        let frame = wrapper.wrap(&payload);

        // Should use 16-bit extended length
        assert_eq!(frame[1], 126);

        let unwrapped = wrapper.unwrap(&frame).unwrap();
        assert_eq!(unwrapped, payload);
    }

    #[test]
    fn test_websocket_extended_length_large() {
        let wrapper = WebSocketFrameWrapper::new(false);
        let payload = vec![0u8; 70000]; // > 65535 bytes

        let frame = wrapper.wrap(&payload);

        // Should use 64-bit extended length
        assert_eq!(frame[1], 127);

        let unwrapped = wrapper.unwrap(&frame).unwrap();
        assert_eq!(unwrapped, payload);
    }

    #[test]
    fn test_websocket_masking() {
        let wrapper = WebSocketFrameWrapper::new(true);
        let payload = b"test data";

        let frame = wrapper.wrap(payload);

        // Mask bit should be set
        assert_eq!(frame[1] & 0x80, 0x80);

        // Should have 4-byte masking key
        // Frame structure for small payload:
        // [0]: FIN + opcode
        // [1]: MASK + length
        // [2-5]: masking key
        // [6+]: masked payload
        assert!(frame.len() >= 6 + payload.len());
    }

    #[test]
    fn test_websocket_client_vs_server() {
        let client_wrapper = WebSocketFrameWrapper::new(true);
        let server_wrapper = WebSocketFrameWrapper::new(false);

        let payload = b"test";

        let client_frame = client_wrapper.wrap(payload);
        let server_frame = server_wrapper.wrap(payload);

        // Client frame should be longer (has masking key)
        assert_eq!(client_frame.len(), server_frame.len() + 4);

        // Both should unwrap correctly
        assert_eq!(client_wrapper.unwrap(&client_frame).unwrap(), payload);
        assert_eq!(server_wrapper.unwrap(&server_frame).unwrap(), payload);
    }

    #[test]
    fn test_websocket_error_too_short() {
        let wrapper = WebSocketFrameWrapper::new(false);
        let short_frame = [0x82]; // Only 1 byte

        assert!(matches!(
            wrapper.unwrap(&short_frame),
            Err(WsError::TooShort)
        ));
    }

    #[test]
    fn test_websocket_error_invalid_opcode() {
        let wrapper = WebSocketFrameWrapper::new(false);
        // Text frame (opcode 0x01) instead of binary
        let invalid_frame = [0x81, 0x04, 0x74, 0x65, 0x73, 0x74];

        assert!(matches!(
            wrapper.unwrap(&invalid_frame),
            Err(WsError::InvalidOpcode)
        ));
    }

    #[test]
    fn test_websocket_error_incomplete() {
        let wrapper = WebSocketFrameWrapper::new(false);
        // Says length is 10 but only provides 4 bytes
        let incomplete = [0x82, 0x0A, 0x00, 0x00, 0x00, 0x00];

        assert!(matches!(
            wrapper.unwrap(&incomplete),
            Err(WsError::IncompleteFrame)
        ));
    }

    #[test]
    fn test_websocket_default() {
        let wrapper = WebSocketFrameWrapper::default();
        assert!(!wrapper.is_client_mode());
    }

    #[test]
    fn test_websocket_is_client_mode() {
        let client = WebSocketFrameWrapper::new(true);
        let server = WebSocketFrameWrapper::new(false);

        assert!(client.is_client_mode());
        assert!(!server.is_client_mode());
    }

    #[test]
    fn test_websocket_empty_payload() {
        let wrapper = WebSocketFrameWrapper::new(false);
        let empty: &[u8] = &[];

        let frame = wrapper.wrap(empty);
        let unwrapped = wrapper.unwrap(&frame).unwrap();

        assert_eq!(unwrapped.len(), 0);
    }

    #[test]
    fn test_websocket_masking_roundtrip() {
        let wrapper = WebSocketFrameWrapper::new(true);

        for i in 0..10 {
            let payload = format!("message {}", i);
            let frame = wrapper.wrap(payload.as_bytes());
            let unwrapped = wrapper.unwrap(&frame).unwrap();

            assert_eq!(unwrapped, payload.as_bytes());
        }
    }

    #[test]
    fn test_websocket_length_boundary_125() {
        let wrapper = WebSocketFrameWrapper::new(false);
        let payload = vec![0x42; 125];

        let frame = wrapper.wrap(&payload);

        // Should use direct length (not extended)
        assert_eq!(frame[1], 125);

        let unwrapped = wrapper.unwrap(&frame).unwrap();
        assert_eq!(unwrapped, payload);
    }

    #[test]
    fn test_websocket_length_boundary_126() {
        let wrapper = WebSocketFrameWrapper::new(false);
        let payload = vec![0x42; 126];

        let frame = wrapper.wrap(&payload);

        // Should use 16-bit extended length
        assert_eq!(frame[1], 126);

        let unwrapped = wrapper.unwrap(&frame).unwrap();
        assert_eq!(unwrapped, payload);
    }

    #[test]
    fn test_websocket_length_boundary_65535() {
        let wrapper = WebSocketFrameWrapper::new(false);
        let payload = vec![0x42; 65535];

        let frame = wrapper.wrap(&payload);

        // Should still use 16-bit extended length
        assert_eq!(frame[1], 126);

        let unwrapped = wrapper.unwrap(&frame).unwrap();
        assert_eq!(unwrapped, payload);
    }

    #[test]
    fn test_websocket_length_boundary_65536() {
        let wrapper = WebSocketFrameWrapper::new(false);
        let payload = vec![0x42; 65536];

        let frame = wrapper.wrap(&payload);

        // Should use 64-bit extended length
        assert_eq!(frame[1], 127);

        let unwrapped = wrapper.unwrap(&frame).unwrap();
        assert_eq!(unwrapped, payload);
    }

    #[test]
    fn test_websocket_error_display() {
        assert_eq!(
            format!("{}", WsError::TooShort),
            "WebSocket frame too short"
        );
        assert_eq!(
            format!("{}", WsError::InvalidOpcode),
            "Invalid WebSocket opcode"
        );
        assert_eq!(
            format!("{}", WsError::IncompleteFrame),
            "Incomplete WebSocket frame"
        );
    }

    #[test]
    fn test_websocket_masking_different_each_time() {
        let wrapper = WebSocketFrameWrapper::new(true);
        let payload = b"test";

        let frame1 = wrapper.wrap(payload);
        let frame2 = wrapper.wrap(payload);

        // Masking keys should be different (random)
        assert_ne!(&frame1[2..6], &frame2[2..6]);

        // But both should unwrap to same payload
        assert_eq!(wrapper.unwrap(&frame1).unwrap(), payload);
        assert_eq!(wrapper.unwrap(&frame2).unwrap(), payload);
    }
}
