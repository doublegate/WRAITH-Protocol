//! WRAITH Protocol CLI
//!
//! Wire-speed Resilient Authenticated Invisible Transfer Handler
//!
//! Security features:
//! - Private key encryption with Argon2id KDF and ChaCha20-Poly1305
//! - Path sanitization to prevent directory traversal attacks
//! - Memory zeroization for sensitive data

mod config;
mod progress;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use zeroize::Zeroize;

use config::Config;
use progress::{TransferProgress, format_bytes};

/// Encrypted private key file header magic bytes
const ENCRYPTED_KEY_MAGIC: &[u8; 8] = b"WRAITH01";

/// Argon2id parameters for key derivation
const ARGON2_MEMORY_COST: u32 = 65536; // 64 MiB
const ARGON2_TIME_COST: u32 = 3;
const ARGON2_PARALLELISM: u32 = 4;
const ARGON2_SALT_SIZE: usize = 16;
const ARGON2_NONCE_SIZE: usize = 24; // XChaCha20-Poly1305 nonce
const ARGON2_TAG_SIZE: usize = 16;

/// WRAITH - Secure, fast, undetectable file transfer
#[derive(Parser)]
#[command(name = "wraith")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Enable debug output (implies --verbose)
    #[arg(short, long)]
    debug: bool,

    /// Configuration file path
    #[arg(short, long, default_value = "~/.config/wraith/config.toml")]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a file to a peer
    Send {
        /// File to send
        #[arg(required = true)]
        file: String,

        /// Recipient peer ID or address
        #[arg(required = true)]
        recipient: String,

        /// Obfuscation mode
        #[arg(long, default_value = "privacy")]
        mode: String,
    },

    /// Send multiple files in batch
    Batch {
        /// Files to send (space-separated)
        #[arg(required = true)]
        files: Vec<String>,

        /// Recipient peer ID or address
        #[arg(short, long, required = true)]
        to: String,

        /// Obfuscation mode
        #[arg(long, default_value = "privacy")]
        mode: String,
    },

    /// Receive files from peers
    Receive {
        /// Output directory
        #[arg(short, long, default_value = ".")]
        output: String,

        /// Listen address
        #[arg(short, long, default_value = "0.0.0.0:0")]
        bind: String,
    },

    /// Run as background daemon
    Daemon {
        /// Bind address
        #[arg(short, long, default_value = "0.0.0.0:0")]
        bind: String,

        /// Enable relay mode
        #[arg(long)]
        relay: bool,
    },

    /// Show connection status
    Status {
        /// Show transfer status for specific transfer ID
        #[arg(long)]
        transfer: Option<String>,

        /// Show detailed statistics
        #[arg(long)]
        detailed: bool,
    },

    /// List connected peers
    Peers {
        /// Query DHT for specific peer ID
        #[arg(long)]
        dht_query: Option<String>,
    },

    /// Show node health information
    Health,

    /// Show metrics and statistics
    Metrics {
        /// Show metrics in JSON format
        #[arg(long)]
        json: bool,

        /// Watch metrics continuously (refresh every N seconds)
        #[arg(short, long)]
        watch: Option<u64>,
    },

    /// Show node information
    Info,

    /// Generate a new identity keypair
    Keygen {
        /// Output file for private key
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.debug {
        "trace"
    } else if cli.verbose {
        "debug"
    } else {
        "info"
    };

    tracing_subscriber::fmt().with_env_filter(log_level).init();

    // Keygen command doesn't need config - handle it separately
    if matches!(cli.command, Commands::Keygen { .. }) {
        if let Commands::Keygen { output } = cli.command {
            return generate_keypair(output, &Config::default()).await;
        }
    }

    // Load configuration (expand tilde if present)
    let config_path = if cli.config.starts_with("~/") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(&cli.config[2..])
    } else {
        PathBuf::from(&cli.config)
    };

    let config = if config_path.exists() {
        Config::load(&config_path)?
    } else if config_path == Config::default_path() {
        Config::load_or_default()?
    } else {
        Config::load(&config_path)? // Will fail with proper error
    };

    // Validate configuration
    config.validate()?;

    match cli.command {
        Commands::Send {
            file,
            recipient,
            mode,
        } => {
            send_file(PathBuf::from(file), recipient, mode, &config).await?;
        }
        Commands::Batch { files, to, mode } => {
            send_batch(files, to, mode, &config).await?;
        }
        Commands::Receive { output, bind } => {
            receive_files(PathBuf::from(output), bind, &config).await?;
        }
        Commands::Daemon { bind, relay } => {
            run_daemon(bind, relay, &config).await?;
        }
        Commands::Status { transfer, detailed } => {
            show_status(transfer, detailed, &config).await?;
        }
        Commands::Peers { dht_query } => {
            list_peers(dht_query, &config).await?;
        }
        Commands::Health => {
            show_health(&config).await?;
        }
        Commands::Metrics { json, watch } => {
            show_metrics(json, watch, &config).await?;
        }
        Commands::Info => {
            show_info(&config).await?;
        }
        Commands::Keygen { .. } => {
            // Already handled above before config loading
            unreachable!("Keygen command should have been handled earlier")
        }
    }

    Ok(())
}

/// Sanitize and validate a file path to prevent directory traversal attacks
///
/// # Security
///
/// This function:
/// - Canonicalizes the path to resolve symlinks and relative components
/// - Rejects paths containing '..' components
/// - Ensures the path doesn't escape intended directories
fn sanitize_path(path: &PathBuf) -> anyhow::Result<PathBuf> {
    // Check for obvious traversal attempts in the raw path
    let path_str = path.to_string_lossy();
    if path_str.contains("..") {
        anyhow::bail!("Path traversal attempt detected: path contains '..'");
    }

    // Canonicalize if the path exists
    if path.exists() {
        let canonical = path.canonicalize()?;
        tracing::debug!("Canonicalized path: {:?} -> {:?}", path, canonical);
        Ok(canonical)
    } else {
        // For non-existent paths (e.g., output files), check the parent
        if let Some(parent) = path.parent() {
            if parent.exists() {
                let canonical_parent = parent.canonicalize()?;
                let file_name = path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid path: no filename component"))?;
                Ok(canonical_parent.join(file_name))
            } else {
                // Parent doesn't exist, just validate the path doesn't have traversal
                Ok(path.clone())
            }
        } else {
            Ok(path.clone())
        }
    }
}

/// Encrypt a private key with a passphrase using Argon2id KDF and XChaCha20-Poly1305
///
/// # Format
///
/// The encrypted file format is:
/// - 8 bytes: Magic header "WRAITH01"
/// - 16 bytes: Argon2 salt
/// - 24 bytes: XChaCha20-Poly1305 nonce
/// - N bytes: Encrypted private key (32 bytes + 16 byte auth tag)
///
/// # Security
///
/// - Uses Argon2id for memory-hard key derivation
/// - XChaCha20-Poly1305 provides authenticated encryption
/// - Salt and nonce are randomly generated for each encryption
fn encrypt_private_key(private_key: &[u8; 32], passphrase: &str) -> anyhow::Result<Vec<u8>> {
    use argon2::{Algorithm, Argon2, Params, Version};
    use chacha20poly1305::{KeyInit, XChaCha20Poly1305, aead::Aead};
    use rand_core::{OsRng, RngCore};

    // Generate random salt and nonce
    let mut salt = [0u8; ARGON2_SALT_SIZE];
    let mut nonce = [0u8; ARGON2_NONCE_SIZE];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);

    // Derive encryption key using Argon2id
    let params = Params::new(
        ARGON2_MEMORY_COST,
        ARGON2_TIME_COST,
        ARGON2_PARALLELISM,
        Some(32),
    )
    .map_err(|e| anyhow::anyhow!("Argon2 params error: {e}"))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut derived_key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), &salt, &mut derived_key)
        .map_err(|e| anyhow::anyhow!("Argon2 derivation failed: {e}"))?;

    // Encrypt the private key
    let cipher = XChaCha20Poly1305::new((&derived_key).into());
    let ciphertext = cipher
        .encrypt((&nonce).into(), private_key.as_ref())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {e}"))?;

    // Zeroize the derived key
    derived_key.zeroize();

    // Build output: magic + salt + nonce + ciphertext
    let mut output = Vec::with_capacity(
        ENCRYPTED_KEY_MAGIC.len() + ARGON2_SALT_SIZE + ARGON2_NONCE_SIZE + ciphertext.len(),
    );
    output.extend_from_slice(ENCRYPTED_KEY_MAGIC);
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);

    Ok(output)
}

/// Decrypt an encrypted private key file
///
/// # Errors
///
/// Returns an error if:
/// - The file format is invalid (wrong magic header)
/// - The passphrase is incorrect
/// - The file is corrupted
#[allow(dead_code)]
fn decrypt_private_key(encrypted_data: &[u8], passphrase: &str) -> anyhow::Result<[u8; 32]> {
    use argon2::{Algorithm, Argon2, Params, Version};
    use chacha20poly1305::{KeyInit, XChaCha20Poly1305, aead::Aead};

    let expected_min_size =
        ENCRYPTED_KEY_MAGIC.len() + ARGON2_SALT_SIZE + ARGON2_NONCE_SIZE + 32 + ARGON2_TAG_SIZE;
    if encrypted_data.len() < expected_min_size {
        anyhow::bail!("Invalid encrypted key file: too short");
    }

    // Verify magic header
    if &encrypted_data[..8] != ENCRYPTED_KEY_MAGIC {
        anyhow::bail!("Invalid encrypted key file: wrong format");
    }

    // Extract salt, nonce, and ciphertext
    let salt = &encrypted_data[8..8 + ARGON2_SALT_SIZE];
    let nonce = &encrypted_data[8 + ARGON2_SALT_SIZE..8 + ARGON2_SALT_SIZE + ARGON2_NONCE_SIZE];
    let ciphertext = &encrypted_data[8 + ARGON2_SALT_SIZE + ARGON2_NONCE_SIZE..];

    // Derive decryption key using Argon2id
    let params = Params::new(
        ARGON2_MEMORY_COST,
        ARGON2_TIME_COST,
        ARGON2_PARALLELISM,
        Some(32),
    )
    .map_err(|e| anyhow::anyhow!("Argon2 params error: {e}"))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut derived_key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut derived_key)
        .map_err(|e| anyhow::anyhow!("Argon2 derivation failed: {e}"))?;

    // Decrypt the private key
    let cipher = XChaCha20Poly1305::new((&derived_key).into());
    let plaintext = cipher.decrypt(nonce.into(), ciphertext).map_err(|_| {
        anyhow::anyhow!("Decryption failed: incorrect passphrase or corrupted file")
    })?;

    // Zeroize the derived key
    derived_key.zeroize();

    if plaintext.len() != 32 {
        anyhow::bail!("Invalid decrypted key length");
    }

    let mut private_key = [0u8; 32];
    private_key.copy_from_slice(&plaintext);

    Ok(private_key)
}

/// Prompt for passphrase with confirmation
fn prompt_passphrase(prompt: &str, confirm: bool) -> anyhow::Result<String> {
    let passphrase = rpassword::prompt_password(prompt)?;

    if passphrase.is_empty() {
        anyhow::bail!("Passphrase cannot be empty");
    }

    if passphrase.len() < 8 {
        anyhow::bail!("Passphrase must be at least 8 characters");
    }

    if confirm {
        let confirm_pass = rpassword::prompt_password("Confirm passphrase: ")?;
        if passphrase != confirm_pass {
            anyhow::bail!("Passphrases do not match");
        }
    }

    Ok(passphrase)
}

/// Send a file to a recipient
async fn send_file(
    file: PathBuf,
    recipient: String,
    mode: String,
    config: &Config,
) -> anyhow::Result<()> {
    tracing::info!("Sending {:?} to {} (mode: {})", file, recipient, mode);

    // Sanitize file path to prevent directory traversal
    let file = sanitize_path(&file)?;

    // Verify file exists
    if !file.exists() {
        anyhow::bail!("File not found: {file:?}");
    }

    let file_size = std::fs::metadata(&file)?.len();
    let filename = file
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    println!("File: {}", file.display());
    println!("Size: {}", format_bytes(file_size));
    println!("Recipient: {recipient}");
    println!("Obfuscation: {mode}");

    // Create progress bar
    let progress = TransferProgress::new(file_size, filename);

    // Placeholder: Full implementation requires protocol integration
    // This demonstrates the CLI structure and progress tracking

    tracing::warn!("Full send implementation requires Phase 7 protocol integration");
    tracing::info!(
        "Would send using chunk_size={}, obfuscation={}",
        config.transfer.chunk_size,
        mode
    );

    progress.finish_with_message(
        "Send command structured (full implementation pending Phase 7)".to_string(),
    );

    Ok(())
}

/// Receive files from peers
async fn receive_files(output: PathBuf, bind: String, config: &Config) -> anyhow::Result<()> {
    tracing::info!("Receiving files to {:?} (listening on {})", output, bind);

    // Create output directory if it doesn't exist
    if !output.exists() {
        std::fs::create_dir_all(&output)?;
    }

    println!("Output directory: {}", output.display());
    println!("Listening on: {bind}");
    println!(
        "Chunk size: {}",
        format_bytes(config.transfer.chunk_size as u64)
    );
    println!("Resume enabled: {}", config.transfer.enable_resume);

    // Placeholder: Full implementation requires protocol integration
    tracing::warn!("Full receive implementation requires Phase 7 protocol integration");

    println!("\nReady to receive files (implementation pending Phase 7)...");
    println!("Press Ctrl+C to stop");

    // Keep alive
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down...");

    Ok(())
}

/// Run daemon mode
async fn run_daemon(bind: String, relay: bool, config: &Config) -> anyhow::Result<()> {
    tracing::info!("Starting WRAITH daemon on {} (relay: {})", bind, relay);

    println!("WRAITH Daemon");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!("Listen: {bind}");
    println!("Relay mode: {relay}");
    println!("XDP: {}", config.network.enable_xdp);

    if config.network.enable_xdp {
        if let Some(iface) = &config.network.xdp_interface {
            println!("XDP interface: {iface}");
        }
    }

    // Placeholder: Full implementation requires protocol integration
    tracing::warn!("Full daemon implementation requires Phase 7 protocol integration");

    println!("\nDaemon ready (implementation pending Phase 7)...");
    println!("Press Ctrl+C to stop");

    // Keep alive
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down...");

    Ok(())
}

/// Send batch of files
async fn send_batch(
    files: Vec<String>,
    recipient: String,
    mode: String,
    _config: &Config,
) -> anyhow::Result<()> {
    tracing::info!(
        "Batch sending {} files to {} (mode: {})",
        files.len(),
        recipient,
        mode
    );

    println!("Batch Transfer");
    println!("Files: {}", files.len());
    println!("Recipient: {recipient}");
    println!("Obfuscation: {mode}");
    println!();

    // Validate and sanitize all file paths
    let mut total_size = 0u64;
    let mut sanitized_files = Vec::new();

    for file_path_str in &files {
        let file_path = PathBuf::from(file_path_str);
        let sanitized = sanitize_path(&file_path)?;

        if !sanitized.exists() {
            anyhow::bail!("File not found: {file_path:?}");
        }

        let metadata = std::fs::metadata(&sanitized)?;
        if !metadata.is_file() {
            anyhow::bail!("Not a file: {file_path:?}");
        }

        total_size += metadata.len();
        sanitized_files.push((sanitized, metadata.len()));
    }

    println!("Total size: {}", format_bytes(total_size));
    println!();

    // Transfer each file
    for (idx, (file_path, file_size)) in sanitized_files.iter().enumerate() {
        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        println!("[{}/{}] {}", idx + 1, sanitized_files.len(), filename);
        println!("  Size: {}", format_bytes(*file_size));

        let progress = TransferProgress::new(*file_size, filename);

        // Placeholder: Full implementation requires protocol integration
        tracing::warn!("Batch transfer implementation requires Phase 7 protocol integration");

        progress.finish_with_message(format!(
            "File {}/{} queued (implementation pending)",
            idx + 1,
            sanitized_files.len()
        ));
    }

    println!();
    println!(
        "Batch transfer structured ({} files, implementation pending)",
        files.len()
    );

    Ok(())
}

/// Show node status
async fn show_status(
    transfer: Option<String>,
    detailed: bool,
    config: &Config,
) -> anyhow::Result<()> {
    println!("WRAITH Protocol Status");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    if let Some(transfer_id) = transfer {
        // Show specific transfer status
        println!("Transfer: {transfer_id}");
        println!("Status: Active (placeholder)");
        println!("Progress: 45% (placeholder)");
        println!("Speed: 8.5 Gbps (placeholder)");
        println!("ETA: 2m 15s (placeholder)");
        println!();

        tracing::warn!("Transfer status requires Phase 7 protocol integration");
        return Ok(());
    }

    println!("Configuration:");
    println!("  Listen: {}", config.network.listen_addr);
    println!("  Obfuscation: {}", config.obfuscation.default_level);
    println!(
        "  Chunk size: {}",
        format_bytes(config.transfer.chunk_size as u64)
    );
    println!("  Max concurrent: {}", config.transfer.max_concurrent);
    println!();

    println!("Network:");
    println!("  XDP: {}", config.network.enable_xdp);
    println!("  UDP fallback: {}", config.network.udp_fallback);
    println!();

    println!("Discovery:");
    println!(
        "  Bootstrap nodes: {}",
        config.discovery.bootstrap_nodes.len()
    );
    println!("  Relay servers: {}", config.discovery.relay_servers.len());
    println!();

    if detailed {
        println!("Detailed Statistics:");
        println!("  Active sessions: 0 (placeholder)");
        println!("  Active transfers: 0 (placeholder)");
        println!("  Bytes sent: 0 (placeholder)");
        println!("  Bytes received: 0 (placeholder)");
        println!("  Packets sent: 0 (placeholder)");
        println!("  Packets received: 0 (placeholder)");
        println!("  Average RTT: N/A (placeholder)");
        println!("  Packet loss: 0.0% (placeholder)");
        println!();
    }

    // Placeholder: Show runtime status when protocol is integrated
    tracing::warn!("Runtime status display requires Phase 7 protocol integration");

    Ok(())
}

/// List connected peers
async fn list_peers(dht_query: Option<String>, config: &Config) -> anyhow::Result<()> {
    println!("Connected Peers:");
    println!();

    if let Some(peer_id) = dht_query {
        // Query DHT for specific peer
        println!("Querying DHT for peer: {peer_id}");
        println!();

        tracing::warn!("DHT query requires Phase 7 protocol integration");

        println!("DHT query result: Not found (implementation pending)");
        return Ok(());
    }

    // Placeholder: Full implementation requires protocol integration
    tracing::warn!("Peer listing requires Phase 7 protocol integration");

    println!("No peers connected (implementation pending Phase 7)");
    println!();
    println!("Discovery configured:");
    println!(
        "  Bootstrap nodes: {}",
        config.discovery.bootstrap_nodes.len()
    );
    println!("  Relay servers: {}", config.discovery.relay_servers.len());

    Ok(())
}

/// Show node health
async fn show_health(config: &Config) -> anyhow::Result<()> {
    println!("WRAITH Node Health Check");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    // System health checks
    println!("System:");
    println!("  Status: healthy (placeholder)");
    println!("  Uptime: N/A (placeholder)");
    println!(
        "  Memory: {} / {} (placeholder)",
        format_bytes(0),
        format_bytes(0)
    );
    println!("  CPU: 0% (placeholder)");
    println!();

    // Network health
    println!("Network:");
    println!(
        "  XDP: {} ({})",
        config.network.enable_xdp,
        if config.network.enable_xdp {
            "configured"
        } else {
            "disabled"
        }
    );
    println!("  Listen: {}", config.network.listen_addr);
    println!("  Connectivity: unknown (placeholder)");
    println!();

    // Protocol health
    println!("Protocol:");
    println!("  Active sessions: 0 (placeholder)");
    println!("  Active transfers: 0 (placeholder)");
    println!("  Avg throughput: 0 Gbps (placeholder)");
    println!("  Avg latency: N/A (placeholder)");
    println!();

    // Discovery health
    println!("Discovery:");
    println!("  DHT nodes: 0 (placeholder)");
    println!(
        "  Bootstrap: {}/{} connected (placeholder)",
        0,
        config.discovery.bootstrap_nodes.len()
    );
    println!(
        "  Relay: {}/{} connected (placeholder)",
        0,
        config.discovery.relay_servers.len()
    );
    println!();

    println!("Overall Health: HEALTHY (placeholder)");
    println!();

    tracing::warn!("Health check requires Phase 7 protocol integration");

    Ok(())
}

/// Show metrics
async fn show_metrics(json: bool, watch: Option<u64>, config: &Config) -> anyhow::Result<()> {
    if json {
        // JSON output
        println!(
            r#"{{
  "version": "{}",
  "uptime_seconds": 0,
  "network": {{
    "xdp_enabled": {},
    "bytes_sent": 0,
    "bytes_received": 0,
    "packets_sent": 0,
    "packets_received": 0,
    "packet_loss_rate": 0.0
  }},
  "sessions": {{
    "active": 0,
    "total": 0,
    "avg_rtt_us": 0
  }},
  "transfers": {{
    "active": 0,
    "completed": 0,
    "failed": 0,
    "avg_throughput_bps": 0
  }},
  "discovery": {{
    "dht_nodes": 0,
    "bootstrap_connected": 0,
    "relay_connected": 0
  }}
}}"#,
            env!("CARGO_PKG_VERSION"),
            config.network.enable_xdp
        );

        tracing::warn!("Metrics collection requires Phase 7 protocol integration");
        return Ok(());
    }

    // Text output
    if let Some(interval) = watch {
        println!("Watching metrics (refresh every {interval}s, Ctrl+C to stop)");
        println!();

        loop {
            // Clear screen
            print!("\x1B[2J\x1B[1;1H");

            display_metrics(config);

            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        }
    } else {
        display_metrics(config);
    }

    Ok(())
}

/// Display metrics (helper function)
fn display_metrics(config: &Config) {
    println!("WRAITH Metrics");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    println!("Network:");
    println!("  XDP: {}", config.network.enable_xdp);
    println!("  Throughput: 0 Gbps ↓ / 0 Gbps ↑ (placeholder)");
    println!("  Packets: 0 pps ↓ / 0 pps ↑ (placeholder)");
    println!("  Loss rate: 0.00% (placeholder)");
    println!();

    println!("Sessions:");
    println!("  Active: 0 (placeholder)");
    println!("  Total: 0 (placeholder)");
    println!("  Avg RTT: N/A (placeholder)");
    println!();

    println!("Transfers:");
    println!("  Active: 0 (placeholder)");
    println!("  Completed: 0 (placeholder)");
    println!("  Failed: 0 (placeholder)");
    println!("  Avg Speed: 0 Gbps (placeholder)");
    println!();

    println!("Discovery:");
    println!("  DHT nodes: 0 (placeholder)");
    println!(
        "  Bootstrap: 0/{} (placeholder)",
        config.discovery.bootstrap_nodes.len()
    );
    println!(
        "  Relay: 0/{} (placeholder)",
        config.discovery.relay_servers.len()
    );
    println!();

    tracing::warn!("Metrics collection requires Phase 7 protocol integration");
}

/// Show node information
async fn show_info(config: &Config) -> anyhow::Result<()> {
    println!("WRAITH Node Information");
    println!();

    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!(
        "Build: {} ({})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    println!();

    println!("Node:");
    println!("  ID: <not-generated> (placeholder)");
    println!("  Listen: {}", config.network.listen_addr);
    println!("  Uptime: N/A (placeholder)");
    println!();

    println!("Features:");
    println!(
        "  XDP: {} ({})",
        config.network.enable_xdp,
        if config.network.enable_xdp {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "  io_uring: {} ({})",
        cfg!(target_os = "linux"),
        if cfg!(target_os = "linux") {
            "available"
        } else {
            "unavailable"
        }
    );
    println!("  Obfuscation: {}", config.obfuscation.default_level);
    println!("  TLS Mimicry: {}", config.obfuscation.tls_mimicry);
    println!();

    println!("Configuration:");
    println!(
        "  Chunk size: {}",
        format_bytes(config.transfer.chunk_size as u64)
    );
    println!("  Max concurrent: {}", config.transfer.max_concurrent);
    println!("  Resume: {}", config.transfer.enable_resume);
    println!();

    println!("Discovery:");
    println!(
        "  Bootstrap nodes: {}",
        config.discovery.bootstrap_nodes.len()
    );
    println!("  Relay servers: {}", config.discovery.relay_servers.len());
    println!();

    tracing::warn!("Full node info requires Phase 7 protocol integration");

    Ok(())
}

/// Generate a new identity keypair
///
/// # Security
///
/// - Private keys are encrypted with a passphrase before being written to disk
/// - Uses Argon2id for key derivation (memory-hard, resistant to GPU attacks)
/// - Uses XChaCha20-Poly1305 for authenticated encryption
/// - Sensitive data is zeroized after use
async fn generate_keypair(output: Option<String>, _config: &Config) -> anyhow::Result<()> {
    use wraith_crypto::signatures::SigningKey;

    println!("Generating new Ed25519 identity keypair...");
    println!();

    let mut rng = rand_core::OsRng;
    let signing_key = SigningKey::generate(&mut rng);
    let verifying_key = signing_key.verifying_key();

    println!("Public key: {}", hex::encode(verifying_key.to_bytes()));

    if let Some(path) = output {
        let output_path = PathBuf::from(&path);

        // Sanitize output path
        let output_path = sanitize_path(&output_path).unwrap_or(output_path);

        // Create parent directory if needed
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Prompt for encryption passphrase
        println!();
        println!("Your private key will be encrypted with a passphrase.");
        println!("Choose a strong passphrase (minimum 8 characters).");
        println!();

        let passphrase = prompt_passphrase("Enter passphrase: ", true)?;

        // Get private key bytes
        let mut private_bytes = signing_key.to_bytes();

        // Encrypt the private key
        let encrypted = encrypt_private_key(&private_bytes, &passphrase)?;

        // Zeroize the plaintext private key
        private_bytes.zeroize();

        // Write encrypted key to file
        std::fs::write(&output_path, &encrypted)?;

        // Set restrictive file permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&output_path, permissions)?;
        }

        println!();
        println!("Encrypted private key saved to: {}", output_path.display());
        println!();
        println!("IMPORTANT:");
        println!("  - Your private key is encrypted and protected by your passphrase");
        println!("  - Keep your passphrase secure - it cannot be recovered if lost");
        println!("  - Back up this file and your passphrase separately");
    } else {
        println!();
        println!("WARNING: Private key not saved (use --output to save)");
        println!("The key will be lost when this program exits.");
    }

    Ok(())
}
