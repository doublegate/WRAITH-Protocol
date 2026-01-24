//! Content Reader
//!
//! Handles fetching and displaying published content from the network.
//! Provides content verification and caching.

use crate::database::{Article, ArticleStatus, Database};
use crate::error::{PublishError, PublishResult};
use crate::signatures::{ContentSigner, SignatureInfo, SignedContent};
use crate::storage::DhtStorage;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Reader configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReaderConfig {
    /// Cache fetched content locally
    pub cache_enabled: bool,
    /// Cache TTL in seconds
    pub cache_ttl: u64,
    /// Verify signatures on fetch
    pub verify_signatures: bool,
    /// Maximum content age to display (0 = no limit)
    pub max_content_age: u64,
}

impl Default for ReaderConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            cache_ttl: 3600, // 1 hour
            verify_signatures: true,
            max_content_age: 0, // No limit
        }
    }
}

/// Fetched article with verification status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchedArticle {
    /// Article data
    pub article: Article,
    /// Signature verification info
    pub signature_info: Option<SignatureInfo>,
    /// Whether content was fetched from cache
    pub from_cache: bool,
    /// Fetch timestamp
    pub fetched_at: i64,
}

/// Content reader for fetching and verifying articles
pub struct ContentReader {
    db: Arc<Database>,
    storage: Arc<DhtStorage>,
    config: ReaderConfig,
}

impl ContentReader {
    /// Create a new content reader
    pub fn new(db: Arc<Database>, storage: Arc<DhtStorage>, config: ReaderConfig) -> Self {
        Self {
            db,
            storage,
            config,
        }
    }

    /// Fetch an article by CID
    pub async fn fetch(&self, cid: &str) -> PublishResult<Option<FetchedArticle>> {
        // Check local database first (cache)
        if self.config.cache_enabled
            && let Some(article) = self.db.get_article_by_cid(cid)?
        {
            debug!("Found article in cache: {}", cid);
            return Ok(Some(FetchedArticle {
                article,
                signature_info: None, // Already verified when cached
                from_cache: true,
                fetched_at: Utc::now().timestamp(),
            }));
        }

        // Fetch from DHT storage
        let stored = match self.storage.retrieve(cid).await? {
            Some(s) => s,
            None => {
                warn!("Article not found in DHT: {}", cid);
                return Ok(None);
            }
        };

        // Verify signature if enabled
        let signature_info = if self.config.verify_signatures {
            match SignatureInfo::from_signed_content(&stored.signed_content) {
                Ok(info) => {
                    if !info.valid {
                        warn!("Invalid signature for CID: {}", cid);
                        return Err(PublishError::Crypto(
                            "Content signature verification failed".to_string(),
                        ));
                    }
                    Some(info)
                }
                Err(e) => {
                    warn!("Signature verification error for {}: {}", cid, e);
                    return Err(e);
                }
            }
        } else {
            None
        };

        // Parse article from content
        let article = self.parse_article(&stored.signed_content, cid)?;

        // Cache if enabled
        if self.config.cache_enabled
            && let Err(e) = self.cache_article(&article)
        {
            warn!("Failed to cache article {}: {}", cid, e);
        }

        info!("Fetched article from DHT: {}", cid);

        Ok(Some(FetchedArticle {
            article,
            signature_info,
            from_cache: false,
            fetched_at: Utc::now().timestamp(),
        }))
    }

    /// Fetch multiple articles by CIDs
    pub async fn fetch_many(
        &self,
        cids: &[String],
    ) -> Vec<(String, PublishResult<Option<FetchedArticle>>)> {
        let mut results = Vec::with_capacity(cids.len());

        for cid in cids {
            let result = self.fetch(cid).await;
            results.push((cid.clone(), result));
        }

        results
    }

    /// Verify an article's content integrity
    pub fn verify(&self, article: &Article) -> PublishResult<bool> {
        let cid = article
            .cid
            .as_ref()
            .ok_or_else(|| PublishError::InvalidContent("Article has no CID".to_string()))?;

        // Serialize article content for verification
        let content_bytes = serde_json::to_vec(&ArticleContent {
            title: article.title.clone(),
            content: article.content.clone(),
            author_id: article.author_id.clone(),
            author_name: article.author_name.clone(),
            tags: article.tags.clone(),
        })?;

        Ok(ContentSigner::verify_cid(&content_bytes, cid))
    }

    /// Get recently fetched articles
    pub fn get_recent(&self, limit: usize) -> PublishResult<Vec<Article>> {
        // For now, return published articles from local DB
        let all = self.db.list_published()?;
        Ok(all.into_iter().take(limit).collect())
    }

    /// Search articles by query
    pub fn search(&self, query: &str, limit: usize) -> PublishResult<Vec<Article>> {
        self.db.search_articles(query, limit).map_err(|e| e.into())
    }

    /// Parse article from signed content
    fn parse_article(&self, signed: &SignedContent, cid: &str) -> PublishResult<Article> {
        let content: ArticleContent = serde_json::from_slice(&signed.content)?;

        Ok(Article {
            id: uuid::Uuid::new_v4().to_string(),
            cid: Some(cid.to_string()),
            title: content.title,
            content: content.content,
            author_id: content.author_id,
            author_name: content.author_name,
            created_at: signed.signed_at,
            updated_at: signed.signed_at,
            published_at: Some(signed.signed_at),
            tags: content.tags,
            image_url: None,
            status: ArticleStatus::Published,
        })
    }

    /// Cache article in local database
    fn cache_article(&self, article: &Article) -> PublishResult<()> {
        // Check if already cached
        if let Some(cid) = &article.cid
            && self.db.get_article_by_cid(cid)?.is_some()
        {
            return Ok(()); // Already cached
        }

        self.db.create_article(article)?;
        Ok(())
    }

    /// Clear cache for a specific CID
    pub fn clear_cache(&self, cid: &str) -> PublishResult<bool> {
        if let Some(article) = self.db.get_article_by_cid(cid)? {
            self.db.delete_article(&article.id)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Clear all cached articles
    pub fn clear_all_cache(&self) -> PublishResult<usize> {
        let articles = self.db.list_published()?;
        let mut count = 0;

        for article in articles {
            // Only clear articles we don't own
            // (In practice, we'd check against local identity)
            if let Err(e) = self.db.delete_article(&article.id) {
                warn!("Failed to delete cached article {}: {}", article.id, e);
            } else {
                count += 1;
            }
        }

        Ok(count)
    }
}

/// Article content for serialization (subset of Article)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArticleContent {
    title: String,
    content: String,
    author_id: String,
    author_name: Option<String>,
    tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::ContentSigner;
    use crate::storage::StorageConfig;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use tempfile::tempdir;

    fn setup() -> (ContentReader, Arc<DhtStorage>) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        let storage = Arc::new(DhtStorage::new(StorageConfig::default()));
        let reader = ContentReader::new(db, storage.clone(), ReaderConfig::default());
        (reader, storage)
    }

    #[tokio::test]
    async fn test_fetch_nonexistent() {
        let (reader, _storage) = setup();

        let result = reader.fetch("nonexistent-cid").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_store_and_fetch() {
        let (reader, storage) = setup();

        // Create and store signed content
        let signing_key = SigningKey::generate(&mut OsRng);
        let signer = ContentSigner::new(Some(signing_key));

        let content = ArticleContent {
            title: "Test Article".to_string(),
            content: "# Hello\n\nThis is test content.".to_string(),
            author_id: "test-author".to_string(),
            author_name: Some("Test Author".to_string()),
            tags: vec!["test".to_string()],
        };

        let content_bytes = serde_json::to_vec(&content).unwrap();
        let signed = signer.sign(&content_bytes).unwrap();
        let cid = signed.cid.clone();

        storage.store(signed).await.unwrap();

        // Fetch
        let fetched = reader.fetch(&cid).await.unwrap().unwrap();
        assert_eq!(fetched.article.title, "Test Article");
        assert!(fetched.signature_info.is_some());
        assert!(fetched.signature_info.unwrap().valid);
    }

    #[tokio::test]
    async fn test_caching() {
        let (reader, storage) = setup();

        // Store content
        let signing_key = SigningKey::generate(&mut OsRng);
        let signer = ContentSigner::new(Some(signing_key));

        let content = ArticleContent {
            title: "Cached Article".to_string(),
            content: "Content".to_string(),
            author_id: "author".to_string(),
            author_name: None,
            tags: vec![],
        };

        let content_bytes = serde_json::to_vec(&content).unwrap();
        let signed = signer.sign(&content_bytes).unwrap();
        let cid = signed.cid.clone();

        storage.store(signed).await.unwrap();

        // First fetch (from DHT)
        let fetched1 = reader.fetch(&cid).await.unwrap().unwrap();
        assert!(!fetched1.from_cache);

        // Second fetch (from cache)
        let fetched2 = reader.fetch(&cid).await.unwrap().unwrap();
        assert!(fetched2.from_cache);
    }

    #[test]
    fn test_search() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        let storage = Arc::new(DhtStorage::new(StorageConfig::default()));
        let reader = ContentReader::new(db.clone(), storage, ReaderConfig::default());

        // Create searchable articles
        let article1 = Article {
            id: "article-1".to_string(),
            cid: Some("cid-1".to_string()),
            title: "Rust Programming Guide".to_string(),
            content: "Learn Rust programming language".to_string(),
            author_id: "author".to_string(),
            author_name: None,
            created_at: 0,
            updated_at: 0,
            published_at: Some(0),
            tags: vec!["rust".to_string()],
            image_url: None,
            status: ArticleStatus::Published,
        };

        db.create_article(&article1).unwrap();

        // Search
        let results = reader.search("Rust", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Rust Programming Guide");
    }
}
