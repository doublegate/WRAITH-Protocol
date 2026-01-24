//! Rules of Engagement (RoE) Module
//!
//! This module handles the loading, validation, and enforcement of Rules of Engagement
//! documents. RoE documents must be cryptographically signed to be valid.
//!
//! ## Security Requirements
//! - All RoE documents must be signed with Ed25519
//! - Signature verification is mandatory before any operation
//! - RoE contains authorized targets, time windows, and operator credentials

use crate::error::{ReconError, Result};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Rules of Engagement document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesOfEngagement {
    /// Unique document identifier
    pub id: String,
    /// Document version
    pub version: String,
    /// Organization name
    pub organization: String,
    /// Engagement title
    pub title: String,
    /// Engagement description
    pub description: String,
    /// Authorized operator identifiers (public key hashes)
    pub authorized_operators: Vec<String>,
    /// Client/target organization name
    pub client_name: String,
    /// Engagement start time (RFC 3339)
    pub start_time: DateTime<Utc>,
    /// Engagement end time (RFC 3339)
    pub end_time: DateTime<Utc>,
    /// Authorized target CIDRs
    pub authorized_cidrs: Vec<String>,
    /// Authorized target domains
    pub authorized_domains: Vec<String>,
    /// Excluded targets (always off-limits)
    pub excluded_targets: Vec<String>,
    /// Authorized techniques (MITRE ATT&CK IDs)
    pub authorized_techniques: Vec<String>,
    /// Prohibited techniques (never allowed)
    pub prohibited_techniques: Vec<String>,
    /// Maximum data exfiltration rate (bytes/second)
    pub max_exfil_rate: Option<u64>,
    /// Maximum total data exfiltration (bytes)
    pub max_exfil_total: Option<u64>,
    /// Emergency contact information
    pub emergency_contacts: Vec<EmergencyContact>,
    /// Additional constraints
    pub constraints: Vec<String>,
    /// Document creation timestamp
    pub created_at: DateTime<Utc>,
    /// Document signer public key (hex-encoded)
    pub signer_public_key: String,
    /// Document signature (hex-encoded)
    pub signature: String,
}

/// Emergency contact information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyContact {
    pub name: String,
    pub role: String,
    pub phone: String,
    pub email: String,
}

/// Result of RoE validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub signature_valid: bool,
    pub time_window_active: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl RulesOfEngagement {
    /// Create a new RoE document (for testing purposes)
    pub fn new(
        id: String,
        organization: String,
        title: String,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            version: "1.0".to_string(),
            organization,
            title,
            description: String::new(),
            authorized_operators: Vec::new(),
            client_name: String::new(),
            start_time,
            end_time,
            authorized_cidrs: Vec::new(),
            authorized_domains: Vec::new(),
            excluded_targets: Vec::new(),
            authorized_techniques: Vec::new(),
            prohibited_techniques: Vec::new(),
            max_exfil_rate: None,
            max_exfil_total: None,
            emergency_contacts: Vec::new(),
            constraints: Vec::new(),
            created_at: Utc::now(),
            signer_public_key: String::new(),
            signature: String::new(),
        }
    }

    /// Get the data to be signed (all fields except signature)
    pub fn signing_data(&self) -> Vec<u8> {
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

    /// Verify the document signature
    pub fn verify_signature(&self) -> Result<bool> {
        if self.signature.is_empty() || self.signer_public_key.is_empty() {
            return Ok(false);
        }

        let public_key_bytes = hex::decode(&self.signer_public_key).map_err(|e| {
            ReconError::RoESignatureInvalid(format!("Invalid public key hex: {}", e))
        })?;

        if public_key_bytes.len() != 32 {
            return Err(ReconError::RoESignatureInvalid(
                "Public key must be 32 bytes".to_string(),
            ));
        }

        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(&public_key_bytes);
        let verifying_key = VerifyingKey::from_bytes(&pk_array)?;

        let signature_bytes = hex::decode(&self.signature).map_err(|e| {
            ReconError::RoESignatureInvalid(format!("Invalid signature hex: {}", e))
        })?;

        if signature_bytes.len() != 64 {
            return Err(ReconError::RoESignatureInvalid(
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

    /// Check if the engagement window is currently active
    pub fn is_window_active(&self) -> bool {
        let now = Utc::now();
        now >= self.start_time && now <= self.end_time
    }

    /// Get time remaining in the engagement window (in seconds)
    pub fn time_remaining(&self) -> Option<i64> {
        let now = Utc::now();
        if now > self.end_time {
            None
        } else if now < self.start_time {
            Some((self.end_time - self.start_time).num_seconds())
        } else {
            Some((self.end_time - now).num_seconds())
        }
    }

    /// Validate the RoE document
    pub fn validate(&self) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check required fields
        if self.id.is_empty() {
            errors.push("Document ID is required".to_string());
        }
        if self.organization.is_empty() {
            errors.push("Organization is required".to_string());
        }
        if self.title.is_empty() {
            errors.push("Engagement title is required".to_string());
        }
        if self.authorized_operators.is_empty() {
            errors.push("At least one authorized operator is required".to_string());
        }
        if self.authorized_cidrs.is_empty() && self.authorized_domains.is_empty() {
            errors.push("At least one authorized target (CIDR or domain) is required".to_string());
        }
        if self.emergency_contacts.is_empty() {
            warnings.push("No emergency contacts defined".to_string());
        }

        // Check time window
        if self.end_time <= self.start_time {
            errors.push("End time must be after start time".to_string());
        }

        // Check signature
        let signature_valid = match self.verify_signature() {
            Ok(valid) => valid,
            Err(e) => {
                errors.push(format!("Signature verification error: {}", e));
                false
            }
        };

        if !signature_valid {
            errors.push("Document signature is invalid".to_string());
        }

        // Check time window status
        let time_window_active = self.is_window_active();
        if !time_window_active {
            warnings.push("Engagement window is not currently active".to_string());
        }

        ValidationResult {
            valid: errors.is_empty() && signature_valid,
            signature_valid,
            time_window_active,
            errors,
            warnings,
        }
    }

    /// Get authorized techniques as a HashSet for efficient lookup
    pub fn authorized_techniques_set(&self) -> HashSet<String> {
        self.authorized_techniques.iter().cloned().collect()
    }

    /// Get prohibited techniques as a HashSet for efficient lookup
    pub fn prohibited_techniques_set(&self) -> HashSet<String> {
        self.prohibited_techniques.iter().cloned().collect()
    }

    /// Check if a technique is authorized
    pub fn is_technique_authorized(&self, technique_id: &str) -> bool {
        let prohibited = self.prohibited_techniques_set();
        if prohibited.contains(technique_id) {
            return false;
        }

        // If authorized_techniques is empty, all non-prohibited techniques are allowed
        if self.authorized_techniques.is_empty() {
            return true;
        }

        self.authorized_techniques_set().contains(technique_id)
    }
}

/// RoE Manager for loading and managing RoE documents
pub struct RoEManager {
    /// Currently active RoE
    active_roe: Option<RulesOfEngagement>,
    /// Trusted signer public keys (hex-encoded)
    trusted_signers: Vec<String>,
}

impl RoEManager {
    /// Create a new RoE manager
    pub fn new() -> Self {
        Self {
            active_roe: None,
            trusted_signers: Vec::new(),
        }
    }

    /// Add a trusted signer public key
    pub fn add_trusted_signer(&mut self, public_key_hex: String) {
        if !self.trusted_signers.contains(&public_key_hex) {
            self.trusted_signers.push(public_key_hex);
        }
    }

    /// Load an RoE document
    pub fn load(&mut self, roe: RulesOfEngagement) -> Result<ValidationResult> {
        let validation = roe.validate();

        if !validation.valid {
            return Ok(validation);
        }

        // Check if signer is trusted
        if !self.trusted_signers.is_empty()
            && !self.trusted_signers.contains(&roe.signer_public_key)
        {
            return Err(ReconError::RoESignatureInvalid(
                "Signer is not in the trusted signers list".to_string(),
            ));
        }

        self.active_roe = Some(roe);
        Ok(validation)
    }

    /// Get the active RoE
    pub fn active(&self) -> Option<&RulesOfEngagement> {
        self.active_roe.as_ref()
    }

    /// Clear the active RoE
    pub fn clear(&mut self) {
        self.active_roe = None;
    }

    /// Check if an RoE is loaded and valid
    pub fn is_active(&self) -> bool {
        self.active_roe
            .as_ref()
            .is_some_and(|roe| roe.is_window_active())
    }
}

impl Default for RoEManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    fn create_test_roe() -> RulesOfEngagement {
        let now = Utc::now();
        let mut roe = RulesOfEngagement::new(
            "test-roe-001".to_string(),
            "Test Security Org".to_string(),
            "Penetration Test Engagement".to_string(),
            now - Duration::hours(1),
            now + Duration::hours(23),
        );
        roe.authorized_operators.push("operator-001".to_string());
        roe.authorized_cidrs.push("192.168.1.0/24".to_string());
        roe.authorized_domains.push("example.com".to_string());
        roe.authorized_techniques.push("T1046".to_string());
        roe.authorized_techniques.push("T1040".to_string());
        roe
    }

    fn sign_roe(roe: &mut RulesOfEngagement) {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        roe.signer_public_key = hex::encode(verifying_key.to_bytes());

        let signing_data = roe.signing_data();
        let signature = signing_key.sign(&signing_data);
        roe.signature = hex::encode(signature.to_bytes());
    }

    #[test]
    fn test_roe_creation() {
        let roe = create_test_roe();
        assert_eq!(roe.id, "test-roe-001");
        assert_eq!(roe.organization, "Test Security Org");
        assert!(roe.is_window_active());
    }

    #[test]
    fn test_roe_signature_verification() {
        let mut roe = create_test_roe();
        sign_roe(&mut roe);

        let result = roe.verify_signature().unwrap();
        assert!(result);
    }

    #[test]
    fn test_roe_validation() {
        let mut roe = create_test_roe();
        sign_roe(&mut roe);

        let validation = roe.validate();
        assert!(validation.valid);
        assert!(validation.signature_valid);
        assert!(validation.time_window_active);
        assert!(validation.errors.is_empty());
    }

    #[test]
    fn test_roe_invalid_signature() {
        let mut roe = create_test_roe();
        sign_roe(&mut roe);

        // Tamper with the document
        roe.title = "Modified Title".to_string();

        let result = roe.verify_signature().unwrap();
        assert!(!result);
    }

    #[test]
    fn test_technique_authorization() {
        let roe = create_test_roe();
        assert!(roe.is_technique_authorized("T1046"));
        assert!(roe.is_technique_authorized("T1040"));
        assert!(!roe.is_technique_authorized("T9999")); // Not in list
    }

    #[test]
    fn test_prohibited_techniques() {
        let mut roe = create_test_roe();
        roe.prohibited_techniques.push("T1486".to_string()); // Ransomware

        assert!(!roe.is_technique_authorized("T1486"));
    }

    #[test]
    fn test_roe_manager() {
        let mut manager = RoEManager::new();
        let mut roe = create_test_roe();
        sign_roe(&mut roe);

        let validation = manager.load(roe).unwrap();
        assert!(validation.valid);
        assert!(manager.is_active());
        assert!(manager.active().is_some());

        manager.clear();
        assert!(!manager.is_active());
    }

    #[test]
    fn test_time_remaining() {
        let now = Utc::now();
        let roe = RulesOfEngagement::new(
            "test".to_string(),
            "org".to_string(),
            "title".to_string(),
            now - Duration::hours(1),
            now + Duration::hours(1),
        );

        let remaining = roe.time_remaining().unwrap();
        assert!(remaining > 0);
        assert!(remaining <= 3600);
    }
}
