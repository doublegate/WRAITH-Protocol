//! SQLite Database for WRAITH Share
//!
//! Manages groups, members, shared files, versions, capabilities, activity logs, and share links.

use anyhow::{Context, Result};
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Database connection manager for share data
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open or create the share database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Configure SQLite for better performance
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "cache_size", 10000)?;
        conn.pragma_update(None, "temp_store", "MEMORY")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Self {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;

        Ok(db)
    }

    /// Create all database tables
    fn create_tables(&self) -> Result<()> {
        let conn = self.conn.lock();

        // Groups table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS groups (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                created_at INTEGER NOT NULL,
                created_by TEXT NOT NULL
            )",
            [],
        )?;

        // Group members table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS group_members (
                group_id TEXT NOT NULL,
                peer_id TEXT NOT NULL,
                display_name TEXT,
                role TEXT NOT NULL CHECK (role IN ('admin', 'write', 'read')),
                joined_at INTEGER NOT NULL,
                invited_by TEXT NOT NULL,
                public_key BLOB NOT NULL,
                PRIMARY KEY (group_id, peer_id),
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Shared files table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS shared_files (
                id TEXT PRIMARY KEY,
                group_id TEXT NOT NULL,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                size INTEGER NOT NULL,
                mime_type TEXT,
                uploaded_by TEXT NOT NULL,
                uploaded_at INTEGER NOT NULL,
                current_version INTEGER NOT NULL DEFAULT 1,
                hash TEXT NOT NULL,
                deleted INTEGER DEFAULT 0,
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // File versions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS file_versions (
                file_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                size INTEGER NOT NULL,
                hash TEXT NOT NULL,
                uploaded_by TEXT NOT NULL,
                uploaded_at INTEGER NOT NULL,
                storage_path TEXT,
                PRIMARY KEY (file_id, version),
                FOREIGN KEY (file_id) REFERENCES shared_files(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // File capabilities table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS file_capabilities (
                file_id TEXT NOT NULL,
                peer_id TEXT NOT NULL,
                permission TEXT NOT NULL CHECK (permission IN ('read', 'write')),
                encrypted_key BLOB NOT NULL,
                granted_by TEXT NOT NULL,
                granted_at INTEGER NOT NULL,
                signature BLOB NOT NULL,
                PRIMARY KEY (file_id, peer_id),
                FOREIGN KEY (file_id) REFERENCES shared_files(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Activity log table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS activity_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                group_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                actor_id TEXT NOT NULL,
                target_id TEXT,
                target_name TEXT,
                details TEXT,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Share links table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS share_links (
                id TEXT PRIMARY KEY,
                file_id TEXT NOT NULL,
                created_by TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER,
                password_hash TEXT,
                max_downloads INTEGER,
                download_count INTEGER DEFAULT 0,
                revoked INTEGER DEFAULT 0,
                FOREIGN KEY (file_id) REFERENCES shared_files(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Pending invitations table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS pending_invitations (
                id TEXT PRIMARY KEY,
                group_id TEXT NOT NULL,
                group_name TEXT NOT NULL,
                invited_by TEXT NOT NULL,
                invited_by_name TEXT NOT NULL,
                role TEXT NOT NULL CHECK (role IN ('admin', 'write', 'read')),
                expires_at INTEGER NOT NULL,
                invite_code TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // Local identity table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS local_identity (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                peer_id TEXT NOT NULL,
                display_name TEXT NOT NULL,
                public_key BLOB NOT NULL,
                private_key BLOB NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_group_members_group
             ON group_members(group_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_shared_files_group
             ON shared_files(group_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_versions_file
             ON file_versions(file_id, version DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_file_capabilities_file
             ON file_capabilities(file_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_activity_log_group
             ON activity_log(group_id, timestamp DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_share_links_file
             ON share_links(file_id)",
            [],
        )?;

        Ok(())
    }

    // =========================================================================
    // Local Identity Operations
    // =========================================================================

    /// Get or create local identity
    pub fn get_local_identity(&self) -> Result<Option<LocalIdentity>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT peer_id, display_name, public_key, private_key, created_at
             FROM local_identity WHERE id = 1",
            [],
            |row| {
                Ok(LocalIdentity {
                    peer_id: row.get(0)?,
                    display_name: row.get(1)?,
                    public_key: row.get(2)?,
                    private_key: row.get(3)?,
                    created_at: row.get(4)?,
                })
            },
        )
        .optional()
        .context("Failed to get local identity")
    }

    /// Save local identity
    pub fn save_local_identity(&self, identity: &LocalIdentity) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO local_identity (id, peer_id, display_name, public_key, private_key, created_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5)",
            params![
                identity.peer_id,
                identity.display_name,
                identity.public_key,
                identity.private_key,
                identity.created_at,
            ],
        )?;
        Ok(())
    }

    // =========================================================================
    // Group Operations
    // =========================================================================

    /// Create a new group
    pub fn create_group(&self, group: &Group) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO groups (id, name, description, created_at, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                group.id,
                group.name,
                group.description,
                group.created_at,
                group.created_by,
            ],
        )?;
        Ok(())
    }

    /// Get a group by ID
    pub fn get_group(&self, group_id: &str) -> Result<Option<Group>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, name, description, created_at, created_by
             FROM groups WHERE id = ?1",
            params![group_id],
            |row| {
                Ok(Group {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    created_by: row.get(4)?,
                })
            },
        )
        .optional()
        .context("Failed to get group")
    }

    /// List all groups
    pub fn list_groups(&self) -> Result<Vec<Group>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at, created_by
             FROM groups ORDER BY name ASC",
        )?;

        let groups = stmt
            .query_map([], |row| {
                Ok(Group {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    created_by: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(groups)
    }

    /// Delete a group
    pub fn delete_group(&self, group_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM groups WHERE id = ?1", params![group_id])?;
        Ok(())
    }

    // =========================================================================
    // Member Operations
    // =========================================================================

    /// Add a member to a group
    pub fn add_group_member(&self, member: &GroupMember) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO group_members (group_id, peer_id, display_name, role, joined_at, invited_by, public_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                member.group_id,
                member.peer_id,
                member.display_name,
                member.role,
                member.joined_at,
                member.invited_by,
                member.public_key,
            ],
        )?;
        Ok(())
    }

    /// Get a member by group and peer ID
    pub fn get_group_member(&self, group_id: &str, peer_id: &str) -> Result<Option<GroupMember>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT group_id, peer_id, display_name, role, joined_at, invited_by, public_key
             FROM group_members WHERE group_id = ?1 AND peer_id = ?2",
            params![group_id, peer_id],
            |row| {
                Ok(GroupMember {
                    group_id: row.get(0)?,
                    peer_id: row.get(1)?,
                    display_name: row.get(2)?,
                    role: row.get(3)?,
                    joined_at: row.get(4)?,
                    invited_by: row.get(5)?,
                    public_key: row.get(6)?,
                })
            },
        )
        .optional()
        .context("Failed to get group member")
    }

    /// List all members of a group
    pub fn list_group_members(&self, group_id: &str) -> Result<Vec<GroupMember>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT group_id, peer_id, display_name, role, joined_at, invited_by, public_key
             FROM group_members WHERE group_id = ?1 ORDER BY joined_at ASC",
        )?;

        let members = stmt
            .query_map(params![group_id], |row| {
                Ok(GroupMember {
                    group_id: row.get(0)?,
                    peer_id: row.get(1)?,
                    display_name: row.get(2)?,
                    role: row.get(3)?,
                    joined_at: row.get(4)?,
                    invited_by: row.get(5)?,
                    public_key: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(members)
    }

    /// Remove a member from a group
    pub fn remove_group_member(&self, group_id: &str, peer_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM group_members WHERE group_id = ?1 AND peer_id = ?2",
            params![group_id, peer_id],
        )?;
        Ok(())
    }

    /// Update member role
    pub fn update_member_role(&self, group_id: &str, peer_id: &str, role: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE group_members SET role = ?1 WHERE group_id = ?2 AND peer_id = ?3",
            params![role, group_id, peer_id],
        )?;
        Ok(())
    }

    /// Count members in a group
    pub fn count_group_members(&self, group_id: &str) -> Result<i64> {
        let conn = self.conn.lock();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM group_members WHERE group_id = ?1",
            params![group_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    // =========================================================================
    // Shared File Operations
    // =========================================================================

    /// Create a shared file
    pub fn create_shared_file(&self, file: &SharedFile) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO shared_files (id, group_id, name, path, size, mime_type, uploaded_by, uploaded_at, current_version, hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                file.id,
                file.group_id,
                file.name,
                file.path,
                file.size,
                file.mime_type,
                file.uploaded_by,
                file.uploaded_at,
                file.current_version,
                file.hash,
            ],
        )?;
        Ok(())
    }

    /// Get a shared file by ID
    pub fn get_shared_file(&self, file_id: &str) -> Result<Option<SharedFile>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, group_id, name, path, size, mime_type, uploaded_by, uploaded_at, current_version, hash
             FROM shared_files WHERE id = ?1 AND deleted = 0",
            params![file_id],
            |row| {
                Ok(SharedFile {
                    id: row.get(0)?,
                    group_id: row.get(1)?,
                    name: row.get(2)?,
                    path: row.get(3)?,
                    size: row.get(4)?,
                    mime_type: row.get(5)?,
                    uploaded_by: row.get(6)?,
                    uploaded_at: row.get(7)?,
                    current_version: row.get(8)?,
                    hash: row.get(9)?,
                })
            },
        )
        .optional()
        .context("Failed to get shared file")
    }

    /// List files in a group
    pub fn list_group_files(&self, group_id: &str) -> Result<Vec<SharedFile>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, group_id, name, path, size, mime_type, uploaded_by, uploaded_at, current_version, hash
             FROM shared_files WHERE group_id = ?1 AND deleted = 0 ORDER BY path ASC",
        )?;

        let files = stmt
            .query_map(params![group_id], |row| {
                Ok(SharedFile {
                    id: row.get(0)?,
                    group_id: row.get(1)?,
                    name: row.get(2)?,
                    path: row.get(3)?,
                    size: row.get(4)?,
                    mime_type: row.get(5)?,
                    uploaded_by: row.get(6)?,
                    uploaded_at: row.get(7)?,
                    current_version: row.get(8)?,
                    hash: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    /// Search files in a group
    pub fn search_group_files(&self, group_id: &str, query: &str) -> Result<Vec<SharedFile>> {
        let conn = self.conn.lock();
        let pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, group_id, name, path, size, mime_type, uploaded_by, uploaded_at, current_version, hash
             FROM shared_files WHERE group_id = ?1 AND deleted = 0 AND (name LIKE ?2 OR path LIKE ?2)
             ORDER BY path ASC",
        )?;

        let files = stmt
            .query_map(params![group_id, pattern], |row| {
                Ok(SharedFile {
                    id: row.get(0)?,
                    group_id: row.get(1)?,
                    name: row.get(2)?,
                    path: row.get(3)?,
                    size: row.get(4)?,
                    mime_type: row.get(5)?,
                    uploaded_by: row.get(6)?,
                    uploaded_at: row.get(7)?,
                    current_version: row.get(8)?,
                    hash: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(files)
    }

    /// Mark a file as deleted
    pub fn delete_shared_file(&self, file_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE shared_files SET deleted = 1 WHERE id = ?1",
            params![file_id],
        )?;
        Ok(())
    }

    /// Update file version
    pub fn update_file_version(
        &self,
        file_id: &str,
        version: i64,
        hash: &str,
        size: i64,
    ) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE shared_files SET current_version = ?1, hash = ?2, size = ?3, uploaded_at = ?4 WHERE id = ?5",
            params![version, hash, size, Utc::now().timestamp(), file_id],
        )?;
        Ok(())
    }

    // =========================================================================
    // File Version Operations
    // =========================================================================

    /// Create a file version
    pub fn create_file_version(&self, version: &FileVersion) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO file_versions (file_id, version, size, hash, uploaded_by, uploaded_at, storage_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                version.file_id,
                version.version,
                version.size,
                version.hash,
                version.uploaded_by,
                version.uploaded_at,
                version.storage_path,
            ],
        )?;
        Ok(())
    }

    /// Get file versions
    pub fn get_file_versions(&self, file_id: &str) -> Result<Vec<FileVersion>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT file_id, version, size, hash, uploaded_by, uploaded_at, storage_path
             FROM file_versions WHERE file_id = ?1 ORDER BY version DESC",
        )?;

        let versions = stmt
            .query_map(params![file_id], |row| {
                Ok(FileVersion {
                    file_id: row.get(0)?,
                    version: row.get(1)?,
                    size: row.get(2)?,
                    hash: row.get(3)?,
                    uploaded_by: row.get(4)?,
                    uploaded_at: row.get(5)?,
                    storage_path: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(versions)
    }

    /// Get a specific file version
    pub fn get_file_version(&self, file_id: &str, version: i64) -> Result<Option<FileVersion>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT file_id, version, size, hash, uploaded_by, uploaded_at, storage_path
             FROM file_versions WHERE file_id = ?1 AND version = ?2",
            params![file_id, version],
            |row| {
                Ok(FileVersion {
                    file_id: row.get(0)?,
                    version: row.get(1)?,
                    size: row.get(2)?,
                    hash: row.get(3)?,
                    uploaded_by: row.get(4)?,
                    uploaded_at: row.get(5)?,
                    storage_path: row.get(6)?,
                })
            },
        )
        .optional()
        .context("Failed to get file version")
    }

    /// Get next version number for a file
    pub fn get_next_version_number(&self, file_id: &str) -> Result<i64> {
        let conn = self.conn.lock();
        let max_version: Option<i64> = conn.query_row(
            "SELECT MAX(version) FROM file_versions WHERE file_id = ?1",
            params![file_id],
            |row| row.get(0),
        )?;
        Ok(max_version.unwrap_or(0) + 1)
    }

    /// Prune old versions (keep most recent N)
    pub fn prune_old_versions(&self, file_id: &str, max_versions: i64) -> Result<Vec<String>> {
        let conn = self.conn.lock();

        // Get versions to delete
        let mut stmt = conn.prepare(
            "SELECT storage_path FROM file_versions
             WHERE file_id = ?1 AND storage_path IS NOT NULL
             ORDER BY version DESC
             LIMIT -1 OFFSET ?2",
        )?;

        let paths: Vec<String> = stmt
            .query_map(params![file_id, max_versions], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Delete old versions from database
        conn.execute(
            "DELETE FROM file_versions
             WHERE file_id = ?1 AND version NOT IN (
                SELECT version FROM file_versions
                WHERE file_id = ?1
                ORDER BY version DESC
                LIMIT ?2
             )",
            params![file_id, max_versions],
        )?;

        Ok(paths)
    }

    // =========================================================================
    // File Capability Operations
    // =========================================================================

    /// Create a file capability
    pub fn create_file_capability(&self, capability: &FileCapability) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO file_capabilities (file_id, peer_id, permission, encrypted_key, granted_by, granted_at, signature)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                capability.file_id,
                capability.peer_id,
                capability.permission,
                capability.encrypted_key,
                capability.granted_by,
                capability.granted_at,
                capability.signature,
            ],
        )?;
        Ok(())
    }

    /// Get capability for a file and peer
    pub fn get_file_capability(
        &self,
        file_id: &str,
        peer_id: &str,
    ) -> Result<Option<FileCapability>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT file_id, peer_id, permission, encrypted_key, granted_by, granted_at, signature
             FROM file_capabilities WHERE file_id = ?1 AND peer_id = ?2",
            params![file_id, peer_id],
            |row| {
                Ok(FileCapability {
                    file_id: row.get(0)?,
                    peer_id: row.get(1)?,
                    permission: row.get(2)?,
                    encrypted_key: row.get(3)?,
                    granted_by: row.get(4)?,
                    granted_at: row.get(5)?,
                    signature: row.get(6)?,
                })
            },
        )
        .optional()
        .context("Failed to get file capability")
    }

    /// List capabilities for a file
    pub fn list_file_capabilities(&self, file_id: &str) -> Result<Vec<FileCapability>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT file_id, peer_id, permission, encrypted_key, granted_by, granted_at, signature
             FROM file_capabilities WHERE file_id = ?1",
        )?;

        let capabilities = stmt
            .query_map(params![file_id], |row| {
                Ok(FileCapability {
                    file_id: row.get(0)?,
                    peer_id: row.get(1)?,
                    permission: row.get(2)?,
                    encrypted_key: row.get(3)?,
                    granted_by: row.get(4)?,
                    granted_at: row.get(5)?,
                    signature: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(capabilities)
    }

    /// Delete capabilities for a file
    pub fn delete_file_capabilities(&self, file_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM file_capabilities WHERE file_id = ?1",
            params![file_id],
        )?;
        Ok(())
    }

    /// Delete capability for a specific peer
    pub fn delete_peer_capability(&self, file_id: &str, peer_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM file_capabilities WHERE file_id = ?1 AND peer_id = ?2",
            params![file_id, peer_id],
        )?;
        Ok(())
    }

    // =========================================================================
    // Activity Log Operations
    // =========================================================================

    /// Log an activity event
    pub fn log_activity(&self, event: &ActivityEvent) -> Result<i64> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO activity_log (group_id, event_type, actor_id, target_id, target_name, details, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                event.group_id,
                event.event_type,
                event.actor_id,
                event.target_id,
                event.target_name,
                event.details,
                event.timestamp,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Get activity log for a group
    pub fn get_activity_log(
        &self,
        group_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ActivityEvent>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, group_id, event_type, actor_id, target_id, target_name, details, timestamp
             FROM activity_log WHERE group_id = ?1
             ORDER BY timestamp DESC
             LIMIT ?2 OFFSET ?3",
        )?;

        let events = stmt
            .query_map(params![group_id, limit, offset], |row| {
                Ok(ActivityEvent {
                    id: row.get(0)?,
                    group_id: row.get(1)?,
                    event_type: row.get(2)?,
                    actor_id: row.get(3)?,
                    target_id: row.get(4)?,
                    target_name: row.get(5)?,
                    details: row.get(6)?,
                    timestamp: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    /// Prune old activity logs (keep most recent N per group)
    pub fn prune_activity_log(&self, group_id: &str, max_events: i64) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM activity_log
             WHERE group_id = ?1 AND id NOT IN (
                SELECT id FROM activity_log
                WHERE group_id = ?1
                ORDER BY timestamp DESC
                LIMIT ?2
             )",
            params![group_id, max_events],
        )?;
        Ok(())
    }

    // =========================================================================
    // Share Link Operations
    // =========================================================================

    /// Create a share link
    pub fn create_share_link(&self, link: &ShareLink) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO share_links (id, file_id, created_by, created_at, expires_at, password_hash, max_downloads, download_count, revoked)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                link.id,
                link.file_id,
                link.created_by,
                link.created_at,
                link.expires_at,
                link.password_hash,
                link.max_downloads,
                link.download_count,
                link.revoked as i32,
            ],
        )?;
        Ok(())
    }

    /// Get a share link by ID
    pub fn get_share_link(&self, link_id: &str) -> Result<Option<ShareLink>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, file_id, created_by, created_at, expires_at, password_hash, max_downloads, download_count, revoked
             FROM share_links WHERE id = ?1",
            params![link_id],
            |row| {
                Ok(ShareLink {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    created_by: row.get(2)?,
                    created_at: row.get(3)?,
                    expires_at: row.get(4)?,
                    password_hash: row.get(5)?,
                    max_downloads: row.get(6)?,
                    download_count: row.get(7)?,
                    revoked: row.get::<_, i32>(8)? != 0,
                })
            },
        )
        .optional()
        .context("Failed to get share link")
    }

    /// List share links for a file
    pub fn list_file_share_links(&self, file_id: &str) -> Result<Vec<ShareLink>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, file_id, created_by, created_at, expires_at, password_hash, max_downloads, download_count, revoked
             FROM share_links WHERE file_id = ?1 ORDER BY created_at DESC",
        )?;

        let links = stmt
            .query_map(params![file_id], |row| {
                Ok(ShareLink {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    created_by: row.get(2)?,
                    created_at: row.get(3)?,
                    expires_at: row.get(4)?,
                    password_hash: row.get(5)?,
                    max_downloads: row.get(6)?,
                    download_count: row.get(7)?,
                    revoked: row.get::<_, i32>(8)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(links)
    }

    /// Increment download count for a share link
    pub fn increment_link_download_count(&self, link_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE share_links SET download_count = download_count + 1 WHERE id = ?1",
            params![link_id],
        )?;
        Ok(())
    }

    /// Revoke a share link
    pub fn revoke_share_link(&self, link_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE share_links SET revoked = 1 WHERE id = ?1",
            params![link_id],
        )?;
        Ok(())
    }

    // =========================================================================
    // Pending Invitation Operations
    // =========================================================================

    /// Save a pending invitation
    pub fn save_pending_invitation(&self, invitation: &PendingInvitation) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO pending_invitations (id, group_id, group_name, invited_by, invited_by_name, role, expires_at, invite_code, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                invitation.id,
                invitation.group_id,
                invitation.group_name,
                invitation.invited_by,
                invitation.invited_by_name,
                invitation.role,
                invitation.expires_at,
                invitation.invite_code,
                invitation.created_at,
            ],
        )?;
        Ok(())
    }

    /// Get a pending invitation by ID
    pub fn get_pending_invitation(&self, invitation_id: &str) -> Result<Option<PendingInvitation>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, group_id, group_name, invited_by, invited_by_name, role, expires_at, invite_code, created_at
             FROM pending_invitations WHERE id = ?1",
            params![invitation_id],
            |row| {
                Ok(PendingInvitation {
                    id: row.get(0)?,
                    group_id: row.get(1)?,
                    group_name: row.get(2)?,
                    invited_by: row.get(3)?,
                    invited_by_name: row.get(4)?,
                    role: row.get(5)?,
                    expires_at: row.get(6)?,
                    invite_code: row.get(7)?,
                    created_at: row.get(8)?,
                })
            },
        )
        .optional()
        .context("Failed to get pending invitation")
    }

    /// List pending invitations
    pub fn list_pending_invitations(&self) -> Result<Vec<PendingInvitation>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, group_id, group_name, invited_by, invited_by_name, role, expires_at, invite_code, created_at
             FROM pending_invitations ORDER BY created_at DESC",
        )?;

        let invitations = stmt
            .query_map([], |row| {
                Ok(PendingInvitation {
                    id: row.get(0)?,
                    group_id: row.get(1)?,
                    group_name: row.get(2)?,
                    invited_by: row.get(3)?,
                    invited_by_name: row.get(4)?,
                    role: row.get(5)?,
                    expires_at: row.get(6)?,
                    invite_code: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(invitations)
    }

    /// Delete a pending invitation
    pub fn delete_pending_invitation(&self, invitation_id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM pending_invitations WHERE id = ?1",
            params![invitation_id],
        )?;
        Ok(())
    }

    // =========================================================================
    // Settings Operations
    // =========================================================================

    /// Get a setting value
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .context("Failed to get setting")
    }

    /// Set a setting value
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }
}

// =============================================================================
// Data Models
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalIdentity {
    pub peer_id: String,
    pub display_name: String,
    pub public_key: Vec<u8>,
    pub private_key: Vec<u8>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub group_id: String,
    pub peer_id: String,
    pub display_name: Option<String>,
    pub role: String, // "admin", "write", "read"
    pub joined_at: i64,
    pub invited_by: String,
    pub public_key: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedFile {
    pub id: String,
    pub group_id: String,
    pub name: String,
    pub path: String,
    pub size: i64,
    pub mime_type: Option<String>,
    pub uploaded_by: String,
    pub uploaded_at: i64,
    pub current_version: i64,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    pub file_id: String,
    pub version: i64,
    pub size: i64,
    pub hash: String,
    pub uploaded_by: String,
    pub uploaded_at: i64,
    pub storage_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCapability {
    pub file_id: String,
    pub peer_id: String,
    pub permission: String, // "read" or "write"
    pub encrypted_key: Vec<u8>,
    pub granted_by: String,
    pub granted_at: i64,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEvent {
    #[serde(default)]
    pub id: i64,
    pub group_id: String,
    pub event_type: String,
    pub actor_id: String,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub details: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareLink {
    pub id: String,
    pub file_id: String,
    pub created_by: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub password_hash: Option<String>,
    pub max_downloads: Option<i64>,
    pub download_count: i64,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingInvitation {
    pub id: String,
    pub group_id: String,
    pub group_name: String,
    pub invited_by: String,
    pub invited_by_name: String,
    pub role: String, // "admin", "write", "read"
    pub expires_at: i64,
    pub invite_code: String,
    pub created_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Verify tables exist
        let groups = db.list_groups().unwrap();
        assert!(groups.is_empty());
    }

    #[test]
    fn test_group_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let group = Group {
            id: "test-group-id".to_string(),
            name: "Test Group".to_string(),
            description: Some("A test group".to_string()),
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
        };

        db.create_group(&group).unwrap();

        let retrieved = db.get_group(&group.id).unwrap().unwrap();
        assert_eq!(retrieved.name, group.name);
        assert_eq!(retrieved.description, group.description);
    }

    #[test]
    fn test_member_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Create group first
        let group = Group {
            id: "test-group-id".to_string(),
            name: "Test Group".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
        };
        db.create_group(&group).unwrap();

        // Add member
        let member = GroupMember {
            group_id: group.id.clone(),
            peer_id: "peer-456".to_string(),
            display_name: Some("Test User".to_string()),
            role: "write".to_string(),
            joined_at: Utc::now().timestamp(),
            invited_by: "peer-123".to_string(),
            public_key: vec![1, 2, 3, 4],
        };

        db.add_group_member(&member).unwrap();

        let retrieved = db
            .get_group_member(&group.id, &member.peer_id)
            .unwrap()
            .unwrap();
        assert_eq!(retrieved.display_name, member.display_name);
        assert_eq!(retrieved.role, "write");

        // Update role
        db.update_member_role(&group.id, &member.peer_id, "admin")
            .unwrap();
        let updated = db
            .get_group_member(&group.id, &member.peer_id)
            .unwrap()
            .unwrap();
        assert_eq!(updated.role, "admin");
    }

    #[test]
    fn test_shared_file_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Create group first
        let group = Group {
            id: "test-group-id".to_string(),
            name: "Test Group".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
        };
        db.create_group(&group).unwrap();

        // Create file
        let file = SharedFile {
            id: "file-123".to_string(),
            group_id: group.id.clone(),
            name: "test.txt".to_string(),
            path: "/documents/test.txt".to_string(),
            size: 1024,
            mime_type: Some("text/plain".to_string()),
            uploaded_by: "peer-123".to_string(),
            uploaded_at: Utc::now().timestamp(),
            current_version: 1,
            hash: "abc123".to_string(),
        };

        db.create_shared_file(&file).unwrap();

        let retrieved = db.get_shared_file(&file.id).unwrap().unwrap();
        assert_eq!(retrieved.name, file.name);
        assert_eq!(retrieved.size, file.size);

        // Search
        let results = db.search_group_files(&group.id, "test").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_activity_log() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Create group first
        let group = Group {
            id: "test-group-id".to_string(),
            name: "Test Group".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
        };
        db.create_group(&group).unwrap();

        // Log events
        for i in 0..5 {
            let event = ActivityEvent {
                id: 0,
                group_id: group.id.clone(),
                event_type: "file_upload".to_string(),
                actor_id: "peer-123".to_string(),
                target_id: Some(format!("file-{}", i)),
                target_name: Some(format!("file{}.txt", i)),
                details: None,
                timestamp: Utc::now().timestamp(),
            };
            db.log_activity(&event).unwrap();
        }

        let events = db.get_activity_log(&group.id, 10, 0).unwrap();
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn test_share_link_operations() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Create group and file first
        let group = Group {
            id: "test-group-id".to_string(),
            name: "Test Group".to_string(),
            description: None,
            created_at: Utc::now().timestamp(),
            created_by: "peer-123".to_string(),
        };
        db.create_group(&group).unwrap();

        let file = SharedFile {
            id: "file-123".to_string(),
            group_id: group.id.clone(),
            name: "test.txt".to_string(),
            path: "/test.txt".to_string(),
            size: 1024,
            mime_type: None,
            uploaded_by: "peer-123".to_string(),
            uploaded_at: Utc::now().timestamp(),
            current_version: 1,
            hash: "abc123".to_string(),
        };
        db.create_shared_file(&file).unwrap();

        // Create share link
        let link = ShareLink {
            id: "link-123".to_string(),
            file_id: file.id.clone(),
            created_by: "peer-123".to_string(),
            created_at: Utc::now().timestamp(),
            expires_at: Some(Utc::now().timestamp() + 86400),
            password_hash: None,
            max_downloads: Some(10),
            download_count: 0,
            revoked: false,
        };

        db.create_share_link(&link).unwrap();

        let retrieved = db.get_share_link(&link.id).unwrap().unwrap();
        assert_eq!(retrieved.max_downloads, Some(10));
        assert_eq!(retrieved.download_count, 0);

        // Increment download count
        db.increment_link_download_count(&link.id).unwrap();
        let updated = db.get_share_link(&link.id).unwrap().unwrap();
        assert_eq!(updated.download_count, 1);

        // Revoke
        db.revoke_share_link(&link.id).unwrap();
        let revoked = db.get_share_link(&link.id).unwrap().unwrap();
        assert!(revoked.revoked);
    }
}
