use crate::domain::models::{Article, CreateFeed, Feed, Log, LogWithFeed, NewArticle, Tag};
use chrono::Utc;
use sqlx::{Error as SqlxError, Row, SqlitePool};

pub async fn create_feed(pool: &SqlitePool, create_feed: CreateFeed) -> Result<Feed, SqlxError> {
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

pub async fn get_feed_by_id(pool: &SqlitePool, feed_id: i64) -> Result<Option<Feed>, SqlxError> {
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

#[allow(dead_code)]
pub async fn get_feed_article_count(pool: &SqlitePool, feed_id: i64) -> Result<i64, SqlxError> {
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

#[allow(dead_code)]
pub async fn get_feed_unread_count(pool: &SqlitePool, feed_id: i64) -> Result<i64, SqlxError> {
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
    article: NewArticle,
) -> Result<Option<Article>, SqlxError> {
    let now = Utc::now();

    let result = sqlx::query_as::<_, Article>(
        r#"
        INSERT INTO articles (feed_id, guid, title, url, content, summary, author, published_at, og_image, og_description, og_site_name, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(feed_id, guid) DO NOTHING
        RETURNING *
        "#,
    )
    .bind(article.feed_id)
    .bind(&article.guid)
    .bind(&article.title)
    .bind(&article.url)
    .bind(&article.content)
    .bind(&article.summary)
    .bind(&article.author)
    .bind(article.published_at)
    .bind(&article.og_image)
    .bind(&article.og_description)
    .bind(&article.og_site_name)
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

pub async fn update_feed_details(
    pool: &SqlitePool,
    feed_id: i64,
    title: Option<String>,
    description: Option<String>,
    site_url: Option<String>,
    etag: Option<String>,
    last_modified: Option<String>,
) -> Result<(), SqlxError> {
    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE feeds
        SET title = COALESCE(?, title),
            description = COALESCE(?, description),
            site_url = ?,
            last_fetched_at = ?,
            etag = ?,
            last_modified = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(title)
    .bind(description)
    .bind(site_url)
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

// Article query methods

#[allow(clippy::too_many_arguments)]
pub async fn list_articles(
    pool: &SqlitePool,
    feed_id: Option<i64>,
    is_read: Option<bool>,
    is_starred: Option<bool>,
    search_query: Option<String>,
    date_from: Option<chrono::DateTime<chrono::Utc>>,
    date_to: Option<chrono::DateTime<chrono::Utc>>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Article>, SqlxError> {
    let mut query_str = String::from("SELECT a.* FROM articles a");

    // Add FTS join when search is active
    if search_query.is_some() {
        query_str.push_str(" INNER JOIN articles_fts ON a.id = articles_fts.rowid");
    }

    query_str.push_str(" WHERE 1=1");

    // Add search filter
    if search_query.is_some() {
        query_str.push_str(" AND articles_fts MATCH ?");
    }

    // Add existing filters
    if feed_id.is_some() {
        query_str.push_str(" AND a.feed_id = ?");
    }
    if is_read.is_some() {
        query_str.push_str(" AND a.is_read = ?");
    }
    if is_starred.is_some() {
        query_str.push_str(" AND a.is_starred = ?");
    }

    // Add date range filters
    if date_from.is_some() {
        query_str.push_str(" AND a.published_at >= ?");
    }
    if date_to.is_some() {
        query_str.push_str(" AND a.published_at <= ?");
    }

    query_str.push_str(" ORDER BY a.published_at DESC, a.created_at DESC LIMIT ? OFFSET ?");

    let mut query = sqlx::query_as::<_, Article>(&query_str);

    // Bind parameters in correct order
    if let Some(search) = search_query {
        query = query.bind(search);
    }
    if let Some(fid) = feed_id {
        query = query.bind(fid);
    }
    if let Some(read) = is_read {
        query = query.bind(read);
    }
    if let Some(starred) = is_starred {
        query = query.bind(starred);
    }
    if let Some(from) = date_from {
        query = query.bind(from);
    }
    if let Some(to) = date_to {
        query = query.bind(to);
    }

    let articles = query.bind(limit).bind(offset).fetch_all(pool).await?;

    Ok(articles)
}

pub async fn get_article_by_id(
    pool: &SqlitePool,
    article_id: i64,
) -> Result<Option<Article>, SqlxError> {
    let article = sqlx::query_as::<_, Article>("SELECT * FROM articles WHERE id = ?")
        .bind(article_id)
        .fetch_optional(pool)
        .await?;

    Ok(article)
}

pub async fn update_article_read_status(
    pool: &SqlitePool,
    article_id: i64,
    is_read: bool,
) -> Result<(), SqlxError> {
    let now = Utc::now();

    sqlx::query("UPDATE articles SET is_read = ?, updated_at = ? WHERE id = ?")
        .bind(is_read)
        .bind(now)
        .bind(article_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_article_starred_status(
    pool: &SqlitePool,
    article_id: i64,
    is_starred: bool,
) -> Result<(), SqlxError> {
    let now = Utc::now();

    sqlx::query("UPDATE articles SET is_starred = ?, updated_at = ? WHERE id = ?")
        .bind(is_starred)
        .bind(now)
        .bind(article_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn update_article_opengraph(
    pool: &SqlitePool,
    article_id: i64,
    og_image: Option<String>,
    og_description: Option<String>,
    og_site_name: Option<String>,
) -> Result<(), SqlxError> {
    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE articles
        SET og_image = COALESCE(?, og_image),
            og_description = COALESCE(?, og_description),
            og_site_name = COALESCE(?, og_site_name),
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(og_image)
    .bind(og_description)
    .bind(og_site_name)
    .bind(now)
    .bind(article_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn mark_all_articles_read(
    pool: &SqlitePool,
    feed_id: Option<i64>,
) -> Result<u64, SqlxError> {
    let now = Utc::now();

    let result = if let Some(fid) = feed_id {
        sqlx::query(
            "UPDATE articles SET is_read = 1, updated_at = ? WHERE feed_id = ? AND is_read = 0",
        )
        .bind(now)
        .bind(fid)
        .execute(pool)
        .await?
    } else {
        sqlx::query("UPDATE articles SET is_read = 1, updated_at = ? WHERE is_read = 0")
            .bind(now)
            .execute(pool)
            .await?
    };

    Ok(result.rows_affected())
}

pub async fn get_total_unread_count(pool: &SqlitePool) -> Result<i64, SqlxError> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM articles WHERE is_read = 0")
        .fetch_one(pool)
        .await?;

    Ok(count.0)
}

// Tag operations
pub async fn get_feed_tags(pool: &SqlitePool, feed_id: i64) -> Result<Vec<Tag>, SqlxError> {
    let tags = sqlx::query_as::<_, Tag>(
        r#"
        SELECT t.* FROM tags t
        INNER JOIN feed_tags ft ON ft.tag_id = t.id
        WHERE ft.feed_id = ?
        ORDER BY t.name ASC
        "#,
    )
    .bind(feed_id)
    .fetch_all(pool)
    .await?;

    Ok(tags)
}

// Log operations
pub async fn insert_log(
    pool: &SqlitePool,
    feed_id: i64,
    log_type: &str,
    status_code: Option<i32>,
    error_message: Option<&str>,
    retry_after: Option<&str>,
) -> Result<(), SqlxError> {
    sqlx::query(
        r#"
        INSERT INTO logs (feed_id, log_type, status_code, error_message, retry_after)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(feed_id)
    .bind(log_type)
    .bind(status_code)
    .bind(error_message)
    .bind(retry_after)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_logs_with_feeds(
    pool: &SqlitePool,
    feed_id: Option<i64>,
    log_type: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<LogWithFeed>, SqlxError> {
    let mut query = String::from(
        r#"
        SELECT
            l.id, l.feed_id, l.log_type, l.status_code, l.error_message, l.retry_after, l.fetched_at,
            f.title as feed_title, f.url as feed_url
        FROM logs l
        INNER JOIN feeds f ON f.id = l.feed_id
        WHERE 1=1
        "#,
    );

    let mut bindings: Vec<String> = Vec::new();

    if let Some(id) = feed_id {
        query.push_str(" AND l.feed_id = ?");
        bindings.push(id.to_string());
    }

    if let Some(lt) = log_type {
        query.push_str(" AND l.log_type = ?");
        bindings.push(lt.to_string());
    }

    query.push_str(" ORDER BY l.fetched_at DESC LIMIT ? OFFSET ?");
    bindings.push(limit.to_string());
    bindings.push(offset.to_string());

    let mut sqlx_query = sqlx::query(&query);
    for binding in &bindings {
        sqlx_query = sqlx_query.bind(binding);
    }

    let rows = sqlx_query.fetch_all(pool).await?;

    let mut logs = Vec::new();
    for row in rows {
        let log = Log {
            id: row.get("id"),
            feed_id: row.get("feed_id"),
            log_type: row.get("log_type"),
            status_code: row.get("status_code"),
            error_message: row.get("error_message"),
            retry_after: row.get("retry_after"),
            fetched_at: row.get("fetched_at"),
        };

        let log_with_feed = LogWithFeed {
            log,
            feed_title: row.get("feed_title"),
            feed_url: row.get("feed_url"),
        };

        logs.push(log_with_feed);
    }

    Ok(logs)
}

/// Update feed's TTL and fetch interval (for adaptive mode)
pub async fn update_feed_ttl(
    pool: &SqlitePool,
    feed_id: i64,
    ttl_minutes: Option<i64>,
    fetch_interval_minutes: i64,
) -> Result<(), SqlxError> {
    sqlx::query!(
        r#"
        UPDATE feeds
        SET ttl_minutes = ?,
            fetch_interval_minutes = ?,
            updated_at = datetime('now')
        WHERE id = ?
        "#,
        ttl_minutes,
        fetch_interval_minutes,
        feed_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update only TTL (for custom frequency mode - store but don't use)
pub async fn update_feed_ttl_only(
    pool: &SqlitePool,
    feed_id: i64,
    ttl_minutes: Option<i64>,
) -> Result<(), SqlxError> {
    sqlx::query!(
        r#"
        UPDATE feeds
        SET ttl_minutes = ?,
            updated_at = datetime('now')
        WHERE id = ?
        "#,
        ttl_minutes,
        feed_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update feed's fetch frequency preference
#[allow(dead_code)]
pub async fn update_feed_frequency(
    pool: &SqlitePool,
    feed_id: i64,
    fetch_frequency: &str,
    fetch_interval_minutes: i64,
) -> Result<(), SqlxError> {
    sqlx::query!(
        r#"
        UPDATE feeds
        SET fetch_frequency = ?,
            fetch_interval_minutes = ?,
            updated_at = datetime('now')
        WHERE id = ?
        "#,
        fetch_frequency,
        fetch_interval_minutes,
        feed_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Update feed's editable properties (title, URL, description, frequency and color)
#[allow(clippy::too_many_arguments)]
pub async fn update_feed_properties(
    pool: &SqlitePool,
    feed_id: i64,
    title: &str,
    url: &str,
    description: Option<&str>,
    fetch_frequency: &str,
    fetch_interval_minutes: i64,
    color: &str,
) -> Result<(), SqlxError> {
    sqlx::query!(
        r#"
        UPDATE feeds
        SET title = ?,
            url = ?,
            description = ?,
            fetch_frequency = ?,
            fetch_interval_minutes = ?,
            color = ?,
            updated_at = datetime('now')
        WHERE id = ?
        "#,
        title,
        url,
        description,
        fetch_frequency,
        fetch_interval_minutes,
        color,
        feed_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        pool
    }

    #[tokio::test]
    async fn test_create_and_get_feed() {
        let pool = setup_test_db().await;

        let feed_data = CreateFeed {
            url: "https://example.com/feed".to_string(),
            title: "Test Feed".to_string(),
            description: Some("Test description".to_string()),
        };

        let feed = super::create_feed(&pool, feed_data)
            .await
            .expect("Failed to create feed");

        assert_eq!(feed.url, "https://example.com/feed");
        assert_eq!(feed.title, "Test Feed");
        assert_eq!(feed.description, Some("Test description".to_string()));

        let retrieved = get_feed_by_id(&pool, feed.id)
            .await
            .expect("Failed to get feed")
            .expect("Feed not found");

        assert_eq!(retrieved.id, feed.id);
        assert_eq!(retrieved.url, feed.url);
    }

    #[tokio::test]
    async fn test_list_feeds() {
        let pool = setup_test_db().await;

        let feed1 = CreateFeed {
            url: "https://example.com/feed1".to_string(),
            title: "Feed 1".to_string(),
            description: None,
        };
        let feed2 = CreateFeed {
            url: "https://example.com/feed2".to_string(),
            title: "Feed 2".to_string(),
            description: None,
        };

        super::create_feed(&pool, feed1).await.unwrap();
        super::create_feed(&pool, feed2).await.unwrap();

        let feeds = list_feeds(&pool).await.expect("Failed to list feeds");
        assert_eq!(feeds.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_feed() {
        let pool = setup_test_db().await;

        let create_feed_data = CreateFeed {
            url: "https://example.com/feed".to_string(),
            title: "Test Feed".to_string(),
            description: None,
        };

        let feed = super::create_feed(&pool, create_feed_data).await.unwrap();

        let deleted = delete_feed(&pool, feed.id).await.unwrap();
        assert!(deleted);

        let retrieved = get_feed_by_id(&pool, feed.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_feed() {
        let pool = setup_test_db().await;

        let deleted = delete_feed(&pool, 9999).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_insert_article_if_new() {
        let pool = setup_test_db().await;

        let feed = super::create_feed(
            &pool,
            CreateFeed {
                url: "https://example.com/feed".to_string(),
                title: "Test Feed".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        let article = insert_article_if_new(
            &pool,
            NewArticle {
                feed_id: feed.id,
                guid: "guid-123".to_string(),
                title: "Test Article".to_string(),
                url: Some("https://example.com/article".to_string()),
                content: Some("Article content".to_string()),
                summary: Some("Summary".to_string()),
                author: Some("Author".to_string()),
                published_at: Some(Utc::now()),
                og_image: None,
                og_description: None,
                og_site_name: None,
            },
        )
        .await
        .unwrap();

        assert!(article.is_some());
        let article = article.unwrap();
        assert_eq!(article.title, "Test Article");
        assert_eq!(article.guid, "guid-123");

        // Try to insert same article again (should be ignored due to conflict)
        let duplicate = insert_article_if_new(
            &pool,
            NewArticle {
                feed_id: feed.id,
                guid: "guid-123".to_string(),
                title: "Test Article Updated".to_string(),
                url: Some("https://example.com/article".to_string()),
                content: Some("Updated content".to_string()),
                summary: None,
                author: None,
                published_at: None,
                og_image: None,
                og_description: None,
                og_site_name: None,
            },
        )
        .await
        .unwrap();

        assert!(duplicate.is_none());
    }

    #[tokio::test]
    async fn test_update_article_read_status() {
        let pool = setup_test_db().await;

        let feed = super::create_feed(
            &pool,
            CreateFeed {
                url: "https://example.com/feed".to_string(),
                title: "Test Feed".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        let article = insert_article_if_new(
            &pool,
            NewArticle {
                feed_id: feed.id,
                guid: "guid-123".to_string(),
                title: "Test Article".to_string(),
                url: None,
                content: None,
                summary: None,
                author: None,
                published_at: None,
                og_image: None,
                og_description: None,
                og_site_name: None,
            },
        )
        .await
        .unwrap()
        .unwrap();

        assert!(!article.is_read);

        update_article_read_status(&pool, article.id, true)
            .await
            .unwrap();

        let updated = get_article_by_id(&pool, article.id).await.unwrap().unwrap();
        assert!(updated.is_read);
    }

    #[tokio::test]
    async fn test_mark_all_articles_read() {
        let pool = setup_test_db().await;

        let feed = super::create_feed(
            &pool,
            CreateFeed {
                url: "https://example.com/feed".to_string(),
                title: "Test Feed".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        // Insert multiple articles
        for i in 1..=3 {
            insert_article_if_new(
                &pool,
                NewArticle {
                    feed_id: feed.id,
                    guid: format!("guid-{}", i),
                    title: format!("Article {}", i),
                    url: None,
                    content: None,
                    summary: None,
                    author: None,
                    published_at: None,
                    og_image: None,
                    og_description: None,
                    og_site_name: None,
                },
            )
            .await
            .unwrap();
        }

        let unread_count = get_total_unread_count(&pool).await.unwrap();
        assert_eq!(unread_count, 3);

        let affected = mark_all_articles_read(&pool, None).await.unwrap();
        assert_eq!(affected, 3);

        let unread_count = get_total_unread_count(&pool).await.unwrap();
        assert_eq!(unread_count, 0);
    }

    #[tokio::test]
    async fn test_get_feed_article_count() {
        let pool = setup_test_db().await;

        let feed = super::create_feed(
            &pool,
            CreateFeed {
                url: "https://example.com/feed".to_string(),
                title: "Test Feed".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        let count = get_feed_article_count(&pool, feed.id).await.unwrap();
        assert_eq!(count, 0);

        insert_article_if_new(
            &pool,
            NewArticle {
                feed_id: feed.id,
                guid: "guid-1".to_string(),
                title: "Article 1".to_string(),
                url: None,
                content: None,
                summary: None,
                author: None,
                published_at: None,
                og_image: None,
                og_description: None,
                og_site_name: None,
            },
        )
        .await
        .unwrap();

        let count = get_feed_article_count(&pool, feed.id).await.unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_update_feed_metadata() {
        let pool = setup_test_db().await;

        let feed = super::create_feed(
            &pool,
            CreateFeed {
                url: "https://example.com/feed".to_string(),
                title: "Test Feed".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        assert!(feed.etag.is_none());
        assert!(feed.last_modified.is_none());

        update_feed_metadata(
            &pool,
            feed.id,
            Some("etag-123".to_string()),
            Some("Mon, 01 Jan 2024 00:00:00 GMT".to_string()),
        )
        .await
        .unwrap();

        let updated = get_feed_by_id(&pool, feed.id).await.unwrap().unwrap();
        assert_eq!(updated.etag, Some("etag-123".to_string()));
        assert_eq!(
            updated.last_modified,
            Some("Mon, 01 Jan 2024 00:00:00 GMT".to_string())
        );
    }

    #[tokio::test]
    async fn test_list_articles_with_filters() {
        let pool = setup_test_db().await;

        let feed = super::create_feed(
            &pool,
            CreateFeed {
                url: "https://example.com/feed".to_string(),
                title: "Test Feed".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        // Insert articles with different read statuses
        let article1 = insert_article_if_new(
            &pool,
            NewArticle {
                feed_id: feed.id,
                guid: "guid-1".to_string(),
                title: "Unread Article".to_string(),
                url: None,
                content: None,
                summary: None,
                author: None,
                published_at: None,
                og_image: None,
                og_description: None,
                og_site_name: None,
            },
        )
        .await
        .unwrap()
        .unwrap();

        let article2 = insert_article_if_new(
            &pool,
            NewArticle {
                feed_id: feed.id,
                guid: "guid-2".to_string(),
                title: "Read Article".to_string(),
                url: None,
                content: None,
                summary: None,
                author: None,
                published_at: None,
                og_image: None,
                og_description: None,
                og_site_name: None,
            },
        )
        .await
        .unwrap()
        .unwrap();

        update_article_read_status(&pool, article2.id, true)
            .await
            .unwrap();

        // Test filter by unread
        let unread = list_articles(&pool, None, Some(false), None, None, None, None, 10, 0)
            .await
            .unwrap();
        assert_eq!(unread.len(), 1);
        assert_eq!(unread[0].id, article1.id);

        // Test filter by read
        let read = list_articles(&pool, None, Some(true), None, None, None, None, 10, 0)
            .await
            .unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].id, article2.id);

        // Test no filter
        let all = list_articles(&pool, None, None, None, None, None, None, 10, 0)
            .await
            .unwrap();
        assert_eq!(all.len(), 2);
    }
}
