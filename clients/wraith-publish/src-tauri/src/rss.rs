//! RSS Feed Generation
//!
//! Generates RSS feeds for published articles, enabling syndication
//! and feed reader compatibility.

use crate::database::{Article, Database};
use crate::error::PublishResult;
use crate::markdown::MarkdownProcessor;
use chrono::{DateTime, TimeZone, Utc};
use rss::{Channel, ChannelBuilder, Guid, Item, ItemBuilder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// RSS feed configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedConfig {
    /// Feed title
    pub title: String,
    /// Feed description
    pub description: String,
    /// Base URL for article links
    pub base_url: String,
    /// Feed language (e.g., "en-us")
    pub language: String,
    /// Maximum items in feed
    pub max_items: usize,
    /// Include full content or excerpt only
    pub full_content: bool,
    /// Excerpt length for summary
    pub excerpt_length: usize,
}

impl Default for FeedConfig {
    fn default() -> Self {
        Self {
            title: "WRAITH Publish Feed".to_string(),
            description: "Decentralized, censorship-resistant content".to_string(),
            base_url: "wraith://publish".to_string(),
            language: "en-us".to_string(),
            max_items: 50,
            full_content: false,
            excerpt_length: 300,
        }
    }
}

/// RSS feed generator
pub struct FeedGenerator {
    db: Arc<Database>,
    config: FeedConfig,
    markdown: MarkdownProcessor,
}

impl FeedGenerator {
    /// Create a new feed generator
    pub fn new(db: Arc<Database>, config: FeedConfig) -> Self {
        Self {
            db,
            config,
            markdown: MarkdownProcessor::new(),
        }
    }

    /// Generate RSS feed for all published articles
    pub fn generate(&self) -> PublishResult<String> {
        let articles = self.db.list_published()?;
        self.generate_from_articles(&articles)
    }

    /// Generate RSS feed for articles by author
    pub fn generate_for_author(&self, author_id: &str) -> PublishResult<String> {
        let all_articles = self.db.list_published()?;
        let articles: Vec<Article> = all_articles
            .into_iter()
            .filter(|a| a.author_id == author_id)
            .collect();
        self.generate_from_articles(&articles)
    }

    /// Generate RSS feed for articles with specific tag
    pub fn generate_for_tag(&self, tag: &str) -> PublishResult<String> {
        let all_articles = self.db.list_published()?;
        let articles: Vec<Article> = all_articles
            .into_iter()
            .filter(|a| a.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)))
            .collect();
        self.generate_from_articles(&articles)
    }

    /// Generate RSS feed from a list of articles
    fn generate_from_articles(&self, articles: &[Article]) -> PublishResult<String> {
        let items: Vec<Item> = articles
            .iter()
            .take(self.config.max_items)
            .filter_map(|article| self.article_to_item(article).ok())
            .collect();

        let channel = ChannelBuilder::default()
            .title(&self.config.title)
            .description(&self.config.description)
            .link(&self.config.base_url)
            .language(Some(self.config.language.clone()))
            .items(items)
            .build();

        Ok(channel.to_string())
    }

    /// Convert an article to an RSS item
    fn article_to_item(&self, article: &Article) -> PublishResult<Item> {
        let link = if let Some(cid) = &article.cid {
            format!("{}/article/{}", self.config.base_url, cid)
        } else {
            format!("{}/article/{}", self.config.base_url, article.id)
        };

        let content = if self.config.full_content {
            self.markdown.to_html(&article.content)
        } else {
            self.markdown
                .extract_excerpt(&article.content, self.config.excerpt_length)
        };

        let pub_date = article
            .published_at
            .map(timestamp_to_rfc2822)
            .unwrap_or_default();

        let guid = Guid {
            value: article.cid.clone().unwrap_or_else(|| article.id.clone()),
            permalink: false,
        };

        let item = ItemBuilder::default()
            .title(Some(article.title.clone()))
            .link(Some(link))
            .description(Some(content))
            .author(article.author_name.clone())
            .guid(Some(guid))
            .pub_date(Some(pub_date))
            .build();

        Ok(item)
    }

    /// Get feed configuration
    pub fn config(&self) -> &FeedConfig {
        &self.config
    }

    /// Update feed configuration
    pub fn set_config(&mut self, config: FeedConfig) {
        self.config = config;
    }
}

/// Convert Unix timestamp to RFC 2822 date format
fn timestamp_to_rfc2822(timestamp: i64) -> String {
    Utc.timestamp_opt(timestamp, 0)
        .single()
        .map(|dt: DateTime<Utc>| dt.to_rfc2822())
        .unwrap_or_default()
}

/// Feed info for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedInfo {
    /// Feed title
    pub title: String,
    /// Feed description
    pub description: String,
    /// Feed URL
    pub url: String,
    /// Number of items
    pub item_count: usize,
    /// Last updated timestamp
    pub updated_at: Option<i64>,
}

impl FeedInfo {
    /// Create feed info from channel
    pub fn from_channel(channel: &Channel, url: &str) -> Self {
        Self {
            title: channel.title().to_string(),
            description: channel.description().to_string(),
            url: url.to_string(),
            item_count: channel.items().len(),
            updated_at: channel
                .last_build_date()
                .and_then(|d| DateTime::parse_from_rfc2822(d).ok())
                .map(|dt| dt.timestamp()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::ArticleStatus;
    use tempfile::tempdir;

    fn setup() -> FeedGenerator {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        FeedGenerator::new(db, FeedConfig::default())
    }

    fn create_test_article(
        db: &Database,
        title: &str,
        cid: &str,
        author_id: &str,
        tags: Vec<String>,
    ) {
        let article = Article {
            id: uuid::Uuid::new_v4().to_string(),
            cid: Some(cid.to_string()),
            title: title.to_string(),
            content: format!("# {}\n\nArticle content here.", title),
            author_id: author_id.to_string(),
            author_name: Some("Test Author".to_string()),
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            published_at: Some(chrono::Utc::now().timestamp()),
            tags,
            image_url: None,
            status: ArticleStatus::Published,
        };
        db.create_article(&article).unwrap();
    }

    #[test]
    fn test_generate_empty_feed() {
        let generator = setup();
        let xml = generator.generate().unwrap();

        assert!(xml.contains("<rss"));
        assert!(xml.contains("WRAITH Publish Feed"));
    }

    #[test]
    fn test_generate_with_articles() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());

        create_test_article(
            &db,
            "Article One",
            "cid-1",
            "author-1",
            vec!["rust".to_string()],
        );
        create_test_article(
            &db,
            "Article Two",
            "cid-2",
            "author-1",
            vec!["python".to_string()],
        );

        let generator = FeedGenerator::new(db, FeedConfig::default());
        let xml = generator.generate().unwrap();

        assert!(xml.contains("Article One"));
        assert!(xml.contains("Article Two"));
        assert!(xml.contains("cid-1"));
        assert!(xml.contains("cid-2"));
    }

    #[test]
    fn test_generate_for_author() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());

        create_test_article(&db, "Author 1 Article", "cid-1", "author-1", vec![]);
        create_test_article(&db, "Author 2 Article", "cid-2", "author-2", vec![]);

        let generator = FeedGenerator::new(db, FeedConfig::default());
        let xml = generator.generate_for_author("author-1").unwrap();

        assert!(xml.contains("Author 1 Article"));
        assert!(!xml.contains("Author 2 Article"));
    }

    #[test]
    fn test_generate_for_tag() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());

        create_test_article(
            &db,
            "Rust Article",
            "cid-1",
            "author-1",
            vec!["rust".to_string()],
        );
        create_test_article(
            &db,
            "Python Article",
            "cid-2",
            "author-1",
            vec!["python".to_string()],
        );

        let generator = FeedGenerator::new(db, FeedConfig::default());
        let xml = generator.generate_for_tag("rust").unwrap();

        assert!(xml.contains("Rust Article"));
        assert!(!xml.contains("Python Article"));
    }

    #[test]
    fn test_timestamp_to_rfc2822() {
        let ts = 1704067200; // 2024-01-01 00:00:00 UTC
        let result = timestamp_to_rfc2822(ts);
        assert!(result.contains("2024"));
    }

    #[test]
    fn test_full_content_config() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());

        create_test_article(&db, "Full Content Test", "cid-1", "author-1", vec![]);

        let config = FeedConfig {
            full_content: true,
            ..Default::default()
        };

        let generator = FeedGenerator::new(db, config);
        let xml = generator.generate().unwrap();

        // Full content should include HTML
        assert!(xml.contains("<h1>"));
    }
}
