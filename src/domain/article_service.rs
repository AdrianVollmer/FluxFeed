use crate::domain::models::Article;
use crate::infrastructure::repository;
use sqlx::SqlitePool;
use thiserror::Error;

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
}
