use crate::api::feeds::AppState;
use crate::domain::models::NewArticle;
use crate::infrastructure::{repository, rss_fetcher};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use std::time::Duration;

#[derive(Serialize)]
pub struct FetchResponse {
    message: String,
    feeds_updated: usize,
    new_articles: usize,
}

pub async fn trigger_fetch(State(state): State<AppState>) -> impl IntoResponse {
    match perform_fetch(&state).await {
        Ok((feeds_updated, new_articles)) => {
            let response = FetchResponse {
                message: "Feed fetch completed successfully".to_string(),
                feeds_updated,
                new_articles,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            // Log the actual error server-side for debugging
            tracing::error!("Manual fetch failed: {}", e);
            // Return a generic message to the client to avoid leaking internal details
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(FetchResponse {
                    message: "Feed fetch failed. Please try again later.".to_string(),
                    feeds_updated: 0,
                    new_articles: 0,
                }),
            )
        }
    }
}

async fn perform_fetch(state: &AppState) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    tracing::info!("Manual feed fetch triggered");

    let feeds = repository::get_feeds_to_update(&state.db_pool).await?;
    tracing::info!("Found {} feeds to fetch", feeds.len());

    if feeds.is_empty() {
        return Ok((0, 0));
    }

    let fetcher = rss_fetcher::RssFetcher::new()?;
    let mut new_articles_count = 0;
    let mut updated_feeds_count = 0;

    for feed in feeds {
        tracing::info!("Fetching: {} ({})", feed.title, feed.url);

        match fetcher
            .fetch_feed(
                &feed.url,
                feed.etag.as_deref(),
                feed.last_modified.as_deref(),
            )
            .await
        {
            Ok(rss_fetcher::FetchResult::Updated {
                feed: parsed_feed,
                etag,
                last_modified,
                ttl: _,
            }) => {
                tracing::info!(
                    "Feed updated: {} ({} entries)",
                    feed.title,
                    parsed_feed.entries.len()
                );

                repository::update_feed_metadata(&state.db_pool, feed.id, etag, last_modified)
                    .await?;

                for entry in parsed_feed.entries {
                    let guid = generate_guid(&entry);
                    let title = extract_title(&entry);
                    let url = extract_url(&entry);
                    let content = extract_content(&entry);
                    let summary = extract_summary(&entry);
                    let author = extract_author(&entry);
                    let published_at = extract_published_date(&entry);

                    match repository::insert_article_if_new(
                        &state.db_pool,
                        NewArticle {
                            feed_id: feed.id,
                            guid,
                            title,
                            url,
                            content,
                            summary,
                            author,
                            published_at,
                            og_image: None,
                            og_description: None,
                            og_site_name: None,
                        },
                    )
                    .await
                    {
                        Ok(Some(_)) => new_articles_count += 1,
                        Ok(None) => {}
                        Err(e) => tracing::warn!("Failed to insert article: {}", e),
                    }
                }

                updated_feeds_count += 1;
            }
            Ok(rss_fetcher::FetchResult::NotModified) => {
                tracing::info!("Feed not modified: {}", feed.title);
                repository::touch_feed(&state.db_pool, feed.id).await?;
            }
            Err(e) => {
                tracing::warn!("Failed to fetch feed {}: {}", feed.url, e);
                repository::touch_feed(&state.db_pool, feed.id).await?;
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tracing::info!(
        "Manual fetch complete: {} feeds updated, {} new articles",
        updated_feeds_count,
        new_articles_count
    );

    Ok((updated_feeds_count, new_articles_count))
}

// Helper functions (same as scheduler)
use chrono::Utc;

fn generate_guid(entry: &feed_rs::model::Entry) -> String {
    if !entry.id.is_empty() {
        entry.id.clone()
    } else if let Some(link) = entry.links.first() {
        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.as_str())
            .unwrap_or("");
        format!("{}-{}", link.href, title)
    } else {
        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.as_str())
            .unwrap_or("untitled");
        let date = entry
            .published
            .or(entry.updated)
            .map(|d| d.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        format!("{}-{}", title, date)
    }
}

fn extract_title(entry: &feed_rs::model::Entry) -> String {
    entry
        .title
        .as_ref()
        .map(|t| t.content.clone())
        .unwrap_or_else(|| "Untitled".to_string())
}

fn extract_url(entry: &feed_rs::model::Entry) -> Option<String> {
    entry.links.first().map(|link| link.href.clone())
}

fn extract_content(entry: &feed_rs::model::Entry) -> Option<String> {
    entry.content.as_ref().and_then(|c| {
        c.body.as_ref().map(|body| {
            if body.len() > 100_000 {
                format!("{}...", &body[..100_000])
            } else {
                body.clone()
            }
        })
    })
}

fn extract_summary(entry: &feed_rs::model::Entry) -> Option<String> {
    entry.summary.as_ref().map(|s| {
        if s.content.len() > 1000 {
            format!("{}...", &s.content[..1000])
        } else {
            s.content.clone()
        }
    })
}

fn extract_author(entry: &feed_rs::model::Entry) -> Option<String> {
    entry.authors.first().map(|author| author.name.clone())
}

fn extract_published_date(entry: &feed_rs::model::Entry) -> Option<chrono::DateTime<Utc>> {
    entry
        .published
        .or(entry.updated)
        .map(|dt| dt.with_timezone(&Utc))
}
