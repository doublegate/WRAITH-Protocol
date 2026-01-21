// SQLCipher Database for Encrypted Message Storage

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Database connection manager
pub struct Database {
    /// The underlying SQLCipher connection
    pub conn: Connection,
}

impl Database {
    /// Open or create encrypted database
    pub fn open<P: AsRef<Path>>(path: P, password: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Set SQLCipher encryption key
        conn.pragma_update(None, "key", password)?;

        // Configure SQLCipher settings for best security
        conn.pragma_update(None, "cipher_page_size", 4096)?;
        conn.pragma_update(None, "kdf_iter", 64000)?;
        conn.pragma_update(None, "cipher_hmac_algorithm", "HMAC_SHA512")?;
        conn.pragma_update(None, "cipher_kdf_algorithm", "PBKDF2_HMAC_SHA512")?;

        let db = Self { conn };
        db.create_tables()?;

        Ok(db)
    }

    /// Create database tables
    fn create_tables(&self) -> Result<()> {
        // Contacts table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS contacts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                peer_id TEXT UNIQUE NOT NULL,
                display_name TEXT,
                identity_key BLOB NOT NULL,
                safety_number TEXT NOT NULL,
                verified INTEGER DEFAULT 0,
                blocked INTEGER DEFAULT 0,
                created_at INTEGER NOT NULL,
                last_seen INTEGER
            )",
            [],
        )?;

        // Conversations table (1:1 and groups)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                type TEXT NOT NULL CHECK(type IN ('direct', 'group')),
                peer_id TEXT,
                group_id TEXT,
                display_name TEXT,
                avatar BLOB,
                muted INTEGER DEFAULT 0,
                archived INTEGER DEFAULT 0,
                last_message_id INTEGER,
                last_message_at INTEGER,
                unread_count INTEGER DEFAULT 0,
                expires_in INTEGER,
                FOREIGN KEY (last_message_id) REFERENCES messages(id)
            )",
            [],
        )?;

        // Messages table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                conversation_id INTEGER NOT NULL,
                sender_peer_id TEXT NOT NULL,
                content_type TEXT NOT NULL CHECK(content_type IN ('text', 'media', 'voice', 'file')),
                body TEXT,
                media_path TEXT,
                media_mime_type TEXT,
                media_size INTEGER,
                timestamp INTEGER NOT NULL,
                sent INTEGER DEFAULT 0,
                delivered INTEGER DEFAULT 0,
                read_by_me INTEGER DEFAULT 0,
                expires_at INTEGER,
                direction TEXT NOT NULL CHECK(direction IN ('incoming', 'outgoing')),
                FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Group members table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS group_members (
                group_id TEXT NOT NULL,
                peer_id TEXT NOT NULL,
                role TEXT NOT NULL CHECK(role IN ('admin', 'member')),
                joined_at INTEGER NOT NULL,
                PRIMARY KEY (group_id, peer_id)
            )",
            [],
        )?;

        // Ratchet states table (Double Ratchet state persistence)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS ratchet_states (
                peer_id TEXT PRIMARY KEY,
                state_json TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create indexes for performance
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_conversation
             ON messages(conversation_id, timestamp DESC)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_sender
             ON messages(sender_peer_id)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_contacts_peer_id
             ON contacts(peer_id)",
            [],
        )?;

        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_group_members_group
             ON group_members(group_id)",
            [],
        )?;

        Ok(())
    }

    // MARK: - Contact Operations

    pub fn insert_contact(&self, contact: &Contact) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO contacts (peer_id, display_name, identity_key, safety_number, verified, blocked, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                contact.peer_id,
                contact.display_name,
                contact.identity_key,
                contact.safety_number,
                contact.verified as i32,
                contact.blocked as i32,
                contact.created_at,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_contact(&self, peer_id: &str) -> Result<Option<Contact>> {
        self.conn
            .query_row(
                "SELECT id, peer_id, display_name, identity_key, safety_number,
                        verified, blocked, created_at, last_seen
                 FROM contacts WHERE peer_id = ?1",
                params![peer_id],
                |row| {
                    Ok(Contact {
                        id: row.get(0)?,
                        peer_id: row.get(1)?,
                        display_name: row.get(2)?,
                        identity_key: row.get(3)?,
                        safety_number: row.get(4)?,
                        verified: row.get::<_, i32>(5)? != 0,
                        blocked: row.get::<_, i32>(6)? != 0,
                        created_at: row.get(7)?,
                        last_seen: row.get(8)?,
                    })
                },
            )
            .optional()
            .context("Failed to get contact")
    }

    pub fn list_contacts(&self) -> Result<Vec<Contact>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, peer_id, display_name, identity_key, safety_number,
                    verified, blocked, created_at, last_seen
             FROM contacts ORDER BY display_name ASC",
        )?;

        let contacts = stmt
            .query_map([], |row| {
                Ok(Contact {
                    id: row.get(0)?,
                    peer_id: row.get(1)?,
                    display_name: row.get(2)?,
                    identity_key: row.get(3)?,
                    safety_number: row.get(4)?,
                    verified: row.get::<_, i32>(5)? != 0,
                    blocked: row.get::<_, i32>(6)? != 0,
                    created_at: row.get(7)?,
                    last_seen: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(contacts)
    }

    // MARK: - Conversation Operations

    pub fn create_conversation(&self, conv: &NewConversation) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO conversations (type, peer_id, group_id, display_name, last_message_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                conv.conv_type,
                conv.peer_id,
                conv.group_id,
                conv.display_name,
                Utc::now().timestamp()
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_conversation(&self, id: i64) -> Result<Option<Conversation>> {
        self.conn
            .query_row(
                "SELECT id, type, peer_id, group_id, display_name, avatar, muted,
                        archived, last_message_id, last_message_at, unread_count, expires_in
                 FROM conversations WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Conversation {
                        id: row.get(0)?,
                        conv_type: row.get(1)?,
                        peer_id: row.get(2)?,
                        group_id: row.get(3)?,
                        display_name: row.get(4)?,
                        avatar: row.get(5)?,
                        muted: row.get::<_, i32>(6)? != 0,
                        archived: row.get::<_, i32>(7)? != 0,
                        last_message_id: row.get(8)?,
                        last_message_at: row.get(9)?,
                        unread_count: row.get(10)?,
                        expires_in: row.get(11)?,
                    })
                },
            )
            .optional()
            .context("Failed to get conversation")
    }

    pub fn list_conversations(&self) -> Result<Vec<Conversation>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, type, peer_id, group_id, display_name, avatar, muted,
                    archived, last_message_id, last_message_at, unread_count, expires_in
             FROM conversations
             WHERE archived = 0
             ORDER BY last_message_at DESC",
        )?;

        let conversations = stmt
            .query_map([], |row| {
                Ok(Conversation {
                    id: row.get(0)?,
                    conv_type: row.get(1)?,
                    peer_id: row.get(2)?,
                    group_id: row.get(3)?,
                    display_name: row.get(4)?,
                    avatar: row.get(5)?,
                    muted: row.get::<_, i32>(6)? != 0,
                    archived: row.get::<_, i32>(7)? != 0,
                    last_message_id: row.get(8)?,
                    last_message_at: row.get(9)?,
                    unread_count: row.get(10)?,
                    expires_in: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(conversations)
    }

    /// Count active (non-archived) conversations
    pub fn count_conversations(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM conversations WHERE archived = 0",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    // MARK: - Message Operations

    pub fn insert_message(&self, msg: &Message) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO messages (conversation_id, sender_peer_id, content_type, body,
                                   media_path, media_mime_type, media_size, timestamp,
                                   sent, delivered, read_by_me, expires_at, direction)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                msg.conversation_id,
                msg.sender_peer_id,
                msg.content_type,
                msg.body,
                msg.media_path,
                msg.media_mime_type,
                msg.media_size,
                msg.timestamp,
                msg.sent as i32,
                msg.delivered as i32,
                msg.read_by_me as i32,
                msg.expires_at,
                msg.direction,
            ],
        )?;

        let message_id = self.conn.last_insert_rowid();

        // Update conversation's last message
        self.conn.execute(
            "UPDATE conversations
             SET last_message_id = ?1, last_message_at = ?2
             WHERE id = ?3",
            params![message_id, msg.timestamp, msg.conversation_id],
        )?;

        // Increment unread count for incoming messages
        if msg.direction == "incoming" {
            self.conn.execute(
                "UPDATE conversations
                 SET unread_count = unread_count + 1
                 WHERE id = ?1",
                params![msg.conversation_id],
            )?;
        }

        Ok(message_id)
    }

    pub fn get_messages(
        &self,
        conversation_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Message>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, conversation_id, sender_peer_id, content_type, body,
                    media_path, media_mime_type, media_size, timestamp,
                    sent, delivered, read_by_me, expires_at, direction
             FROM messages
             WHERE conversation_id = ?1
             ORDER BY timestamp DESC
             LIMIT ?2 OFFSET ?3",
        )?;

        let messages = stmt
            .query_map(params![conversation_id, limit, offset], |row| {
                Ok(Message {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    sender_peer_id: row.get(2)?,
                    content_type: row.get(3)?,
                    body: row.get(4)?,
                    media_path: row.get(5)?,
                    media_mime_type: row.get(6)?,
                    media_size: row.get(7)?,
                    timestamp: row.get(8)?,
                    sent: row.get::<_, i32>(9)? != 0,
                    delivered: row.get::<_, i32>(10)? != 0,
                    read_by_me: row.get::<_, i32>(11)? != 0,
                    expires_at: row.get(12)?,
                    direction: row.get(13)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    pub fn mark_as_read(&self, conversation_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE messages
             SET read_by_me = 1
             WHERE conversation_id = ?1 AND direction = 'incoming' AND read_by_me = 0",
            params![conversation_id],
        )?;

        self.conn.execute(
            "UPDATE conversations
             SET unread_count = 0
             WHERE id = ?1",
            params![conversation_id],
        )?;

        Ok(())
    }

    /// Mark a specific message as sent
    pub fn mark_message_sent(&self, message_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE messages SET sent = 1 WHERE id = ?1",
            params![message_id],
        )?;
        Ok(())
    }

    /// Mark a specific message as delivered
    pub fn mark_message_delivered(&self, message_id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE messages SET delivered = 1 WHERE id = ?1",
            params![message_id],
        )?;
        Ok(())
    }

    // MARK: - Ratchet State Operations

    pub fn save_ratchet_state(&self, peer_id: &str, state_json: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO ratchet_states (peer_id, state_json, updated_at)
             VALUES (?1, ?2, ?3)",
            params![peer_id, state_json, Utc::now().timestamp()],
        )?;

        Ok(())
    }

    pub fn load_ratchet_state(&self, peer_id: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT state_json FROM ratchet_states WHERE peer_id = ?1",
                params![peer_id],
                |row| row.get(0),
            )
            .optional()
            .context("Failed to load ratchet state")
    }
}

// MARK: - Data Models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: i64,
    pub peer_id: String,
    pub display_name: Option<String>,
    pub identity_key: Vec<u8>,
    pub safety_number: String,
    pub verified: bool,
    pub blocked: bool,
    pub created_at: i64,
    pub last_seen: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: i64,
    pub conv_type: String, // "direct" or "group"
    pub peer_id: Option<String>,
    pub group_id: Option<String>,
    pub display_name: Option<String>,
    pub avatar: Option<Vec<u8>>,
    pub muted: bool,
    pub archived: bool,
    pub last_message_id: Option<i64>,
    pub last_message_at: Option<i64>,
    pub unread_count: i64,
    pub expires_in: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConversation {
    pub conv_type: String,
    pub peer_id: Option<String>,
    pub group_id: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    #[serde(default)]
    pub id: i64,
    pub conversation_id: i64,
    pub sender_peer_id: String,
    pub content_type: String, // "text", "media", "voice", "file"
    pub body: Option<String>,
    pub media_path: Option<String>,
    pub media_mime_type: Option<String>,
    pub media_size: Option<i64>,
    pub timestamp: i64,
    pub sent: bool,
    pub delivered: bool,
    pub read_by_me: bool,
    pub expires_at: Option<i64>,
    pub direction: String, // "incoming" or "outgoing"
}
