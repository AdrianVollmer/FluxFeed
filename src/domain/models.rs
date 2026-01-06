use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Feed {
    pub id: i64,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub site_url: Option<String>,
    pub group_id: Option<i64>,
    pub last_fetched_at: Option<DateTime<Utc>>,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
    pub fetch_interval_minutes: i64,
    pub color: String,
    pub fetch_frequency: String,
    pub ttl_minutes: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFeed {
    pub url: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewArticle {
    pub feed_id: i64,
    pub guid: String,
    pub title: String,
    pub url: Option<String>,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub og_image: Option<String>,
    pub og_description: Option<String>,
    pub og_site_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Article {
    pub id: i64,
    pub feed_id: i64,
    pub guid: String,
    pub title: String,
    pub url: Option<String>,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub is_read: bool,
    pub is_starred: bool,
    pub og_image: Option<String>,
    pub og_description: Option<String>,
    pub og_site_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[allow(dead_code)]
pub struct FeedTag {
    pub feed_id: i64,
    pub tag_id: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Group {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub position: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents a group with its children for tree rendering
#[derive(Debug, Clone, Serialize)]
pub struct GroupNode {
    pub group: Group,
    pub children: Vec<GroupNode>,
    pub feeds: Vec<Feed>,
}

/// Represents an item in a flattened tree view (for template iteration)
#[derive(Debug, Clone, Serialize)]
pub enum FlatTreeItem {
    Group { group: Group, depth: usize },
    Feed { feed: Feed, depth: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Log {
    pub id: i64,
    pub feed_id: i64,
    pub log_type: String,
    pub status_code: Option<i32>,
    pub error_message: Option<String>,
    pub retry_after: Option<String>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogWithFeed {
    pub log: Log,
    pub feed_title: String,
    pub feed_url: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_feed_serialization() {
        let feed = CreateFeed {
            url: "https://example.com/feed".to_string(),
            title: "Test Feed".to_string(),
            description: Some("A test feed".to_string()),
        };

        let json = serde_json::to_string(&feed).unwrap();
        let deserialized: CreateFeed = serde_json::from_str(&json).unwrap();

        assert_eq!(feed.url, deserialized.url);
        assert_eq!(feed.title, deserialized.title);
        assert_eq!(feed.description, deserialized.description);
    }

    #[test]
    fn test_create_feed_without_description() {
        let feed = CreateFeed {
            url: "https://example.com/feed".to_string(),
            title: "Test Feed".to_string(),
            description: None,
        };

        assert_eq!(feed.url, "https://example.com/feed");
        assert_eq!(feed.title, "Test Feed");
        assert!(feed.description.is_none());
    }

    #[test]
    fn test_tag_serialization() {
        let now = Utc::now();
        let tag = Tag {
            id: 1,
            name: "Tech".to_string(),
            created_at: now,
        };

        let json = serde_json::to_string(&tag).unwrap();
        let deserialized: Tag = serde_json::from_str(&json).unwrap();

        assert_eq!(tag.id, deserialized.id);
        assert_eq!(tag.name, deserialized.name);
    }
}
