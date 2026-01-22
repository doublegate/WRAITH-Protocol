//! Content Management
//!
//! Manages article creation, updates, and deletion.

use crate::database::{Article, ArticleStatus, Database};
use crate::error::{PublishError, PublishResult};
use crate::state::AppState;
use chrono::Utc;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Content manager for articles
pub struct ContentManager {
    db: Arc<Database>,
    state: Arc<AppState>,
}

impl ContentManager {
    /// Create a new content manager
    pub fn new(db: Arc<Database>, state: Arc<AppState>) -> Self {
        Self { db, state }
    }

    /// Create a new draft article
    pub fn create_draft(
        &self,
        title: &str,
        content: &str,
        tags: Vec<String>,
    ) -> PublishResult<Article> {
        let peer_id = self
            .state
            .get_peer_id()
            .ok_or_else(|| PublishError::Config("Identity not initialized".to_string()))?;

        let author_name = self.state.get_display_name();
        let now = Utc::now().timestamp();

        let article = Article {
            id: Uuid::new_v4().to_string(),
            cid: None,
            title: title.to_string(),
            content: content.to_string(),
            author_id: peer_id,
            author_name: Some(author_name),
            created_at: now,
            updated_at: now,
            published_at: None,
            tags,
            image_url: None,
            status: ArticleStatus::Draft,
        };

        self.db.create_article(&article)?;
        info!("Created draft: {} ({})", article.title, article.id);

        Ok(article)
    }

    /// Update a draft article
    pub fn update_draft(
        &self,
        id: &str,
        title: &str,
        content: &str,
        tags: Vec<String>,
        image_url: Option<String>,
    ) -> PublishResult<Article> {
        let mut article = self
            .db
            .get_article(id)?
            .ok_or_else(|| PublishError::DraftNotFound(id.to_string()))?;

        // Only allow updating drafts
        if article.status != ArticleStatus::Draft {
            return Err(PublishError::InvalidContent(
                "Cannot update a published article directly. Use update_published instead."
                    .to_string(),
            ));
        }

        article.title = title.to_string();
        article.content = content.to_string();
        article.tags = tags;
        article.image_url = image_url;
        article.updated_at = Utc::now().timestamp();

        self.db.update_article(&article)?;
        info!("Updated draft: {} ({})", article.title, article.id);

        Ok(article)
    }

    /// Delete a draft article
    pub fn delete_draft(&self, id: &str) -> PublishResult<()> {
        let article = self
            .db
            .get_article(id)?
            .ok_or_else(|| PublishError::DraftNotFound(id.to_string()))?;

        // Only allow deleting drafts
        if article.status == ArticleStatus::Published {
            return Err(PublishError::InvalidContent(
                "Cannot delete a published article. Unpublish it first.".to_string(),
            ));
        }

        self.db.delete_article(id)?;
        info!("Deleted draft: {} ({})", article.title, id);

        Ok(())
    }

    /// List all drafts
    pub fn list_drafts(&self) -> PublishResult<Vec<Article>> {
        Ok(self.db.list_drafts()?)
    }

    /// Get an article by ID
    pub fn get_article(&self, id: &str) -> PublishResult<Option<Article>> {
        Ok(self.db.get_article(id)?)
    }

    /// Get an article by CID
    pub fn get_article_by_cid(&self, cid: &str) -> PublishResult<Option<Article>> {
        Ok(self.db.get_article_by_cid(cid)?)
    }

    /// Archive an article (soft delete)
    pub fn archive_article(&self, id: &str) -> PublishResult<Article> {
        let mut article = self
            .db
            .get_article(id)?
            .ok_or_else(|| PublishError::ArticleNotFound(id.to_string()))?;

        article.status = ArticleStatus::Archived;
        article.updated_at = Utc::now().timestamp();

        self.db.update_article(&article)?;
        info!("Archived article: {} ({})", article.title, id);

        Ok(article)
    }

    /// Restore an archived article to draft
    pub fn restore_article(&self, id: &str) -> PublishResult<Article> {
        let mut article = self
            .db
            .get_article(id)?
            .ok_or_else(|| PublishError::ArticleNotFound(id.to_string()))?;

        if article.status != ArticleStatus::Archived {
            return Err(PublishError::InvalidContent(
                "Article is not archived".to_string(),
            ));
        }

        article.status = ArticleStatus::Draft;
        article.updated_at = Utc::now().timestamp();

        self.db.update_article(&article)?;
        info!("Restored article to draft: {} ({})", article.title, id);

        Ok(article)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup() -> (ContentManager, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        let db = Arc::new(db);
        let state = Arc::new(AppState::new(
            Database::open(&db_path).unwrap(),
            dir.path().to_path_buf(),
        ));
        state.initialize().unwrap();

        let manager = ContentManager::new(db, state);
        (manager, dir)
    }

    #[test]
    fn test_create_and_get_draft() {
        let (manager, _dir) = setup();

        let article = manager
            .create_draft(
                "Test Article",
                "# Hello\n\nThis is content.",
                vec!["test".to_string()],
            )
            .unwrap();

        assert_eq!(article.title, "Test Article");
        assert_eq!(article.status, ArticleStatus::Draft);

        let retrieved = manager.get_article(&article.id).unwrap().unwrap();
        assert_eq!(retrieved.id, article.id);
    }

    #[test]
    fn test_update_draft() {
        let (manager, _dir) = setup();

        let article = manager
            .create_draft("Original Title", "Original content", vec![])
            .unwrap();

        let updated = manager
            .update_draft(
                &article.id,
                "Updated Title",
                "Updated content",
                vec!["new-tag".to_string()],
                None,
            )
            .unwrap();

        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.tags, vec!["new-tag".to_string()]);
    }

    #[test]
    fn test_delete_draft() {
        let (manager, _dir) = setup();

        let article = manager
            .create_draft("To Delete", "Content", vec![])
            .unwrap();

        manager.delete_draft(&article.id).unwrap();

        let retrieved = manager.get_article(&article.id).unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_list_drafts() {
        let (manager, _dir) = setup();

        manager
            .create_draft("Draft 1", "Content 1", vec![])
            .unwrap();
        manager
            .create_draft("Draft 2", "Content 2", vec![])
            .unwrap();

        let drafts = manager.list_drafts().unwrap();
        assert_eq!(drafts.len(), 2);
    }

    #[test]
    fn test_archive_and_restore() {
        let (manager, _dir) = setup();

        let article = manager
            .create_draft("To Archive", "Content", vec![])
            .unwrap();

        let archived = manager.archive_article(&article.id).unwrap();
        assert_eq!(archived.status, ArticleStatus::Archived);

        let restored = manager.restore_article(&article.id).unwrap();
        assert_eq!(restored.status, ArticleStatus::Draft);
    }
}
