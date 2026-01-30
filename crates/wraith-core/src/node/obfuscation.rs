//! Obfuscation integration for traffic analysis resistance
//!
//! Integrates padding, timing obfuscation, and protocol mimicry to make
//! WRAITH traffic indistinguishable from normal network activity.

use crate::node::config::{MimicryMode, TimingMode};
use crate::node::session::PeerConnection;
use crate::node::{Node, NodeError};
use std::time::Duration;
use wraith_transport::transport::Transport;

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

    /// Apply padding to packet using configured strategy
    fn apply_padding(&self, data: &mut Vec<u8>) -> Result<(), NodeError> {
        // Delegate to the padding strategy
        let strategy = crate::node::padding_strategy::create_padding_strategy(
            self.inner.config.obfuscation.padding_mode,
        );
        strategy.apply(data)
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
        session: &PeerConnection,
        data: &[u8],
    ) -> Result<(), NodeError> {
        let original_size = data.len();
        let mut packet = data.to_vec();

        // 1. Apply padding
        self.apply_obfuscation(&mut packet)?;
        let padded_size = packet.len();
        let padding_added = (padded_size - original_size) as u64;

        // 2. Apply timing delay
        let delay = self.get_timing_delay();
        let delay_us = delay.as_micros() as u64;

        if !delay.is_zero() {
            tracing::trace!("Applying timing delay: {:?}", delay);
            tokio::time::sleep(delay).await;
        }

        // 3. Wrap in protocol mimicry
        let wrapped = self.wrap_protocol(&packet)?;

        // 4. Send via transport
        let transport = self.get_transport().await?;
        transport
            .send_to(&wrapped, session.peer_addr())
            .await
            .map_err(|e| NodeError::Transport(e.to_string().into()))?;

        // 5. Update obfuscation statistics
        if let Ok(mut stats) = self.inner.obfuscation_stats.try_lock() {
            stats.padding_bytes += padding_added;
            stats.total_delay_us += delay_us;
            stats.wrapped_packets += 1;

            // Update rolling average packet size
            let total_packets = stats.wrapped_packets;
            stats.avg_padded_size = ((stats.avg_padded_size as u64 * (total_packets - 1)
                + padded_size as u64)
                / total_packets) as usize;
        }

        tracing::trace!(
            "Sent obfuscated packet: {} bytes (original: {}, padded: {}, padding: {} bytes, delay: {} us)",
            wrapped.len(),
            original_size,
            padded_size,
            padding_added,
            delay_us
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
        // Use wraith-obfuscation TlsRecordWrapper for protocol mimicry
        let mut wrapper = self
            .inner
            .tls_wrapper
            .try_lock()
            .map_err(|_| NodeError::Other("TLS wrapper lock contention".into()))?;

        let wrapped = wrapper.wrap(data);

        tracing::trace!(
            "Wrapped {} bytes as TLS (total: {} bytes)",
            data.len(),
            wrapped.len()
        );

        Ok(wrapped)
    }

    /// Wrap as WebSocket frame
    fn wrap_as_websocket(&self, data: &[u8]) -> Result<Vec<u8>, NodeError> {
        // Use wraith-obfuscation WebSocketFrameWrapper for protocol mimicry
        let wrapper = &self.inner.websocket_wrapper;
        let wrapped = wrapper.wrap(data);

        tracing::trace!(
            "Wrapped {} bytes as WebSocket (total: {} bytes)",
            data.len(),
            wrapped.len()
        );

        Ok(wrapped)
    }

    /// Wrap as DNS-over-HTTPS query/response
    fn wrap_as_doh(&self, data: &[u8]) -> Result<Vec<u8>, NodeError> {
        // Use wraith-obfuscation DohTunnel for protocol mimicry
        // Note: DohTunnel creates DNS query packets with EDNS0 OPT records
        let tunnel = &self.inner.doh_tunnel;
        let wrapped = tunnel
            .create_dns_query("wraith.local", data)
            .map_err(|_| NodeError::Obfuscation("DNS label encoding failed".into()))?;

        tracing::trace!(
            "Wrapped {} bytes as DoH (total: {} bytes)",
            data.len(),
            wrapped.len()
        );

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
        // Use wraith-obfuscation TlsRecordWrapper for protocol demimicry
        let wrapper = self
            .inner
            .tls_wrapper
            .try_lock()
            .map_err(|_| NodeError::Other("TLS wrapper lock contention".into()))?;

        let unwrapped = wrapper
            .unwrap(data)
            .map_err(|e| NodeError::Other(format!("TLS unwrap failed: {e}").into()))?;

        tracing::trace!(
            "Unwrapped TLS: {} bytes -> {} bytes",
            data.len(),
            unwrapped.len()
        );

        Ok(unwrapped)
    }

    /// Unwrap WebSocket frame
    fn unwrap_websocket(&self, data: &[u8]) -> Result<Vec<u8>, NodeError> {
        // Use wraith-obfuscation WebSocketFrameWrapper for protocol demimicry
        let wrapper = &self.inner.websocket_wrapper;

        let unwrapped = wrapper
            .unwrap(data)
            .map_err(|e| NodeError::Other(format!("WebSocket unwrap failed: {e}").into()))?;

        tracing::trace!(
            "Unwrapped WebSocket: {} bytes -> {} bytes",
            data.len(),
            unwrapped.len()
        );

        Ok(unwrapped)
    }

    /// Unwrap DNS-over-HTTPS
    fn unwrap_doh(&self, data: &[u8]) -> Result<Vec<u8>, NodeError> {
        // Use wraith-obfuscation DohTunnel for protocol demimicry
        let tunnel = &self.inner.doh_tunnel;

        let unwrapped = tunnel
            .parse_dns_response(data)
            .map_err(|e| NodeError::Other(format!("DoH unwrap failed: {e}").into()))?;

        tracing::trace!(
            "Unwrapped DoH: {} bytes -> {} bytes",
            data.len(),
            unwrapped.len()
        );

        Ok(unwrapped)
    }

    /// Get current obfuscation statistics
    pub fn get_obfuscation_stats(&self) -> ObfuscationStats {
        // Return current obfuscation statistics from Node state
        self.inner
            .obfuscation_stats
            .try_lock()
            .map(|stats| stats.clone())
            .unwrap_or_default()
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

        // Should have DNS header + question section + EDNS0 OPT + payload
        // Actual size depends on domain name length and EDNS0 encoding
        assert!(
            wrapped.len() > data.len(),
            "DoH wrapper should add DNS protocol overhead"
        );
        assert!(
            wrapped.len() >= 12,
            "DoH wrapper should have at least 12-byte DNS header"
        );
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

    #[tokio::test]
    async fn test_cover_traffic_config_default() {
        use crate::node::config::{CoverTrafficConfig, CoverTrafficDistribution};

        let config = CoverTrafficConfig::default();

        assert!(!config.enabled);
        assert_eq!(config.rate, 10.0);
        assert_eq!(config.distribution, CoverTrafficDistribution::Constant);
    }

    #[tokio::test]
    async fn test_obfuscation_pipeline_none() {
        // Test with no obfuscation (default config)
        let node = Node::new_random().await.unwrap();
        let data = vec![1, 2, 3, 4, 5];

        // Apply obfuscation should not change data with None mode
        let mut padded = data.clone();
        node.apply_obfuscation(&mut padded).unwrap();
        assert_eq!(padded, data);

        // Wrap protocol should not change data with None mode
        let wrapped = node.wrap_protocol(&data).unwrap();
        assert_eq!(wrapped, data);

        // Unwrap protocol should not change data with None mode
        let unwrapped = node.unwrap_protocol(&wrapped).unwrap();
        assert_eq!(unwrapped, data);
    }

    #[tokio::test]
    async fn test_timing_delay_fixed() {
        use crate::node::config::{NodeConfig, ObfuscationConfig, TimingMode};
        use std::time::Duration;

        let config = NodeConfig {
            obfuscation: ObfuscationConfig {
                timing_mode: TimingMode::Fixed(Duration::from_millis(10)),
                ..Default::default()
            },
            ..Default::default()
        };

        let node = Node::new_with_config(config).await.unwrap();
        let delay = node.get_timing_delay();

        assert_eq!(delay, Duration::from_millis(10));
    }

    #[tokio::test]
    async fn test_timing_delay_uniform() {
        use crate::node::config::{NodeConfig, ObfuscationConfig, TimingMode};
        use std::time::Duration;

        let config = NodeConfig {
            obfuscation: ObfuscationConfig {
                timing_mode: TimingMode::Uniform {
                    min: Duration::from_millis(5),
                    max: Duration::from_millis(15),
                },
                ..Default::default()
            },
            ..Default::default()
        };

        let node = Node::new_with_config(config).await.unwrap();

        // Test multiple samples to verify range
        for _ in 0..10 {
            let delay = node.get_timing_delay();
            assert!(delay >= Duration::from_millis(5));
            assert!(delay <= Duration::from_millis(15));
        }
    }

    #[tokio::test]
    async fn test_padding_power_of_two() {
        use crate::node::config::{NodeConfig, ObfuscationConfig, PaddingMode};

        let config = NodeConfig {
            obfuscation: ObfuscationConfig {
                padding_mode: PaddingMode::PowerOfTwo,
                ..Default::default()
            },
            ..Default::default()
        };

        let node = Node::new_with_config(config).await.unwrap();

        // 100 bytes should pad to 128
        let mut data = vec![0u8; 100];
        node.apply_obfuscation(&mut data).unwrap();
        assert_eq!(data.len(), 128);

        // 200 bytes should pad to 256
        let mut data = vec![0u8; 200];
        node.apply_obfuscation(&mut data).unwrap();
        assert_eq!(data.len(), 256);
    }

    #[tokio::test]
    async fn test_padding_size_classes() {
        use crate::node::config::{NodeConfig, ObfuscationConfig, PaddingMode};

        let config = NodeConfig {
            obfuscation: ObfuscationConfig {
                padding_mode: PaddingMode::SizeClasses,
                ..Default::default()
            },
            ..Default::default()
        };

        let node = Node::new_with_config(config).await.unwrap();

        // 100 bytes should pad to 256
        let mut data = vec![0u8; 100];
        node.apply_obfuscation(&mut data).unwrap();
        assert_eq!(data.len(), 256);

        // 300 bytes should pad to 512
        let mut data = vec![0u8; 300];
        node.apply_obfuscation(&mut data).unwrap();
        assert_eq!(data.len(), 512);
    }

    #[tokio::test]
    async fn test_tls_wrap_unwrap_roundtrip() {
        use crate::node::config::{MimicryMode, NodeConfig, ObfuscationConfig};

        let config = NodeConfig {
            obfuscation: ObfuscationConfig {
                mimicry_mode: MimicryMode::Tls,
                ..Default::default()
            },
            ..Default::default()
        };

        let node = Node::new_with_config(config).await.unwrap();
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];

        let wrapped = node.wrap_protocol(&data).unwrap();
        let unwrapped = node.unwrap_protocol(&wrapped).unwrap();

        assert_eq!(unwrapped, data);
    }

    #[tokio::test]
    async fn test_websocket_wrap_unwrap_roundtrip() {
        use crate::node::config::{MimicryMode, NodeConfig, ObfuscationConfig};

        let config = NodeConfig {
            obfuscation: ObfuscationConfig {
                mimicry_mode: MimicryMode::WebSocket,
                ..Default::default()
            },
            ..Default::default()
        };

        let node = Node::new_with_config(config).await.unwrap();
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];

        let wrapped = node.wrap_protocol(&data).unwrap();
        let unwrapped = node.unwrap_protocol(&wrapped).unwrap();

        assert_eq!(unwrapped, data);
    }

    #[tokio::test]
    async fn test_doh_wrap_unwrap_roundtrip() {
        use crate::node::config::{MimicryMode, NodeConfig, ObfuscationConfig};

        let config = NodeConfig {
            obfuscation: ObfuscationConfig {
                mimicry_mode: MimicryMode::DoH,
                ..Default::default()
            },
            ..Default::default()
        };

        let node = Node::new_with_config(config).await.unwrap();
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];

        let wrapped = node.wrap_protocol(&data).unwrap();
        let unwrapped = node.unwrap_protocol(&wrapped).unwrap();

        assert_eq!(unwrapped, data);
    }

    #[tokio::test]
    async fn test_full_obfuscation_pipeline() {
        use crate::node::config::{MimicryMode, NodeConfig, ObfuscationConfig, PaddingMode};

        let config = NodeConfig {
            obfuscation: ObfuscationConfig {
                padding_mode: PaddingMode::SizeClasses,
                mimicry_mode: MimicryMode::Tls,
                ..Default::default()
            },
            ..Default::default()
        };

        let node = Node::new_with_config(config).await.unwrap();
        let original_data = vec![1, 2, 3, 4, 5];

        // Apply full pipeline
        let mut padded = original_data.clone();
        node.apply_obfuscation(&mut padded).unwrap();
        assert!(padded.len() >= original_data.len()); // Padded

        let wrapped = node.wrap_protocol(&padded).unwrap();
        assert!(wrapped.len() > padded.len()); // TLS header added

        // Verify unwrap recovers padded data
        let unwrapped = node.unwrap_protocol(&wrapped).unwrap();
        assert_eq!(unwrapped, padded);
    }
}
