use crate::api::feeds::AppState;
use crate::infrastructure::{repository, rss_fetcher};
use chrono::Utc;
use std::time::Duration;
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn start_scheduler(state: AppState) -> Result<JobScheduler, Box<dyn std::error::Error>> {
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

/// Fetch and process a single feed, inserting new articles
pub async fn fetch_single_feed(
    pool: &sqlx::SqlitePool,
    feed: &crate::domain::models::Feed,
) -> Result<FetchSingleFeedResult, Box<dyn std::error::Error>> {
    tracing::debug!("Processing feed: {} ({})", feed.title, feed.url);

    let fetcher = rss_fetcher::RssFetcher::new()?;
    let mut new_articles_count = 0;

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

            // Log successful fetch
            repository::insert_log(pool, feed.id, "success", None, None, None).await?;

            // Update feed metadata from RSS (including title, description, site_url)
            let rss_title = parsed_feed.title.as_ref().map(|t| t.content.clone());
            let rss_description = parsed_feed.description.as_ref().map(|d| d.content.clone());
            let feed_site_url = parsed_feed.links.first().map(|link| link.href.clone());

            // Implement description fallback logic:
            // If no description exists in DB, use RSS feed's title
            // If RSS feed has no title, use URL as description
            let feed_description = if feed.description.is_none() {
                rss_description
                    .or_else(|| rss_title.clone())
                    .or_else(|| Some(feed.url.clone()))
            } else {
                // Keep existing description
                None
            };

            repository::update_feed_details(
                pool,
                feed.id,
                rss_title,
                feed_description,
                feed_site_url,
                etag,
                last_modified,
            )
            .await?;

            // Insert new articles without OpenGraph data first (for speed)
            let mut article_ids_to_fetch = Vec::new();

            for entry in parsed_feed.entries {
                let guid = generate_guid(&entry);
                let title = extract_title(&entry);
                let url = extract_url(&entry);
                let content = extract_content(&entry);
                let summary = extract_summary(&entry);
                let author = extract_author(&entry);
                let published_at = extract_published_date(&entry);

                // Insert article without OpenGraph data
                match repository::insert_article_if_new(
                    pool,
                    feed.id,
                    guid,
                    title,
                    url.clone(),
                    content,
                    summary,
                    author,
                    published_at,
                    None, // og_image - will be fetched in background
                    None, // og_description - will be fetched in background
                    None, // og_site_name - will be fetched in background
                )
                .await
                {
                    Ok(Some(article)) => {
                        new_articles_count += 1;
                        // Queue this article for OpenGraph fetching if it has a URL
                        if url.is_some() {
                            article_ids_to_fetch.push((article.id, url.unwrap()));
                        }
                    }
                    Ok(None) => {
                        // Article already exists (duplicate)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to insert article: {}", e);
                    }
                }
            }

            // Spawn background task to fetch OpenGraph metadata
            if !article_ids_to_fetch.is_empty() {
                let pool_clone = pool.clone();
                tokio::spawn(async move {
                    fetch_opengraph_for_articles(pool_clone, article_ids_to_fetch).await;
                });
            }

            Ok(FetchSingleFeedResult::Updated { new_articles_count })
        }
        Ok(rss_fetcher::FetchResult::NotModified) => {
            tracing::debug!("Feed not modified: {}", feed.title);

            // Log not modified fetch
            repository::insert_log(pool, feed.id, "not_modified", None, None, None).await?;

            // Just update last_fetched_at
            repository::touch_feed(pool, feed.id).await?;

            Ok(FetchSingleFeedResult::NotModified)
        }
        Err(e) => {
            tracing::warn!("Failed to fetch feed {}: {}", feed.url, e);

            // Extract error details for logging
            let (log_type, status_code, retry_after) = match &e {
                rss_fetcher::FetchError::RequestFailed {
                    status,
                    retry_after,
                    ..
                } => {
                    let log_type = if status.as_u16() == 429 {
                        "rate_limited"
                    } else {
                        "error"
                    };
                    (
                        log_type,
                        Some(status.as_u16() as i32),
                        retry_after.as_deref(),
                    )
                }
                _ => ("error", None, None),
            };

            let error_message = e.to_string();

            // Log the fetch failure
            repository::insert_log(
                pool,
                feed.id,
                log_type,
                status_code,
                Some(&error_message),
                retry_after,
            )
            .await?;

            // Still update last_fetched_at to avoid hammering broken feeds
            repository::touch_feed(pool, feed.id).await?;

            Err(e.into())
        }
    }
}

pub enum FetchSingleFeedResult {
    Updated { new_articles_count: usize },
    NotModified,
}

async fn fetch_all_feeds(state: &AppState) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting feed fetch cycle");

    // Get all feeds that need updating
    let feeds = repository::get_feeds_to_update(&state.db_pool).await?;

    tracing::info!("Found {} feeds to update", feeds.len());

    if feeds.is_empty() {
        return Ok(());
    }

    let mut new_articles_total = 0;
    let mut updated_feeds_count = 0;

    // Process feeds sequentially with rate limiting
    for feed in feeds {
        match fetch_single_feed(&state.db_pool, &feed).await {
            Ok(FetchSingleFeedResult::Updated { new_articles_count }) => {
                new_articles_total += new_articles_count;
                updated_feeds_count += 1;
            }
            Ok(FetchSingleFeedResult::NotModified) => {
                // Feed not modified
            }
            Err(e) => {
                tracing::warn!("Failed to fetch feed {}: {}", feed.url, e);
            }
        }

        // Rate limiting: 500ms delay between requests
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tracing::info!(
        "Feed fetch cycle complete: {} feeds updated, {} new articles",
        updated_feeds_count,
        new_articles_total
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
        let title = entry
            .title
            .as_ref()
            .map(|t| t.content.as_str())
            .unwrap_or("");
        format!("{}-{}", link.href, title)
    } else {
        // Fallback: use title + published date
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
            // Sanitize HTML to prevent XSS attacks
            // Don't truncate - let CSS handle visual limiting to avoid breaking HTML tags
            ammonia::clean(body)
        })
    })
}

fn extract_summary(entry: &feed_rs::model::Entry) -> Option<String> {
    entry.summary.as_ref().map(|s| {
        // Sanitize HTML to prevent XSS attacks
        // Don't truncate - let CSS handle visual limiting to avoid breaking HTML tags
        ammonia::clean(&s.content)
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

/// Fetch OpenGraph metadata for multiple articles in the background
async fn fetch_opengraph_for_articles(
    pool: sqlx::SqlitePool,
    articles: Vec<(i64, String)>, // (article_id, url)
) {
    let article_count = articles.len();
    tracing::info!(
        "Starting background OpenGraph fetch for {} articles",
        article_count
    );

    for (article_id, url) in articles {
        // Fetch OpenGraph metadata
        let (og_image, og_description, og_site_name) = extract_opengraph_from_url(&url).await;

        // Update article with OpenGraph data if any was found
        if og_image.is_some() || og_description.is_some() || og_site_name.is_some() {
            match repository::update_article_opengraph(
                &pool,
                article_id,
                og_image,
                og_description,
                og_site_name,
            )
            .await
            {
                Ok(_) => {
                    tracing::debug!("Updated OpenGraph data for article {}", article_id);
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to update OpenGraph for article {}: {}",
                        article_id,
                        e
                    );
                }
            }
        }

        // Small delay between requests to be a good citizen
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    tracing::info!(
        "Completed background OpenGraph fetch for {} articles",
        article_count
    );
}

async fn extract_opengraph_from_url(
    url_str: &str,
) -> (Option<String>, Option<String>, Option<String>) {
    // Try to fetch and parse OpenGraph metadata
    match webpage::Webpage::from_url(url_str, webpage::WebpageOptions::default()) {
        Ok(webpage) => {
            let og_image = webpage
                .html
                .opengraph
                .images
                .first()
                .map(|img| img.url.clone());
            let og_description = webpage
                .html
                .opengraph
                .properties
                .get("og:description")
                .cloned();
            let og_site_name = webpage
                .html
                .opengraph
                .properties
                .get("og:site_name")
                .cloned();

            tracing::debug!(
                "Extracted OpenGraph from {}: image={:?}, desc={:?}, site={:?}",
                url_str,
                og_image.is_some(),
                og_description.is_some(),
                og_site_name.is_some()
            );

            (og_image, og_description, og_site_name)
        }
        Err(e) => {
            tracing::debug!("Failed to extract OpenGraph from {}: {}", url_str, e);
            (None, None, None)
        }
    }
}
