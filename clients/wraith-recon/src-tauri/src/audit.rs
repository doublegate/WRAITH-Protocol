//! Tamper-Evident Audit Logging Module
//!
//! This module provides cryptographically signed audit logging with Merkle tree
//! chain verification. Every operation is logged with an Ed25519 signature and
//! linked to previous entries via hash chaining.
//!
//! ## Security Requirements
//! - All log entries are signed with Ed25519
//! - Entries are chained via BLAKE3 hashes (Merkle tree)
//! - Chain integrity can be verified at any time
//! - Logs are exportable for compliance/review

use crate::error::{ReconError, Result};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

/// Audit event severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditLevel {
    /// Informational message
    Info,
    /// Warning (potential issue)
    Warning,
    /// Error (operation failed)
    Error,
    /// Critical (security-relevant)
    Critical,
    /// Emergency (kill switch, breach detected)
    Emergency,
}

/// Audit event category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditCategory {
    /// System startup/shutdown
    System,
    /// RoE operations
    RulesOfEngagement,
    /// Scope changes
    ScopeChange,
    /// Timing events
    TimingEvent,
    /// Kill switch events
    KillSwitch,
    /// Reconnaissance operations
    Reconnaissance,
    /// Channel operations
    Channel,
    /// Data transfer
    DataTransfer,
    /// Configuration changes
    Configuration,
    /// Authentication/authorization
    Authentication,
    /// Audit system events
    AuditSystem,
}

/// MITRE ATT&CK technique reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MitreReference {
    /// Technique ID (e.g., "T1046")
    pub technique_id: String,
    /// Technique name
    pub technique_name: String,
    /// Tactic (e.g., "Discovery")
    pub tactic: String,
}

/// A single audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Entry sequence number
    pub sequence: u64,
    /// Entry identifier
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Severity level
    pub level: AuditLevel,
    /// Event category
    pub category: AuditCategory,
    /// Event summary
    pub summary: String,
    /// Detailed event data (JSON)
    pub details: Option<String>,
    /// Target affected (if applicable)
    pub target: Option<String>,
    /// Operator identifier
    pub operator_id: String,
    /// MITRE ATT&CK reference (if applicable)
    pub mitre_ref: Option<MitreReference>,
    /// Previous entry hash (for chain verification)
    pub previous_hash: String,
    /// Entry hash (BLAKE3)
    pub entry_hash: String,
    /// Ed25519 signature (hex-encoded)
    pub signature: String,
    /// Signer public key (hex-encoded)
    pub signer_public_key: String,
}

impl AuditEntry {
    /// Compute the data to be hashed/signed
    pub fn signing_data(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(&self.sequence.to_le_bytes());
        data.extend(self.id.as_bytes());
        data.extend(self.timestamp.to_rfc3339().as_bytes());
        data.extend(&(self.level as u8).to_le_bytes());
        data.extend(&(self.category as u8).to_le_bytes());
        data.extend(self.summary.as_bytes());
        if let Some(ref details) = self.details {
            data.extend(details.as_bytes());
        }
        if let Some(ref target) = self.target {
            data.extend(target.as_bytes());
        }
        data.extend(self.operator_id.as_bytes());
        if let Some(ref mitre) = self.mitre_ref {
            data.extend(mitre.technique_id.as_bytes());
        }
        data.extend(self.previous_hash.as_bytes());
        data
    }

    /// Compute the entry hash
    pub fn compute_hash(&self) -> String {
        let data = self.signing_data();
        let hash = blake3::hash(&data);
        hex::encode(hash.as_bytes())
    }

    /// Verify the entry signature
    pub fn verify_signature(&self) -> Result<bool> {
        if self.signature.is_empty() || self.signer_public_key.is_empty() {
            return Ok(false);
        }

        let public_key_bytes = hex::decode(&self.signer_public_key)
            .map_err(|e| ReconError::AuditChainTampered(format!("Invalid public key: {}", e)))?;

        if public_key_bytes.len() != 32 {
            return Err(ReconError::AuditChainTampered(
                "Public key must be 32 bytes".to_string(),
            ));
        }

        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(&public_key_bytes);
        let verifying_key = VerifyingKey::from_bytes(&pk_array)?;

        let signature_bytes = hex::decode(&self.signature)
            .map_err(|e| ReconError::AuditChainTampered(format!("Invalid signature: {}", e)))?;

        if signature_bytes.len() != 64 {
            return Err(ReconError::AuditChainTampered(
                "Signature must be 64 bytes".to_string(),
            ));
        }

        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&signature_bytes);
        let signature = Signature::from_bytes(&sig_array);

        let signing_data = self.signing_data();
        match verifying_key.verify(&signing_data, &signature) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Verify the entry hash
    pub fn verify_hash(&self) -> bool {
        self.compute_hash() == self.entry_hash
    }
}

/// Audit chain verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerificationResult {
    /// Whether the entire chain is valid
    pub valid: bool,
    /// Total entries verified
    pub entries_verified: usize,
    /// First invalid entry sequence (if any)
    pub first_invalid_sequence: Option<u64>,
    /// Verification errors
    pub errors: Vec<String>,
}

/// Audit log manager
pub struct AuditManager {
    /// Signing key for entries
    signing_key: SigningKey,
    /// Verifying key
    verifying_key: VerifyingKey,
    /// Operator identifier
    operator_id: String,
    /// In-memory log buffer
    entries: parking_lot::Mutex<VecDeque<AuditEntry>>,
    /// Current sequence number
    sequence: std::sync::atomic::AtomicU64,
    /// Maximum in-memory entries
    max_entries: usize,
    /// Last entry hash
    last_hash: parking_lot::Mutex<String>,
}

impl AuditManager {
    /// Create a new audit manager
    pub fn new(operator_id: String) -> Self {
        let mut csprng = rand::rngs::OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        Self {
            signing_key,
            verifying_key,
            operator_id,
            entries: parking_lot::Mutex::new(VecDeque::new()),
            sequence: std::sync::atomic::AtomicU64::new(0),
            max_entries: 10000,
            last_hash: parking_lot::Mutex::new(String::from("genesis")),
        }
    }

    /// Create with a specific signing key
    pub fn with_key(operator_id: String, signing_key: SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
            operator_id,
            entries: parking_lot::Mutex::new(VecDeque::new()),
            sequence: std::sync::atomic::AtomicU64::new(0),
            max_entries: 10000,
            last_hash: parking_lot::Mutex::new(String::from("genesis")),
        }
    }

    /// Get the public key (hex-encoded)
    pub fn public_key(&self) -> String {
        hex::encode(self.verifying_key.to_bytes())
    }

    /// Log an event
    pub fn log(
        &self,
        level: AuditLevel,
        category: AuditCategory,
        summary: &str,
        details: Option<&str>,
        target: Option<&str>,
        mitre_ref: Option<MitreReference>,
    ) -> AuditEntry {
        let sequence = self
            .sequence
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let previous_hash = self.last_hash.lock().clone();

        let mut entry = AuditEntry {
            sequence,
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level,
            category,
            summary: summary.to_string(),
            details: details.map(String::from),
            target: target.map(String::from),
            operator_id: self.operator_id.clone(),
            mitre_ref,
            previous_hash,
            entry_hash: String::new(),
            signature: String::new(),
            signer_public_key: self.public_key(),
        };

        // Compute hash
        entry.entry_hash = entry.compute_hash();

        // Sign entry
        let signing_data = entry.signing_data();
        let signature = self.signing_key.sign(&signing_data);
        entry.signature = hex::encode(signature.to_bytes());

        // Update last hash
        *self.last_hash.lock() = entry.entry_hash.clone();

        // Store entry
        let mut entries = self.entries.lock();
        entries.push_back(entry.clone());

        // Trim if over limit
        while entries.len() > self.max_entries {
            entries.pop_front();
        }

        entry
    }

    /// Log an info event
    pub fn info(&self, category: AuditCategory, summary: &str) -> AuditEntry {
        self.log(AuditLevel::Info, category, summary, None, None, None)
    }

    /// Log a warning event
    pub fn warning(&self, category: AuditCategory, summary: &str) -> AuditEntry {
        self.log(AuditLevel::Warning, category, summary, None, None, None)
    }

    /// Log an error event
    pub fn error(&self, category: AuditCategory, summary: &str, details: &str) -> AuditEntry {
        self.log(
            AuditLevel::Error,
            category,
            summary,
            Some(details),
            None,
            None,
        )
    }

    /// Log a critical event
    pub fn critical(&self, category: AuditCategory, summary: &str, details: &str) -> AuditEntry {
        self.log(
            AuditLevel::Critical,
            category,
            summary,
            Some(details),
            None,
            None,
        )
    }

    /// Log an emergency event
    pub fn emergency(&self, category: AuditCategory, summary: &str, details: &str) -> AuditEntry {
        self.log(
            AuditLevel::Emergency,
            category,
            summary,
            Some(details),
            None,
            None,
        )
    }

    /// Log a reconnaissance operation
    pub fn log_recon(
        &self,
        summary: &str,
        target: &str,
        technique_id: &str,
        technique_name: &str,
        tactic: &str,
    ) -> AuditEntry {
        let mitre = MitreReference {
            technique_id: technique_id.to_string(),
            technique_name: technique_name.to_string(),
            tactic: tactic.to_string(),
        };
        self.log(
            AuditLevel::Info,
            AuditCategory::Reconnaissance,
            summary,
            None,
            Some(target),
            Some(mitre),
        )
    }

    /// Get entries since a timestamp
    pub fn entries_since(&self, since: DateTime<Utc>) -> Vec<AuditEntry> {
        let entries = self.entries.lock();
        entries
            .iter()
            .filter(|e| e.timestamp >= since)
            .cloned()
            .collect()
    }

    /// Get all entries
    pub fn all_entries(&self) -> Vec<AuditEntry> {
        self.entries.lock().iter().cloned().collect()
    }

    /// Get entries by category
    pub fn entries_by_category(&self, category: AuditCategory) -> Vec<AuditEntry> {
        let entries = self.entries.lock();
        entries
            .iter()
            .filter(|e| e.category == category)
            .cloned()
            .collect()
    }

    /// Verify the entire audit chain
    pub fn verify_chain(&self) -> ChainVerificationResult {
        let entries = self.entries.lock();
        let mut errors = Vec::new();
        let mut first_invalid = None;
        let mut expected_previous = "genesis".to_string();

        for entry in entries.iter() {
            // Verify hash
            if !entry.verify_hash() {
                errors.push(format!(
                    "Entry {} hash mismatch (sequence: {})",
                    entry.id, entry.sequence
                ));
                if first_invalid.is_none() {
                    first_invalid = Some(entry.sequence);
                }
            }

            // Verify chain link
            if entry.previous_hash != expected_previous {
                errors.push(format!(
                    "Entry {} chain broken (sequence: {})",
                    entry.id, entry.sequence
                ));
                if first_invalid.is_none() {
                    first_invalid = Some(entry.sequence);
                }
            }

            // Verify signature
            match entry.verify_signature() {
                Ok(true) => {}
                Ok(false) => {
                    errors.push(format!(
                        "Entry {} signature invalid (sequence: {})",
                        entry.id, entry.sequence
                    ));
                    if first_invalid.is_none() {
                        first_invalid = Some(entry.sequence);
                    }
                }
                Err(e) => {
                    errors.push(format!(
                        "Entry {} signature verification error: {} (sequence: {})",
                        entry.id, e, entry.sequence
                    ));
                    if first_invalid.is_none() {
                        first_invalid = Some(entry.sequence);
                    }
                }
            }

            expected_previous = entry.entry_hash.clone();
        }

        ChainVerificationResult {
            valid: errors.is_empty(),
            entries_verified: entries.len(),
            first_invalid_sequence: first_invalid,
            errors,
        }
    }

    /// Export entries to JSON
    pub fn export_json(&self) -> Result<String> {
        let entries = self.all_entries();
        serde_json::to_string_pretty(&entries).map_err(|e| ReconError::Internal(e.to_string()))
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        self.entries.lock().len()
    }
}

/// Thread-safe wrapper for sharing
pub type SharedAuditManager = Arc<AuditManager>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_manager_creation() {
        let manager = AuditManager::new("test-operator".to_string());
        assert_eq!(manager.entry_count(), 0);
    }

    #[test]
    fn test_log_entry() {
        let manager = AuditManager::new("test-operator".to_string());
        let entry = manager.info(AuditCategory::System, "System started");

        assert_eq!(entry.sequence, 0);
        assert_eq!(entry.summary, "System started");
        assert_eq!(entry.level, AuditLevel::Info);
        assert_eq!(entry.category, AuditCategory::System);
        assert!(!entry.entry_hash.is_empty());
        assert!(!entry.signature.is_empty());
    }

    #[test]
    fn test_entry_signature_verification() {
        let manager = AuditManager::new("test-operator".to_string());
        let entry = manager.info(AuditCategory::System, "Test entry");

        assert!(entry.verify_signature().unwrap());
    }

    #[test]
    fn test_entry_hash_verification() {
        let manager = AuditManager::new("test-operator".to_string());
        let entry = manager.info(AuditCategory::System, "Test entry");

        assert!(entry.verify_hash());
    }

    #[test]
    fn test_chain_integrity() {
        let manager = AuditManager::new("test-operator".to_string());

        // Create multiple entries
        manager.info(AuditCategory::System, "Entry 1");
        manager.info(AuditCategory::System, "Entry 2");
        manager.info(AuditCategory::System, "Entry 3");

        let result = manager.verify_chain();
        assert!(result.valid);
        assert_eq!(result.entries_verified, 3);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_tampered_entry_detection() {
        let manager = AuditManager::new("test-operator".to_string());
        manager.info(AuditCategory::System, "Entry 1");

        // Manually tamper with entry
        {
            let mut entries = manager.entries.lock();
            if let Some(entry) = entries.get_mut(0) {
                entry.summary = "Tampered summary".to_string();
            }
        }

        let result = manager.verify_chain();
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_recon_logging() {
        let manager = AuditManager::new("test-operator".to_string());
        let entry = manager.log_recon(
            "Port scan completed",
            "192.168.1.0/24",
            "T1046",
            "Network Service Discovery",
            "Discovery",
        );

        assert!(entry.mitre_ref.is_some());
        let mitre = entry.mitre_ref.unwrap();
        assert_eq!(mitre.technique_id, "T1046");
    }

    #[test]
    fn test_entries_since() {
        let manager = AuditManager::new("test-operator".to_string());
        let before = Utc::now();

        manager.info(AuditCategory::System, "Entry 1");
        manager.info(AuditCategory::System, "Entry 2");

        let entries = manager.entries_since(before);
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_entries_by_category() {
        let manager = AuditManager::new("test-operator".to_string());

        manager.info(AuditCategory::System, "System entry");
        manager.info(AuditCategory::Reconnaissance, "Recon entry");
        manager.info(AuditCategory::System, "Another system entry");

        let system_entries = manager.entries_by_category(AuditCategory::System);
        assert_eq!(system_entries.len(), 2);

        let recon_entries = manager.entries_by_category(AuditCategory::Reconnaissance);
        assert_eq!(recon_entries.len(), 1);
    }

    #[test]
    fn test_export_json() {
        let manager = AuditManager::new("test-operator".to_string());
        manager.info(AuditCategory::System, "Test entry");

        let json = manager.export_json().unwrap();
        assert!(json.contains("Test entry"));
        assert!(json.contains("sequence"));
    }

    #[test]
    fn test_different_log_levels() {
        let manager = AuditManager::new("test-operator".to_string());

        manager.info(AuditCategory::System, "Info");
        manager.warning(AuditCategory::System, "Warning");
        manager.error(AuditCategory::System, "Error", "details");
        manager.critical(AuditCategory::System, "Critical", "details");
        manager.emergency(AuditCategory::KillSwitch, "Emergency", "details");

        assert_eq!(manager.entry_count(), 5);

        let result = manager.verify_chain();
        assert!(result.valid);
    }
}
