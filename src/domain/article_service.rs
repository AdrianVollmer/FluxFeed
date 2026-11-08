use crate::domain::models::{Article, FeedPublishStats};
use crate::infrastructure::repository;
use sqlx::SqlitePool;
use thiserror::Error;

/// Number of smart feed pills shown under the articles header, not counting
/// the "All" pill or the trailing "more filters" pill.
pub const SMART_FEED_PILL_LIMIT: usize = 4;

#[derive(Error, Debug)]
pub enum ArticleServiceError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Article not found")]
    NotFound,
}

pub async fn toggle_read_status(
    pool: &SqlitePool,
    article_id: i64,
) -> Result<Article, ArticleServiceError> {
    let article = repository::get_article_by_id(pool, article_id)
        .await?
        .ok_or(ArticleServiceError::NotFound)?;

    let new_status = !article.is_read;
    repository::update_article_read_status(pool, article_id, new_status).await?;

    let updated = repository::get_article_by_id(pool, article_id)
        .await?
        .ok_or(ArticleServiceError::NotFound)?;

    Ok(updated)
}

pub async fn toggle_starred_status(
    pool: &SqlitePool,
    article_id: i64,
) -> Result<Article, ArticleServiceError> {
    let article = repository::get_article_by_id(pool, article_id)
        .await?
        .ok_or(ArticleServiceError::NotFound)?;

    let new_status = !article.is_starred;
    repository::update_article_starred_status(pool, article_id, new_status).await?;

    let updated = repository::get_article_by_id(pool, article_id)
        .await?
        .ok_or(ArticleServiceError::NotFound)?;

    Ok(updated)
}

pub async fn mark_all_read(
    pool: &SqlitePool,
    feed_id: Option<i64>,
) -> Result<u64, ArticleServiceError> {
    Ok(repository::mark_all_articles_read(pool, feed_id).await?)
}

/// Fetch and rank the feeds shown as "smart" pills under the articles header.
pub async fn get_smart_feed_pills(
    pool: &SqlitePool,
    limit: usize,
) -> Result<Vec<FeedPublishStats>, ArticleServiceError> {
    let stats = repository::get_feed_publish_stats_for_unread(pool).await?;
    Ok(rank_feeds_by_rarity(stats, limit))
}

/// Rank feeds by how rarely they publish, rarest first, and keep only the top `limit`.
///
/// A feed's rarity is the average time between its articles: the longer that gap, the
/// more likely a reader is to miss its occasional unread article among feeds that post
/// constantly. Feeds with fewer than two articles have no measurable interval, so they
/// are treated as maximally rare and sort first.
fn rank_feeds_by_rarity(mut stats: Vec<FeedPublishStats>, limit: usize) -> Vec<FeedPublishStats> {
    stats.sort_by(|a, b| {
        rarity_seconds(b)
            .partial_cmp(&rarity_seconds(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    stats.truncate(limit);
    stats
}

/// Average number of seconds between a feed's articles. `f64::MAX` when it can't be
/// computed, so such feeds sort as the rarest.
fn rarity_seconds(stats: &FeedPublishStats) -> f64 {
    match (stats.first_published_at, stats.last_published_at) {
        (Some(first), Some(last)) if stats.article_count > 1 && last > first => {
            (last - first).num_seconds() as f64 / (stats.article_count - 1) as f64
        }
        _ => f64::MAX,
    }
}

pub async fn mark_as_read(
    pool: &SqlitePool,
    article_id: i64,
) -> Result<Article, ArticleServiceError> {
    let article = repository::get_article_by_id(pool, article_id)
        .await?
        .ok_or(ArticleServiceError::NotFound)?;

    // Only update if not already read
    if !article.is_read {
        repository::update_article_read_status(pool, article_id, true).await?;
    }

    let updated = repository::get_article_by_id(pool, article_id)
        .await?
        .ok_or(ArticleServiceError::NotFound)?;

    Ok(updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_article_service_error_display() {
        let err = ArticleServiceError::NotFound;
        assert_eq!(err.to_string(), "Article not found");
    }

    #[test]
    fn test_article_service_error_from_sqlx() {
        let sqlx_err = sqlx::Error::RowNotFound;
        let article_err: ArticleServiceError = sqlx_err.into();
        assert!(matches!(article_err, ArticleServiceError::DatabaseError(_)));
    }

    fn stats(
        feed_id: i64,
        article_count: i64,
        span_days: i64,
        unread_count: i64,
    ) -> FeedPublishStats {
        let last = chrono::Utc::now();
        let first = last - chrono::Duration::days(span_days);
        FeedPublishStats {
            feed_id,
            title: format!("Feed {}", feed_id),
            color: "#000000".to_string(),
            unread_count,
            article_count,
            first_published_at: Some(first),
            last_published_at: Some(last),
        }
    }

    #[test]
    fn rank_feeds_by_rarity_puts_infrequent_publisher_first() {
        // Feed 1 posts daily over 10 days (10 articles), feed 2 posts twice a year (2
        // articles, 180 day gap) - feed 2 should rank first despite fewer unread articles.
        let frequent = stats(1, 10, 10, 3);
        let rare = stats(2, 2, 180, 1);

        let ranked = rank_feeds_by_rarity(vec![frequent, rare], 10);

        assert_eq!(ranked[0].feed_id, 2);
        assert_eq!(ranked[1].feed_id, 1);
    }

    #[test]
    fn rank_feeds_by_rarity_treats_single_article_feed_as_rarest() {
        let established = stats(1, 20, 30, 5);
        let mut single = stats(2, 1, 0, 1);
        single.first_published_at = single.last_published_at;

        let ranked = rank_feeds_by_rarity(vec![established, single], 10);

        assert_eq!(ranked[0].feed_id, 2);
    }

    #[test]
    fn rank_feeds_by_rarity_respects_limit() {
        let feeds = vec![stats(1, 5, 5, 1), stats(2, 5, 5, 1), stats(3, 5, 5, 1)];

        let ranked = rank_feeds_by_rarity(feeds, 2);

        assert_eq!(ranked.len(), 2);
    }
}
