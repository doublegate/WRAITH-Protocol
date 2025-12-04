//! Node configuration

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

// Note: The Node module provides configuration types for all subsystems.
// Actual implementations require the respective crates as dependencies:
// - wraith-transport for transport layer
// - wraith-obfuscation for obfuscation
// - wraith-discovery for peer discovery

/// Node configuration
#[derive(Debug, Clone)]
pub struct NodeConfig {
    /// Listen address for incoming connections
    pub listen_addr: SocketAddr,

    /// Transport configuration
    pub transport: TransportConfig,

    /// Obfuscation configuration
    pub obfuscation: ObfuscationConfig,

    /// Discovery configuration
    pub discovery: DiscoveryConfig,

    /// Transfer configuration
    pub transfer: TransferConfig,

    /// Logging configuration
    pub logging: LoggingConfig,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8420".parse().unwrap(),
            transport: TransportConfig::default(),
            obfuscation: ObfuscationConfig::default(),
            discovery: DiscoveryConfig::default(),
            transfer: TransferConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Transport layer configuration
#[derive(Debug, Clone)]
pub struct TransportConfig {
    /// Enable AF_XDP (requires root and compatible NIC)
    pub enable_xdp: bool,

    /// Enable io_uring for file I/O (Linux only)
    pub enable_io_uring: bool,

    /// UDP socket buffer size
    pub udp_buffer_size: usize,

    /// Number of worker threads (defaults to num_cpus)
    pub worker_threads: Option<usize>,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Idle timeout before closing sessions
    pub idle_timeout: Duration,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            enable_xdp: false, // Requires root
            enable_io_uring: true,
            udp_buffer_size: 2 * 1024 * 1024, // 2 MB
            worker_threads: None,             // Use all CPUs
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(180), // 3 minutes
        }
    }
}

/// Obfuscation configuration
#[derive(Debug, Clone)]
pub struct ObfuscationConfig {
    /// Padding mode
    pub padding_mode: PaddingMode,

    /// Timing obfuscation mode
    pub timing_mode: TimingMode,

    /// Protocol mimicry mode
    pub mimicry_mode: MimicryMode,
}

impl Default for ObfuscationConfig {
    fn default() -> Self {
        Self {
            padding_mode: PaddingMode::None,
            timing_mode: TimingMode::None,
            mimicry_mode: MimicryMode::None,
        }
    }
}

/// Padding modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaddingMode {
    /// No padding
    None,
    /// Power-of-two padding
    PowerOfTwo,
    /// Size class padding
    SizeClasses,
    /// Constant rate padding
    ConstantRate,
    /// Statistical padding
    Statistical,
}

/// Timing obfuscation modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimingMode {
    /// No timing obfuscation
    None,
    /// Fixed delay
    Fixed(Duration),
    /// Uniform distribution
    Uniform {
        /// Minimum delay
        min: Duration,
        /// Maximum delay
        max: Duration,
    },
    /// Normal distribution
    Normal {
        /// Mean delay
        mean: Duration,
        /// Standard deviation
        stddev: Duration,
    },
    /// Exponential distribution
    Exponential {
        /// Mean delay
        mean: Duration,
    },
}

/// Protocol mimicry modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MimicryMode {
    /// No protocol mimicry
    None,
    /// TLS 1.3 mimicry
    Tls,
    /// WebSocket mimicry
    WebSocket,
    /// DNS-over-HTTPS mimicry
    DoH,
}

/// Discovery configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Enable DHT for peer discovery
    pub enable_dht: bool,

    /// Bootstrap nodes for DHT
    pub bootstrap_nodes: Vec<SocketAddr>,

    /// Enable NAT traversal
    pub enable_nat_traversal: bool,

    /// Enable relay fallback
    pub enable_relay: bool,

    /// Relay servers
    pub relay_servers: Vec<SocketAddr>,

    /// DHT announcement interval
    pub announcement_interval: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_dht: true,
            bootstrap_nodes: Vec::new(),
            enable_nat_traversal: true,
            enable_relay: true,
            relay_servers: Vec::new(),
            announcement_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Transfer configuration
#[derive(Debug, Clone)]
pub struct TransferConfig {
    /// Chunk size for file transfers
    pub chunk_size: usize,

    /// Maximum concurrent transfers
    pub max_concurrent_transfers: usize,

    /// Maximum concurrent chunks per transfer
    pub max_concurrent_chunks: usize,

    /// Download directory
    pub download_dir: PathBuf,

    /// Enable resume support
    pub enable_resume: bool,

    /// Enable multi-peer downloads
    pub enable_multi_peer: bool,

    /// Maximum peers per transfer
    pub max_peers_per_transfer: usize,
}

impl Default for TransferConfig {
    fn default() -> Self {
        Self {
            chunk_size: 256 * 1024, // 256 KB (DEFAULT_CHUNK_SIZE)
            max_concurrent_transfers: 10,
            max_concurrent_chunks: 4,
            download_dir: PathBuf::from("."), // Default to current directory
            enable_resume: true,
            enable_multi_peer: true,
            max_peers_per_transfer: 5,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Log level
    pub level: LogLevel,

    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            enable_metrics: false,
        }
    }
}

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Trace level
    Trace,
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warn level
    Warn,
    /// Error level
    Error,
}
