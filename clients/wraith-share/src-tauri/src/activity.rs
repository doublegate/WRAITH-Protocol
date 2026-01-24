//! Activity Logging
//!
//! Records and retrieves activity events for groups.

use crate::database::{ActivityEvent, Database};
use crate::error::ShareResult;
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// Maximum events to keep per group (default)
pub const DEFAULT_MAX_EVENTS: i64 = 1000;

/// Activity event types
pub mod event_types {
    // Group events
    pub const GROUP_CREATED: &str = "group_created";
    pub const GROUP_DELETED: &str = "group_deleted";
    pub const GROUP_UPDATED: &str = "group_updated";

    // Member events
    pub const MEMBER_INVITED: &str = "member_invited";
    pub const MEMBER_JOINED: &str = "member_joined";
    pub const MEMBER_REMOVED: &str = "member_removed";
    pub const ROLE_CHANGED: &str = "role_changed";

    // File events
    pub const FILE_UPLOADED: &str = "file_uploaded";
    pub const FILE_UPDATED: &str = "file_updated";
    pub const FILE_DOWNLOADED: &str = "file_downloaded";
    pub const FILE_DELETED: &str = "file_deleted";
    pub const FILE_RESTORED: &str = "file_restored";

    // Link events
    pub const LINK_CREATED: &str = "link_created";
    pub const LINK_REVOKED: &str = "link_revoked";
    pub const LINK_ACCESSED: &str = "link_accessed";
}

/// Activity logger handles activity event recording and retrieval
pub struct ActivityLogger {
    db: Arc<Database>,
    state: Arc<AppState>,
}

/// Activity event for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityInfo {
    pub id: i64,
    pub group_id: String,
    pub event_type: String,
    pub event_category: String,
    pub actor_id: String,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub details: Option<String>,
    pub timestamp: i64,
    pub human_readable: String,
}

impl ActivityLogger {
    /// Create a new activity logger
    pub fn new(db: Arc<Database>, state: Arc<AppState>) -> Self {
        Self { db, state }
    }

    /// Log a new activity event
    pub fn log_event(
        &self,
        group_id: &str,
        event_type: &str,
        actor_id: &str,
        target_id: Option<&str>,
        target_name: Option<&str>,
        details: Option<&str>,
    ) -> ShareResult<i64> {
        let event = ActivityEvent {
            id: 0,
            group_id: group_id.to_string(),
            event_type: event_type.to_string(),
            actor_id: actor_id.to_string(),
            target_id: target_id.map(String::from),
            target_name: target_name.map(String::from),
            details: details.map(String::from),
            timestamp: Utc::now().timestamp(),
        };

        let id = self.db.log_activity(&event)?;

        // Prune old events
        self.db
            .prune_activity_log(group_id, self.state.max_activity_events)?;

        info!("Logged activity: {} in group {}", event_type, group_id);

        Ok(id)
    }

    /// Get activity log for a group
    pub fn get_activity_log(
        &self,
        group_id: &str,
        limit: i64,
        offset: i64,
    ) -> ShareResult<Vec<ActivityInfo>> {
        let events = self.db.get_activity_log(group_id, limit, offset)?;

        let activity_infos = events
            .into_iter()
            .map(|e| self.to_activity_info(e))
            .collect();

        Ok(activity_infos)
    }

    /// Get recent activity across all groups (for dashboard)
    pub fn get_recent_activity(&self, limit: i64) -> ShareResult<Vec<ActivityInfo>> {
        let groups = self.db.list_groups()?;
        let mut all_events = Vec::new();

        for group in groups {
            let events = self.db.get_activity_log(&group.id, limit, 0)?;
            all_events.extend(events);
        }

        // Sort by timestamp descending
        all_events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // Take only the requested limit
        let result = all_events
            .into_iter()
            .take(limit as usize)
            .map(|e| self.to_activity_info(e))
            .collect();

        Ok(result)
    }

    /// Filter activity log by event type
    pub fn filter_by_type(
        &self,
        group_id: &str,
        event_type: &str,
        limit: i64,
    ) -> ShareResult<Vec<ActivityInfo>> {
        let events = self.db.get_activity_log(group_id, 1000, 0)?;

        let filtered: Vec<ActivityInfo> = events
            .into_iter()
            .filter(|e| e.event_type == event_type)
            .take(limit as usize)
            .map(|e| self.to_activity_info(e))
            .collect();

        Ok(filtered)
    }

    /// Filter activity log by actor
    pub fn filter_by_actor(
        &self,
        group_id: &str,
        actor_id: &str,
        limit: i64,
    ) -> ShareResult<Vec<ActivityInfo>> {
        let events = self.db.get_activity_log(group_id, 1000, 0)?;

        let filtered: Vec<ActivityInfo> = events
            .into_iter()
            .filter(|e| e.actor_id == actor_id)
            .take(limit as usize)
            .map(|e| self.to_activity_info(e))
            .collect();

        Ok(filtered)
    }

    /// Filter activity log by target file
    pub fn filter_by_file(
        &self,
        group_id: &str,
        file_id: &str,
        limit: i64,
    ) -> ShareResult<Vec<ActivityInfo>> {
        let events = self.db.get_activity_log(group_id, 1000, 0)?;

        let filtered: Vec<ActivityInfo> = events
            .into_iter()
            .filter(|e| e.target_id.as_deref() == Some(file_id))
            .take(limit as usize)
            .map(|e| self.to_activity_info(e))
            .collect();

        Ok(filtered)
    }

    /// Search activity log
    pub fn search_activity(
        &self,
        group_id: &str,
        query: &str,
        limit: i64,
    ) -> ShareResult<Vec<ActivityInfo>> {
        let events = self.db.get_activity_log(group_id, 1000, 0)?;
        let query_lower = query.to_lowercase();

        let filtered: Vec<ActivityInfo> = events
            .into_iter()
            .filter(|e| {
                e.event_type.to_lowercase().contains(&query_lower)
                    || e.target_name
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || e.details
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .take(limit as usize)
            .map(|e| self.to_activity_info(e))
            .collect();

        Ok(filtered)
    }

    /// Convert database event to UI-friendly format
    fn to_activity_info(&self, event: ActivityEvent) -> ActivityInfo {
        let event_category = self.get_event_category(&event.event_type);
        let actor_name = self.resolve_actor_name(&event.group_id, &event.actor_id);
        let human_readable = self.format_human_readable(&event, actor_name.as_deref());

        ActivityInfo {
            id: event.id,
            group_id: event.group_id,
            event_type: event.event_type,
            event_category,
            actor_id: event.actor_id,
            actor_name,
            target_id: event.target_id,
            target_name: event.target_name,
            details: event.details,
            timestamp: event.timestamp,
            human_readable,
        }
    }

    /// Get category for an event type
    fn get_event_category(&self, event_type: &str) -> String {
        match event_type {
            e if e.starts_with("group_") => "group".to_string(),
            e if e.starts_with("member_") || e.starts_with("role_") => "member".to_string(),
            e if e.starts_with("file_") => "file".to_string(),
            e if e.starts_with("link_") => "link".to_string(),
            _ => "other".to_string(),
        }
    }

    /// Resolve actor name from database
    fn resolve_actor_name(&self, group_id: &str, actor_id: &str) -> Option<String> {
        // Check if actor is the local user
        if let Some(local_peer_id) = self.state.get_peer_id()
            && actor_id == local_peer_id
        {
            return Some(self.state.get_display_name());
        }

        // Look up member name
        if let Ok(Some(member)) = self.db.get_group_member(group_id, actor_id) {
            return member.display_name;
        }

        None
    }

    /// Format event as human-readable text
    fn format_human_readable(&self, event: &ActivityEvent, actor_name: Option<&str>) -> String {
        let actor = actor_name.unwrap_or(&event.actor_id[..8.min(event.actor_id.len())]);
        let target = event
            .target_name
            .as_deref()
            .unwrap_or(event.target_id.as_deref().unwrap_or("unknown"));

        match event.event_type.as_str() {
            event_types::GROUP_CREATED => format!("{} created the group", actor),
            event_types::GROUP_DELETED => format!("{} deleted the group", actor),
            event_types::GROUP_UPDATED => format!("{} updated the group settings", actor),
            event_types::MEMBER_INVITED => {
                format!("{} invited {} to the group", actor, target)
            }
            event_types::MEMBER_JOINED => format!("{} joined the group", actor),
            event_types::MEMBER_REMOVED => format!("{} removed {} from the group", actor, target),
            event_types::ROLE_CHANGED => {
                let details = event.details.as_deref().unwrap_or("");
                format!("{} changed {}'s role: {}", actor, target, details)
            }
            event_types::FILE_UPLOADED => format!("{} uploaded {}", actor, target),
            event_types::FILE_UPDATED => format!("{} updated {}", actor, target),
            event_types::FILE_DOWNLOADED => format!("{} downloaded {}", actor, target),
            event_types::FILE_DELETED => format!("{} deleted {}", actor, target),
            event_types::FILE_RESTORED => format!("{} restored {}", actor, target),
            event_types::LINK_CREATED => format!("{} created a share link for {}", actor, target),
            event_types::LINK_REVOKED => format!("{} revoked a share link for {}", actor, target),
            event_types::LINK_ACCESSED => format!("Share link for {} was accessed", target),
            _ => format!("{} performed {} on {}", actor, event.event_type, target),
        }
    }
}

/// Activity statistics for a group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityStats {
    pub group_id: String,
    pub total_events: i64,
    pub events_today: i64,
    pub events_this_week: i64,
    pub most_active_member: Option<String>,
    pub most_accessed_file: Option<String>,
}

impl ActivityLogger {
    /// Get activity statistics for a group
    pub fn get_activity_stats(&self, group_id: &str) -> ShareResult<ActivityStats> {
        let events = self.db.get_activity_log(group_id, 1000, 0)?;
        let now = Utc::now().timestamp();
        let day_ago = now - 86400;
        let week_ago = now - 604800;

        let total_events = events.len() as i64;
        let events_today = events.iter().filter(|e| e.timestamp > day_ago).count() as i64;
        let events_this_week = events.iter().filter(|e| e.timestamp > week_ago).count() as i64;

        // Count events per actor
        let mut actor_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for event in &events {
            *actor_counts.entry(event.actor_id.clone()).or_insert(0) += 1;
        }
        let most_active_member = actor_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(id, _)| id);

        // Count file accesses
        let mut file_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for event in &events {
            if let Some(target_id) = &event.target_id
                && event.event_type.starts_with("file_")
            {
                *file_counts.entry(target_id.clone()).or_insert(0) += 1;
            }
        }
        let most_accessed_file = file_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(id, _)| id);

        Ok(ActivityStats {
            group_id: group_id.to_string(),
            total_events,
            events_today,
            events_this_week,
            most_active_member,
            most_accessed_file,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Group;
    use tempfile::tempdir;

    fn create_test_env() -> (Arc<Database>, Arc<AppState>, ActivityLogger) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        let state = Arc::new(AppState::new(
            Database::open(&db_path).unwrap(),
            dir.path().to_path_buf(),
        ));
        state.initialize().unwrap();

        let logger = ActivityLogger::new(db.clone(), state.clone());

        (db, state, logger)
    }

    #[test]
    fn test_log_and_retrieve_events() {
        let (db, _state, logger) = create_test_env();

        // Create group
        let group = Group {
            id: "test-group".to_string(),
            name: "Test Group".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
        };
        db.create_group(&group).unwrap();

        // Log some events
        logger
            .log_event(
                &group.id,
                event_types::FILE_UPLOADED,
                "peer-123",
                Some("file-1"),
                Some("document.pdf"),
                Some("Size: 1024 bytes"),
            )
            .unwrap();

        logger
            .log_event(
                &group.id,
                event_types::MEMBER_JOINED,
                "peer-456",
                Some("peer-456"),
                Some("New User"),
                None,
            )
            .unwrap();

        // Retrieve events
        let events = logger.get_activity_log(&group.id, 10, 0).unwrap();
        assert_eq!(events.len(), 2);

        // Verify event data
        let upload_event = events
            .iter()
            .find(|e| e.event_type == event_types::FILE_UPLOADED)
            .unwrap();
        assert_eq!(upload_event.target_name, Some("document.pdf".to_string()));
        assert_eq!(upload_event.event_category, "file");
    }

    #[test]
    fn test_filter_by_type() {
        let (db, _state, logger) = create_test_env();

        // Create group
        let group = Group {
            id: "test-group".to_string(),
            name: "Test Group".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
        };
        db.create_group(&group).unwrap();

        // Log mixed events
        logger
            .log_event(
                &group.id,
                event_types::FILE_UPLOADED,
                "peer-123",
                None,
                None,
                None,
            )
            .unwrap();
        logger
            .log_event(
                &group.id,
                event_types::FILE_DOWNLOADED,
                "peer-123",
                None,
                None,
                None,
            )
            .unwrap();
        logger
            .log_event(
                &group.id,
                event_types::MEMBER_JOINED,
                "peer-456",
                None,
                None,
                None,
            )
            .unwrap();

        // Filter by file upload
        let uploads = logger
            .filter_by_type(&group.id, event_types::FILE_UPLOADED, 10)
            .unwrap();
        assert_eq!(uploads.len(), 1);
    }

    #[test]
    fn test_human_readable_format() {
        let (db, _state, logger) = create_test_env();

        // Create group
        let group = Group {
            id: "test-group".to_string(),
            name: "Test Group".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
        };
        db.create_group(&group).unwrap();

        // Log event
        logger
            .log_event(
                &group.id,
                event_types::FILE_UPLOADED,
                "peer-123",
                Some("file-1"),
                Some("report.pdf"),
                None,
            )
            .unwrap();

        // Get events
        let events = logger.get_activity_log(&group.id, 10, 0).unwrap();
        let event = &events[0];

        // Verify human readable format
        assert!(event.human_readable.contains("uploaded"));
        assert!(event.human_readable.contains("report.pdf"));
    }

    #[test]
    fn test_activity_stats() {
        let (db, _state, logger) = create_test_env();

        // Create group
        let group = Group {
            id: "test-group".to_string(),
            name: "Test Group".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
        };
        db.create_group(&group).unwrap();

        // Log events
        for i in 0..5 {
            logger
                .log_event(
                    &group.id,
                    event_types::FILE_UPLOADED,
                    "peer-123",
                    Some(&format!("file-{}", i)),
                    None,
                    None,
                )
                .unwrap();
        }

        // Get stats
        let stats = logger.get_activity_stats(&group.id).unwrap();
        assert_eq!(stats.total_events, 5);
        assert_eq!(stats.events_today, 5);
        assert_eq!(stats.most_active_member, Some("peer-123".to_string()));
    }
}
