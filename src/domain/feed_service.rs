use crate::domain::models::{CreateFeed, Feed};
use crate::infrastructure::{repository, scheduler};
use sqlx::SqlitePool;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FeedServiceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Feed not found")]
    NotFound,

    #[error("Invalid feed URL: {0}")]
    InvalidUrl(String),

    #[error("Duplicate feed URL")]
    DuplicateUrl,

    #[error("Feed fetch failed: {0}")]
    FetchError(String),

    #[error("Invalid fetch frequency: must be 'adaptive' or hours between 1-168")]
    InvalidFrequency,
}

pub async fn create_feed(
    pool: &SqlitePool,
    url: String,
    title: Option<String>,
) -> Result<Feed, FeedServiceError> {
    // Basic URL validation
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(FeedServiceError::InvalidUrl(
            "URL must start with http:// or https://".to_string(),
        ));
    }

    // Use provided title or default to URL temporarily
    // It will be updated from RSS feed metadata after fetching
    let feed_title = title.unwrap_or_else(|| url.clone());

    let create_feed = CreateFeed {
        url,
        title: feed_title,
        description: None,
    };

    let feed = match repository::create_feed(pool, create_feed).await {
        Ok(feed) => feed,
        Err(sqlx::Error::Database(db_err)) if db_err.message().contains("UNIQUE constraint") => {
            return Err(FeedServiceError::DuplicateUrl);
        }
        Err(e) => return Err(FeedServiceError::DatabaseError(e)),
    };

    // Immediately fetch the feed to populate metadata and articles
    tracing::info!("Fetching new feed immediately: {}", feed.url);
    match scheduler::fetch_single_feed(pool, &feed).await {
        Ok(_) => {
            tracing::info!("Successfully fetched new feed: {}", feed.url);
        }
        Err(e) => {
            tracing::warn!("Failed to fetch new feed {}: {}", feed.url, e);
            // Don't fail the creation, just log the error
            // The feed is still created, it will be fetched by the scheduler later
        }
    }

    // Reload feed from database to get updated metadata
    let updated_feed = repository::get_feed_by_id(pool, feed.id)
        .await?
        .ok_or(FeedServiceError::NotFound)?;

    Ok(updated_feed)
}

pub async fn list_all_feeds(pool: &SqlitePool) -> Result<Vec<Feed>, FeedServiceError> {
    Ok(repository::list_feeds(pool).await?)
}

pub async fn delete_feed(pool: &SqlitePool, feed_id: i64) -> Result<(), FeedServiceError> {
    let deleted = repository::delete_feed(pool, feed_id).await?;

    if deleted {
        Ok(())
    } else {
        Err(FeedServiceError::NotFound)
    }
}

pub async fn get_feed_stats(
    pool: &SqlitePool,
    feed_id: i64,
) -> Result<(i64, i64), FeedServiceError> {
    let total = repository::get_feed_article_count(pool, feed_id).await?;
    let unread = repository::get_feed_unread_count(pool, feed_id).await?;

    Ok((total, unread))
}

/// Parse and validate fetch frequency
/// Returns fetch_interval_minutes
pub fn parse_fetch_frequency(frequency: &str) -> Result<i64, FeedServiceError> {
    match frequency.trim() {
        "adaptive" => Ok(60), // Default 1 hour for adaptive
        hours_str => {
            let hours = hours_str
                .parse::<i64>()
                .map_err(|_| FeedServiceError::InvalidFrequency)?;

            if hours < 1 || hours > 168 {
                return Err(FeedServiceError::InvalidFrequency);
            }

            Ok(hours * 60) // Convert hours to minutes
        }
    }
}
