//! Application State Management
//!
//! Manages shared state across Tauri commands for WRAITH Share.

use crate::database::{Database, LocalIdentity};
use crate::error::ShareResult;
use ed25519_dalek::{SigningKey, VerifyingKey};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519Secret};

/// Application state shared across all Tauri commands
pub struct AppState {
    /// Database connection
    pub db: Arc<Database>,
    /// Application data directory
    pub app_data_dir: PathBuf,
    /// File storage directory
    pub files_dir: PathBuf,
    /// Version storage directory
    pub versions_dir: PathBuf,
    /// Local identity (signing key pair)
    pub signing_key: Arc<RwLock<Option<SigningKey>>>,
    /// Local X25519 secret for encryption
    pub encryption_secret: Arc<RwLock<Option<X25519Secret>>>,
    /// Local peer ID
    pub local_peer_id: Arc<RwLock<Option<String>>>,
    /// Display name
    pub display_name: Arc<RwLock<String>>,
    /// Maximum versions to keep per file
    pub max_versions: i64,
    /// Maximum activity log events per group
    pub max_activity_events: i64,
}

impl AppState {
    /// Create new application state
    pub fn new(db: Database, app_data_dir: PathBuf) -> Self {
        let files_dir = app_data_dir.join("files");
        let versions_dir = app_data_dir.join("versions");

        Self {
            db: Arc::new(db),
            app_data_dir,
            files_dir,
            versions_dir,
            signing_key: Arc::new(RwLock::new(None)),
            encryption_secret: Arc::new(RwLock::new(None)),
            local_peer_id: Arc::new(RwLock::new(None)),
            display_name: Arc::new(RwLock::new("Anonymous".to_string())),
            max_versions: 10,
            max_activity_events: 1000,
        }
    }

    /// Initialize the application state (load or create identity)
    pub fn initialize(&self) -> ShareResult<()> {
        // Create directories
        std::fs::create_dir_all(&self.files_dir)?;
        std::fs::create_dir_all(&self.versions_dir)?;

        // Load or create identity
        if let Ok(Some(identity)) = self.db.get_local_identity() {
            self.load_identity(&identity)?;
            info!("Loaded existing identity: {}", identity.peer_id);
        } else {
            let identity = self.create_identity("Anonymous")?;
            info!("Created new identity: {}", identity.peer_id);
        }

        Ok(())
    }

    /// Load identity from database
    fn load_identity(&self, identity: &LocalIdentity) -> ShareResult<()> {
        // Load signing key
        let signing_key_bytes: [u8; 32] = identity.private_key[..32].try_into().map_err(|_| {
            crate::error::ShareError::Crypto("Invalid private key length".to_string())
        })?;
        let signing_key = SigningKey::from_bytes(&signing_key_bytes);

        // Load X25519 secret (derived from signing key for simplicity)
        let x25519_secret = X25519Secret::from(signing_key_bytes);

        *self.signing_key.write() = Some(signing_key);
        *self.encryption_secret.write() = Some(x25519_secret);
        *self.local_peer_id.write() = Some(identity.peer_id.clone());
        *self.display_name.write() = identity.display_name.clone();

        Ok(())
    }

    /// Create a new identity
    fn create_identity(&self, display_name: &str) -> ShareResult<LocalIdentity> {
        use rand::rngs::OsRng;

        // Generate Ed25519 signing key
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        // Derive X25519 from signing key
        let signing_bytes = signing_key.to_bytes();
        let x25519_secret = X25519Secret::from(signing_bytes);
        let x25519_public = X25519PublicKey::from(&x25519_secret);

        // Generate peer ID from public key hash
        let peer_id = hex::encode(&blake3::hash(verifying_key.as_bytes()).as_bytes()[..16]);

        // Combine public keys (Ed25519 + X25519)
        let mut public_key = Vec::with_capacity(64);
        public_key.extend_from_slice(verifying_key.as_bytes());
        public_key.extend_from_slice(x25519_public.as_bytes());

        // Store private key (Ed25519 only, X25519 is derived)
        let private_key = signing_key.to_bytes().to_vec();

        let identity = LocalIdentity {
            peer_id: peer_id.clone(),
            display_name: display_name.to_string(),
            public_key,
            private_key,
            created_at: chrono::Utc::now().timestamp(),
        };

        self.db.save_local_identity(&identity)?;

        *self.signing_key.write() = Some(signing_key);
        *self.encryption_secret.write() = Some(x25519_secret);
        *self.local_peer_id.write() = Some(peer_id);
        *self.display_name.write() = display_name.to_string();

        Ok(identity)
    }

    /// Get the local peer ID
    pub fn get_peer_id(&self) -> Option<String> {
        self.local_peer_id.read().clone()
    }

    /// Get the display name
    pub fn get_display_name(&self) -> String {
        self.display_name.read().clone()
    }

    /// Update the display name
    pub fn set_display_name(&self, name: &str) -> ShareResult<()> {
        if let Some(mut identity) = self.db.get_local_identity()? {
            identity.display_name = name.to_string();
            self.db.save_local_identity(&identity)?;
            *self.display_name.write() = name.to_string();
        }
        Ok(())
    }

    /// Get the signing key
    pub fn get_signing_key(&self) -> Option<SigningKey> {
        self.signing_key.read().clone()
    }

    /// Get the verifying key (public key for signatures)
    pub fn get_verifying_key(&self) -> Option<VerifyingKey> {
        self.signing_key
            .read()
            .as_ref()
            .map(|sk| sk.verifying_key())
    }

    /// Get the X25519 encryption secret
    pub fn get_encryption_secret(&self) -> Option<X25519Secret> {
        self.encryption_secret.read().clone()
    }

    /// Get the X25519 public key
    pub fn get_encryption_public_key(&self) -> Option<X25519PublicKey> {
        self.encryption_secret
            .read()
            .as_ref()
            .map(X25519PublicKey::from)
    }

    /// Get file storage path for a file
    pub fn get_file_storage_path(&self, file_id: &str) -> PathBuf {
        self.files_dir.join(file_id)
    }

    /// Get version storage path for a file version
    pub fn get_version_storage_path(&self, file_id: &str, version: i64) -> PathBuf {
        self.versions_dir.join(format!("{}_v{}", file_id, version))
    }

    /// Get combined public key bytes (Ed25519 + X25519)
    pub fn get_public_key_bytes(&self) -> Option<Vec<u8>> {
        let verifying_key = self.get_verifying_key()?;
        let x25519_public = self.get_encryption_public_key()?;

        let mut combined = Vec::with_capacity(64);
        combined.extend_from_slice(verifying_key.as_bytes());
        combined.extend_from_slice(x25519_public.as_bytes());
        Some(combined)
    }

    /// Parse combined public key bytes into Ed25519 and X25519 components
    pub fn parse_public_key_bytes(bytes: &[u8]) -> ShareResult<(VerifyingKey, X25519PublicKey)> {
        if bytes.len() != 64 {
            return Err(crate::error::ShareError::Crypto(
                "Invalid public key length".to_string(),
            ));
        }

        let ed25519_bytes: [u8; 32] = bytes[..32]
            .try_into()
            .map_err(|_| crate::error::ShareError::Crypto("Invalid Ed25519 key".to_string()))?;

        let x25519_bytes: [u8; 32] = bytes[32..]
            .try_into()
            .map_err(|_| crate::error::ShareError::Crypto("Invalid X25519 key".to_string()))?;

        let verifying_key = VerifyingKey::from_bytes(&ed25519_bytes)
            .map_err(|e| crate::error::ShareError::Crypto(format!("Invalid Ed25519 key: {}", e)))?;

        let x25519_public = X25519PublicKey::from(x25519_bytes);

        Ok((verifying_key, x25519_public))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_state_initialization() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let state = AppState::new(db, dir.path().to_path_buf());

        state.initialize().unwrap();

        assert!(state.get_peer_id().is_some());
        assert!(state.get_signing_key().is_some());
        assert!(state.get_encryption_secret().is_some());
    }

    #[test]
    fn test_identity_persistence() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // Create identity
        {
            let db = Database::open(&db_path).unwrap();
            let state = AppState::new(db, dir.path().to_path_buf());
            state.initialize().unwrap();
            state.set_display_name("Test User").unwrap();
        }

        // Load identity
        {
            let db = Database::open(&db_path).unwrap();
            let state = AppState::new(db, dir.path().to_path_buf());
            state.initialize().unwrap();

            assert_eq!(state.get_display_name(), "Test User");
        }
    }

    #[test]
    fn test_public_key_parsing() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let state = AppState::new(db, dir.path().to_path_buf());

        state.initialize().unwrap();

        let public_key_bytes = state.get_public_key_bytes().unwrap();
        assert_eq!(public_key_bytes.len(), 64);

        let (verifying_key, x25519_public) =
            AppState::parse_public_key_bytes(&public_key_bytes).unwrap();

        assert_eq!(
            verifying_key.as_bytes(),
            state.get_verifying_key().unwrap().as_bytes()
        );
        assert_eq!(
            x25519_public.as_bytes(),
            state.get_encryption_public_key().unwrap().as_bytes()
        );
    }
}
