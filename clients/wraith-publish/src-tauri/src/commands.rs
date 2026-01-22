//! Tauri IPC Commands for WRAITH Publish
//!
//! Provides the command interface between the frontend and backend.

use crate::content::ContentManager;
use crate::database::{Article, Database, Image};
use crate::error::PublishError;
use crate::markdown::{Heading, MarkdownProcessor};
use crate::propagation::{PropagationStatus, PropagationTracker};
use crate::publisher::Publisher;
use crate::reader::{ContentReader, FetchedArticle};
use crate::rss::FeedGenerator;
use crate::signatures::{ContentSigner, SignatureInfo, SignedContent};
use crate::state::AppState;
use crate::storage::{DhtStorage, StorageStats};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

/// Application result type for Tauri commands
type CmdResult<T> = Result<T, PublishError>;

/// Shared managers state
pub struct Managers {
    pub content_manager: ContentManager,
    pub publisher: Publisher,
    pub content_reader: ContentReader,
    pub dht_storage: Arc<DhtStorage>,
    pub feed_generator: FeedGenerator,
    pub propagation_tracker: PropagationTracker,
    pub markdown: MarkdownProcessor,
}

// =============================================================================
// Identity Commands
// =============================================================================

/// Get local peer ID
#[tauri::command]
pub async fn get_peer_id(state: State<'_, Arc<AppState>>) -> CmdResult<String> {
    state
        .get_peer_id()
        .ok_or_else(|| PublishError::Config("Identity not initialized".to_string()))
}

/// Get display name
#[tauri::command]
pub async fn get_display_name(state: State<'_, Arc<AppState>>) -> CmdResult<String> {
    Ok(state.get_display_name())
}

/// Set display name
#[tauri::command]
pub async fn set_display_name(state: State<'_, Arc<AppState>>, name: String) -> CmdResult<()> {
    state.set_display_name(&name)
}

// =============================================================================
// Draft Commands
// =============================================================================

/// Create a new draft article
#[tauri::command]
pub async fn create_draft(
    managers: State<'_, Managers>,
    title: String,
    content: String,
    tags: Vec<String>,
) -> CmdResult<Article> {
    managers
        .content_manager
        .create_draft(&title, &content, tags)
}

/// Update a draft article
#[tauri::command]
pub async fn update_draft(
    managers: State<'_, Managers>,
    id: String,
    title: String,
    content: String,
    tags: Vec<String>,
    image_url: Option<String>,
) -> CmdResult<Article> {
    managers
        .content_manager
        .update_draft(&id, &title, &content, tags, image_url)
}

/// Delete a draft article
#[tauri::command]
pub async fn delete_draft(managers: State<'_, Managers>, id: String) -> CmdResult<()> {
    managers.content_manager.delete_draft(&id)
}

/// List all drafts
#[tauri::command]
pub async fn list_drafts(managers: State<'_, Managers>) -> CmdResult<Vec<Article>> {
    managers.content_manager.list_drafts()
}

/// Get a draft by ID
#[tauri::command]
pub async fn get_draft(managers: State<'_, Managers>, id: String) -> CmdResult<Option<Article>> {
    managers.content_manager.get_article(&id)
}

// =============================================================================
// Publish Commands
// =============================================================================

/// Publish an article to the DHT
#[tauri::command]
pub async fn publish_article(
    managers: State<'_, Managers>,
    id: String,
) -> CmdResult<PublishResult> {
    // Publish and get CID
    let cid = managers.publisher.publish(&id)?;

    // Start tracking propagation
    managers.propagation_tracker.start(&cid);

    // Simulate propagation in background
    let tracker = managers.propagation_tracker.clone();
    let cid_clone = cid.clone();
    tokio::spawn(async move {
        tracker.simulate_propagation(&cid_clone).await;
    });

    // Get updated article
    let article = managers
        .content_manager
        .get_article(&id)?
        .ok_or(PublishError::ArticleNotFound(id))?;

    Ok(PublishResult { article, cid })
}

/// Unpublish an article
#[tauri::command]
pub async fn unpublish_article(managers: State<'_, Managers>, cid: String) -> CmdResult<Article> {
    managers.publisher.unpublish(&cid)?;

    // Get updated article
    managers
        .publisher
        .fetch_article(&cid)?
        .ok_or(PublishError::ArticleNotFound(cid))
}

/// List all published articles
#[tauri::command]
pub async fn list_published(managers: State<'_, Managers>) -> CmdResult<Vec<Article>> {
    managers.publisher.list_published()
}

/// Get an article by ID or CID
#[tauri::command]
pub async fn get_article(managers: State<'_, Managers>, id: String) -> CmdResult<Option<Article>> {
    // Try by ID first
    if let Some(article) = managers.content_manager.get_article(&id)? {
        return Ok(Some(article));
    }

    // Try by CID
    managers.content_manager.get_article_by_cid(&id)
}

// =============================================================================
// Content Commands
// =============================================================================

/// Fetch content by CID from DHT
#[tauri::command]
pub async fn fetch_content(
    managers: State<'_, Managers>,
    cid: String,
) -> CmdResult<Option<FetchedArticle>> {
    managers.content_reader.fetch(&cid).await
}

/// Verify content integrity
#[tauri::command]
pub async fn verify_content(managers: State<'_, Managers>, id: String) -> CmdResult<bool> {
    let article = managers
        .content_manager
        .get_article(&id)?
        .ok_or(PublishError::ArticleNotFound(id))?;

    managers.content_reader.verify(&article)
}

/// Search articles by query
#[tauri::command]
pub async fn search_articles(
    managers: State<'_, Managers>,
    query: String,
    limit: Option<usize>,
) -> CmdResult<Vec<Article>> {
    managers.content_reader.search(&query, limit.unwrap_or(20))
}

// =============================================================================
// Signature Commands
// =============================================================================

/// Sign content with Ed25519
#[tauri::command]
pub async fn sign_content(
    state: State<'_, Arc<AppState>>,
    content: Vec<u8>,
) -> CmdResult<SignedContent> {
    let signing_key = state
        .get_signing_key()
        .ok_or_else(|| PublishError::Crypto("No signing key available".to_string()))?;

    let signer = ContentSigner::new(Some(signing_key));
    signer.sign(&content)
}

/// Verify a content signature
#[tauri::command]
pub async fn verify_signature(signed_content: SignedContent) -> CmdResult<SignatureInfo> {
    SignatureInfo::from_signed_content(&signed_content)
}

// =============================================================================
// Storage Commands
// =============================================================================

/// Pin content locally
#[tauri::command]
pub async fn pin_content(managers: State<'_, Managers>, cid: String) -> CmdResult<bool> {
    managers.dht_storage.pin(&cid)
}

/// Unpin content
#[tauri::command]
pub async fn unpin_content(managers: State<'_, Managers>, cid: String) -> CmdResult<bool> {
    managers.dht_storage.unpin(&cid)
}

/// List pinned content CIDs
#[tauri::command]
pub async fn list_pinned(managers: State<'_, Managers>) -> CmdResult<Vec<String>> {
    Ok(managers.dht_storage.list_pinned())
}

/// Get storage statistics
#[tauri::command]
pub async fn get_storage_stats(managers: State<'_, Managers>) -> CmdResult<StorageStats> {
    Ok(managers.dht_storage.stats())
}

// =============================================================================
// Propagation Commands
// =============================================================================

/// Get propagation status for a CID
#[tauri::command]
pub async fn get_propagation_status(
    managers: State<'_, Managers>,
    cid: String,
) -> CmdResult<Option<PropagationStatus>> {
    Ok(managers.propagation_tracker.get(&cid))
}

/// List all active propagations
#[tauri::command]
pub async fn list_active_propagations(
    managers: State<'_, Managers>,
) -> CmdResult<Vec<PropagationStatus>> {
    Ok(managers.propagation_tracker.get_active())
}

// =============================================================================
// RSS Commands
// =============================================================================

/// Generate RSS feed for all published articles
#[tauri::command]
pub async fn generate_rss_feed(managers: State<'_, Managers>) -> CmdResult<String> {
    managers.feed_generator.generate()
}

/// Generate RSS feed for a specific author
#[tauri::command]
pub async fn generate_author_feed(
    managers: State<'_, Managers>,
    author_id: String,
) -> CmdResult<String> {
    managers.feed_generator.generate_for_author(&author_id)
}

/// Generate RSS feed for a specific tag
#[tauri::command]
pub async fn generate_tag_feed(managers: State<'_, Managers>, tag: String) -> CmdResult<String> {
    managers.feed_generator.generate_for_tag(&tag)
}

// =============================================================================
// Markdown Commands
// =============================================================================

/// Render markdown to HTML
#[tauri::command]
pub async fn render_markdown(managers: State<'_, Managers>, markdown: String) -> CmdResult<String> {
    Ok(managers.markdown.to_html(&markdown))
}

/// Extract metadata from markdown
#[tauri::command]
pub async fn extract_metadata(
    managers: State<'_, Managers>,
    markdown: String,
) -> CmdResult<MarkdownMetadata> {
    Ok(MarkdownMetadata {
        title: managers.markdown.extract_title(&markdown),
        excerpt: managers.markdown.extract_excerpt(&markdown, 200),
        word_count: managers.markdown.word_count(&markdown),
        reading_time: managers.markdown.reading_time(&markdown),
        headings: managers.markdown.extract_headings(&markdown),
    })
}

// =============================================================================
// Image Commands
// =============================================================================

/// Upload an image
#[tauri::command]
pub async fn upload_image(
    db: State<'_, Arc<Database>>,
    data: Vec<u8>,
    mime_type: String,
) -> CmdResult<String> {
    // Generate CID from image data
    let cid = hex::encode(&blake3::hash(&data).as_bytes()[..32]);

    db.store_image(&cid, &data, &mime_type)?;

    Ok(cid)
}

/// Get an image by CID
#[tauri::command]
pub async fn get_image(db: State<'_, Arc<Database>>, cid: String) -> CmdResult<Option<Image>> {
    db.get_image(&cid).map_err(|e| e.into())
}

/// Delete an image
#[tauri::command]
pub async fn delete_image(db: State<'_, Arc<Database>>, cid: String) -> CmdResult<()> {
    db.delete_image(&cid).map_err(|e| e.into())
}

// =============================================================================
// Response Types
// =============================================================================

/// Publish result with article and CID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    pub article: Article,
    pub cid: String,
}

/// Markdown metadata extraction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownMetadata {
    pub title: Option<String>,
    pub excerpt: String,
    pub word_count: usize,
    pub reading_time: u32,
    pub headings: Vec<Heading>,
}

// =============================================================================
// Clone implementations for managers that need it
// =============================================================================

impl Clone for PropagationTracker {
    fn clone(&self) -> Self {
        PropagationTracker::new(3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publish_result_serialization() {
        let result = PublishResult {
            article: Article {
                id: "test-id".to_string(),
                cid: Some("test-cid".to_string()),
                title: "Test".to_string(),
                content: "Content".to_string(),
                author_id: "author".to_string(),
                author_name: None,
                created_at: 0,
                updated_at: 0,
                published_at: Some(0),
                tags: vec![],
                image_url: None,
                status: crate::database::ArticleStatus::Published,
            },
            cid: "test-cid".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test-cid"));
    }

    #[test]
    fn test_markdown_metadata_serialization() {
        let metadata = MarkdownMetadata {
            title: Some("Test Title".to_string()),
            excerpt: "Test excerpt...".to_string(),
            word_count: 100,
            reading_time: 1,
            headings: vec![],
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("Test Title"));
        assert!(json.contains("\"reading_time\":1"));
    }
}
