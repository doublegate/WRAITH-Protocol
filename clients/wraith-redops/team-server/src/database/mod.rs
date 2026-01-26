use crate::models::listener::Listener;
use crate::models::{Artifact, Campaign, Command, CommandResult, Credential, Implant};
use anyhow::Result;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;
use sqlx::{PgPool, Row};
use std::env;
use uuid::Uuid;

pub struct Database {
    pool: PgPool,
    hmac_key: Vec<u8>,
    master_key: [u8; 32],
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        let hmac_key = env::var("HMAC_SECRET")
            .expect("HMAC_SECRET environment variable must be set")
            .into_bytes();

        let master_key_str = env::var("MASTER_KEY")
            .expect("MASTER_KEY environment variable must be set (64 hex chars)");
        
        let mut master_key = [0u8; 32];
        let decoded = hex::decode(&master_key_str).expect("Failed to decode MASTER_KEY hex");
        if decoded.len() != 32 {
            panic!("MASTER_KEY must be exactly 32 bytes (64 hex chars)");
        }
        master_key.copy_from_slice(&decoded);

        Self {
            pool,
            hmac_key,
            master_key,
        }
    }

    /// Encrypts data using XChaCha20Poly1305 and the Master Key.
    /// Returns: [Nonce (24 bytes)] + [Ciphertext]
    fn encrypt_data(&self, plaintext: &[u8]) -> Result<Vec<u8>> {
        let key = Key::from_slice(&self.master_key);
        let cipher = XChaCha20Poly1305::new(key);

        let mut nonce_bytes = [0u8; 24];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = XNonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("Encryption error: {}", e))?;

        let mut result = Vec::with_capacity(24 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    /// Decrypts data using XChaCha20Poly1305 and the Master Key.
    /// Expects: [Nonce (24 bytes)] + [Ciphertext]
    fn decrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        if data.len() < 24 {
            return Err(anyhow::anyhow!("Data too short for decryption"));
        }

        let key = Key::from_slice(&self.master_key);
        let cipher = XChaCha20Poly1305::new(key);

        let nonce = XNonce::from_slice(&data[..24]);
        let ciphertext = &data[24..];

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption error: {}", e))?;

        Ok(plaintext)
    }

    /// Get a reference to the underlying database connection pool.
    #[allow(dead_code)]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // --- Campaign Operations ---
    pub async fn create_campaign(&self, name: &str, description: &str) -> Result<Campaign> {
        let rec = sqlx::query_as::<_, Campaign>(
            "INSERT INTO campaigns (name, description, status) VALUES ($1, $2, 'active') RETURNING id, name, description, status, start_date, end_date, created_at"
        )
        .bind(name)
        .bind(description)
        .fetch_one(&self.pool)
        .await?;

        Ok(rec)
    }

    pub async fn list_campaigns(&self) -> Result<Vec<Campaign>> {
        let recs = sqlx::query_as::<_, Campaign>(
            "SELECT id, name, description, status, start_date, end_date, created_at FROM campaigns ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(recs)
    }

    pub async fn get_campaign(&self, id: Uuid) -> Result<Campaign> {
        let rec = sqlx::query_as::<_, Campaign>(
            "SELECT id, name, description, status, start_date, end_date, created_at FROM campaigns WHERE id = $1"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(rec)
    }

    pub async fn update_campaign(
        &self,
        id: Uuid,
        name: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
    ) -> Result<Campaign> {
        let mut query_builder = sqlx::QueryBuilder::new("UPDATE campaigns SET ");
        let mut separated = query_builder.separated(", ");

        if let Some(n) = name {
            separated.push("name = ");
            separated.push_bind_unseparated(n);
        }
        if let Some(d) = description {
            separated.push("description = ");
            separated.push_bind_unseparated(d);
        }
        if let Some(s) = status {
            separated.push("status = ");
            separated.push_bind_unseparated(s);
        }

        query_builder.push(" WHERE id = ");
        query_builder.push_bind(id);
        query_builder
            .push(" RETURNING id, name, description, status, start_date, end_date, created_at");

        let rec = query_builder
            .build_query_as::<Campaign>()
            .fetch_one(&self.pool)
            .await?;

        Ok(rec)
    }

    // --- Implant Operations ---
    pub async fn register_implant(&self, implant: &Implant) -> Result<Uuid> {
        let row = sqlx::query(
            "INSERT INTO implants (campaign_id, hostname, internal_ip, external_ip, os_type, os_version, architecture, username, domain, privileges, implant_version, status) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'active') RETURNING id"
        )
        .bind(implant.campaign_id)
        .bind(&implant.hostname)
        .bind(implant.internal_ip)
        .bind(implant.external_ip)
        .bind(&implant.os_type)
        .bind(&implant.os_version)
        .bind(&implant.architecture)
        .bind(&implant.username)
        .bind(&implant.domain)
        .bind(&implant.privileges)
        .bind(&implant.implant_version)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("id")?)
    }

    pub async fn update_implant_checkin(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE implants SET last_checkin = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_implants(&self) -> Result<Vec<Implant>> {
        let recs = sqlx::query_as::<_, Implant>(
            "SELECT * FROM implants ORDER BY last_checkin DESC NULLS LAST",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(recs)
    }

    pub async fn get_implant(&self, id: Uuid) -> Result<Implant> {
        let rec = sqlx::query_as::<_, Implant>("SELECT * FROM implants WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(rec)
    }

    pub async fn kill_implant(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE implants SET status = 'killed' WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Listener Operations ---
    pub async fn create_listener(
        &self,
        name: &str,
        l_type: &str,
        bind_addr: &str,
        config: serde_json::Value,
    ) -> Result<Listener> {
        let rec = sqlx::query_as::<_, Listener>(
            "INSERT INTO listeners (name, type, bind_address, config, status) VALUES ($1, $2, $3::inet, $4, 'active') RETURNING id, name, type, bind_address::text, config, status"
        )
        .bind(name)
        .bind(l_type)
        .bind(bind_addr)
        .bind(config)
        .fetch_one(&self.pool)
        .await?;
        Ok(rec)
    }

    pub async fn list_listeners(&self) -> Result<Vec<Listener>> {
        let recs = sqlx::query_as::<_, Listener>(
            "SELECT id, name, type, bind_address::text, config, status FROM listeners ORDER BY name"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(recs)
    }

    pub async fn get_listener(&self, id: Uuid) -> Result<Option<Listener>> {
        let rec = sqlx::query_as::<_, Listener>(
            "SELECT id, name, type, bind_address::text, config, status FROM listeners WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(rec)
    }

    pub async fn update_listener_status(&self, id: Uuid, status: &str) -> Result<()> {
        sqlx::query("UPDATE listeners SET status = $1 WHERE id = $2")
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Command Operations ---
    pub async fn queue_command(
        &self,
        implant_id: Uuid,
        cmd_type: &str,
        payload: &[u8],
    ) -> Result<Uuid> {
        // ENCRYPT PAYLOAD AT REST
        let encrypted_payload = self.encrypt_data(payload)?;

        let row = sqlx::query(
            "INSERT INTO commands (implant_id, command_type, payload, status) VALUES ($1, $2, $3, 'pending') RETURNING id"
        )
        .bind(implant_id)
        .bind(cmd_type)
        .bind(encrypted_payload)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("id")?)
    }

    pub async fn get_pending_commands(&self, implant_id: Uuid) -> Result<Vec<Command>> {
        let mut recs = sqlx::query_as::<_, Command>(
            "UPDATE commands SET status = 'sent', sent_at = NOW() WHERE id IN (SELECT id FROM commands WHERE implant_id = $1 AND status = 'pending' ORDER BY priority ASC, created_at ASC FOR UPDATE SKIP LOCKED) RETURNING *"
        )
        .bind(implant_id)
        .fetch_all(&self.pool)
        .await?;

        // DECRYPT PAYLOADS
        for cmd in &mut recs {
            if let Some(payload) = &cmd.payload {
                let plaintext = self.decrypt_data(payload)?;
                cmd.payload = Some(plaintext);
            }
        }

        Ok(recs)
    }

    pub async fn update_command_result(&self, command_id: Uuid, output: &[u8]) -> Result<()> {
        // ENCRYPT RESULT AT REST
        let encrypted_output = self.encrypt_data(output)?;

        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE commands SET status = 'completed', completed_at = NOW() WHERE id = $1")
            .bind(command_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("INSERT INTO command_results (command_id, output) VALUES ($1, $2)")
            .bind(command_id)
            .bind(encrypted_output)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn list_commands(&self, implant_id: Uuid) -> Result<Vec<Command>> {
        let mut recs = sqlx::query_as::<_, Command>(
            "SELECT * FROM commands WHERE implant_id = $1 ORDER BY created_at DESC",
        )
        .bind(implant_id)
        .fetch_all(&self.pool)
        .await?;

        // DECRYPT PAYLOADS FOR DISPLAY
        for cmd in &mut recs {
            if let Some(payload) = &cmd.payload {
                // If decryption fails (e.g. key changed), we might return raw or empty
                if let Ok(plaintext) = self.decrypt_data(payload) {
                    cmd.payload = Some(plaintext);
                }
            }
        }

        Ok(recs)
    }

    pub async fn cancel_command(&self, command_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE commands SET status = 'cancelled' WHERE id = $1 AND status = 'pending'",
        )
        .bind(command_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_command_result(&self, command_id: Uuid) -> Result<Option<CommandResult>> {
        let mut rec = sqlx::query_as::<_, CommandResult>(
            "SELECT * FROM command_results WHERE command_id = $1",
        )
        .bind(command_id)
        .fetch_optional(&self.pool)
        .await?;

        // DECRYPT RESULT
        if let Some(r) = &mut rec {
            if let Some(cipher_output) = &r.output {
                if let Ok(plaintext) = self.decrypt_data(cipher_output) {
                    r.output = Some(plaintext);
                }
            }
        }

        Ok(rec)
    }

    // --- Artifact Operations ---
    pub async fn list_artifacts(&self) -> Result<Vec<Artifact>> {
        let recs = sqlx::query_as::<_, Artifact>(
            "SELECT id, implant_id, command_id, filename, original_path, file_hash_sha256, file_hash_blake3, file_size, mime_type, collected_at, metadata, NULL as content FROM artifacts ORDER BY collected_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(recs)
    }

    pub async fn get_artifact(&self, id: Uuid) -> Result<Artifact> {
        let mut rec = sqlx::query_as::<_, Artifact>("SELECT * FROM artifacts WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

        // DECRYPT CONTENT
        if let Some(cipher_content) = &rec.content {
            if let Ok(plaintext) = self.decrypt_data(cipher_content) {
                rec.content = Some(plaintext);
            }
        }

        Ok(rec)
    }

    pub async fn create_artifact(
        &self,
        implant_id: Uuid,
        filename: &str,
        content: &[u8],
    ) -> Result<Uuid> {
        // ENCRYPT ARTIFACT CONTENT AT REST
        let encrypted_content = self.encrypt_data(content)?;

        let row = sqlx::query(
            "INSERT INTO artifacts (implant_id, filename, content, collected_at) VALUES ($1, $2, $3, NOW()) RETURNING id"
        )
        .bind(implant_id)
        .bind(filename)
        .bind(encrypted_content)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.try_get("id")?)
    }

    // --- Credential Operations ---
    pub async fn list_credentials(&self) -> Result<Vec<Credential>> {
        let recs =
            sqlx::query_as::<_, Credential>("SELECT * FROM credentials ORDER BY collected_at DESC")
                .fetch_all(&self.pool)
                .await?;
        Ok(recs)
    }

    // --- Operator Operations ---
    pub async fn get_operator_by_username(
        &self,
        username: &str,
    ) -> Result<Option<crate::models::Operator>> {
        let rec = sqlx::query_as::<_, crate::models::Operator>(
            "SELECT * FROM operators WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(rec)
    }

    pub async fn get_operator(&self, id: Uuid) -> Result<Option<crate::models::Operator>> {
        let rec =
            sqlx::query_as::<_, crate::models::Operator>("SELECT * FROM operators WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(rec)
    }

    // --- Audit Logging ---
    pub async fn log_activity(
        &self,
        operator_id: Option<Uuid>,
        implant_id: Option<Uuid>,
        action: &str,
        details: serde_json::Value,
        success: bool,
    ) -> Result<()> {
        let timestamp = chrono::Utc::now();

        type HmacSha256 = Hmac<Sha256>;
        let mut mac: HmacSha256 =
            Mac::new_from_slice(&self.hmac_key).expect("HMAC can take key of any size");

        mac.update(timestamp.to_rfc3339().as_bytes());
        if let Some(oid) = operator_id {
            mac.update(oid.as_bytes());
        }
        if let Some(iid) = implant_id {
            mac.update(iid.as_bytes());
        }
        mac.update(action.as_bytes());
        mac.update(details.to_string().as_bytes());
        mac.update(&[if success { 1 } else { 0 }]);

        let signature = mac.finalize().into_bytes().to_vec();

        sqlx::query(
            "INSERT INTO activity_log (timestamp, operator_id, implant_id, action, details, success, signature) VALUES ($1, $2, $3, $4, $5, $6, $7)"
        )
        .bind(timestamp)
        .bind(operator_id)
        .bind(implant_id)
        .bind(action)
        .bind(details)
        .bind(success)
        .bind(signature)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // --- Persistence Operations ---
    pub async fn list_persistence(&self, implant_id: Uuid) -> Result<Vec<crate::models::PersistenceItem>> {
        let recs = sqlx::query_as::<_, crate::models::PersistenceItem>(
            "SELECT * FROM persistence WHERE implant_id = $1 ORDER BY created_at DESC"
        )
        .bind(implant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(recs)
    }

    pub async fn remove_persistence(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM persistence WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn add_persistence(&self, implant_id: Uuid, method: &str, details: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO persistence (implant_id, method, details) VALUES ($1, $2, $3)"
        )
        .bind(implant_id)
        .bind(method)
        .bind(details)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}