use crate::domain::models::{CreateFeed, Feed};
use crate::infrastructure::repository;
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

    // Use provided title or default to URL
    let feed_title = title.unwrap_or_else(|| url.clone());

    let create_feed = CreateFeed {
        url,
        title: feed_title,
        description: None,
    };

    match repository::create_feed(pool, create_feed).await {
        Ok(feed) => Ok(feed),
        Err(sqlx::Error::Database(db_err))
            if db_err.message().contains("UNIQUE constraint") =>
        {
            Err(FeedServiceError::DuplicateUrl)
        }
        Err(e) => Err(FeedServiceError::DatabaseError(e)),
    }
}

pub async fn list_all_feeds(pool: &SqlitePool) -> Result<Vec<Feed>, FeedServiceError> {
    Ok(repository::list_feeds(pool).await?)
}

pub async fn delete_feed(
    pool: &SqlitePool,
    feed_id: i64,
) -> Result<(), FeedServiceError> {
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
