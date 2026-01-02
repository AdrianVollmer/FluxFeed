use crate::domain::models::{Article, CreateFeed, Feed};
use chrono::{DateTime, Utc};
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

// Article repository methods

pub async fn insert_article_if_new(
    pool: &SqlitePool,
    feed_id: i64,
    guid: String,
    title: String,
    url: Option<String>,
    content: Option<String>,
    summary: Option<String>,
    author: Option<String>,
    published_at: Option<DateTime<Utc>>,
) -> Result<Option<Article>, SqlxError> {
    let now = Utc::now();

    let result = sqlx::query_as::<_, Article>(
        r#"
        INSERT INTO articles (feed_id, guid, title, url, content, summary, author, published_at, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(feed_id, guid) DO NOTHING
        RETURNING *
        "#,
    )
    .bind(feed_id)
    .bind(&guid)
    .bind(&title)
    .bind(&url)
    .bind(&content)
    .bind(&summary)
    .bind(&author)
    .bind(published_at)
    .bind(now)
    .bind(now)
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

pub async fn update_feed_metadata(
    pool: &SqlitePool,
    feed_id: i64,
    etag: Option<String>,
    last_modified: Option<String>,
) -> Result<(), SqlxError> {
    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE feeds
        SET last_fetched_at = ?,
            etag = ?,
            last_modified = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(now)
    .bind(etag)
    .bind(last_modified)
    .bind(now)
    .bind(feed_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn touch_feed(pool: &SqlitePool, feed_id: i64) -> Result<(), SqlxError> {
    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE feeds
        SET last_fetched_at = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(now)
    .bind(now)
    .bind(feed_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_feeds_to_update(pool: &SqlitePool) -> Result<Vec<Feed>, SqlxError> {
    let feeds = sqlx::query_as::<_, Feed>(
        r#"
        SELECT * FROM feeds
        WHERE last_fetched_at IS NULL
           OR datetime(last_fetched_at, '+' || fetch_interval_minutes || ' minutes') <= datetime('now')
        ORDER BY last_fetched_at ASC NULLS FIRST
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(feeds)
}
