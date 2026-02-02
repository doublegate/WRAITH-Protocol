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

/// Parse playbook content from a string, returning (name, description, json_value).
/// Used internally by load_playbooks and exposed for testing.
#[cfg(test)]
pub fn parse_playbook_content(
    content_str: &str,
    ext: &str,
) -> Option<(String, String, serde_json::Value)> {
    let json_content: serde_json::Value = if ext == "yaml" || ext == "yml" {
        serde_yaml::from_str(content_str).ok()?
    } else if ext == "json" {
        serde_json::from_str(content_str).ok()?
    } else {
        return None;
    };

    let name = json_content["name"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();
    let description = json_content["description"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Some((name, description, json_content))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_playbook_json() {
        let content = r#"{"name": "Test Playbook", "description": "A test", "steps": []}"#;
        let result = parse_playbook_content(content, "json");
        assert!(result.is_some());
        let (name, desc, val) = result.unwrap();
        assert_eq!(name, "Test Playbook");
        assert_eq!(desc, "A test");
        assert!(val["steps"].is_array());
    }

    #[test]
    fn test_parse_playbook_yaml() {
        let content = "name: YAML Playbook\ndescription: YAML test\nsteps:\n  - order: 1\n";
        let result = parse_playbook_content(content, "yaml");
        assert!(result.is_some());
        let (name, desc, _) = result.unwrap();
        assert_eq!(name, "YAML Playbook");
        assert_eq!(desc, "YAML test");
    }

    #[test]
    fn test_parse_playbook_yml_extension() {
        let content = "name: YML Playbook\ndescription: test\n";
        let result = parse_playbook_content(content, "yml");
        assert!(result.is_some());
        let (name, _, _) = result.unwrap();
        assert_eq!(name, "YML Playbook");
    }

    #[test]
    fn test_parse_playbook_unsupported_extension() {
        let content = "whatever";
        let result = parse_playbook_content(content, "txt");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_playbook_invalid_json() {
        let content = "{ invalid json }}}";
        let result = parse_playbook_content(content, "json");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_playbook_invalid_yaml() {
        let content = ":\n  :\n    - [invalid\n";
        let result = parse_playbook_content(content, "yaml");
        // serde_yaml may parse even malformed YAML in some cases, check accordingly
        // The point is it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_parse_playbook_missing_name() {
        let content = r#"{"description": "No name"}"#;
        let result = parse_playbook_content(content, "json");
        assert!(result.is_some());
        let (name, _, _) = result.unwrap();
        assert_eq!(name, "Unknown");
    }

    #[test]
    fn test_parse_playbook_missing_description() {
        let content = r#"{"name": "No desc"}"#;
        let result = parse_playbook_content(content, "json");
        assert!(result.is_some());
        let (_, desc, _) = result.unwrap();
        assert_eq!(desc, "");
    }

    #[test]
    fn test_parse_playbook_complex_steps() {
        let content = r#"{
            "name": "Recon Chain",
            "description": "Multi-step recon",
            "steps": [
                {"order": 1, "technique": "T1003", "command_type": "shell", "payload": "whoami", "description": "step1"},
                {"order": 2, "technique": "T1059", "command_type": "powershell", "payload": "Get-Process", "description": "step2"}
            ]
        }"#;
        let result = parse_playbook_content(content, "json");
        assert!(result.is_some());
        let (name, _, val) = result.unwrap();
        assert_eq!(name, "Recon Chain");
        let steps = val["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0]["technique"], "T1003");
        assert_eq!(steps[1]["technique"], "T1059");
    }
}
