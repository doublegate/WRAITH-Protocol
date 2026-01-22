//! Stream Discovery
//!
//! Handles stream search, trending, and category browsing.

use crate::database::{Database, Stream};
use crate::error::StreamResult;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Stream category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub stream_count: i64,
}

/// Stream discovery service
pub struct StreamDiscovery {
    db: Arc<Database>,
}

/// Search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub streams: Vec<StreamSummary>,
    pub total_count: i64,
    pub query: String,
}

/// Stream summary for listings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSummary {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub duration: Option<i64>,
    pub view_count: i64,
    pub is_live: bool,
    pub category: Option<String>,
    pub created_at: i64,
}

impl StreamDiscovery {
    /// Create a new discovery service
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Search streams by query
    pub fn search(&self, query: &str, limit: i64) -> StreamResult<SearchResults> {
        let streams = self.db.search_streams(query, limit)?;
        let total_count = streams.len() as i64;

        let summaries: Vec<StreamSummary> = streams
            .into_iter()
            .map(|s| self.stream_to_summary(s))
            .collect();

        Ok(SearchResults {
            streams: summaries,
            total_count,
            query: query.to_string(),
        })
    }

    /// Get trending streams
    pub fn get_trending(&self, limit: i64) -> StreamResult<Vec<StreamSummary>> {
        let streams = self.db.get_trending_streams(limit)?;

        Ok(streams
            .into_iter()
            .map(|s| self.stream_to_summary(s))
            .collect())
    }

    /// Get recent streams
    pub fn get_recent(&self, limit: i64, offset: i64) -> StreamResult<Vec<StreamSummary>> {
        let streams = self.db.list_streams(limit, offset)?;

        Ok(streams
            .into_iter()
            .map(|s| self.stream_to_summary(s))
            .collect())
    }

    /// Get streams by category
    pub fn get_by_category(&self, category: &str, limit: i64) -> StreamResult<Vec<StreamSummary>> {
        // Use search with category filter
        let query = format!("category:{}", category);
        let results = self.search(&query, limit)?;
        Ok(results.streams)
    }

    /// Get available categories
    pub fn get_categories(&self) -> StreamResult<Vec<Category>> {
        // Return predefined categories
        // In a real implementation, these would be stored in the database
        Ok(vec![
            Category {
                id: "technology".to_string(),
                name: "Technology".to_string(),
                stream_count: 0,
            },
            Category {
                id: "gaming".to_string(),
                name: "Gaming".to_string(),
                stream_count: 0,
            },
            Category {
                id: "music".to_string(),
                name: "Music".to_string(),
                stream_count: 0,
            },
            Category {
                id: "education".to_string(),
                name: "Education".to_string(),
                stream_count: 0,
            },
            Category {
                id: "entertainment".to_string(),
                name: "Entertainment".to_string(),
                stream_count: 0,
            },
            Category {
                id: "news".to_string(),
                name: "News".to_string(),
                stream_count: 0,
            },
            Category {
                id: "sports".to_string(),
                name: "Sports".to_string(),
                stream_count: 0,
            },
            Category {
                id: "other".to_string(),
                name: "Other".to_string(),
                stream_count: 0,
            },
        ])
    }

    /// Convert stream to summary
    fn stream_to_summary(&self, stream: Stream) -> StreamSummary {
        StreamSummary {
            id: stream.id,
            title: stream.title,
            description: stream.description,
            thumbnail_url: stream
                .thumbnail_hash
                .map(|h| format!("/thumbnails/{}.jpg", h)),
            duration: stream.duration,
            view_count: stream.view_count,
            is_live: stream.is_live,
            category: stream.category,
            created_at: stream.created_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;

    fn create_test_discovery() -> (StreamDiscovery, Arc<Database>, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Arc::new(Database::open(&db_path).unwrap());
        (StreamDiscovery::new(db.clone()), db, dir)
    }

    #[test]
    fn test_search() {
        let (discovery, db, _dir) = create_test_discovery();

        // Create test streams
        for i in 0..5 {
            let stream = Stream {
                id: format!("stream-{}", i),
                title: format!("Test Stream {}", i),
                description: Some(format!("Description for stream {}", i)),
                created_at: Utc::now().timestamp(),
                created_by: "peer-1".to_string(),
                thumbnail_hash: None,
                duration: Some(3600),
                is_live: false,
                status: "ready".to_string(),
                view_count: i as i64 * 100,
                category: Some("Technology".to_string()),
                tags: Some("test".to_string()),
            };
            db.create_stream(&stream).unwrap();
        }

        // Search
        let results = discovery.search("Test Stream", 10).unwrap();
        assert_eq!(results.streams.len(), 5);
        assert_eq!(results.query, "Test Stream");
    }

    #[test]
    fn test_get_trending() {
        let (discovery, db, _dir) = create_test_discovery();

        // Create streams with different view counts
        for i in 0..5 {
            let stream = Stream {
                id: format!("stream-{}", i),
                title: format!("Stream {}", i),
                description: None,
                created_at: Utc::now().timestamp(),
                created_by: "peer-1".to_string(),
                thumbnail_hash: None,
                duration: Some(3600),
                is_live: false,
                status: "ready".to_string(),
                view_count: (5 - i) as i64 * 100, // Decreasing views
                category: None,
                tags: None,
            };
            db.create_stream(&stream).unwrap();
        }

        let trending = discovery.get_trending(3).unwrap();
        assert_eq!(trending.len(), 3);
        // Should be sorted by view count (highest first)
        assert!(trending[0].view_count >= trending[1].view_count);
    }

    #[test]
    fn test_get_categories() {
        let (discovery, _, _dir) = create_test_discovery();

        let categories = discovery.get_categories().unwrap();
        assert!(!categories.is_empty());
        assert!(categories.iter().any(|c| c.id == "technology"));
    }
}
