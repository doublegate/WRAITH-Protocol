//! SQLite Database for WRAITH Publish
//!
//! Manages articles, drafts, images, and full-text search.

use anyhow::{Context, Result};
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Database connection manager for publish data
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open or create the publish database
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

        // Articles table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS articles (
                id TEXT PRIMARY KEY,
                cid TEXT,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                author_id TEXT NOT NULL,
                author_name TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                published_at INTEGER,
                tags TEXT,
                image_url TEXT,
                status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published', 'archived'))
            )",
            [],
        )?;

        // Full-text search virtual table
        conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS articles_fts USING fts5(
                title,
                content,
                tags,
                content='articles',
                content_rowid='rowid'
            )",
            [],
        )?;

        // Triggers to keep FTS in sync
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS articles_ai AFTER INSERT ON articles BEGIN
                INSERT INTO articles_fts(rowid, title, content, tags)
                VALUES (new.rowid, new.title, new.content, new.tags);
            END",
            [],
        )?;

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS articles_ad AFTER DELETE ON articles BEGIN
                INSERT INTO articles_fts(articles_fts, rowid, title, content, tags)
                VALUES ('delete', old.rowid, old.title, old.content, old.tags);
            END",
            [],
        )?;

        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS articles_au AFTER UPDATE ON articles BEGIN
                INSERT INTO articles_fts(articles_fts, rowid, title, content, tags)
                VALUES ('delete', old.rowid, old.title, old.content, old.tags);
                INSERT INTO articles_fts(rowid, title, content, tags)
                VALUES (new.rowid, new.title, new.content, new.tags);
            END",
            [],
        )?;

        // Images table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS images (
                cid TEXT PRIMARY KEY,
                data BLOB NOT NULL,
                mime_type TEXT NOT NULL,
                uploaded_at INTEGER NOT NULL
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

        // Settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // Create indexes
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_articles_status
             ON articles(status)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_articles_author
             ON articles(author_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_articles_published
             ON articles(published_at DESC)",
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
    // Article Operations
    // =========================================================================

    /// Create a new article (draft)
    pub fn create_article(&self, article: &Article) -> Result<()> {
        let conn = self.conn.lock();
        let tags_json = serde_json::to_string(&article.tags)?;

        conn.execute(
            "INSERT INTO articles (id, cid, title, content, author_id, author_name, created_at, updated_at, published_at, tags, image_url, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                article.id,
                article.cid,
                article.title,
                article.content,
                article.author_id,
                article.author_name,
                article.created_at,
                article.updated_at,
                article.published_at,
                tags_json,
                article.image_url,
                article.status.as_str(),
            ],
        )?;
        Ok(())
    }

    /// Get an article by ID
    pub fn get_article(&self, id: &str) -> Result<Option<Article>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, cid, title, content, author_id, author_name, created_at, updated_at, published_at, tags, image_url, status
             FROM articles WHERE id = ?1",
            params![id],
            |row| {
                let tags_json: String = row.get(9)?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                let status_str: String = row.get(11)?;

                Ok(Article {
                    id: row.get(0)?,
                    cid: row.get(1)?,
                    title: row.get(2)?,
                    content: row.get(3)?,
                    author_id: row.get(4)?,
                    author_name: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    published_at: row.get(8)?,
                    tags,
                    image_url: row.get(10)?,
                    status: ArticleStatus::parse(&status_str),
                })
            },
        )
        .optional()
        .context("Failed to get article")
    }

    /// Get an article by CID
    pub fn get_article_by_cid(&self, cid: &str) -> Result<Option<Article>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, cid, title, content, author_id, author_name, created_at, updated_at, published_at, tags, image_url, status
             FROM articles WHERE cid = ?1",
            params![cid],
            |row| {
                let tags_json: String = row.get(9)?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                let status_str: String = row.get(11)?;

                Ok(Article {
                    id: row.get(0)?,
                    cid: row.get(1)?,
                    title: row.get(2)?,
                    content: row.get(3)?,
                    author_id: row.get(4)?,
                    author_name: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    published_at: row.get(8)?,
                    tags,
                    image_url: row.get(10)?,
                    status: ArticleStatus::parse(&status_str),
                })
            },
        )
        .optional()
        .context("Failed to get article by CID")
    }

    /// Update an article
    pub fn update_article(&self, article: &Article) -> Result<()> {
        let conn = self.conn.lock();
        let tags_json = serde_json::to_string(&article.tags)?;

        conn.execute(
            "UPDATE articles SET cid = ?1, title = ?2, content = ?3, author_name = ?4, updated_at = ?5, published_at = ?6, tags = ?7, image_url = ?8, status = ?9
             WHERE id = ?10",
            params![
                article.cid,
                article.title,
                article.content,
                article.author_name,
                article.updated_at,
                article.published_at,
                tags_json,
                article.image_url,
                article.status.as_str(),
                article.id,
            ],
        )?;
        Ok(())
    }

    /// Delete an article
    pub fn delete_article(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM articles WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// List drafts
    pub fn list_drafts(&self) -> Result<Vec<Article>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, cid, title, content, author_id, author_name, created_at, updated_at, published_at, tags, image_url, status
             FROM articles WHERE status = 'draft' ORDER BY updated_at DESC",
        )?;

        let articles = stmt
            .query_map([], |row| {
                let tags_json: String = row.get(9)?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                let status_str: String = row.get(11)?;

                Ok(Article {
                    id: row.get(0)?,
                    cid: row.get(1)?,
                    title: row.get(2)?,
                    content: row.get(3)?,
                    author_id: row.get(4)?,
                    author_name: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    published_at: row.get(8)?,
                    tags,
                    image_url: row.get(10)?,
                    status: ArticleStatus::parse(&status_str),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(articles)
    }

    /// List published articles
    pub fn list_published(&self) -> Result<Vec<Article>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, cid, title, content, author_id, author_name, created_at, updated_at, published_at, tags, image_url, status
             FROM articles WHERE status = 'published' ORDER BY published_at DESC",
        )?;

        let articles = stmt
            .query_map([], |row| {
                let tags_json: String = row.get(9)?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                let status_str: String = row.get(11)?;

                Ok(Article {
                    id: row.get(0)?,
                    cid: row.get(1)?,
                    title: row.get(2)?,
                    content: row.get(3)?,
                    author_id: row.get(4)?,
                    author_name: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    published_at: row.get(8)?,
                    tags,
                    image_url: row.get(10)?,
                    status: ArticleStatus::parse(&status_str),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(articles)
    }

    /// Full-text search articles
    pub fn search_articles(&self, query: &str, limit: usize) -> Result<Vec<Article>> {
        let conn = self.conn.lock();

        // FTS5 search
        let mut stmt = conn.prepare(
            "SELECT a.id, a.cid, a.title, a.content, a.author_id, a.author_name, a.created_at, a.updated_at, a.published_at, a.tags, a.image_url, a.status
             FROM articles a
             INNER JOIN articles_fts fts ON a.rowid = fts.rowid
             WHERE articles_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let articles = stmt
            .query_map(params![query, limit as i64], |row| {
                let tags_json: String = row.get(9)?;
                let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
                let status_str: String = row.get(11)?;

                Ok(Article {
                    id: row.get(0)?,
                    cid: row.get(1)?,
                    title: row.get(2)?,
                    content: row.get(3)?,
                    author_id: row.get(4)?,
                    author_name: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    published_at: row.get(8)?,
                    tags,
                    image_url: row.get(10)?,
                    status: ArticleStatus::parse(&status_str),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(articles)
    }

    // =========================================================================
    // Image Operations
    // =========================================================================

    /// Store an image
    pub fn store_image(&self, cid: &str, data: &[u8], mime_type: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO images (cid, data, mime_type, uploaded_at) VALUES (?1, ?2, ?3, ?4)",
            params![cid, data, mime_type, Utc::now().timestamp()],
        )?;
        Ok(())
    }

    /// Get an image by CID
    pub fn get_image(&self, cid: &str) -> Result<Option<Image>> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT cid, data, mime_type, uploaded_at FROM images WHERE cid = ?1",
            params![cid],
            |row| {
                Ok(Image {
                    cid: row.get(0)?,
                    data: row.get(1)?,
                    mime_type: row.get(2)?,
                    uploaded_at: row.get(3)?,
                })
            },
        )
        .optional()
        .context("Failed to get image")
    }

    /// Delete an image
    pub fn delete_image(&self, cid: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM images WHERE cid = ?1", params![cid])?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArticleStatus {
    Draft,
    Published,
    Archived,
}

impl ArticleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ArticleStatus::Draft => "draft",
            ArticleStatus::Published => "published",
            ArticleStatus::Archived => "archived",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "published" => ArticleStatus::Published,
            "archived" => ArticleStatus::Archived,
            _ => ArticleStatus::Draft,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub id: String,
    pub cid: Option<String>,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub author_name: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub published_at: Option<i64>,
    pub tags: Vec<String>,
    pub image_url: Option<String>,
    pub status: ArticleStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Image {
    pub cid: String,
    pub data: Vec<u8>,
    pub mime_type: String,
    pub uploaded_at: i64,
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
        let drafts = db.list_drafts().unwrap();
        assert!(drafts.is_empty());
    }

    #[test]
    fn test_article_crud() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let article = Article {
            id: "test-article-id".to_string(),
            cid: None,
            title: "Test Article".to_string(),
            content: "# Hello World\n\nThis is a test.".to_string(),
            author_id: "peer-123".to_string(),
            author_name: Some("Test Author".to_string()),
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            published_at: None,
            tags: vec!["test".to_string(), "example".to_string()],
            image_url: None,
            status: ArticleStatus::Draft,
        };

        // Create
        db.create_article(&article).unwrap();

        // Read
        let retrieved = db.get_article(&article.id).unwrap().unwrap();
        assert_eq!(retrieved.title, article.title);
        assert_eq!(retrieved.tags, article.tags);
        assert_eq!(retrieved.status, ArticleStatus::Draft);

        // Update
        let mut updated = retrieved.clone();
        updated.title = "Updated Title".to_string();
        updated.status = ArticleStatus::Published;
        updated.published_at = Some(Utc::now().timestamp());
        db.update_article(&updated).unwrap();

        let retrieved = db.get_article(&article.id).unwrap().unwrap();
        assert_eq!(retrieved.title, "Updated Title");
        assert_eq!(retrieved.status, ArticleStatus::Published);

        // Delete
        db.delete_article(&article.id).unwrap();
        let retrieved = db.get_article(&article.id).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_list_drafts_and_published() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Create a draft
        let draft = Article {
            id: "draft-1".to_string(),
            cid: None,
            title: "Draft Article".to_string(),
            content: "Draft content".to_string(),
            author_id: "peer-123".to_string(),
            author_name: None,
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            published_at: None,
            tags: vec![],
            image_url: None,
            status: ArticleStatus::Draft,
        };
        db.create_article(&draft).unwrap();

        // Create a published article
        let published = Article {
            id: "published-1".to_string(),
            cid: Some("cid-123".to_string()),
            title: "Published Article".to_string(),
            content: "Published content".to_string(),
            author_id: "peer-123".to_string(),
            author_name: None,
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            published_at: Some(Utc::now().timestamp()),
            tags: vec![],
            image_url: None,
            status: ArticleStatus::Published,
        };
        db.create_article(&published).unwrap();

        // List drafts
        let drafts = db.list_drafts().unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].id, "draft-1");

        // List published
        let published_list = db.list_published().unwrap();
        assert_eq!(published_list.len(), 1);
        assert_eq!(published_list[0].id, "published-1");
    }

    #[test]
    fn test_image_storage() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let cid = "image-cid-123";
        let data = b"fake image data";
        let mime_type = "image/png";

        // Store
        db.store_image(cid, data, mime_type).unwrap();

        // Retrieve
        let image = db.get_image(cid).unwrap().unwrap();
        assert_eq!(image.cid, cid);
        assert_eq!(image.data, data);
        assert_eq!(image.mime_type, mime_type);

        // Delete
        db.delete_image(cid).unwrap();
        let image = db.get_image(cid).unwrap();
        assert!(image.is_none());
    }

    #[test]
    fn test_full_text_search() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Create articles with different content
        let article1 = Article {
            id: "article-1".to_string(),
            cid: None,
            title: "Rust Programming".to_string(),
            content: "Rust is a systems programming language".to_string(),
            author_id: "peer-123".to_string(),
            author_name: None,
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            published_at: None,
            tags: vec!["rust".to_string(), "programming".to_string()],
            image_url: None,
            status: ArticleStatus::Draft,
        };
        db.create_article(&article1).unwrap();

        let article2 = Article {
            id: "article-2".to_string(),
            cid: None,
            title: "Python Basics".to_string(),
            content: "Python is a high-level programming language".to_string(),
            author_id: "peer-123".to_string(),
            author_name: None,
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            published_at: None,
            tags: vec!["python".to_string(), "programming".to_string()],
            image_url: None,
            status: ArticleStatus::Draft,
        };
        db.create_article(&article2).unwrap();

        // Search for "Rust"
        let results = db.search_articles("Rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "article-1");

        // Search for "programming"
        let results = db.search_articles("programming", 10).unwrap();
        assert_eq!(results.len(), 2);
    }
}
