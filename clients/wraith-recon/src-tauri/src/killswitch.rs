//! Kill Switch Module
//!
//! This module provides emergency shutdown capability through cryptographically
//! signed halt signals. When activated, ALL operations immediately cease.
//!
//! ## Security Requirements
//! - Kill switch signals must be signed by authorized operators
//! - Activation is immediate and cannot be undone without restart
//! - All active operations are terminated

use crate::error::{ReconError, Result};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Kill switch signal (signed halt command)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillSwitchSignal {
    /// Signal identifier
    pub id: String,
    /// Reason for activation
    pub reason: String,
    /// Operator who initiated the kill switch
    pub operator_id: String,
    /// Timestamp of signal creation
    pub timestamp: DateTime<Utc>,
    /// Signer public key (hex-encoded)
    pub signer_public_key: String,
    /// Signal signature (hex-encoded)
    pub signature: String,
}

impl KillSwitchSignal {
    /// Create a new kill switch signal (unsigned)
    pub fn new(reason: String, operator_id: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            reason,
            operator_id,
            timestamp: Utc::now(),
            signer_public_key: String::new(),
            signature: String::new(),
        }
    }

    /// Get the data to be signed
    pub fn signing_data(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(self.id.as_bytes());
        data.extend(self.reason.as_bytes());
        data.extend(self.operator_id.as_bytes());
        data.extend(self.timestamp.to_rfc3339().as_bytes());
        data.extend(self.signer_public_key.as_bytes());
        data
    }

    /// Verify the signal signature
    pub fn verify_signature(&self) -> Result<bool> {
        if self.signature.is_empty() || self.signer_public_key.is_empty() {
            return Ok(false);
        }

        let public_key_bytes = hex::decode(&self.signer_public_key).map_err(|e| {
            ReconError::InvalidKillSwitchSignal(format!("Invalid public key: {}", e))
        })?;

        if public_key_bytes.len() != 32 {
            return Err(ReconError::InvalidKillSwitchSignal(
                "Public key must be 32 bytes".to_string(),
            ));
        }

        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(&public_key_bytes);
        let verifying_key = VerifyingKey::from_bytes(&pk_array)?;

        let signature_bytes = hex::decode(&self.signature).map_err(|e| {
            ReconError::InvalidKillSwitchSignal(format!("Invalid signature: {}", e))
        })?;

        if signature_bytes.len() != 64 {
            return Err(ReconError::InvalidKillSwitchSignal(
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
}

/// Kill switch state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KillSwitchState {
    /// Whether the kill switch is activated
    pub activated: bool,
    /// Time of activation (if activated)
    pub activated_at: Option<DateTime<Utc>>,
    /// Activation reason (if activated)
    pub reason: Option<String>,
    /// Operator who activated (if activated)
    pub activated_by: Option<String>,
    /// Signal ID (if activated)
    pub signal_id: Option<String>,
}

/// Kill switch manager
pub struct KillSwitchManager {
    /// Whether the kill switch is activated
    activated: Arc<AtomicBool>,
    /// Activation details
    state: parking_lot::Mutex<KillSwitchState>,
    /// Authorized operator public keys (hex-encoded)
    authorized_operators: parking_lot::Mutex<Vec<String>>,
    /// Shutdown callbacks
    shutdown_callbacks: parking_lot::Mutex<Vec<Box<dyn Fn() + Send + Sync>>>,
}

impl KillSwitchManager {
    /// Create a new kill switch manager
    pub fn new() -> Self {
        Self {
            activated: Arc::new(AtomicBool::new(false)),
            state: parking_lot::Mutex::new(KillSwitchState::default()),
            authorized_operators: parking_lot::Mutex::new(Vec::new()),
            shutdown_callbacks: parking_lot::Mutex::new(Vec::new()),
        }
    }

    /// Add an authorized operator
    pub fn add_authorized_operator(&self, public_key_hex: String) {
        let mut ops = self.authorized_operators.lock();
        if !ops.contains(&public_key_hex) {
            ops.push(public_key_hex);
        }
    }

    /// Register a shutdown callback
    pub fn on_shutdown<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.shutdown_callbacks.lock().push(Box::new(callback));
    }

    /// Check if the kill switch is activated
    pub fn is_activated(&self) -> bool {
        self.activated.load(Ordering::SeqCst)
    }

    /// Get the current state
    pub fn state(&self) -> KillSwitchState {
        self.state.lock().clone()
    }

    /// Activate the kill switch with a signed signal
    pub fn activate(&self, signal: KillSwitchSignal) -> Result<()> {
        // Verify the signal signature
        if !signal.verify_signature()? {
            return Err(ReconError::InvalidKillSwitchSignal(
                "Signal signature verification failed".to_string(),
            ));
        }

        // Check if the signer is authorized
        let authorized = self.authorized_operators.lock();
        if !authorized.is_empty() && !authorized.contains(&signal.signer_public_key) {
            return Err(ReconError::InvalidKillSwitchSignal(
                "Signer is not an authorized operator".to_string(),
            ));
        }

        // Activate the kill switch
        self.activated.store(true, Ordering::SeqCst);

        // Update state
        {
            let mut state = self.state.lock();
            state.activated = true;
            state.activated_at = Some(Utc::now());
            state.reason = Some(signal.reason.clone());
            state.activated_by = Some(signal.operator_id.clone());
            state.signal_id = Some(signal.id.clone());
        }

        // Execute shutdown callbacks
        let callbacks = self.shutdown_callbacks.lock();
        for callback in callbacks.iter() {
            callback();
        }

        tracing::warn!(
            "KILL SWITCH ACTIVATED by {} ({}): {}",
            signal.operator_id,
            signal.id,
            signal.reason
        );

        Ok(())
    }

    /// Activate the kill switch manually (for emergency use)
    /// This bypasses signature verification
    pub fn activate_manual(&self, reason: &str, operator: &str) {
        self.activated.store(true, Ordering::SeqCst);

        {
            let mut state = self.state.lock();
            state.activated = true;
            state.activated_at = Some(Utc::now());
            state.reason = Some(reason.to_string());
            state.activated_by = Some(operator.to_string());
            state.signal_id = Some(format!("manual-{}", uuid::Uuid::new_v4()));
        }

        // Execute shutdown callbacks
        let callbacks = self.shutdown_callbacks.lock();
        for callback in callbacks.iter() {
            callback();
        }

        tracing::warn!("KILL SWITCH ACTIVATED MANUALLY by {}: {}", operator, reason);
    }

    /// Validate that operations can proceed (kill switch not activated)
    pub fn validate(&self) -> Result<()> {
        if self.is_activated() {
            let state = self.state.lock();
            Err(ReconError::KillSwitchActivated(
                state
                    .reason
                    .clone()
                    .unwrap_or_else(|| "Unknown reason".to_string()),
            ))
        } else {
            Ok(())
        }
    }

    /// Get the atomic flag for sharing
    pub fn activated_flag(&self) -> Arc<AtomicBool> {
        self.activated.clone()
    }
}

impl Default for KillSwitchManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Kill switch guard for operation validation
pub struct KillSwitchGuard<'a> {
    manager: &'a KillSwitchManager,
}

impl<'a> KillSwitchGuard<'a> {
    pub fn new(manager: &'a KillSwitchManager) -> Self {
        Self { manager }
    }

    /// Check if operations can proceed
    pub fn check(&self) -> Result<()> {
        self.manager.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use rand::rngs::OsRng;

    fn create_signed_signal(reason: &str, operator: &str) -> (KillSwitchSignal, String) {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let public_key_hex = hex::encode(verifying_key.to_bytes());

        let mut signal = KillSwitchSignal::new(reason.to_string(), operator.to_string());
        signal.signer_public_key = public_key_hex.clone();

        let signing_data = signal.signing_data();
        let signature = signing_key.sign(&signing_data);
        signal.signature = hex::encode(signature.to_bytes());

        (signal, public_key_hex)
    }

    #[test]
    fn test_kill_switch_signal_creation() {
        let signal = KillSwitchSignal::new("Emergency".to_string(), "operator-001".to_string());
        assert_eq!(signal.reason, "Emergency");
        assert_eq!(signal.operator_id, "operator-001");
    }

    #[test]
    fn test_signal_signature_verification() {
        let (signal, _) = create_signed_signal("Test reason", "test-operator");
        assert!(signal.verify_signature().unwrap());
    }

    #[test]
    fn test_tampered_signal() {
        let (mut signal, _) = create_signed_signal("Original reason", "test-operator");
        signal.reason = "Modified reason".to_string();
        assert!(!signal.verify_signature().unwrap());
    }

    #[test]
    fn test_kill_switch_manager_creation() {
        let manager = KillSwitchManager::new();
        assert!(!manager.is_activated());
    }

    #[test]
    fn test_kill_switch_activation() {
        let manager = KillSwitchManager::new();
        let (signal, _) = create_signed_signal("Test halt", "admin");

        manager.activate(signal).unwrap();
        assert!(manager.is_activated());

        let state = manager.state();
        assert!(state.activated);
        assert_eq!(state.reason, Some("Test halt".to_string()));
        assert_eq!(state.activated_by, Some("admin".to_string()));
    }

    #[test]
    fn test_kill_switch_with_authorized_operators() {
        let manager = KillSwitchManager::new();
        let (signal, public_key) = create_signed_signal("Authorized halt", "admin");

        // Add the operator as authorized
        manager.add_authorized_operator(public_key);

        manager.activate(signal).unwrap();
        assert!(manager.is_activated());
    }

    #[test]
    fn test_kill_switch_unauthorized_operator() {
        let manager = KillSwitchManager::new();
        let (signal, _) = create_signed_signal("Unauthorized halt", "hacker");

        // Add a different operator as authorized
        manager.add_authorized_operator("different_key".to_string());

        let result = manager.activate(signal);
        assert!(result.is_err());
        assert!(!manager.is_activated());
    }

    #[test]
    fn test_kill_switch_manual_activation() {
        let manager = KillSwitchManager::new();
        manager.activate_manual("Emergency", "admin");

        assert!(manager.is_activated());
        let state = manager.state();
        assert_eq!(state.reason, Some("Emergency".to_string()));
    }

    #[test]
    fn test_kill_switch_validate() {
        let manager = KillSwitchManager::new();

        // Should be OK when not activated
        assert!(manager.validate().is_ok());

        // Activate
        manager.activate_manual("Test", "admin");

        // Should fail when activated
        assert!(manager.validate().is_err());
    }

    #[test]
    fn test_kill_switch_guard() {
        let manager = KillSwitchManager::new();
        let guard = KillSwitchGuard::new(&manager);

        assert!(guard.check().is_ok());

        manager.activate_manual("Test", "admin");

        assert!(guard.check().is_err());
    }

    #[test]
    fn test_shutdown_callback() {
        use std::sync::atomic::AtomicBool;

        let manager = KillSwitchManager::new();
        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_called_clone = callback_called.clone();

        manager.on_shutdown(move || {
            callback_called_clone.store(true, Ordering::SeqCst);
        });

        assert!(!callback_called.load(Ordering::SeqCst));

        manager.activate_manual("Test", "admin");

        assert!(callback_called.load(Ordering::SeqCst));
    }
}
