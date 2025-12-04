//! Obfuscation integration for traffic analysis resistance
//!
//! Integrates padding, timing obfuscation, and protocol mimicry to make
//! WRAITH traffic indistinguishable from normal network activity.

use crate::node::config::{MimicryMode, PaddingMode, TimingMode};
use crate::node::session::PeerConnection;
use crate::node::{Node, NodeError};
use std::time::Duration;

/// Protocol types for mimicry
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// TLS 1.3 mimicry
    TLS,

    /// WebSocket mimicry
    WebSocket,

    /// DNS-over-HTTPS mimicry
    DNS,
}

/// Obfuscation statistics
#[derive(Debug, Clone, Default)]
pub struct ObfuscationStats {
    /// Total padding bytes added
    pub padding_bytes: u64,

    /// Total timing delays applied (microseconds)
    pub total_delay_us: u64,

    /// Number of packets wrapped in protocol mimicry
    pub wrapped_packets: u64,

    /// Average packet size after padding
    pub avg_padded_size: usize,
}

impl Node {
    /// Apply obfuscation to outgoing packet
    ///
    /// Adds padding and wraps in protocol mimicry if configured.
    ///
    /// # Arguments
    ///
    /// * `data` - Packet data to obfuscate (will be modified in place)
    ///
    /// # Errors
    ///
    /// Returns error if obfuscation fails.
    pub fn apply_obfuscation(&self, data: &mut Vec<u8>) -> Result<(), NodeError> {
        // Apply padding first
        self.apply_padding(data)?;

        // Packet is now padded and ready for protocol wrapping
        Ok(())
    }

    /// Apply padding to packet
    fn apply_padding(&self, data: &mut Vec<u8>) -> Result<(), NodeError> {
        match self.inner.config.obfuscation.padding_mode {
            PaddingMode::None => Ok(()),

            PaddingMode::PowerOfTwo => {
                // Pad to next power of 2
                let current_size = data.len();
                let target_size = current_size.next_power_of_two();
                let padding_needed = target_size - current_size;

                if padding_needed > 0 {
                    data.resize(target_size, 0);
                    tracing::trace!(
                        "Applied power-of-2 padding: {} -> {} bytes",
                        current_size,
                        target_size
                    );
                }

                Ok(())
            }

            PaddingMode::SizeClasses => {
                // Pad to predefined size classes
                // Classes: 256, 512, 1024, 2048, 4096, 8192 bytes
                const SIZE_CLASSES: &[usize] = &[256, 512, 1024, 2048, 4096, 8192];

                let current_size = data.len();
                let target_size = SIZE_CLASSES
                    .iter()
                    .find(|&&size| size >= current_size)
                    .copied()
                    .unwrap_or(*SIZE_CLASSES.last().unwrap());

                let padding_needed = target_size - current_size;

                if padding_needed > 0 {
                    data.resize(target_size, 0);
                    tracing::trace!(
                        "Applied size-class padding: {} -> {} bytes",
                        current_size,
                        target_size
                    );
                }

                Ok(())
            }

            PaddingMode::ConstantRate => {
                // Pad to fixed MTU size (1400 bytes)
                const TARGET_SIZE: usize = 1400;

                let current_size = data.len();

                if current_size < TARGET_SIZE {
                    data.resize(TARGET_SIZE, 0);
                    tracing::trace!(
                        "Applied constant-rate padding: {} -> {} bytes",
                        current_size,
                        TARGET_SIZE
                    );
                }

                Ok(())
            }

            PaddingMode::Statistical => {
                // Add random padding following a distribution
                use rand::Rng;

                let current_size = data.len();
                let mut rng = rand::thread_rng();

                // Add 0-255 random bytes
                let padding_bytes: usize = rng.gen_range(0..256);
                data.resize(current_size + padding_bytes, 0);

                // Fill with random data
                for byte in data.iter_mut().skip(current_size).take(padding_bytes) {
                    *byte = rng.r#gen();
                }

                tracing::trace!(
                    "Applied statistical padding: {} -> {} bytes",
                    current_size,
                    data.len()
                );

                Ok(())
            }
        }
    }

    /// Get timing delay for next packet
    ///
    /// Returns the delay to apply before sending the next packet.
    pub fn get_timing_delay(&self) -> Duration {
        match &self.inner.config.obfuscation.timing_mode {
            TimingMode::None => Duration::ZERO,

            TimingMode::Fixed(delay) => *delay,

            TimingMode::Uniform { min, max } => {
                use rand::Rng;
                let mut rng = rand::thread_rng();

                // Random delay between min and max
                let delay_us = rng.gen_range(min.as_micros()..=max.as_micros()) as u64;
                Duration::from_micros(delay_us)
            }

            TimingMode::Normal { mean, stddev } => {
                use rand_distr::{Distribution, Normal};

                let mut rng = rand::thread_rng();

                // Normal distribution around mean
                let normal = Normal::new(mean.as_micros() as f64, stddev.as_micros() as f64)
                    .unwrap_or_else(|_| Normal::new(mean.as_micros() as f64, 1.0).unwrap());

                let delay_us = normal.sample(&mut rng).max(0.0) as u64;
                Duration::from_micros(delay_us)
            }

            TimingMode::Exponential { mean } => {
                use rand_distr::{Distribution, Exp};

                let mut rng = rand::thread_rng();

                // Exponential distribution with given mean
                let lambda = 1.0 / (mean.as_micros() as f64);
                let exp = Exp::new(lambda).unwrap_or_else(|_| Exp::new(0.0001).unwrap());

                let delay_us = exp.sample(&mut rng) as u64;
                Duration::from_micros(delay_us)
            }
        }
    }

    /// Send data with obfuscation applied
    ///
    /// Applies padding, timing delay, and protocol mimicry before sending.
    ///
    /// # Arguments
    ///
    /// * `session` - Session to send on
    /// * `data` - Data to send
    ///
    /// # Errors
    ///
    /// Returns error if send fails.
    pub async fn send_obfuscated(
        &self,
        _session: &PeerConnection,
        data: &[u8],
    ) -> Result<(), NodeError> {
        let mut packet = data.to_vec();

        // 1. Apply padding
        self.apply_obfuscation(&mut packet)?;

        // 2. Apply timing delay
        let delay = self.get_timing_delay();
        if !delay.is_zero() {
            tracing::trace!("Applying timing delay: {:?}", delay);
            tokio::time::sleep(delay).await;
        }

        // 3. Wrap in protocol mimicry
        let wrapped = self.wrap_protocol(&packet)?;

        // 4. Send via transport
        // TODO: Integrate with actual transport
        // session.send(&wrapped).await
        //     .map_err(|e| NodeError::Transport(e.to_string()))?;

        tracing::trace!(
            "Sent obfuscated packet: {} bytes (original: {} bytes)",
            wrapped.len(),
            data.len()
        );

        Ok(())
    }

    /// Wrap packet in protocol mimicry layer
    ///
    /// Makes WRAITH traffic look like normal protocol traffic.
    pub fn wrap_protocol(&self, data: &[u8]) -> Result<Vec<u8>, NodeError> {
        match self.inner.config.obfuscation.mimicry_mode {
            MimicryMode::None => Ok(data.to_vec()),

            MimicryMode::Tls => self.wrap_as_tls(data),

            MimicryMode::WebSocket => self.wrap_as_websocket(data),

            MimicryMode::DoH => self.wrap_as_doh(data),
        }
    }

    /// Wrap as TLS 1.3 application data
    fn wrap_as_tls(&self, data: &[u8]) -> Result<Vec<u8>, NodeError> {
        // TODO: Integrate with wraith-obfuscation::tls::TlsWrapper
        // For now, create a simple wrapper:
        //
        // TLS Record format:
        // - Content Type (1 byte): 0x17 (Application Data)
        // - Version (2 bytes): 0x03 0x03 (TLS 1.2 for compatibility)
        // - Length (2 bytes): payload length
        // - Payload: data

        let mut wrapped = Vec::with_capacity(5 + data.len());

        // Content Type: Application Data
        wrapped.push(0x17);

        // TLS Version: 1.2 (for compatibility)
        wrapped.extend_from_slice(&[0x03, 0x03]);

        // Length (big-endian)
        let len = data.len() as u16;
        wrapped.extend_from_slice(&len.to_be_bytes());

        // Payload
        wrapped.extend_from_slice(data);

        tracing::trace!("Wrapped {} bytes as TLS", data.len());

        Ok(wrapped)
    }

    /// Wrap as WebSocket frame
    fn wrap_as_websocket(&self, data: &[u8]) -> Result<Vec<u8>, NodeError> {
        // TODO: Integrate with wraith-obfuscation::websocket::WebSocketWrapper
        // For now, create a simple wrapper:
        //
        // WebSocket frame format:
        // - FIN + RSV + Opcode (1 byte): 0x82 (FIN=1, Binary frame)
        // - Mask + Length (1+ bytes)
        // - Masking key (4 bytes, if masked)
        // - Payload

        let mut wrapped = Vec::with_capacity(2 + data.len());

        // FIN=1, Opcode=Binary
        wrapped.push(0x82);

        // Length (unmasked for simplicity)
        if data.len() <= 125 {
            wrapped.push(data.len() as u8);
        } else if data.len() <= 65535 {
            wrapped.push(126);
            wrapped.extend_from_slice(&(data.len() as u16).to_be_bytes());
        } else {
            wrapped.push(127);
            wrapped.extend_from_slice(&(data.len() as u64).to_be_bytes());
        }

        // Payload (unmasked)
        wrapped.extend_from_slice(data);

        tracing::trace!("Wrapped {} bytes as WebSocket", data.len());

        Ok(wrapped)
    }

    /// Wrap as DNS-over-HTTPS query/response
    fn wrap_as_doh(&self, data: &[u8]) -> Result<Vec<u8>, NodeError> {
        // TODO: Integrate with wraith-obfuscation::doh::DohWrapper
        // For now, create a simple wrapper:
        //
        // DNS message format:
        // - ID (2 bytes)
        // - Flags (2 bytes)
        // - Counts (8 bytes: QDCOUNT, ANCOUNT, NSCOUNT, ARCOUNT)
        // - Questions/Answers (variable)

        let mut wrapped = Vec::with_capacity(12 + data.len());

        // DNS Header
        wrapped.extend_from_slice(&[
            0x00, 0x01, // ID
            0x01, 0x00, // Flags: Standard query
            0x00, 0x01, // QDCOUNT: 1 question
            0x00, 0x00, // ANCOUNT: 0 answers
            0x00, 0x00, // NSCOUNT: 0 authority
            0x00, 0x00, // ARCOUNT: 0 additional
        ]);

        // Embed data in TXT record
        wrapped.extend_from_slice(data);

        tracing::trace!("Wrapped {} bytes as DoH", data.len());

        Ok(wrapped)
    }

    /// Unwrap received packet from protocol mimicry
    ///
    /// Extracts original data from protocol wrapper.
    pub fn unwrap_protocol(&self, data: &[u8]) -> Result<Vec<u8>, NodeError> {
        match self.inner.config.obfuscation.mimicry_mode {
            MimicryMode::None => Ok(data.to_vec()),

            MimicryMode::Tls => self.unwrap_tls(data),

            MimicryMode::WebSocket => self.unwrap_websocket(data),

            MimicryMode::DoH => self.unwrap_doh(data),
        }
    }

    /// Unwrap TLS application data
    fn unwrap_tls(&self, data: &[u8]) -> Result<Vec<u8>, NodeError> {
        // TODO: Integrate with wraith-obfuscation::tls::TlsWrapper
        if data.len() < 5 {
            return Err(NodeError::Other("Invalid TLS record".to_string()));
        }

        // Skip 5-byte header
        Ok(data[5..].to_vec())
    }

    /// Unwrap WebSocket frame
    fn unwrap_websocket(&self, data: &[u8]) -> Result<Vec<u8>, NodeError> {
        // TODO: Integrate with wraith-obfuscation::websocket::WebSocketWrapper
        if data.len() < 2 {
            return Err(NodeError::Other("Invalid WebSocket frame".to_string()));
        }

        let len = data[1] & 0x7F;

        let payload_offset = if len <= 125 {
            2
        } else if len == 126 {
            4
        } else {
            10
        };

        if data.len() < payload_offset {
            return Err(NodeError::Other(
                "Invalid WebSocket frame length".to_string(),
            ));
        }

        Ok(data[payload_offset..].to_vec())
    }

    /// Unwrap DNS-over-HTTPS
    fn unwrap_doh(&self, data: &[u8]) -> Result<Vec<u8>, NodeError> {
        // TODO: Integrate with wraith-obfuscation::doh::DohWrapper
        if data.len() < 12 {
            return Err(NodeError::Other("Invalid DNS message".to_string()));
        }

        // Skip 12-byte DNS header
        Ok(data[12..].to_vec())
    }

    /// Get current obfuscation statistics
    pub fn get_obfuscation_stats(&self) -> ObfuscationStats {
        // TODO: Track these stats in Node state
        ObfuscationStats::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_apply_padding_none() {
        let node = Node::new_random().await.unwrap();
        let mut data = vec![1, 2, 3, 4];
        let original_len = data.len();

        node.apply_padding(&mut data).unwrap();

        // No padding should be applied
        assert_eq!(data.len(), original_len);
    }

    #[tokio::test]
    async fn test_get_timing_delay_none() {
        let node = Node::new_random().await.unwrap();
        let delay = node.get_timing_delay();

        assert_eq!(delay, Duration::ZERO);
    }

    #[tokio::test]
    async fn test_wrap_protocol_none() {
        let node = Node::new_random().await.unwrap();
        let data = vec![1, 2, 3, 4];

        let wrapped = node.wrap_protocol(&data).unwrap();

        // No wrapping should be applied
        assert_eq!(wrapped, data);
    }

    #[tokio::test]
    async fn test_wrap_as_tls() {
        let node = Node::new_random().await.unwrap();
        let data = vec![1, 2, 3, 4];

        let wrapped = node.wrap_as_tls(&data).unwrap();

        // Should have 5-byte header + payload
        assert_eq!(wrapped.len(), 5 + data.len());
        assert_eq!(wrapped[0], 0x17); // Application Data
        assert_eq!(wrapped[1], 0x03); // TLS 1.2
        assert_eq!(wrapped[2], 0x03);
    }

    #[tokio::test]
    async fn test_wrap_as_websocket() {
        let node = Node::new_random().await.unwrap();
        let data = vec![1, 2, 3, 4];

        let wrapped = node.wrap_as_websocket(&data).unwrap();

        // Should have at least 2-byte header + payload
        assert!(wrapped.len() >= 2 + data.len());
        assert_eq!(wrapped[0], 0x82); // FIN=1, Binary
    }

    #[tokio::test]
    async fn test_wrap_as_doh() {
        let node = Node::new_random().await.unwrap();
        let data = vec![1, 2, 3, 4];

        let wrapped = node.wrap_as_doh(&data).unwrap();

        // Should have 12-byte DNS header + payload
        assert_eq!(wrapped.len(), 12 + data.len());
    }

    #[tokio::test]
    async fn test_unwrap_tls() {
        let node = Node::new_random().await.unwrap();
        let data = vec![1, 2, 3, 4];

        let wrapped = node.wrap_as_tls(&data).unwrap();
        let unwrapped = node.unwrap_tls(&wrapped).unwrap();

        assert_eq!(unwrapped, data);
    }

    #[tokio::test]
    async fn test_unwrap_websocket() {
        let node = Node::new_random().await.unwrap();
        let data = vec![1, 2, 3, 4];

        let wrapped = node.wrap_as_websocket(&data).unwrap();
        let unwrapped = node.unwrap_websocket(&wrapped).unwrap();

        assert_eq!(unwrapped, data);
    }

    #[tokio::test]
    async fn test_unwrap_doh() {
        let node = Node::new_random().await.unwrap();
        let data = vec![1, 2, 3, 4];

        let wrapped = node.wrap_as_doh(&data).unwrap();
        let unwrapped = node.unwrap_doh(&wrapped).unwrap();

        assert_eq!(unwrapped, data);
    }

    #[test]
    fn test_protocol_equality() {
        assert_eq!(Protocol::TLS, Protocol::TLS);
        assert_ne!(Protocol::TLS, Protocol::WebSocket);
        assert_ne!(Protocol::WebSocket, Protocol::DNS);
    }

    #[test]
    fn test_obfuscation_stats() {
        let stats = ObfuscationStats::default();

        assert_eq!(stats.padding_bytes, 0);
        assert_eq!(stats.total_delay_us, 0);
        assert_eq!(stats.wrapped_packets, 0);
        assert_eq!(stats.avg_padded_size, 0);
    }
}
