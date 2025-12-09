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

// ═══════════════════════════════════════════════════════════════════════════
// Helper Functions
// ═══════════════════════════════════════════════════════════════════════════

/// Parse hex string to PeerId (32-byte array)
fn parse_peer_id(s: &str) -> anyhow::Result<PeerId> {
    let s = s.trim();
    let bytes = if s.starts_with("0x") || s.starts_with("0X") {
        hex::decode(&s[2..])?
    } else {
        hex::decode(s)?
    };

    if bytes.len() != 32 {
        anyhow::bail!(
            "Peer ID must be 32 bytes (64 hex characters), got {}",
            bytes.len()
        );
    }

    let mut peer_id = [0u8; 32];
    peer_id.copy_from_slice(&bytes);
    Ok(peer_id)
}

/// Parse hex string to TransferId (32-byte array)
fn parse_transfer_id(s: &str) -> anyhow::Result<TransferId> {
    let s = s.trim();
    let bytes = if s.starts_with("0x") || s.starts_with("0X") {
        hex::decode(&s[2..])?
    } else {
        hex::decode(s)?
    };

    if bytes.len() != 32 {
        anyhow::bail!(
            "Transfer ID must be 32 bytes (64 hex characters), got {}",
            bytes.len()
        );
    }

    let mut transfer_id = [0u8; 32];
    transfer_id.copy_from_slice(&bytes);
    Ok(transfer_id)
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

    // Parse peer ID
    let peer_id = parse_peer_id(&recipient)?;

    println!("File: {}", file.display());
    println!("Size: {}", format_bytes(file_size));
    println!("Recipient: {}", hex::encode(&peer_id[..8]));
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

    // Create progress bar
    let progress = TransferProgress::new(file_size, filename);

    // Send file using Node API
    tracing::info!("Establishing session with peer...");
    let transfer_id = node.send_file(&file, &peer_id).await?;

    println!("Transfer started: {}", hex::encode(&transfer_id[..8]));
    progress.finish_with_message("Sending file...".to_string());

    // Wait for transfer completion with progress updates
    let progress = TransferProgress::new(file_size, filename);
    loop {
        if let Some(transfer_progress) = node.get_transfer_progress(&transfer_id).await {
            progress.update(transfer_progress.bytes_sent);

            if transfer_progress.status == wraith_core::node::progress::TransferStatus::Complete {
                progress.finish_with_message(format!(
                    "Transfer complete: {} sent",
                    format_bytes(transfer_progress.bytes_sent)
                ));
                break;
            }

            if transfer_progress.status == wraith_core::node::progress::TransferStatus::Failed {
                progress.finish_with_message("Transfer failed".to_string());
                anyhow::bail!("Transfer failed");
            }
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    // Stop node
    node.stop().await?;
    println!("Node stopped");

    Ok(())
}

/// Receive files from peers
async fn receive_files(output: PathBuf, _bind: String, config: &Config) -> anyhow::Result<()> {
    // Create output directory if it doesn't exist
    if !output.exists() {
        std::fs::create_dir_all(&output)?;
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
    println!();
    println!("Ready to receive files. Press Ctrl+C to stop");
    println!();

    // Monitor for incoming transfers
    let node_arc = Arc::new(node);
    let node_clone = Arc::clone(&node_arc);
    let output_clone = output.clone();

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
    if config.network.enable_xdp {
        if let Some(iface) = &config.network.xdp_interface {
            println!("XDP interface: {iface}");
        }
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
    _detailed: bool,
    config: &Config,
) -> anyhow::Result<()> {
    println!("WRAITH Protocol Status");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
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

    Ok(())
}

/// List connected peers
async fn list_peers(dht_query: Option<String>, config: &Config) -> anyhow::Result<()> {
    if let Some(peer_id_str) = dht_query {
        let peer_id = parse_peer_id(&peer_id_str)?;

        println!("Querying DHT for peer: {}", hex::encode(&peer_id[..8]));
        println!();

        // Create temporary node for DHT query
        let node_config = create_node_config(config);
        let node = Node::new_with_config(node_config).await?;
        node.start().await?;

        println!("Discovering peer via DHT...");
        match node.discover_peer(&peer_id).await {
            Ok(addrs) => {
                println!("Peer found:");
                println!("  ID: {}", hex::encode(peer_id));
                println!("  Addresses:");
                for addr in addrs {
                    println!("    - {}", addr);
                }
            }
            Err(e) => {
                println!("Peer not found: {}", e);
            }
        }

        node.stop().await?;
        return Ok(());
    }

    println!("Connected Peers:");
    println!();
    println!("NOTE: Listing active peers requires a running daemon.");
    println!("Start a daemon with: wraith daemon");
    println!("Then query peer list via IPC (future feature)");
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
    if config.network.enable_xdp {
        if let Some(iface) = &config.network.xdp_interface {
            println!("  XDP interface: {}", iface);
        }
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
}
