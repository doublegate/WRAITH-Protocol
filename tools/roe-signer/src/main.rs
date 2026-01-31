//! ROE Signer - Sign WRAITH-Recon Rules of Engagement JSON files with Ed25519
//!
//! This tool replicates the exact `signing_data()` logic from
//! `clients/wraith-recon/src-tauri/src/roe.rs` to produce valid signatures.

use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "roe-signer", about = "Sign WRAITH-Recon ROE files with Ed25519")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate a new Ed25519 keypair
    Keygen {
        /// Output directory for key files
        #[arg(short, long, default_value = ".")]
        output: PathBuf,
    },
    /// Sign an ROE JSON file
    Sign {
        /// Path to the unsigned ROE JSON file
        #[arg(short, long)]
        input: PathBuf,
        /// Path to the Ed25519 secret key file (hex-encoded, 64 bytes = seed + public)
        #[arg(short, long)]
        key: PathBuf,
        /// Output path for signed ROE (defaults to <input>-signed.json)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Verify an ROE JSON file's signature
    Verify {
        /// Path to the signed ROE JSON file
        #[arg(short, long)]
        input: PathBuf,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmergencyContact {
    name: String,
    role: String,
    phone: String,
    email: String,
}

/// Mirrors `RulesOfEngagement` from wraith-recon roe.rs exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RulesOfEngagement {
    id: String,
    version: String,
    organization: String,
    title: String,
    description: String,
    authorized_operators: Vec<String>,
    client_name: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    authorized_cidrs: Vec<String>,
    authorized_domains: Vec<String>,
    excluded_targets: Vec<String>,
    authorized_techniques: Vec<String>,
    prohibited_techniques: Vec<String>,
    max_exfil_rate: Option<u64>,
    max_exfil_total: Option<u64>,
    emergency_contacts: Vec<EmergencyContact>,
    constraints: Vec<String>,
    created_at: DateTime<Utc>,
    signer_public_key: String,
    signature: String,
}

impl RulesOfEngagement {
    /// Exact replica of `signing_data()` from roe.rs lines 118-163.
    fn signing_data(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(self.id.as_bytes());
        data.extend(self.version.as_bytes());
        data.extend(self.organization.as_bytes());
        data.extend(self.title.as_bytes());
        data.extend(self.description.as_bytes());
        for op in &self.authorized_operators {
            data.extend(op.as_bytes());
        }
        data.extend(self.client_name.as_bytes());
        data.extend(self.start_time.to_rfc3339().as_bytes());
        data.extend(self.end_time.to_rfc3339().as_bytes());
        for cidr in &self.authorized_cidrs {
            data.extend(cidr.as_bytes());
        }
        for domain in &self.authorized_domains {
            data.extend(domain.as_bytes());
        }
        for excluded in &self.excluded_targets {
            data.extend(excluded.as_bytes());
        }
        for technique in &self.authorized_techniques {
            data.extend(technique.as_bytes());
        }
        for technique in &self.prohibited_techniques {
            data.extend(technique.as_bytes());
        }
        if let Some(rate) = self.max_exfil_rate {
            data.extend(&rate.to_le_bytes());
        }
        if let Some(total) = self.max_exfil_total {
            data.extend(&total.to_le_bytes());
        }
        for contact in &self.emergency_contacts {
            data.extend(contact.name.as_bytes());
            data.extend(contact.role.as_bytes());
            data.extend(contact.phone.as_bytes());
            data.extend(contact.email.as_bytes());
        }
        for constraint in &self.constraints {
            data.extend(constraint.as_bytes());
        }
        data.extend(self.created_at.to_rfc3339().as_bytes());
        data.extend(self.signer_public_key.as_bytes());
        data
    }
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Keygen { output } => {
            let signing_key = SigningKey::generate(&mut OsRng);
            let verifying_key = signing_key.verifying_key();

            // Write secret key (full 64-byte keypair: 32-byte seed + 32-byte public)
            let secret_path = output.join("roe-signing.key");
            let secret_hex = hex::encode(signing_key.to_keypair_bytes());
            fs::write(&secret_path, &secret_hex)
                .unwrap_or_else(|e| panic!("Failed to write {}: {e}", secret_path.display()));

            // Write public key
            let public_path = output.join("roe-signing.pub");
            let public_hex = hex::encode(verifying_key.to_bytes());
            fs::write(&public_path, &public_hex)
                .unwrap_or_else(|e| panic!("Failed to write {}: {e}", public_path.display()));

            println!("Keypair generated:");
            println!("  Secret key: {}", secret_path.display());
            println!("  Public key: {}", public_path.display());
            println!("  Public key (hex): {public_hex}");
        }

        Command::Sign { input, key, output } => {
            let key_hex = fs::read_to_string(&key)
                .unwrap_or_else(|e| panic!("Failed to read key {}: {e}", key.display()));
            let key_bytes = hex::decode(key_hex.trim())
                .expect("Key file must contain hex-encoded bytes");

            let signing_key = if key_bytes.len() == 64 {
                SigningKey::from_keypair_bytes(&key_bytes.try_into().unwrap())
                    .expect("Invalid 64-byte keypair")
            } else if key_bytes.len() == 32 {
                SigningKey::from_bytes(&key_bytes.try_into().unwrap())
            } else {
                panic!("Key must be 32 bytes (seed) or 64 bytes (keypair), got {}", key_bytes.len());
            };

            let verifying_key = signing_key.verifying_key();

            let json_str = fs::read_to_string(&input)
                .unwrap_or_else(|e| panic!("Failed to read {}: {e}", input.display()));
            let mut roe: RulesOfEngagement =
                serde_json::from_str(&json_str).expect("Failed to parse ROE JSON");

            // Set public key before computing signing data (it's included in the hash)
            roe.signer_public_key = hex::encode(verifying_key.to_bytes());

            let signing_data = roe.signing_data();
            let signature = signing_key.sign(&signing_data);
            roe.signature = hex::encode(signature.to_bytes());

            let out_path = output.unwrap_or_else(|| {
                let stem = input.file_stem().unwrap().to_string_lossy();
                input.with_file_name(format!("{stem}-signed.json"))
            });

            let signed_json = serde_json::to_string_pretty(&roe).expect("Failed to serialize");
            fs::write(&out_path, &signed_json)
                .unwrap_or_else(|e| panic!("Failed to write {}: {e}", out_path.display()));

            println!("Signed ROE written to: {}", out_path.display());
            println!("Public key: {}", roe.signer_public_key);
        }

        Command::Verify { input } => {
            let json_str = fs::read_to_string(&input)
                .unwrap_or_else(|e| panic!("Failed to read {}: {e}", input.display()));
            let roe: RulesOfEngagement =
                serde_json::from_str(&json_str).expect("Failed to parse ROE JSON");

            if roe.signature.is_empty() || roe.signer_public_key.is_empty() {
                println!("UNSIGNED: No signature or public key present.");
                std::process::exit(1);
            }

            let pk_bytes = hex::decode(&roe.signer_public_key).expect("Invalid public key hex");
            let pk_array: [u8; 32] = pk_bytes.try_into().expect("Public key must be 32 bytes");
            let verifying_key =
                VerifyingKey::from_bytes(&pk_array).expect("Invalid Ed25519 public key");

            let sig_bytes = hex::decode(&roe.signature).expect("Invalid signature hex");
            let sig_array: [u8; 64] = sig_bytes.try_into().expect("Signature must be 64 bytes");
            let signature = ed25519_dalek::Signature::from_bytes(&sig_array);

            let signing_data = roe.signing_data();
            match verifying_key.verify_strict(&signing_data, &signature) {
                Ok(()) => {
                    println!("VALID: Signature verified successfully.");
                    println!("  Signer: {}", roe.signer_public_key);
                    println!("  ROE ID: {}", roe.id);
                    println!("  Window: {} to {}", roe.start_time, roe.end_time);
                }
                Err(e) => {
                    println!("INVALID: Signature verification failed: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}
