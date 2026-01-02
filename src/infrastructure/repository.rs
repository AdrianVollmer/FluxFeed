use crate::domain::models::{CreateFeed, Feed};
use chrono::Utc;
use sqlx::{Error as SqlxError, SqlitePool};

pub async fn create_feed(
    pool: &SqlitePool,
    create_feed: CreateFeed,
) -> Result<Feed, SqlxError> {
    let now = Utc::now();

    let feed = sqlx::query_as::<_, Feed>(
        r#"
        INSERT INTO feeds (url, title, description, fetch_interval_minutes, created_at, updated_at)
        VALUES (?, ?, ?, 30, ?, ?)
        RETURNING *
        "#,
    )
    .bind(&create_feed.url)
    .bind(&create_feed.title)
    .bind(&create_feed.description)
    .bind(now)
    .bind(now)
    .fetch_one(pool)
    .await?;

    Ok(feed)
}

pub async fn list_feeds(pool: &SqlitePool) -> Result<Vec<Feed>, SqlxError> {
    let feeds = sqlx::query_as::<_, Feed>(
        r#"
        SELECT * FROM feeds
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(feeds)
}

pub async fn get_feed_by_id(
    pool: &SqlitePool,
    feed_id: i64,
) -> Result<Option<Feed>, SqlxError> {
    let feed = sqlx::query_as::<_, Feed>(
        r#"
        SELECT * FROM feeds
        WHERE id = ?
        "#,
    )
    .bind(feed_id)
    .fetch_optional(pool)
    .await?;

    Ok(feed)
}

pub async fn delete_feed(pool: &SqlitePool, feed_id: i64) -> Result<bool, SqlxError> {
    let result = sqlx::query(
        r#"
        DELETE FROM feeds
        WHERE id = ?
        "#,
    )
    .bind(feed_id)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn get_feed_article_count(
    pool: &SqlitePool,
    feed_id: i64,
) -> Result<i64, SqlxError> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM articles
        WHERE feed_id = ?
        "#,
    )
    .bind(feed_id)
    .fetch_one(pool)
    .await?;

    Ok(count.0)
}

pub async fn get_feed_unread_count(
    pool: &SqlitePool,
    feed_id: i64,
) -> Result<i64, SqlxError> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM articles
        WHERE feed_id = ? AND is_read = 0
        "#,
    )
    .bind(feed_id)
    .fetch_one(pool)
    .await?;

    Ok(count.0)
}
