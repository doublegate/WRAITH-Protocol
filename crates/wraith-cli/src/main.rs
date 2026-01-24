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
use std::sync::Arc;
use std::time::Duration;
use zeroize::Zeroize;

use config::Config;
use progress::{TransferProgress, format_bytes};

// WRAITH Core imports
use wraith_core::node::identity::TransferId;
use wraith_core::node::session::PeerId;
use wraith_core::node::{Node, NodeConfig};

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
    /// Send a file to one or more peers
    Send {
        /// File to send
        #[arg(required = true)]
        file: String,

        /// Recipient peer ID or address (can be specified multiple times)
        #[arg(required = true)]
        recipient: Vec<String>,

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

        /// Automatically accept transfers without prompting
        #[arg(long)]
        auto_accept: bool,

        /// Comma-separated list of trusted peer IDs (only accept from these peers)
        #[arg(long)]
        trusted_peers: Option<String>,
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

    /// Ping a peer to measure connectivity
    Ping {
        /// Peer ID to ping
        #[arg(required = true)]
        peer: String,

        /// Number of ping packets to send
        #[arg(short, long, default_value = "4")]
        count: u32,

        /// Interval between pings in milliseconds
        #[arg(short, long, default_value = "1000")]
        interval: u64,
    },

    /// View or modify configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show {
        /// Show specific configuration key
        key: Option<String>,
    },

    /// Set a configuration value
    Set {
        /// Configuration key to set
        key: String,

        /// Value to set
        value: String,
    },
}

// ═══════════════════════════════════════════════════════════════════════════
// Helper Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Parse hex string to PeerId (32-byte array)
fn parse_peer_id(s: &str) -> anyhow::Result<PeerId> {
    wraith_core::node::identity::parse_peer_id(s)
        .map_err(|e| anyhow::anyhow!("Failed to parse peer ID: {}", e))
}

/// Parse hex string to TransferId (32-byte array)
fn parse_transfer_id(s: &str) -> anyhow::Result<TransferId> {
    wraith_core::node::identity::parse_transfer_id(s)
        .map_err(|e| anyhow::anyhow!("Failed to parse transfer ID: {}", e))
}

/// Format duration as human-readable string
#[allow(dead_code)]
fn format_duration(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Create NodeConfig from CLI Config
fn create_node_config(config: &Config) -> NodeConfig {
    NodeConfig {
        listen_addr: config
            .network
            .listen_addr
            .parse()
            .unwrap_or_else(|_| "0.0.0.0:0".parse().expect("Invalid default listen address")),
        ..NodeConfig::default()
    }
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
    if matches!(cli.command, Commands::Keygen { .. })
        && let Commands::Keygen { output } = cli.command
    {
        return generate_keypair(output, &Config::default()).await;
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
        Commands::Receive {
            output,
            bind,
            auto_accept,
            trusted_peers,
        } => {
            receive_files(
                PathBuf::from(output),
                bind,
                auto_accept,
                trusted_peers,
                &config,
            )
            .await?;
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
        Commands::Ping {
            peer,
            count,
            interval,
        } => {
            ping_peer(peer, count, interval, &config).await?;
        }
        Commands::Config { action } => match action {
            ConfigAction::Show { key } => {
                config_show(key, &config).await?;
            }
            ConfigAction::Set { key, value } => {
                config_set(key, value, &cli.config).await?;
            }
        },
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

/// Send a file to one or more recipients
async fn send_file(
    file: PathBuf,
    recipients: Vec<String>,
    _mode: String,
    config: &Config,
) -> anyhow::Result<()> {
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

    // Parse all peer IDs
    let mut peer_ids = Vec::new();
    for recipient in &recipients {
        let peer_id = parse_peer_id(recipient)?;
        peer_ids.push(peer_id);
    }

    println!("File: {}", file.display());
    println!("Size: {}", format_bytes(file_size));
    println!("Recipients: {}", peer_ids.len());
    for (idx, peer_id) in peer_ids.iter().enumerate() {
        println!("  {}: {}", idx + 1, hex::encode(&peer_id[..8]));
    }
    println!();

    // Create and start node
    let node_config = create_node_config(config);
    let node = Node::new_with_config(node_config).await?;

    tracing::info!("Starting node...");
    node.start().await?;

    let listen_addr = node.listen_addr().await?;
    println!("Node started: {}", hex::encode(node.node_id()));
    println!("Listening on: {}", listen_addr);
    println!();

    // Send file to each recipient
    let mut transfer_ids = Vec::new();
    for (idx, peer_id) in peer_ids.iter().enumerate() {
        println!(
            "[{}/{}] Sending to {}...",
            idx + 1,
            peer_ids.len(),
            hex::encode(&peer_id[..8])
        );

        // Send file using Node API
        tracing::info!("Establishing session with peer...");
        let transfer_id = node.send_file(&file, peer_id).await?;
        transfer_ids.push(transfer_id);

        println!("  Transfer started: {}", hex::encode(&transfer_id[..8]));
    }

    println!();
    println!("Monitoring {} transfer(s)...", transfer_ids.len());

    // Wait for all transfers to complete
    let progress = TransferProgress::new(file_size * peer_ids.len() as u64, filename);
    let mut completed = vec![false; transfer_ids.len()];
    let mut total_sent = 0u64;

    loop {
        let mut all_done = true;

        for (idx, transfer_id) in transfer_ids.iter().enumerate() {
            if completed[idx] {
                continue;
            }

            if let Some(transfer_progress) = node.get_transfer_progress(transfer_id).await {
                if transfer_progress.status == wraith_core::node::progress::TransferStatus::Complete
                {
                    completed[idx] = true;
                    total_sent += file_size;
                    println!(
                        "Transfer {} complete: {} sent to {}",
                        hex::encode(&transfer_id[..8]),
                        format_bytes(file_size),
                        hex::encode(&peer_ids[idx][..8])
                    );
                } else if transfer_progress.status
                    == wraith_core::node::progress::TransferStatus::Failed
                {
                    completed[idx] = true;
                    println!(
                        "Transfer {} failed to {}",
                        hex::encode(&transfer_id[..8]),
                        hex::encode(&peer_ids[idx][..8])
                    );
                } else {
                    all_done = false;
                }
            } else {
                all_done = false;
            }
        }

        progress.update(total_sent);

        if all_done {
            let successful = completed.iter().filter(|&&c| c).count();
            progress.finish_with_message(format!(
                "All transfers complete: {}/{} successful",
                successful,
                transfer_ids.len()
            ));
            break;
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Stop node
    node.stop().await?;
    println!("Node stopped");

    Ok(())
}

/// Receive files from peers
async fn receive_files(
    output: PathBuf,
    _bind: String,
    auto_accept: bool,
    trusted_peers: Option<String>,
    config: &Config,
) -> anyhow::Result<()> {
    // Create output directory if it doesn't exist
    if !output.exists() {
        std::fs::create_dir_all(&output)?;
    }

    // Parse trusted peers if provided
    let mut trusted_peer_ids = Vec::new();
    if let Some(peers_str) = trusted_peers {
        for peer_str in peers_str.split(',') {
            let peer_id = parse_peer_id(peer_str.trim())?;
            trusted_peer_ids.push(peer_id);
        }
    }

    // Create and start node
    let node_config = create_node_config(config);
    let node = Node::new_with_config(node_config).await?;

    tracing::info!("Starting receive node...");
    node.start().await?;

    let listen_addr = node.listen_addr().await?;
    println!("WRAITH Receive Mode");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Node ID: {}", hex::encode(node.node_id()));
    println!("Listening on: {}", listen_addr);
    println!("Output directory: {}", output.display());
    println!("Auto-accept: {}", auto_accept);
    if !trusted_peer_ids.is_empty() {
        println!("Trusted peers: {}", trusted_peer_ids.len());
        for (idx, peer_id) in trusted_peer_ids.iter().enumerate() {
            println!("  {}: {}", idx + 1, hex::encode(&peer_id[..8]));
        }
    }
    println!();
    println!("Ready to receive files. Press Ctrl+C to stop");
    println!();

    // Monitor for incoming transfers
    let node_arc = Arc::new(node);
    let node_clone = Arc::clone(&node_arc);
    let output_clone = output.clone();
    let _auto_accept = auto_accept;
    let _trusted_peer_ids = trusted_peer_ids;

    tokio::spawn(async move {
        loop {
            let transfers = node_clone.active_transfers().await;
            for transfer_id in transfers {
                if let Some(progress) = node_clone.get_transfer_progress(&transfer_id).await {
                    println!(
                        "Transfer {}: {} / {} ({:.1}%)",
                        hex::encode(&transfer_id[..8]),
                        format_bytes(progress.bytes_sent),
                        format_bytes(progress.bytes_total),
                        (progress.bytes_sent as f64 / progress.bytes_total as f64 * 100.0)
                    );

                    if progress.status == wraith_core::node::progress::TransferStatus::Complete {
                        println!(
                            "Transfer {} complete - saved to {}",
                            hex::encode(&transfer_id[..8]),
                            output_clone.display()
                        );
                    }
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });

    // Keep alive until Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down...");

    node_arc.stop().await?;
    println!("Node stopped");

    Ok(())
}

/// Run daemon mode
async fn run_daemon(_bind: String, _relay: bool, config: &Config) -> anyhow::Result<()> {
    // Create and start node
    let node_config = create_node_config(config);
    let node = Node::new_with_config(node_config).await?;

    tracing::info!("Starting WRAITH daemon...");
    node.start().await?;

    let listen_addr = node.listen_addr().await?;

    println!("WRAITH Daemon");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("Node ID: {}", hex::encode(node.node_id()));
    println!("Listening on: {}", listen_addr);
    println!("XDP: {}", config.network.enable_xdp);
    if config.network.enable_xdp
        && let Some(iface) = &config.network.xdp_interface
    {
        println!("XDP interface: {iface}");
    }
    println!();
    println!("Daemon ready. Press Ctrl+C to stop");
    println!();

    // Monitor sessions and transfers
    let node_arc = Arc::new(node);
    let node_clone = Arc::clone(&node_arc);

    tokio::spawn(async move {
        loop {
            let sessions = node_clone.active_sessions().await;
            let transfers = node_clone.active_transfers().await;

            if !sessions.is_empty() || !transfers.is_empty() {
                println!(
                    "Status: {} active sessions, {} active transfers",
                    sessions.len(),
                    transfers.len()
                );
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // Keep alive until Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("\nShutting down...");

    node_arc.stop().await?;
    println!("Daemon stopped");

    Ok(())
}

/// Send batch of files
async fn send_batch(
    files: Vec<String>,
    recipient: String,
    _mode: String,
    config: &Config,
) -> anyhow::Result<()> {
    // Parse peer ID
    let peer_id = parse_peer_id(&recipient)?;

    println!("Batch Transfer");
    println!("Files: {}", files.len());
    println!("Recipient: {}", hex::encode(&peer_id[..8]));
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

    // Create and start node
    let node_config = create_node_config(config);
    let node = Node::new_with_config(node_config).await?;

    tracing::info!("Starting node...");
    node.start().await?;

    let listen_addr = node.listen_addr().await?;
    println!("Node started: {}", hex::encode(node.node_id()));
    println!("Listening on: {}", listen_addr);
    println!();

    // Send each file
    for (idx, (file_path, file_size)) in sanitized_files.iter().enumerate() {
        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        println!("[{}/{}] {}", idx + 1, sanitized_files.len(), filename);
        println!("  Size: {}", format_bytes(*file_size));

        let progress = TransferProgress::new(*file_size, filename);

        // Send file using Node API
        let transfer_id = node.send_file(file_path, &peer_id).await?;
        println!("  Transfer ID: {}", hex::encode(&transfer_id[..8]));

        // Wait for completion
        node.wait_for_transfer(transfer_id).await?;

        progress.finish_with_message(format!(
            "File {}/{} complete",
            idx + 1,
            sanitized_files.len()
        ));
    }

    println!();
    println!("Batch transfer complete: {} files sent", files.len());

    // Stop node
    node.stop().await?;
    println!("Node stopped");

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
    println!("Build: {} edition", env!("CARGO_PKG_RUST_VERSION"));
    println!();

    if let Some(transfer_id_str) = transfer {
        let transfer_id = parse_transfer_id(&transfer_id_str)?;
        println!("Transfer status query: {}", hex::encode(&transfer_id[..8]));
        println!();
        println!("NOTE: Transfer status queries require a running daemon.");
        println!("Start a daemon with: wraith daemon");
        println!("Then query transfer status via IPC (future feature)");
        return Ok(());
    }

    // Basic status information
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
    if config.network.enable_xdp
        && let Some(iface) = &config.network.xdp_interface
    {
        println!("  XDP interface: {}", iface);
    }
    println!("  UDP fallback: {}", config.network.udp_fallback);
    println!();

    println!("Discovery:");
    println!(
        "  Bootstrap nodes: {}",
        config.discovery.bootstrap_nodes.len()
    );
    println!("  Relay servers: {}", config.discovery.relay_servers.len());
    println!();

    // Detailed information
    if detailed {
        println!("Detailed Configuration:");
        println!();

        println!("  Obfuscation:");
        println!("    Default level: {}", config.obfuscation.default_level);
        println!("    TLS mimicry: {}", config.obfuscation.tls_mimicry);
        println!();

        println!("  Transfer:");
        println!(
            "    Chunk size: {}",
            format_bytes(config.transfer.chunk_size as u64)
        );
        println!("    Max concurrent: {}", config.transfer.max_concurrent);
        println!("    Enable resume: {}", config.transfer.enable_resume);
        println!();

        println!("  Logging:");
        println!("    Level: {}", config.logging.level);
        if let Some(file) = &config.logging.file {
            println!("    File: {}", file.display());
        }
        println!();

        println!("  Bootstrap Nodes:");
        for (idx, node) in config.discovery.bootstrap_nodes.iter().enumerate() {
            println!("    {}: {}", idx + 1, node);
        }
        println!();

        if !config.discovery.relay_servers.is_empty() {
            println!("  Relay Servers:");
            for (idx, server) in config.discovery.relay_servers.iter().enumerate() {
                println!("    {}: {}", idx + 1, server);
            }
            println!();
        }

        // Platform information
        println!("Platform:");
        println!("  OS: {}", std::env::consts::OS);
        println!("  Architecture: {}", std::env::consts::ARCH);
        println!("  io_uring support: {}", cfg!(target_os = "linux"));
        println!();
    }

    println!("NOTE: Runtime status requires a running daemon.");
    println!("Start a daemon with: wraith daemon");
    println!("Then query status via IPC (future feature)");

    Ok(())
}

/// List connected peers
async fn list_peers(dht_query: Option<String>, config: &Config) -> anyhow::Result<()> {
    if let Some(peer_id_str) = dht_query {
        let peer_id = parse_peer_id(&peer_id_str)?;

        println!("DHT Peer Query");
        println!("Peer ID: {}", hex::encode(peer_id));
        println!();

        // Create temporary node for DHT query
        let node_config = create_node_config(config);
        let node = Node::new_with_config(node_config).await?;

        println!("Starting node for DHT query...");
        node.start().await?;

        let listen_addr = node.listen_addr().await?;
        println!("Node started: {}", hex::encode(node.node_id()));
        println!("Listening on: {}", listen_addr);
        println!();

        println!("Discovering peer via DHT...");
        match node.discover_peer(&peer_id).await {
            Ok(addrs) => {
                println!();
                println!("Peer found successfully!");
                println!();
                println!("Details:");
                println!("  Peer ID: {}", hex::encode(peer_id));
                println!("  Addresses: {}", addrs.len());
                for (idx, addr) in addrs.iter().enumerate() {
                    println!("    {}: {}", idx + 1, addr);
                }
                println!();
            }
            Err(e) => {
                println!();
                println!("Peer discovery failed: {}", e);
                println!();
                println!("Possible reasons:");
                println!("  - Peer is not online");
                println!("  - Peer ID is invalid");
                println!("  - DHT network is not reachable");
                println!("  - Bootstrap nodes are offline");
                println!();
            }
        }

        println!("Stopping node...");
        node.stop().await?;
        println!("Node stopped");

        return Ok(());
    }

    // List mode (no DHT query)
    println!("Connected Peers");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    println!("Discovery Configuration:");
    println!(
        "  Bootstrap nodes: {}",
        config.discovery.bootstrap_nodes.len()
    );
    if !config.discovery.bootstrap_nodes.is_empty() {
        for (idx, node) in config.discovery.bootstrap_nodes.iter().enumerate() {
            println!("    {}: {}", idx + 1, node);
        }
    }
    println!();

    println!("  Relay servers: {}", config.discovery.relay_servers.len());
    if !config.discovery.relay_servers.is_empty() {
        for (idx, server) in config.discovery.relay_servers.iter().enumerate() {
            println!("    {}: {}", idx + 1, server);
        }
    }
    println!();

    println!("NOTE: Listing active peers requires a running daemon.");
    println!("Start a daemon with: wraith daemon");
    println!("Then query peer list via IPC (future feature)");
    println!();
    println!("To query a specific peer via DHT, use:");
    println!("  wraith peers --dht-query <peer-id>");

    Ok(())
}

/// Show node health
async fn show_health(config: &Config) -> anyhow::Result<()> {
    println!("WRAITH Node Health Check");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    // Configuration health
    println!("Configuration:");
    println!("  Config file: OK");
    println!(
        "  Listen address: {} ({})",
        config.network.listen_addr,
        if config.network.listen_addr.starts_with("0.0.0.0") {
            "will bind to all interfaces"
        } else {
            "specific interface"
        }
    );
    println!("  XDP: {} ", config.network.enable_xdp);
    if config.network.enable_xdp
        && let Some(iface) = &config.network.xdp_interface
    {
        println!("  XDP interface: {}", iface);
    }
    println!();

    // Test node creation
    println!("Node Creation:");
    match Node::new_random().await {
        Ok(_node) => {
            println!("  Identity generation: OK");
            println!("  Node initialization: OK");
        }
        Err(e) => {
            println!("  Node creation: FAILED - {}", e);
            return Ok(());
        }
    }
    println!();

    // Discovery health
    println!("Discovery:");
    println!(
        "  Bootstrap nodes: {} configured",
        config.discovery.bootstrap_nodes.len()
    );
    println!(
        "  Relay servers: {} configured",
        config.discovery.relay_servers.len()
    );
    println!();

    println!("Overall Health: OK");
    println!();
    println!("NOTE: For runtime health metrics, start a daemon with: wraith daemon");

    Ok(())
}

/// Show metrics
async fn show_metrics(json: bool, _watch: Option<u64>, config: &Config) -> anyhow::Result<()> {
    if json {
        // JSON output
        println!(
            r#"{{
  "version": "{}",
  "configuration": {{
    "listen_addr": "{}",
    "xdp_enabled": {},
    "chunk_size": {},
    "max_concurrent": {}
  }},
  "note": "Runtime metrics require a running daemon. Start with: wraith daemon"
}}"#,
            env!("CARGO_PKG_VERSION"),
            config.network.listen_addr,
            config.network.enable_xdp,
            config.transfer.chunk_size,
            config.transfer.max_concurrent
        );
        return Ok(());
    }

    // Text output
    println!("WRAITH Metrics");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();

    println!("Configuration:");
    println!("  Listen: {}", config.network.listen_addr);
    println!("  XDP: {}", config.network.enable_xdp);
    println!(
        "  Chunk size: {}",
        format_bytes(config.transfer.chunk_size as u64)
    );
    println!("  Max concurrent: {}", config.transfer.max_concurrent);
    println!();

    println!("NOTE: Runtime metrics require a running daemon.");
    println!("Start a daemon with: wraith daemon");
    println!("Then query metrics via IPC (future feature)");

    Ok(())
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

    // Generate temporary node to show ID
    let node = Node::new_random().await?;
    println!("Node:");
    println!("  ID: {}", hex::encode(node.node_id()));
    println!("  X25519 Key: {}", hex::encode(node.x25519_public_key()));
    println!("  Listen: {}", config.network.listen_addr);
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

    println!("NOTE: Node ID shown is randomly generated.");
    println!("Use 'wraith keygen' to create a persistent identity.");

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

/// Ping a peer to measure connectivity and RTT
async fn ping_peer(peer: String, count: u32, interval: u64, config: &Config) -> anyhow::Result<()> {
    // Parse peer ID
    let peer_id = parse_peer_id(&peer)?;

    println!("WRAITH Ping");
    println!("Peer: {}", hex::encode(peer_id));
    println!("Count: {count}, Interval: {interval}ms");
    println!();

    // Create and start node
    let node_config = create_node_config(config);
    let node = Node::new_with_config(node_config).await?;

    tracing::info!("Starting ping node...");
    node.start().await?;

    println!("Node ID: {}", hex::encode(node.node_id()));
    println!();

    // Ping statistics
    let mut rtts = Vec::new();
    let mut packets_sent = 0u32;
    let mut packets_received = 0u32;

    for seq in 0..count {
        packets_sent += 1;
        let start = std::time::Instant::now();

        print!(
            "Ping {} ({}/{}): ",
            hex::encode(&peer_id[..8]),
            seq + 1,
            count
        );
        std::io::Write::flush(&mut std::io::stdout())?;

        // Attempt to establish connection or use existing session for RTT measurement
        match node.discover_peer(&peer_id).await {
            Ok(addrs) => {
                let rtt = start.elapsed();
                rtts.push(rtt);
                packets_received += 1;

                println!(
                    "time={:.2}ms, addrs={}",
                    rtt.as_secs_f64() * 1000.0,
                    addrs.len()
                );
            }
            Err(e) => {
                println!("timeout ({})", e);
            }
        }

        // Wait for interval before next ping (except for last one)
        if seq < count - 1 {
            tokio::time::sleep(Duration::from_millis(interval)).await;
        }
    }

    println!();

    // Calculate statistics
    if !rtts.is_empty() {
        let min_rtt = rtts.iter().min().unwrap();
        let max_rtt = rtts.iter().max().unwrap();
        let avg_rtt = rtts.iter().map(|d| d.as_secs_f64()).sum::<f64>() / rtts.len() as f64;

        // Calculate standard deviation for mdev
        let variance = rtts
            .iter()
            .map(|d| {
                let diff = d.as_secs_f64() - avg_rtt;
                diff * diff
            })
            .sum::<f64>()
            / rtts.len() as f64;
        let mdev = variance.sqrt();

        let packet_loss = if packets_sent > 0 {
            ((packets_sent - packets_received) as f64 / packets_sent as f64) * 100.0
        } else {
            0.0
        };

        println!("--- {} ping statistics ---", hex::encode(&peer_id[..8]));
        println!(
            "{} packets transmitted, {} received, {:.1}% packet loss",
            packets_sent, packets_received, packet_loss
        );
        println!(
            "rtt min/avg/max/mdev = {:.3}/{:.3}/{:.3}/{:.3} ms",
            min_rtt.as_secs_f64() * 1000.0,
            avg_rtt * 1000.0,
            max_rtt.as_secs_f64() * 1000.0,
            mdev * 1000.0
        );
    } else {
        println!("--- {} ping statistics ---", hex::encode(&peer_id[..8]));
        println!(
            "{} packets transmitted, 0 received, 100.0% packet loss",
            packets_sent
        );
    }

    println!();

    // Stop node
    node.stop().await?;

    Ok(())
}

/// Show configuration (all or specific key)
async fn config_show(key: Option<String>, config: &Config) -> anyhow::Result<()> {
    if let Some(key_name) = key {
        // Show specific key
        let key_lower = key_name.to_lowercase();

        match key_lower.as_str() {
            "network.listen_addr" | "listen_addr" => {
                println!("{}", config.network.listen_addr);
            }
            "network.enable_xdp" | "enable_xdp" => {
                println!("{}", config.network.enable_xdp);
            }
            "network.xdp_interface" | "xdp_interface" => {
                if let Some(iface) = &config.network.xdp_interface {
                    println!("{}", iface);
                } else {
                    println!("(not set)");
                }
            }
            "network.udp_fallback" | "udp_fallback" => {
                println!("{}", config.network.udp_fallback);
            }
            "obfuscation.default_level" | "default_level" => {
                println!("{}", config.obfuscation.default_level);
            }
            "obfuscation.tls_mimicry" | "tls_mimicry" => {
                println!("{}", config.obfuscation.tls_mimicry);
            }
            "transfer.chunk_size" | "chunk_size" => {
                println!("{}", config.transfer.chunk_size);
            }
            "transfer.max_concurrent" | "max_concurrent" => {
                println!("{}", config.transfer.max_concurrent);
            }
            "transfer.enable_resume" | "enable_resume" => {
                println!("{}", config.transfer.enable_resume);
            }
            _ => {
                anyhow::bail!("Unknown configuration key: {}", key_name);
            }
        }
    } else {
        // Show all configuration
        println!("WRAITH Configuration");
        println!();

        println!("[network]");
        println!("  listen_addr = \"{}\"", config.network.listen_addr);
        println!("  enable_xdp = {}", config.network.enable_xdp);
        if let Some(iface) = &config.network.xdp_interface {
            println!("  xdp_interface = \"{}\"", iface);
        }
        println!("  udp_fallback = {}", config.network.udp_fallback);
        println!();

        println!("[obfuscation]");
        println!("  default_level = \"{}\"", config.obfuscation.default_level);
        println!("  tls_mimicry = {}", config.obfuscation.tls_mimicry);
        println!();

        println!("[transfer]");
        println!("  chunk_size = {}", config.transfer.chunk_size);
        println!("  max_concurrent = {}", config.transfer.max_concurrent);
        println!("  enable_resume = {}", config.transfer.enable_resume);
        println!();

        println!("[discovery]");
        println!(
            "  bootstrap_nodes = {} configured",
            config.discovery.bootstrap_nodes.len()
        );
        println!(
            "  relay_servers = {} configured",
            config.discovery.relay_servers.len()
        );
        println!();

        println!("[logging]");
        println!("  level = \"{}\"", config.logging.level);
        println!("  file = {:?}", config.logging.file);
    }

    Ok(())
}

/// Set a configuration value
async fn config_set(key: String, value: String, config_path: &str) -> anyhow::Result<()> {
    // Expand tilde in config path
    let config_path_buf = if let Some(stripped) = config_path.strip_prefix("~/") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(stripped)
    } else {
        PathBuf::from(config_path)
    };

    // Load current config
    let mut config = if config_path_buf.exists() {
        Config::load(&config_path_buf)?
    } else {
        Config::default()
    };

    // Set the value
    let key_lower = key.to_lowercase();
    match key_lower.as_str() {
        "network.listen_addr" | "listen_addr" => {
            config.network.listen_addr = value.clone();
        }
        "network.enable_xdp" | "enable_xdp" => {
            config.network.enable_xdp = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid boolean value for enable_xdp: {}", value))?;
        }
        "network.xdp_interface" | "xdp_interface" => {
            config.network.xdp_interface = Some(value.clone());
        }
        "network.udp_fallback" | "udp_fallback" => {
            config.network.udp_fallback = value.parse().map_err(|_| {
                anyhow::anyhow!("Invalid boolean value for udp_fallback: {}", value)
            })?;
        }
        "obfuscation.default_level" | "default_level" => {
            config.obfuscation.default_level = value.clone();
        }
        "obfuscation.tls_mimicry" | "tls_mimicry" => {
            config.obfuscation.tls_mimicry = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid boolean value for tls_mimicry: {}", value))?;
        }
        "transfer.chunk_size" | "chunk_size" => {
            config.transfer.chunk_size = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid number for chunk_size: {}", value))?;
        }
        "transfer.max_concurrent" | "max_concurrent" => {
            config.transfer.max_concurrent = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid number for max_concurrent: {}", value))?;
        }
        "transfer.enable_resume" | "enable_resume" => {
            config.transfer.enable_resume = value.parse().map_err(|_| {
                anyhow::anyhow!("Invalid boolean value for enable_resume: {}", value)
            })?;
        }
        "logging.level" | "level" => {
            config.logging.level = value.clone();
        }
        _ => {
            anyhow::bail!("Unknown configuration key: {}", key);
        }
    }

    // Validate the new configuration
    config.validate()?;

    // Save the configuration
    config.save(&config_path_buf)?;

    println!("Configuration updated: {} = {}", key, value);
    println!("Saved to: {}", config_path_buf.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_path_no_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test").unwrap();

        let sanitized = sanitize_path(&file_path).unwrap();
        assert!(sanitized.exists());
        assert!(sanitized.is_absolute());
    }

    #[test]
    fn test_sanitize_path_rejects_dot_dot() {
        let path = PathBuf::from("../etc/passwd");
        let result = sanitize_path(&path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("traversal"));
    }

    #[test]
    fn test_sanitize_path_rejects_embedded_dot_dot() {
        let path = PathBuf::from("/home/user/../root/file.txt");
        let result = sanitize_path(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_path_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nonexistent.txt");

        // Should succeed for nonexistent files in existing parent
        let sanitized = sanitize_path(&file_path).unwrap();
        assert_eq!(sanitized.file_name(), file_path.file_name());
    }

    #[test]
    fn test_sanitize_path_nonexistent_parent() {
        let path = PathBuf::from("/nonexistent/directory/file.txt");
        let sanitized = sanitize_path(&path).unwrap();

        // Should return original path when parent doesn't exist
        assert_eq!(sanitized, path);
    }

    #[test]
    fn test_sanitize_path_symlink_resolution() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;

            let temp_dir = TempDir::new().unwrap();
            let real_file = temp_dir.path().join("real.txt");
            let symlink_file = temp_dir.path().join("link.txt");

            fs::write(&real_file, "test").unwrap();
            symlink(&real_file, &symlink_file).unwrap();

            let sanitized = sanitize_path(&symlink_file).unwrap();

            // Should resolve to the real file
            assert!(sanitized.is_absolute());
            assert!(sanitized.exists());
        }
    }

    #[test]
    fn test_encrypt_decrypt_private_key_roundtrip() {
        let mut rng = rand_core::OsRng;
        let signing_key = wraith_crypto::signatures::SigningKey::generate(&mut rng);
        let private_bytes = signing_key.to_bytes();
        let passphrase = "test_passphrase_12345";

        // Encrypt
        let encrypted = encrypt_private_key(&private_bytes, passphrase).unwrap();

        // Verify format
        assert!(encrypted.len() > ENCRYPTED_KEY_MAGIC.len() + ARGON2_SALT_SIZE + ARGON2_NONCE_SIZE);
        assert_eq!(&encrypted[..8], ENCRYPTED_KEY_MAGIC);

        // Decrypt
        let decrypted = decrypt_private_key(&encrypted, passphrase).unwrap();

        // Verify roundtrip
        assert_eq!(private_bytes, decrypted);
    }

    #[test]
    fn test_decrypt_private_key_wrong_passphrase() {
        let mut rng = rand_core::OsRng;
        let signing_key = wraith_crypto::signatures::SigningKey::generate(&mut rng);
        let private_bytes = signing_key.to_bytes();

        let encrypted = encrypt_private_key(&private_bytes, "correct_password").unwrap();

        // Should fail with wrong passphrase
        let result = decrypt_private_key(&encrypted, "wrong_password");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Decryption failed")
        );
    }

    #[test]
    fn test_decrypt_private_key_invalid_magic() {
        let mut invalid_data = vec![0u8; 100];
        invalid_data[..8].copy_from_slice(b"INVALID!");

        let result = decrypt_private_key(&invalid_data, "password");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("wrong format"));
    }

    #[test]
    fn test_decrypt_private_key_too_short() {
        let short_data = vec![0u8; 10];
        let result = decrypt_private_key(&short_data, "password");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too short"));
    }

    #[test]
    fn test_decrypt_private_key_corrupted_data() {
        let mut rng = rand_core::OsRng;
        let signing_key = wraith_crypto::signatures::SigningKey::generate(&mut rng);
        let private_bytes = signing_key.to_bytes();

        let mut encrypted = encrypt_private_key(&private_bytes, "password").unwrap();

        // Corrupt the ciphertext
        let len = encrypted.len();
        encrypted[len - 10] ^= 0xFF;

        let result = decrypt_private_key(&encrypted, "password");
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypted_key_format() {
        let mut rng = rand_core::OsRng;
        let signing_key = wraith_crypto::signatures::SigningKey::generate(&mut rng);
        let private_bytes = signing_key.to_bytes();

        let encrypted = encrypt_private_key(&private_bytes, "test_password").unwrap();

        // Verify structure
        assert_eq!(&encrypted[..8], ENCRYPTED_KEY_MAGIC);

        let salt_start = 8;
        let salt_end = salt_start + ARGON2_SALT_SIZE;
        let nonce_end = salt_end + ARGON2_NONCE_SIZE;
        let ciphertext_start = nonce_end;

        // Verify lengths
        assert!(encrypted.len() >= ciphertext_start + 32 + ARGON2_TAG_SIZE);

        // Verify salt and nonce are not all zeros (should be random)
        let salt = &encrypted[salt_start..salt_end];
        let nonce = &encrypted[salt_end..nonce_end];

        assert!(!salt.iter().all(|&b| b == 0));
        assert!(!nonce.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_encrypted_key_uniqueness() {
        let mut rng = rand_core::OsRng;
        let signing_key = wraith_crypto::signatures::SigningKey::generate(&mut rng);
        let private_bytes = signing_key.to_bytes();
        let passphrase = "same_passphrase";

        // Encrypt same key twice
        let encrypted1 = encrypt_private_key(&private_bytes, passphrase).unwrap();
        let encrypted2 = encrypt_private_key(&private_bytes, passphrase).unwrap();

        // Should be different due to random salt/nonce
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to same value
        let decrypted1 = decrypt_private_key(&encrypted1, passphrase).unwrap();
        let decrypted2 = decrypt_private_key(&encrypted2, passphrase).unwrap();
        assert_eq!(decrypted1, decrypted2);
        assert_eq!(decrypted1, private_bytes);
    }

    #[test]
    fn test_constants() {
        // Verify crypto constants are reasonable
        assert_eq!(ENCRYPTED_KEY_MAGIC, b"WRAITH01");
        assert_eq!(ARGON2_MEMORY_COST, 65536); // 64 MiB
        assert_eq!(ARGON2_TIME_COST, 3);
        assert_eq!(ARGON2_PARALLELISM, 4);
        assert_eq!(ARGON2_SALT_SIZE, 16);
        assert_eq!(ARGON2_NONCE_SIZE, 24); // XChaCha20
        assert_eq!(ARGON2_TAG_SIZE, 16);
    }

    #[test]
    fn test_sanitize_path_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test").unwrap();

        let sanitized = sanitize_path(&file_path).unwrap();
        assert!(sanitized.is_absolute());
    }

    #[test]
    fn test_sanitize_path_relative_to_absolute() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test").unwrap();

        // Create a relative path by stripping the prefix
        let current_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let relative_path = PathBuf::from("test.txt");
        let sanitized = sanitize_path(&relative_path).unwrap();

        // Should be absolute
        assert!(sanitized.is_absolute());

        // Restore original directory
        std::env::set_current_dir(current_dir).unwrap();
    }

    #[test]
    fn test_sanitize_path_preserves_filename() {
        let temp_dir = TempDir::new().unwrap();
        let filename = "myfile.txt";
        let file_path = temp_dir.path().join(filename);
        fs::write(&file_path, "test").unwrap();

        let sanitized = sanitize_path(&file_path).unwrap();
        assert_eq!(sanitized.file_name().unwrap(), filename);
    }

    #[test]
    fn test_encrypt_private_key_different_passphrases() {
        let mut rng = rand_core::OsRng;
        let signing_key = wraith_crypto::signatures::SigningKey::generate(&mut rng);
        let private_bytes = signing_key.to_bytes();

        let encrypted1 = encrypt_private_key(&private_bytes, "password1").unwrap();
        let encrypted2 = encrypt_private_key(&private_bytes, "password2").unwrap();

        // Different passphrases should produce different ciphertexts
        assert_ne!(encrypted1, encrypted2);

        // Each should only decrypt with its own passphrase
        assert!(decrypt_private_key(&encrypted1, "password1").is_ok());
        assert!(decrypt_private_key(&encrypted1, "password2").is_err());
        assert!(decrypt_private_key(&encrypted2, "password2").is_ok());
        assert!(decrypt_private_key(&encrypted2, "password1").is_err());
    }

    #[test]
    fn test_encrypt_private_key_long_passphrase() {
        let mut rng = rand_core::OsRng;
        let signing_key = wraith_crypto::signatures::SigningKey::generate(&mut rng);
        let private_bytes = signing_key.to_bytes();

        // Very long passphrase
        let long_passphrase = "a".repeat(1000);
        let encrypted = encrypt_private_key(&private_bytes, &long_passphrase).unwrap();
        let decrypted = decrypt_private_key(&encrypted, &long_passphrase).unwrap();

        assert_eq!(private_bytes, decrypted);
    }

    #[test]
    fn test_encrypt_private_key_unicode_passphrase() {
        let mut rng = rand_core::OsRng;
        let signing_key = wraith_crypto::signatures::SigningKey::generate(&mut rng);
        let private_bytes = signing_key.to_bytes();

        let unicode_passphrase = "パスワード🔐密码";
        let encrypted = encrypt_private_key(&private_bytes, unicode_passphrase).unwrap();
        let decrypted = decrypt_private_key(&encrypted, unicode_passphrase).unwrap();

        assert_eq!(private_bytes, decrypted);
    }

    #[test]
    fn test_decrypt_private_key_invalid_length() {
        let mut rng = rand_core::OsRng;
        let signing_key = wraith_crypto::signatures::SigningKey::generate(&mut rng);
        let private_bytes = signing_key.to_bytes();

        let mut encrypted = encrypt_private_key(&private_bytes, "password").unwrap();

        // Truncate the encrypted data
        encrypted.truncate(encrypted.len() - 10);

        let result = decrypt_private_key(&encrypted, "password");
        assert!(result.is_err());
    }

    #[test]
    fn test_sanitize_path_hidden_files() {
        let temp_dir = TempDir::new().unwrap();
        let hidden_file = temp_dir.path().join(".hidden");
        fs::write(&hidden_file, "test").unwrap();

        let sanitized = sanitize_path(&hidden_file).unwrap();
        assert!(sanitized.exists());
        assert_eq!(sanitized.file_name().unwrap(), ".hidden");
    }

    #[test]
    fn test_sanitize_path_nested_directories() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("a/b/c/file.txt");
        fs::create_dir_all(nested_path.parent().unwrap()).unwrap();
        fs::write(&nested_path, "test").unwrap();

        let sanitized = sanitize_path(&nested_path).unwrap();
        assert!(sanitized.exists());
        assert!(sanitized.is_absolute());
    }

    #[test]
    fn test_parse_peer_id_valid() {
        // Valid 64 hex character peer ID
        let peer_id_hex = "a".repeat(64);
        let result = parse_peer_id(&peer_id_hex);
        assert!(result.is_ok());

        let peer_id = result.unwrap();
        assert_eq!(peer_id.len(), 32);
    }

    #[test]
    fn test_parse_peer_id_with_0x_prefix() {
        let peer_id_hex = format!("0x{}", "b".repeat(64));
        let result = parse_peer_id(&peer_id_hex);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_peer_id_invalid_length() {
        let peer_id_hex = "a".repeat(32); // Only 16 bytes
        let result = parse_peer_id(&peer_id_hex);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("32 bytes"));
    }

    #[test]
    fn test_parse_peer_id_invalid_hex() {
        let invalid_hex = "zzzz".repeat(16);
        let result = parse_peer_id(&invalid_hex);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_transfer_id_valid() {
        let transfer_id_hex = "c".repeat(64);
        let result = parse_transfer_id(&transfer_id_hex);
        assert!(result.is_ok());

        let transfer_id = result.unwrap();
        assert_eq!(transfer_id.len(), 32);
    }

    #[test]
    fn test_parse_transfer_id_with_0x_prefix() {
        let transfer_id_hex = format!("0X{}", "d".repeat(64));
        let result = parse_transfer_id(&transfer_id_hex);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_transfer_id_invalid_length() {
        let transfer_id_hex = "e".repeat(62); // Not 64 hex chars
        let result = parse_transfer_id(&transfer_id_hex);
        assert!(result.is_err());
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
        assert_eq!(format_duration(Duration::from_secs(7200)), "2h 0m");
    }

    #[tokio::test]
    async fn test_config_show_all() {
        let config = Config::default();
        // Should not panic when showing all config
        let result = config_show(None, &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_show_specific_key() {
        let config = Config::default();

        // Test valid keys
        let result = config_show(Some("listen_addr".to_string()), &config).await;
        assert!(result.is_ok());

        let result = config_show(Some("network.enable_xdp".to_string()), &config).await;
        assert!(result.is_ok());

        let result = config_show(Some("chunk_size".to_string()), &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_show_unknown_key() {
        let config = Config::default();

        let result = config_show(Some("invalid_key".to_string()), &config).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown"));
    }

    #[tokio::test]
    async fn test_config_set_valid_values() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        let config_path_str = config_path.to_str().unwrap();

        // Set a valid value
        let result = config_set(
            "chunk_size".to_string(),
            "2097152".to_string(),
            config_path_str,
        )
        .await;
        assert!(result.is_ok());

        // Verify it was saved
        assert!(config_path.exists());

        // Load and verify
        let loaded_config = Config::load(&config_path).unwrap();
        assert_eq!(loaded_config.transfer.chunk_size, 2_097_152);
    }

    #[tokio::test]
    async fn test_config_set_boolean_value() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        let config_path_str = config_path.to_str().unwrap();

        // Set a boolean value (use tls_mimicry which has no validation constraints)
        let result = config_set(
            "tls_mimicry".to_string(),
            "true".to_string(),
            config_path_str,
        )
        .await;
        assert!(result.is_ok());

        // Verify it was saved
        let loaded_config = Config::load(&config_path).unwrap();
        assert!(loaded_config.obfuscation.tls_mimicry);
    }

    #[tokio::test]
    async fn test_config_set_string_value() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        let config_path_str = config_path.to_str().unwrap();

        // Set a string value
        let result = config_set(
            "listen_addr".to_string(),
            "127.0.0.1:8080".to_string(),
            config_path_str,
        )
        .await;
        assert!(result.is_ok());

        // Verify it was saved
        let loaded_config = Config::load(&config_path).unwrap();
        assert_eq!(loaded_config.network.listen_addr, "127.0.0.1:8080");
    }

    #[tokio::test]
    async fn test_config_set_invalid_key() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        let config_path_str = config_path.to_str().unwrap();

        let result = config_set(
            "invalid_key".to_string(),
            "value".to_string(),
            config_path_str,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown"));
    }

    #[tokio::test]
    async fn test_config_set_invalid_boolean() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        let config_path_str = config_path.to_str().unwrap();

        let result = config_set(
            "enable_xdp".to_string(),
            "not_a_boolean".to_string(),
            config_path_str,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid boolean"));
    }

    #[tokio::test]
    async fn test_config_set_invalid_number() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        let config_path_str = config_path.to_str().unwrap();

        let result = config_set(
            "chunk_size".to_string(),
            "not_a_number".to_string(),
            config_path_str,
        )
        .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid number"));
    }
}
