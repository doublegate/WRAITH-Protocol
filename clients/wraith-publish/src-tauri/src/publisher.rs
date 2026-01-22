//! Publisher Module
//!
//! Handles publishing articles to the DHT network.

use crate::database::{Article, ArticleStatus, Database};
use crate::error::{PublishError, PublishResult};
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// Published article metadata stored in DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedArticle {
    pub cid: String,
    pub title: String,
    pub content: String,
    pub author_id: String,
    pub author_name: Option<String>,
    pub published_at: i64,
    pub tags: Vec<String>,
    pub image_url: Option<String>,
}

/// Publisher for distributing articles
pub struct Publisher {
    db: Arc<Database>,
    state: Arc<AppState>,
}

impl Publisher {
    /// Create a new publisher
    pub fn new(db: Arc<Database>, state: Arc<AppState>) -> Self {
        Self { db, state }
    }

    /// Publish an article to the DHT
    pub fn publish(&self, article_id: &str) -> PublishResult<String> {
        let mut article = self
            .db
            .get_article(article_id)?
            .ok_or_else(|| PublishError::ArticleNotFound(article_id.to_string()))?;

        // Verify the article belongs to this user
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| PublishError::Config("Identity not initialized".to_string()))?;

        if article.author_id != peer_id {
            return Err(PublishError::Publishing(
                "Cannot publish article owned by another author".to_string(),
            ));
        }

        // Create the published article payload
        let published = PublishedArticle {
            cid: String::new(), // Will be set after hashing
            title: article.title.clone(),
            content: article.content.clone(),
            author_id: article.author_id.clone(),
            author_name: article.author_name.clone(),
            published_at: Utc::now().timestamp(),
            tags: article.tags.clone(),
            image_url: article.image_url.clone(),
        };

        // Serialize and compute CID (content hash)
        let payload = serde_json::to_vec(&published)?;
        let cid = hex::encode(&blake3::hash(&payload).as_bytes()[..32]);

        // Update article with CID and published status
        article.cid = Some(cid.clone());
        article.status = ArticleStatus::Published;
        article.published_at = Some(Utc::now().timestamp());
        article.updated_at = Utc::now().timestamp();

        self.db.update_article(&article)?;

        // In a full implementation, we would store in DHT here:
        // self.dht.store(&format!("article:{}", cid), payload).await?;

        info!("Published article: {} with CID: {}", article.title, cid);

        Ok(cid)
    }

    /// Unpublish an article (remove from DHT)
    pub fn unpublish(&self, cid: &str) -> PublishResult<()> {
        let mut article = self
            .db
            .get_article_by_cid(cid)?
            .ok_or_else(|| PublishError::ArticleNotFound(cid.to_string()))?;

        // Verify the article belongs to this user
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| PublishError::Config("Identity not initialized".to_string()))?;

        if article.author_id != peer_id {
            return Err(PublishError::Publishing(
                "Cannot unpublish article owned by another author".to_string(),
            ));
        }

        // Update status to draft
        article.status = ArticleStatus::Draft;
        article.cid = None;
        article.published_at = None;
        article.updated_at = Utc::now().timestamp();

        self.db.update_article(&article)?;

        // In a full implementation, we would remove from DHT here:
        // self.dht.remove(&format!("article:{}", cid)).await?;

        info!("Unpublished article: {} (CID: {})", article.title, cid);

        Ok(())
    }

    /// Update a published article (creates new CID)
    pub fn update_published(&self, cid: &str) -> PublishResult<String> {
        let article = self
            .db
            .get_article_by_cid(cid)?
            .ok_or_else(|| PublishError::ArticleNotFound(cid.to_string()))?;

        // Verify the article belongs to this user
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| PublishError::Config("Identity not initialized".to_string()))?;

        if article.author_id != peer_id {
            return Err(PublishError::Publishing(
                "Cannot update article owned by another author".to_string(),
            ));
        }

        // Re-publish with new content (will get a new CID)
        self.publish(&article.id)
    }

    /// Fetch an article from DHT by CID
    pub fn fetch_article(&self, cid: &str) -> PublishResult<Option<Article>> {
        // First check local database
        if let Some(article) = self.db.get_article_by_cid(cid)? {
            return Ok(Some(article));
        }

        // In a full implementation, we would fetch from DHT here:
        // let payload = self.dht.get(&format!("article:{}", cid)).await?;
        // let published: PublishedArticle = serde_json::from_slice(&payload)?;

        // For now, return None for remote articles
        Ok(None)
    }

    /// List all published articles by the local user
    pub fn list_published(&self) -> PublishResult<Vec<Article>> {
        Ok(self.db.list_published()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::ContentManager;
    use tempfile::tempdir;

    fn setup() -> (Publisher, ContentManager, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let db = Arc::new(db);
        let state = Arc::new(AppState::new(
            Database::open(&db_path).unwrap(),
            dir.path().to_path_buf(),
        ));
        state.initialize().unwrap();

        let publisher = Publisher::new(db.clone(), state.clone());
        let content_manager = ContentManager::new(db, state);
        (publisher, content_manager, dir)
    }

    #[test]
    fn test_publish_article() {
        let (publisher, content_manager, _dir) = setup();

        // Create a draft
        let draft = content_manager
            .create_draft("Test Article", "Content here", vec!["test".to_string()])
            .unwrap();

        // Publish it
        let cid = publisher.publish(&draft.id).unwrap();
        assert!(!cid.is_empty());

        // Verify it's published
        let article = content_manager.get_article(&draft.id).unwrap().unwrap();
        assert_eq!(article.status, ArticleStatus::Published);
        assert_eq!(article.cid, Some(cid));
    }

    #[test]
    fn test_unpublish_article() {
        let (publisher, content_manager, _dir) = setup();

        // Create and publish
        let draft = content_manager
            .create_draft("Test Article", "Content", vec![])
            .unwrap();
        let cid = publisher.publish(&draft.id).unwrap();

        // Unpublish
        publisher.unpublish(&cid).unwrap();

        // Verify it's a draft again
        let article = content_manager.get_article(&draft.id).unwrap().unwrap();
        assert_eq!(article.status, ArticleStatus::Draft);
        assert!(article.cid.is_none());
    }

    #[test]
    fn test_list_published() {
        let (publisher, content_manager, _dir) = setup();

        // Create and publish two articles
        let draft1 = content_manager
            .create_draft("Article 1", "Content 1", vec![])
            .unwrap();
        let draft2 = content_manager
            .create_draft("Article 2", "Content 2", vec![])
            .unwrap();

        publisher.publish(&draft1.id).unwrap();
        publisher.publish(&draft2.id).unwrap();

        // List published
        let published = publisher.list_published().unwrap();
        assert_eq!(published.len(), 2);
    }

    #[test]
    fn test_fetch_article() {
        let (publisher, content_manager, _dir) = setup();

        // Create and publish
        let draft = content_manager
            .create_draft("Test Article", "Content", vec![])
            .unwrap();
        let cid = publisher.publish(&draft.id).unwrap();

        // Fetch by CID
        let article = publisher.fetch_article(&cid).unwrap().unwrap();
        assert_eq!(article.title, "Test Article");
    }
}
