use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Listener {
    pub id: Uuid,
    pub name: String,
    pub r#type: String, // 'type' is a reserved keyword
    pub bind_address: String, // Store as string for simplicity in model
    // sqlx might need a specific type if DB is INET, but let's assume we cast/convert
    pub config: serde_json::Value,
    pub status: String,
}
