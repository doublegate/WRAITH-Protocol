//! Padding strategy pattern for flexible traffic obfuscation
//!
//! Provides pluggable padding implementations that can be configured per-transfer
//! or dynamically adjusted based on threat level.

use crate::node::NodeError;
use crate::node::config::PaddingMode;

/// Padding strategy trait
///
/// Defines the interface for different padding algorithms that can be
/// applied to outgoing packets for traffic analysis resistance.
pub trait PaddingStrategy: Send + Sync {
    /// Apply padding to the data buffer
    ///
    /// # Arguments
    ///
    /// * `data` - Packet data to pad (modified in place)
    ///
    /// # Errors
    ///
    /// Returns error if padding operation fails.
    fn apply(&self, data: &mut Vec<u8>) -> Result<(), NodeError>;

    /// Get the strategy name for logging/debugging
    fn name(&self) -> &'static str;

    /// Calculate expected overhead for this padding mode
    ///
    /// Returns approximate padding overhead as a ratio (0.0 = no overhead, 1.0 = double size)
    fn expected_overhead(&self) -> f64;
}

/// No padding strategy
#[derive(Debug, Clone, Copy, Default)]
pub struct NonePadding;

impl PaddingStrategy for NonePadding {
    fn apply(&self, _data: &mut Vec<u8>) -> Result<(), NodeError> {
        // No padding applied
        Ok(())
    }

    fn name(&self) -> &'static str {
        "None"
    }

    fn expected_overhead(&self) -> f64 {
        0.0
    }
}

/// Power-of-two padding strategy
///
/// Pads packets to the next power of 2 size to obscure actual payload length.
#[derive(Debug, Clone, Copy, Default)]
pub struct PowerOfTwoPadding;

impl PaddingStrategy for PowerOfTwoPadding {
    fn apply(&self, data: &mut Vec<u8>) -> Result<(), NodeError> {
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

    fn name(&self) -> &'static str {
        "PowerOfTwo"
    }

    fn expected_overhead(&self) -> f64 {
        // Average overhead ~50% (halfway between powers of 2)
        0.5
    }
}

/// Size class padding strategy
///
/// Pads packets to predefined size buckets to hide exact payload sizes.
#[derive(Debug, Clone, Copy, Default)]
pub struct SizeClassesPadding;

impl SizeClassesPadding {
    /// Predefined size classes in bytes
    const SIZE_CLASSES: &'static [usize] = &[256, 512, 1024, 2048, 4096, 8192];
}

impl PaddingStrategy for SizeClassesPadding {
    fn apply(&self, data: &mut Vec<u8>) -> Result<(), NodeError> {
        let current_size = data.len();
        let target_size = Self::SIZE_CLASSES
            .iter()
            .find(|&&size| size >= current_size)
            .copied()
            .unwrap_or(*Self::SIZE_CLASSES.last().unwrap());

        // Only pad if target is larger than current
        if target_size > current_size {
            data.resize(target_size, 0);
            tracing::trace!(
                "Applied size-class padding: {} -> {} bytes",
                current_size,
                target_size
            );
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "SizeClasses"
    }

    fn expected_overhead(&self) -> f64 {
        // Average overhead ~30-40% for typical packet sizes
        0.35
    }
}

/// Constant rate padding strategy
///
/// Pads all packets to a fixed MTU size for consistent packet sizes.
#[derive(Debug, Clone, Copy)]
pub struct ConstantRatePadding {
    /// Target packet size
    target_size: usize,
}

impl Default for ConstantRatePadding {
    fn default() -> Self {
        Self { target_size: 1400 }
    }
}

impl ConstantRatePadding {
    /// Create a new constant rate padding strategy
    ///
    /// # Arguments
    ///
    /// * `target_size` - Fixed target packet size in bytes
    pub fn new(target_size: usize) -> Self {
        Self { target_size }
    }
}

impl PaddingStrategy for ConstantRatePadding {
    fn apply(&self, data: &mut Vec<u8>) -> Result<(), NodeError> {
        let current_size = data.len();

        if current_size < self.target_size {
            data.resize(self.target_size, 0);
            tracing::trace!(
                "Applied constant-rate padding: {} -> {} bytes",
                current_size,
                self.target_size
            );
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "ConstantRate"
    }

    fn expected_overhead(&self) -> f64 {
        // Highly variable depending on payload sizes, assume 50% average
        0.5
    }
}

/// Statistical padding strategy
///
/// Adds random padding following a distribution to defeat traffic analysis.
#[derive(Debug, Clone, Copy, Default)]
pub struct StatisticalPadding;

impl PaddingStrategy for StatisticalPadding {
    fn apply(&self, data: &mut Vec<u8>) -> Result<(), NodeError> {
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

    fn name(&self) -> &'static str {
        "Statistical"
    }

    fn expected_overhead(&self) -> f64 {
        // Average 128 bytes added
        0.128
    }
}

/// Create a padding strategy from configuration
///
/// Factory method to instantiate the appropriate strategy based on config.
pub fn create_padding_strategy(mode: PaddingMode) -> Box<dyn PaddingStrategy> {
    match mode {
        PaddingMode::None => Box::new(NonePadding),
        PaddingMode::PowerOfTwo => Box::new(PowerOfTwoPadding),
        PaddingMode::SizeClasses => Box::new(SizeClassesPadding),
        PaddingMode::ConstantRate => Box::new(ConstantRatePadding::default()),
        PaddingMode::Statistical => Box::new(StatisticalPadding),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_none_padding() {
        let strategy = NonePadding;
        let mut data = vec![1, 2, 3, 4, 5];
        strategy.apply(&mut data).unwrap();

        assert_eq!(data.len(), 5);
        assert_eq!(data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_power_of_two_padding() {
        let strategy = PowerOfTwoPadding;

        // 5 bytes -> 8 bytes
        let mut data = vec![1, 2, 3, 4, 5];
        strategy.apply(&mut data).unwrap();
        assert_eq!(data.len(), 8);

        // 100 bytes -> 128 bytes
        let mut data = vec![0u8; 100];
        strategy.apply(&mut data).unwrap();
        assert_eq!(data.len(), 128);

        // Already power of 2 (no change)
        let mut data = vec![0u8; 64];
        strategy.apply(&mut data).unwrap();
        assert_eq!(data.len(), 64);
    }

    #[test]
    fn test_size_classes_padding() {
        let strategy = SizeClassesPadding;

        // 100 bytes -> 256 bytes
        let mut data = vec![0u8; 100];
        strategy.apply(&mut data).unwrap();
        assert_eq!(data.len(), 256);

        // 500 bytes -> 512 bytes
        let mut data = vec![0u8; 500];
        strategy.apply(&mut data).unwrap();
        assert_eq!(data.len(), 512);

        // 3000 bytes -> 4096 bytes
        let mut data = vec![0u8; 3000];
        strategy.apply(&mut data).unwrap();
        assert_eq!(data.len(), 4096);

        // Over max class -> no padding applied
        let mut data = vec![0u8; 9000];
        strategy.apply(&mut data).unwrap();
        assert_eq!(data.len(), 9000);
    }

    #[test]
    fn test_constant_rate_padding() {
        let strategy = ConstantRatePadding::new(1400);

        // Small packet -> 1400 bytes
        let mut data = vec![0u8; 100];
        strategy.apply(&mut data).unwrap();
        assert_eq!(data.len(), 1400);

        // Already at target (no change)
        let mut data = vec![0u8; 1400];
        strategy.apply(&mut data).unwrap();
        assert_eq!(data.len(), 1400);

        // Over target (no padding)
        let mut data = vec![0u8; 2000];
        strategy.apply(&mut data).unwrap();
        assert_eq!(data.len(), 2000);
    }

    #[test]
    fn test_statistical_padding() {
        let strategy = StatisticalPadding;

        // Statistical padding adds 0-255 random bytes
        let mut data = vec![0u8; 100];
        let original_len = data.len();
        strategy.apply(&mut data).unwrap();

        assert!(data.len() >= original_len);
        assert!(data.len() <= original_len + 255);
    }

    #[test]
    fn test_factory_creation() {
        let strategy = create_padding_strategy(PaddingMode::None);
        assert_eq!(strategy.name(), "None");

        let strategy = create_padding_strategy(PaddingMode::PowerOfTwo);
        assert_eq!(strategy.name(), "PowerOfTwo");

        let strategy = create_padding_strategy(PaddingMode::SizeClasses);
        assert_eq!(strategy.name(), "SizeClasses");

        let strategy = create_padding_strategy(PaddingMode::ConstantRate);
        assert_eq!(strategy.name(), "ConstantRate");

        let strategy = create_padding_strategy(PaddingMode::Statistical);
        assert_eq!(strategy.name(), "Statistical");
    }

    #[test]
    fn test_expected_overhead() {
        let none = NonePadding;
        assert_eq!(none.expected_overhead(), 0.0);

        let power_of_two = PowerOfTwoPadding;
        assert!(power_of_two.expected_overhead() > 0.0);

        let size_classes = SizeClassesPadding;
        assert!(size_classes.expected_overhead() > 0.0);

        let constant_rate = ConstantRatePadding::default();
        assert!(constant_rate.expected_overhead() > 0.0);

        let statistical = StatisticalPadding;
        assert!(statistical.expected_overhead() > 0.0);
    }

    #[test]
    fn test_strategy_names() {
        assert_eq!(NonePadding.name(), "None");
        assert_eq!(PowerOfTwoPadding.name(), "PowerOfTwo");
        assert_eq!(SizeClassesPadding.name(), "SizeClasses");
        assert_eq!(ConstantRatePadding::default().name(), "ConstantRate");
        assert_eq!(StatisticalPadding.name(), "Statistical");
    }
}
