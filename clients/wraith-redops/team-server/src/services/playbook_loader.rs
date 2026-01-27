use crate::database::Database;
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn};

pub async fn load_playbooks(db: Arc<Database>) -> Result<()> {
    let playbooks_dir = Path::new("./playbooks");
    if !playbooks_dir.exists() {
        warn!("Playbooks directory not found: {:?}", playbooks_dir);
        return Ok(());
    }

    let mut count = 0;
    for entry in std::fs::read_dir(playbooks_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && let Some(ext) = path.extension()
        {
            let content_str = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) => {
                    warn!("Failed to read {:?}: {}", path, e);
                    continue;
                }
            };

            let json_content: serde_json::Value = if ext == "yaml" || ext == "yml" {
                match serde_yaml::from_str(&content_str) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("Failed to parse YAML {:?}: {}", path, e);
                        continue;
                    }
                }
            } else if ext == "json" {
                match serde_json::from_str(&content_str) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("Failed to parse JSON {:?}: {}", path, e);
                        continue;
                    }
                }
            } else {
                continue;
            };

            // Parse metadata
            let name = json_content["name"]
                .as_str()
                .unwrap_or("Unknown")
                .to_string();
            let description = json_content["description"]
                .as_str()
                .unwrap_or("")
                .to_string();

            // Insert into DB (Upsert? create_playbook will fail if unique constraint on name.
            // We should probably check if exists or use ON CONFLICT DO NOTHING/UPDATE.
            // Current create_playbook uses INSERT.
            // I'll wrap in match to ignore duplicates for now or log warning.

            match db.create_playbook(&name, &description, json_content).await {
                Ok(_) => {
                    info!("Loaded playbook: {}", name);
                    count += 1;
                }
                Err(e) => warn!(
                    "Failed to load playbook {} (might already exist): {}",
                    name, e
                ),
            }
        }
    }
    info!("Total playbooks loaded: {}", count);
    Ok(())
}
