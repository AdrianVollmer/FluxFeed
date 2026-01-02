use crate::api::feeds::AppState;
use crate::infrastructure::{repository, rss_fetcher};
use chrono::Utc;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn start_scheduler(
    state: AppState,
) -> Result<JobScheduler, Box<dyn std::error::Error>> {
    let scheduler = JobScheduler::new().await?;

    // Fetch all feeds every 5 minutes
    let schedule = "0 */5 * * * *"; // Every 5 minutes

    scheduler
        .add(Job::new_async(schedule, move |_uuid, _lock| {
            let state = state.clone();
            Box::pin(async move {
                if let Err(e) = fetch_all_feeds(&state).await {
                    tracing::error!("Feed fetch cycle failed: {}", e);
                }
            })
        })?)
        .await?;

    scheduler.start().await?;
    tracing::info!("Feed scheduler started (every 5 minutes)");

    Ok(scheduler)
}

async fn fetch_all_feeds(state: &AppState) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting feed fetch cycle");

    // Get all feeds that need updating
    let feeds = repository::get_feeds_to_update(&state.db_pool).await?;

    tracing::info!("Found {} feeds to update", feeds.len());

    if feeds.is_empty() {
        return Ok(());
    }

    let fetcher = rss_fetcher::RssFetcher::new()?;
    let mut new_articles_count = 0;
    let mut updated_feeds_count = 0;

    // Process feeds sequentially with rate limiting
    for feed in feeds {
        tracing::debug!("Processing feed: {} ({})", feed.title, feed.url);

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
            }) => {
                tracing::info!(
                    "Feed updated: {} ({} entries)",
                    feed.title,
                    parsed_feed.entries.len()
                );

                // Update feed metadata
                repository::update_feed_metadata(&state.db_pool, feed.id, etag, last_modified)
                    .await?;

                // Insert new articles
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
                        feed.id,
                        guid,
                        title,
                        url,
                        content,
                        summary,
                        author,
                        published_at,
                    )
                    .await
                    {
                        Ok(Some(_article)) => {
                            new_articles_count += 1;
                        }
                        Ok(None) => {
                            // Article already exists (duplicate)
                        }
                        Err(e) => {
                            tracing::warn!("Failed to insert article: {}", e);
                        }
                    }
                }

                updated_feeds_count += 1;
            }
            Ok(rss_fetcher::FetchResult::NotModified) => {
                tracing::debug!("Feed not modified: {}", feed.title);
                // Just update last_fetched_at
                repository::touch_feed(&state.db_pool, feed.id).await?;
            }
            Err(e) => {
                tracing::warn!("Failed to fetch feed {}: {}", feed.url, e);
                // Still update last_fetched_at to avoid hammering broken feeds
                repository::touch_feed(&state.db_pool, feed.id).await?;
            }
        }

        // Rate limiting: 500ms delay between requests
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tracing::info!(
        "Feed fetch cycle complete: {} feeds updated, {} new articles",
        updated_feeds_count,
        new_articles_count
    );

    Ok(())
}

// Helper functions to extract data from feed entries

fn generate_guid(entry: &feed_rs::model::Entry) -> String {
    // Use entry ID if available and not empty
    if !entry.id.is_empty() {
        entry.id.clone()
    } else if let Some(link) = entry.links.first() {
        // Generate from link + title
        let title = entry.title.as_ref().map(|t| t.content.as_str()).unwrap_or("");
        format!("{}-{}", link.href, title)
    } else {
        // Fallback: use title + published date
        let title = entry.title.as_ref().map(|t| t.content.as_str()).unwrap_or("untitled");
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
            // Limit content size to avoid bloat (100KB max)
            let content = if body.len() > 100_000 {
                format!("{}...", &body[..100_000])
            } else {
                body.clone()
            };

            // Sanitize HTML to prevent XSS attacks
            ammonia::clean(&content)
        })
    })
}

fn extract_summary(entry: &feed_rs::model::Entry) -> Option<String> {
    entry.summary.as_ref().map(|s| {
        // Limit summary size
        let summary = if s.content.len() > 1000 {
            format!("{}...", &s.content[..1000])
        } else {
            s.content.clone()
        };

        // Sanitize HTML to prevent XSS attacks
        ammonia::clean(&summary)
    })
}

fn extract_author(entry: &feed_rs::model::Entry) -> Option<String> {
    entry.authors.first().map(|author| author.name.clone())
}

fn extract_published_date(entry: &feed_rs::model::Entry) -> Option<chrono::DateTime<Utc>> {
    entry.published.or(entry.updated).map(|dt| dt.with_timezone(&Utc))
}
