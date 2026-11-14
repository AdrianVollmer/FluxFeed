# Digest Page Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `/digest` page: a one-feed-at-a-time triage queue of every feed with unread articles, rarest publishers first, with explicit "Next Feed" / "Mark Read & Next" actions and a swipe-left gesture on mobile.

**Architecture:** Almost entirely additive. The queue is computed fresh on every request (no server-side session state) from the existing `get_feed_publish_stats_for_unread` query plus the existing rarity-sort logic (split out of `rank_feeds_by_rarity` so both the smart pills and the digest page share it). Position in the queue lives in a `skip` query parameter. "Mark Read & Next" reuses the existing `/articles/mark-all-read` endpoint unmodified — it already returns `HX-Refresh: true`, which reloads the current `/digest?skip=N` URL, and since marking the feed read removes it from the queue, the same `skip` value now shows what used to be next.

**Tech Stack:** Rust (Axum, Askama, sqlx/SQLite), TypeScript (esbuild), Tailwind CSS, htmx.

**Spec:** `docs/superpowers/specs/2026-11-14-digest-page-design.md`

---

### Task 1: Split `rank_feeds_by_rarity` into a pure sort + a truncating wrapper

Both the existing articles-page smart pills and the new digest queue need "all
feeds with unread articles, sorted rarest-first" — but the pills also cap it to
`SMART_FEED_PILL_LIMIT`, while the digest page wants the whole thing. Split the
sort from the truncation so both callers share one sort implementation.

**Files:**
- Modify: `src/domain/article_service.rs`

- [ ] **Step 1: Replace `rank_feeds_by_rarity` with a pure sort function plus a thin truncating wrapper**

In `src/domain/article_service.rs`, replace this function:

```rust
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
```

with these two:

```rust
/// Sort feeds by how rarely they publish, rarest first.
///
/// A feed's rarity is the average time between its articles: the longer that gap, the
/// more likely a reader is to miss its occasional unread article among feeds that post
/// constantly. Feeds with fewer than two articles have no measurable interval, so they
/// are treated as maximally rare and sort first.
fn sort_feeds_by_rarity(mut stats: Vec<FeedPublishStats>) -> Vec<FeedPublishStats> {
    stats.sort_by(|a, b| {
        rarity_seconds(b)
            .partial_cmp(&rarity_seconds(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    stats
}

/// Sort feeds by rarity (see `sort_feeds_by_rarity`) and keep only the top `limit`.
fn rank_feeds_by_rarity(stats: Vec<FeedPublishStats>, limit: usize) -> Vec<FeedPublishStats> {
    let mut sorted = sort_feeds_by_rarity(stats);
    sorted.truncate(limit);
    sorted
}
```

`get_smart_feed_pills` (which calls `rank_feeds_by_rarity`) needs no changes — it
still gets sorted-and-truncated results.

- [ ] **Step 2: Update the two tests that test sorting (not truncation) to call `sort_feeds_by_rarity`**

In the same file's `#[cfg(test)] mod tests`, rename and update these two tests:

```rust
    #[test]
    fn sort_feeds_by_rarity_puts_infrequent_publisher_first() {
        // Feed 1 posts daily over 10 days (10 articles), feed 2 posts twice a year (2
        // articles, 180 day gap) - feed 2 should rank first despite fewer unread articles.
        let frequent = stats(1, 10, 10, 3);
        let rare = stats(2, 2, 180, 1);

        let sorted = sort_feeds_by_rarity(vec![frequent, rare]);

        assert_eq!(sorted[0].feed_id, 2);
        assert_eq!(sorted[1].feed_id, 1);
    }

    #[test]
    fn sort_feeds_by_rarity_treats_single_article_feed_as_rarest() {
        let established = stats(1, 20, 30, 5);
        let mut single = stats(2, 1, 0, 1);
        single.first_published_at = single.last_published_at;

        let sorted = sort_feeds_by_rarity(vec![established, single]);

        assert_eq!(sorted[0].feed_id, 2);
    }
```

Leave `rank_feeds_by_rarity_respects_limit` exactly as it is — it still tests the
truncating wrapper and should keep calling `rank_feeds_by_rarity(feeds, 2)`.

- [ ] **Step 3: Run the tests**

Run: `SQLX_OFFLINE=true cargo test sort_feeds_by_rarity rank_feeds_by_rarity`

Expected: all 3 tests (`sort_feeds_by_rarity_puts_infrequent_publisher_first`,
`sort_feeds_by_rarity_treats_single_article_feed_as_rarest`,
`rank_feeds_by_rarity_respects_limit`) pass, in both the `fluxfeed` lib target and
the `fluxfeed` bin target (the project compiles the same source as both — this is
normal and expected, not a duplicate-test bug).

- [ ] **Step 4: Commit**

```bash
cd /workspace
git add src/domain/article_service.rs
git commit -m "Split rank_feeds_by_rarity into a pure sort plus a truncating wrapper"
```

---

### Task 2: Add repository test coverage for the unread-only filter

`get_feed_publish_stats_for_unread` already excludes feeds with no unread articles
(via `HAVING unread_count > 0` in its SQL), but there's no test asserting it. The
digest page depends on this exclusion being correct, so close the gap before
building on top of it.

**Files:**
- Modify: `src/infrastructure/repository.rs`

- [ ] **Step 1: Write the test**

Add this test to the `#[cfg(test)] mod tests` block in
`src/infrastructure/repository.rs` (place it near the other feed/article tests,
e.g. right after `test_update_article_read_status`):

```rust
    #[tokio::test]
    async fn test_get_feed_publish_stats_for_unread_excludes_fully_read_feeds() {
        let pool = setup_test_db().await;

        let unread_feed = super::create_feed(
            &pool,
            CreateFeed {
                url: "https://unread.example.com/feed".to_string(),
                title: "Has Unread".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        let read_feed = super::create_feed(
            &pool,
            CreateFeed {
                url: "https://read.example.com/feed".to_string(),
                title: "All Read".to_string(),
                description: None,
            },
        )
        .await
        .unwrap();

        insert_article_if_new(
            &pool,
            NewArticle {
                feed_id: unread_feed.id,
                guid: "unread-guid".to_string(),
                title: "Unread Article".to_string(),
                url: None,
                content: None,
                summary: None,
                author: None,
                published_at: Some(Utc::now()),
                og_image: None,
                og_description: None,
                og_site_name: None,
            },
        )
        .await
        .unwrap();

        let read_article = insert_article_if_new(
            &pool,
            NewArticle {
                feed_id: read_feed.id,
                guid: "read-guid".to_string(),
                title: "Read Article".to_string(),
                url: None,
                content: None,
                summary: None,
                author: None,
                published_at: Some(Utc::now()),
                og_image: None,
                og_description: None,
                og_site_name: None,
            },
        )
        .await
        .unwrap()
        .unwrap();

        update_article_read_status(&pool, read_article.id, true)
            .await
            .unwrap();

        let stats = get_feed_publish_stats_for_unread(&pool).await.unwrap();

        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].feed_id, unread_feed.id);
        assert_eq!(stats[0].unread_count, 1);
    }
```

- [ ] **Step 2: Run the test**

Run: `SQLX_OFFLINE=true cargo test test_get_feed_publish_stats_for_unread_excludes_fully_read_feeds`

Expected: PASS (the SQL already does this correctly — this step is closing a test
gap, not fixing a bug).

- [ ] **Step 3: Commit**

```bash
cd /workspace
git add src/infrastructure/repository.rs
git commit -m "Add test coverage for unread-only filter in feed publish stats"
```

---

### Task 3: Add `article_service::get_digest_queue`

**Files:**
- Modify: `src/domain/article_service.rs`

- [ ] **Step 1: Add the function**

Add this directly above `get_smart_feed_pills` in `src/domain/article_service.rs`:

```rust
/// Fetch the full digest queue: every feed with unread articles, sorted
/// rarest-publisher-first. Unlike `get_smart_feed_pills`, nothing is truncated —
/// the digest page is a queue to work through, not a highlight reel.
pub async fn get_digest_queue(
    pool: &SqlitePool,
) -> Result<Vec<FeedPublishStats>, ArticleServiceError> {
    let stats = repository::get_feed_publish_stats_for_unread(pool).await?;
    Ok(sort_feeds_by_rarity(stats))
}
```

No new test is added for this function on its own: it's a two-line composition of
`repository::get_feed_publish_stats_for_unread` (covered by Task 2's test) and
`sort_feeds_by_rarity` (covered by Task 1's tests), with no independent logic of
its own to test.

- [ ] **Step 2: Build to confirm it compiles**

Run: `SQLX_OFFLINE=true cargo build`

Expected: builds successfully. (`get_digest_queue` isn't called from anywhere yet,
so expect a `dead_code` warning, not an error — that's fine, it'll go away once
Task 6 calls it.)

- [ ] **Step 3: Commit**

```bash
cd /workspace
git add src/domain/article_service.rs
git commit -m "Add article_service::get_digest_queue"
```

---

### Task 4: Make `attach_tags_to_articles` available to other API modules

The digest handler (Task 6) needs to attach tags to the articles it fetches, the
same way the articles page does. Rather than duplicate that batch-fetch-and-attach
logic, widen the visibility of the existing helper.

**Files:**
- Modify: `src/api/articles.rs:72` (the `attach_tags_to_articles` function signature)

- [ ] **Step 1: Change the visibility**

In `src/api/articles.rs`, change:

```rust
/// Fetch tags for all unique feeds and attach to articles
async fn attach_tags_to_articles(
```

to:

```rust
/// Fetch tags for all unique feeds and attach to articles
pub(crate) async fn attach_tags_to_articles(
```

- [ ] **Step 2: Build to confirm nothing breaks**

Run: `SQLX_OFFLINE=true cargo build`

Expected: builds successfully with no new warnings (widening visibility of an
already-used function doesn't trigger `dead_code`).

- [ ] **Step 3: Commit**

```bash
cd /workspace
git add src/api/articles.rs
git commit -m "Make attach_tags_to_articles pub(crate) so the digest page can reuse it"
```

---

### Task 5: Add the `DigestTemplate` struct and its HTML template

Askama checks that the HTML file referenced by `#[template(path = "...")]` exists
*at compile time*, so the template file and the Rust struct that references it have
to land together.

**Files:**
- Create: `src/web/templates/digest/show.html`
- Modify: `src/web/templates.rs`

- [ ] **Step 1: Create the template file**

Create `src/web/templates/digest/show.html`:

```html
{% extends "base.html" %}

{% block title %}Digest - FluxFeed{% endblock %}

{% block scripts %}
<script src="{{ "articles.js"|js_path }}" defer></script>
{% endblock %}

{% block content %}
<div class="max-w-3xl mx-auto">
    <div class="flex justify-between items-center mb-6">
        <h1 class="text-3xl font-bold">Digest</h1>
        {% if !caught_up %}
        <div class="text-sm text-gray-600 dark:text-gray-400">
            Feed {{ queue_position }} of {{ queue_total }}
        </div>
        {% endif %}
    </div>

    {% if caught_up %}
    <div class="card empty-state">
        <div class="empty-state-icon">
            {% include "icons/newspaper.html" %}
        </div>
        <p class="empty-state-title">You're all caught up!</p>
        <p class="empty-state-description">
            No feeds have unread articles right now.
            <a href="/articles" class="text-blue-600 hover:underline">View all articles</a>
        </p>
    </div>
    {% else %}
    <div id="digest-card" class="card mb-6">
        <div class="flex items-center justify-between gap-4 mb-4">
            <div class="flex items-center gap-2 min-w-0">
                <span class="w-3 h-3 rounded-full flex-shrink-0" style="background-color: {{ feed.as_ref().unwrap().color }}"></span>
                <h2 class="text-xl font-semibold truncate">{{ feed.as_ref().unwrap().title }}</h2>
            </div>
            <span class="flex-shrink-0 text-sm text-gray-500 dark:text-gray-400">
                {{ feed.as_ref().unwrap().unread_count }} unread
            </span>
        </div>

        <div class="space-y-4">
            {% include "articles/_article_rows.html" %}
        </div>

        {% if has_more_articles %}
        <p class="mt-4 text-sm">
            <a href="/articles?feed_ids={{ feed.as_ref().unwrap().feed_id }}&amp;is_read=false" class="text-blue-600 hover:underline">
                View all {{ feed.as_ref().unwrap().unread_count }} unread in Articles
            </a>
        </p>
        {% endif %}
    </div>

    <div class="flex flex-col sm:flex-row gap-4 justify-center items-center">
        <a id="digest-next-link"
           href="/digest?skip={{ next_skip }}"
           class="btn btn-secondary w-full sm:w-44 flex items-center justify-center gap-2">
            Next Feed &rarr;
        </a>
        <button
            hx-post="/articles/mark-all-read?feed_ids={{ feed.as_ref().unwrap().feed_id }}"
            hx-confirm="Mark all {{ feed.as_ref().unwrap().unread_count }} articles in &quot;{{ feed.as_ref().unwrap().title }}&quot; as read? This can't be undone."
            class="btn btn-primary w-full sm:w-44 flex items-center justify-center gap-2">
            {% include "icons/check-double.html" %}
            Mark Read &amp; Next
        </button>
    </div>
    {% endif %}
</div>
{% endblock %}
```

Notes on why this is safe to reuse directly:
- `articles/_article_rows.html` just loops `{% for item in articles %}` and
  includes `articles/article_row.html` per item — it only needs an `articles`
  field in scope, which `DigestTemplate` provides (Step 2 below).
- `articles/article_row.html` and the action icons it includes
  (`articles/_action_icons.html`) only reference `item` and a locally-`{% set %}`
  `is_compact` — they don't depend on any other page's ambient template fields
  (no `filters`, no `offset`), so nothing page-specific leaks in.
- `articles.js` is included in the scripts block because
  `article_row.html`'s "Show more / Show less" content-collapse behavior
  (`checkCollapsibleContent`, the `toggle-content` click handler) lives there.

- [ ] **Step 2: Add the `DigestTemplate` struct**

In `src/web/templates.rs`, append this to the end of the file (after
`TagFormTemplate`), as a new section:

```rust

// Digest templates

#[derive(Template)]
#[template(path = "digest/show.html")]
pub struct DigestTemplate {
    pub caught_up: bool,
    pub queue_position: usize, // 1-based; meaningless when caught_up is true
    pub queue_total: usize,
    pub feed: Option<FeedPublishStats>,
    pub articles: Vec<ArticleWithFeed>,
    pub has_more_articles: bool,
    pub next_skip: usize,
}
```

No new imports are needed — `FeedPublishStats` is already imported at the top of
`src/web/templates.rs` (it's used by `ArticlesListTemplate`'s `smart_feed_pills`
field), and `ArticleWithFeed` is defined earlier in this same file.

- [ ] **Step 3: Build to confirm it compiles**

Run: `SQLX_OFFLINE=true cargo build`

Expected: builds successfully (expect a `dead_code` warning on `DigestTemplate`
since nothing constructs it yet — that's resolved in Task 6).

- [ ] **Step 4: Commit**

```bash
cd /workspace
git add src/web/templates/digest/show.html src/web/templates.rs
git commit -m "Add DigestTemplate and its HTML template"
```

---

### Task 6: Add the `/digest` handler

**Files:**
- Create: `src/api/digest.rs`
- Modify: `src/api/mod.rs`

- [ ] **Step 1: Create the handler module**

Create `src/api/digest.rs`:

```rust
use crate::api::articles::attach_tags_to_articles;
use crate::api::feeds::AppState;
use crate::domain::article_service;
use crate::infrastructure::repository;
use crate::web::templates::{DigestTemplate, ErrorTemplate};
use askama::Template;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DigestParams {
    pub skip: Option<usize>,
}

pub async fn show_digest(
    State(state): State<AppState>,
    Query(params): Query<DigestParams>,
) -> Result<Html<String>, AppError> {
    let skip = params.skip.unwrap_or(0);
    let queue = article_service::get_digest_queue(&state.db_pool).await?;
    let queue_total = queue.len();

    if skip >= queue_total {
        let template = DigestTemplate {
            caught_up: true,
            queue_position: 0,
            queue_total,
            feed: None,
            articles: Vec::new(),
            has_more_articles: false,
            next_skip: 0,
        };
        return Ok(Html(template.render()?));
    }

    let feed_stats = queue
        .into_iter()
        .nth(skip)
        .expect("skip was already checked against queue_total above");

    let articles_with_feed = repository::list_articles_with_feeds(
        &state.db_pool,
        Some(vec![feed_stats.feed_id]),
        Some(false),
        None,
        None,
        None,
        None,
        21, // fetch one extra to detect "more than 20" (same pattern as the articles page)
        0,
    )
    .await?;

    let has_more_articles = articles_with_feed.len() > 20;
    let articles: Vec<_> = articles_with_feed.into_iter().take(20).collect();
    let articles = attach_tags_to_articles(&state.db_pool, articles).await?;

    let template = DigestTemplate {
        caught_up: false,
        queue_position: skip + 1,
        queue_total,
        feed: Some(feed_stats),
        articles,
        has_more_articles,
        next_skip: skip + 1,
    };

    Ok(Html(template.render()?))
}

// Error handling

pub enum AppError {
    TemplateError(askama::Error),
    ServiceError(article_service::ArticleServiceError),
    DatabaseError(sqlx::Error),
}

impl From<askama::Error> for AppError {
    fn from(err: askama::Error) -> Self {
        AppError::TemplateError(err)
    }
}

impl From<article_service::ArticleServiceError> for AppError {
    fn from(err: article_service::ArticleServiceError) -> Self {
        AppError::ServiceError(err)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::DatabaseError(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status_code, status_text, message) = match self {
            AppError::TemplateError(err) => {
                tracing::error!("Template error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    "An error occurred while rendering the page. Please try again later."
                        .to_string(),
                )
            }
            AppError::ServiceError(err) => {
                tracing::error!("Digest service error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    "An error occurred while loading the digest. Please try again later."
                        .to_string(),
                )
            }
            AppError::DatabaseError(err) => {
                tracing::error!("Database error: {}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal Server Error".to_string(),
                    "A database error occurred. Please try again later.".to_string(),
                )
            }
        };

        let template = ErrorTemplate {
            status_code: status_code.as_u16(),
            status_text,
            message,
        };

        match template.render() {
            Ok(html) => (status_code, Html(html)).into_response(),
            Err(err) => {
                tracing::error!("Error rendering error template: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}
```

- [ ] **Step 2: Register the module**

In `src/api/mod.rs`, add `pub mod digest;` to the alphabetically-sorted list:

```rust
pub mod articles;
pub mod digest;
pub mod feeds;
pub mod groups;
pub mod health;
pub mod logs;
pub mod manual_fetch;
pub mod reader;
pub mod tags;
```

- [ ] **Step 3: Build to confirm it compiles**

Run: `SQLX_OFFLINE=true cargo build`

Expected: builds successfully. `get_digest_queue`'s and `DigestTemplate`'s
`dead_code` warnings from Tasks 3 and 5 are now gone (both are used here), but
expect a new one: `show_digest` itself isn't routed yet, so there's no warning for
it (binary targets don't warn about unused `pub` items the same way, and it will be
routed in Task 7 anyway).

- [ ] **Step 4: Commit**

```bash
cd /workspace
git add src/api/digest.rs src/api/mod.rs
git commit -m "Add the /digest handler"
```

---

### Task 7: Route `/digest` and add integration test coverage

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/api_integration_tests.rs`

- [ ] **Step 1: Register the route**

In `src/main.rs`, add the route immediately before the existing `/articles` route
(around line 121):

```rust
        .route("/digest", get(api::digest::show_digest))
        .route("/articles", get(api::articles::list_articles))
```

- [ ] **Step 2: Add an integration test for the empty-queue case**

In `tests/api_integration_tests.rs`, add `digest` to the import list:

```rust
use fluxfeed::api::{articles, digest, feeds, health};
```

Add the route to `setup_test_app`'s router, right before `.with_state(state)`:

```rust
        .route("/digest", axum::routing::get(digest::show_digest))
        .with_state(state);
```

Add this test, next to `test_list_articles_empty`:

```rust
#[tokio::test]
async fn test_digest_empty() {
    let (server, _pool) = setup_test_app().await;

    let response = server.get("/digest").await;

    assert_eq!(response.status_code(), StatusCode::OK);
    let body = response.text();
    assert!(body.contains("caught up"));
}
```

- [ ] **Step 3: Run the full test suite**

Run: `just test`

Expected: all tests pass, including the new `test_digest_empty`.

- [ ] **Step 4: Manually verify the route serves a real queue**

Run:

```bash
cd /workspace
rm -f /tmp/fluxfeed-digest-check.db
DATABASE_URL="sqlite:///tmp/fluxfeed-digest-check.db" SQLX_OFFLINE=true PORT=3838 HOST=127.0.0.1 ./target/debug/fluxfeed &
sleep 1
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:3838/digest
curl -s http://127.0.0.1:3838/digest | grep -o "caught up"
kill %1
rm -f /tmp/fluxfeed-digest-check.db
```

Expected: `200`, then `caught up` (no feeds exist yet, so the empty-queue end
screen renders).

- [ ] **Step 5: Commit**

```bash
cd /workspace
git add src/main.rs tests/api_integration_tests.rs
git commit -m "Route GET /digest and add integration test coverage"
```

---

### Task 8: Add "Digest" to the navigation

**Files:**
- Modify: `src/web/templates/components/header.html`

- [ ] **Step 1: Add it as the first item in the desktop nav**

In `src/web/templates/components/header.html`, find the desktop nav block:

```html
                <div class="hidden md:flex space-x-4 md:ml-4">
                    <a href="/articles" class="text-gray-700 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400">
                        Articles
                    </a>
```

and insert a new link before it:

```html
                <div class="hidden md:flex space-x-4 md:ml-4">
                    <a href="/digest" class="text-gray-700 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400">
                        Digest
                    </a>
                    <a href="/articles" class="text-gray-700 dark:text-gray-300 hover:text-blue-600 dark:hover:text-blue-400">
                        Articles
                    </a>
```

- [ ] **Step 2: Add it as the first item in the mobile slide-in nav**

In the same file, find the mobile nav block:

```html
    <nav class="flex flex-col p-4 space-y-1">
        <a href="/articles"
           onclick="document.getElementById('mobile-nav-toggle').checked = false"
           class="px-3 py-2 rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors">
            Articles
        </a>
```

and insert a new link before it:

```html
    <nav class="flex flex-col p-4 space-y-1">
        <a href="/digest"
           onclick="document.getElementById('mobile-nav-toggle').checked = false"
           class="px-3 py-2 rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors">
            Digest
        </a>
        <a href="/articles"
           onclick="document.getElementById('mobile-nav-toggle').checked = false"
           class="px-3 py-2 rounded-lg text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors">
            Articles
        </a>
```

- [ ] **Step 3: Build to confirm it compiles**

Run: `SQLX_OFFLINE=true cargo build`

Expected: builds successfully (template changes are checked at compile time by
Askama).

- [ ] **Step 4: Commit**

```bash
cd /workspace
git add src/web/templates/components/header.html
git commit -m "Add Digest as the first nav item"
```

---

### Task 9: Swipe-left gesture on mobile

**Files:**
- Create: `static/ts/digest.ts`
- Modify: `src/web/templates/digest/show.html`

- [ ] **Step 1: Create the TypeScript file**

Create `static/ts/digest.ts`:

```typescript
/**
 * Digest page functionality
 * - Swipe left on the feed card advances to the next feed, exactly like clicking
 *   the "Next Feed" link. Non-destructive: it never changes read state, so it
 *   needs no confirmation (unlike "Mark Read & Next", which stays a tap + confirm
 *   dialog everywhere, never a bare gesture).
 */

function initDigestSwipe(): void {
  const card = document.getElementById('digest-card');
  const nextLink = document.getElementById('digest-next-link') as HTMLAnchorElement | null;
  if (!card || !nextLink) return;

  let touchStartX = 0;
  let touchStartY = 0;
  const swipeThreshold = 60; // min px to count as a deliberate swipe

  card.addEventListener('touchstart', (e: TouchEvent) => {
    touchStartX = e.touches[0].clientX;
    touchStartY = e.touches[0].clientY;
  }, { passive: true });

  card.addEventListener('touchend', (e: TouchEvent) => {
    const touchEndX = e.changedTouches[0].clientX;
    const touchEndY = e.changedTouches[0].clientY;
    const deltaX = touchEndX - touchStartX;
    const deltaY = Math.abs(touchEndY - touchStartY);

    // Ignore vertical-dominant touches (the user is scrolling, not swiping)
    if (deltaY > Math.abs(deltaX)) return;

    if (deltaX < -swipeThreshold) {
      window.location.href = nextLink.href;
    }
  }, { passive: true });
}

document.addEventListener('DOMContentLoaded', initDigestSwipe);
```

The navigation target is read straight from the existing `#digest-next-link`
element's `href` rather than reconstructed in JS, so the `skip` arithmetic has one
source of truth (the template).

- [ ] **Step 2: Reference it from the template's scripts block**

In `src/web/templates/digest/show.html`, change:

```html
{% block scripts %}
<script src="{{ "articles.js"|js_path }}" defer></script>
{% endblock %}
```

to:

```html
{% block scripts %}
<script src="{{ "articles.js"|js_path }}" defer></script>
<script src="{{ "digest.js"|js_path }}" defer></script>
{% endblock %}
```

- [ ] **Step 3: Build the frontend**

Run: `just frontend`

Expected output includes:

```
Built: static/ts/digest.ts -> digest.<hash>.js
```

(`scripts/build-ts.js` scans every `.ts` file in `static/ts/`, so the new file is
picked up automatically — no build config changes needed.)

- [ ] **Step 4: Build the backend and confirm it still compiles**

Run: `SQLX_OFFLINE=true cargo build`

Expected: builds successfully (the `js_path` filter falls back to the literal
filename if the manifest lookup fails, so even a stale manifest wouldn't break the
build — but Step 3 already regenerated it correctly).

- [ ] **Step 5: Commit**

```bash
cd /workspace
git add static/ts/digest.ts src/web/templates/digest/show.html static/js/dist/manifest.json
git commit -m "Add swipe-left-for-next gesture on the digest page"
```

---

### Task 10: End-to-end verification

No code changes in this task — it's a manual pass to confirm the whole feature
works together, the way prior UI work in this project has been verified
(Playwright against a locally-run server with seeded data).

**Files:** none (verification only)

- [ ] **Step 1: Seed a scratch database with feeds of varying publish frequency**

```bash
cd /workspace
rm -f /tmp/fluxfeed-digest-e2e.db
DATABASE_URL="sqlite:///tmp/fluxfeed-digest-e2e.db" SQLX_OFFLINE=true PORT=3939 HOST=127.0.0.1 ./target/debug/fluxfeed > /tmp/fluxfeed-digest-e2e.log 2>&1 &
echo $! > /tmp/fluxfeed-digest-e2e.pid
sleep 1
curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:3939/health
python3 - <<'EOF'
import sqlite3, datetime
conn = sqlite3.connect('/tmp/fluxfeed-digest-e2e.db')
cur = conn.cursor()
now = datetime.datetime.now(datetime.timezone.utc)

feeds = [
    ("https://a.example/feed", "Daily Tech News", "#3B82F6"),
    ("https://b.example/feed", "Quarterly Journal", "#10B981"),
    ("https://c.example/feed", "Rare Blog", "#F59E0B"),
]
feed_ids = {}
for url, title, color in feeds:
    cur.execute(
        "INSERT INTO feeds (url, title, color, fetch_frequency, fetch_interval_minutes, created_at, updated_at) VALUES (?, ?, ?, 'adaptive', 60, ?, ?)",
        (url, title, color, now.isoformat(), now.isoformat()),
    )
    feed_ids[title] = cur.lastrowid

def add_articles(title, count, interval_days, unread_count):
    fid = feed_ids[title]
    for i in range(count):
        published = now - datetime.timedelta(days=interval_days * (count - i))
        is_read = 0 if i >= count - unread_count else 1
        cur.execute(
            "INSERT INTO articles (feed_id, guid, title, url, published_at, is_read, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (fid, f"guid-{title}-{i}", f"{title} article {i}", f"https://x/{title}/{i}", published.isoformat(), is_read, now.isoformat(), now.isoformat()),
        )

add_articles("Daily Tech News", 30, 1, 5)       # posts daily -> least interesting
add_articles("Quarterly Journal", 4, 90, 1)      # posts every ~90 days -> rare
add_articles("Rare Blog", 1, 0, 1)               # single article ever -> rarest

conn.commit()
conn.close()
print("seeded", feed_ids)
EOF
```

Expected: `200`, then a `seeded {...}` line with three feed IDs.

- [ ] **Step 2: Verify queue ordering and position via Playwright**

```bash
cd /workspace
uv run --with playwright python3 - <<'EOF'
import asyncio
from playwright.async_api import async_playwright

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.launch()
        context = await browser.new_context(viewport={"width": 800, "height": 700})
        page = await context.new_page()

        # Rarest feed (Rare Blog, single article) should be first.
        await page.goto("http://127.0.0.1:3939/digest")
        await page.wait_for_load_state("networkidle")
        print("position text:", await page.locator("text=Feed 1 of 3").count())
        print("feed shown:", await page.locator("h2", has_text="Rare Blog").count())

        # Next Feed moves to position 2 without changing anything.
        await page.get_by_text("Next Feed").click()
        await page.wait_for_load_state("networkidle")
        print("after Next Feed, url:", page.url)
        print("position 2 of 3:", await page.locator("text=Feed 2 of 3").count())
        print("feed shown:", await page.locator("h2", has_text="Quarterly Journal").count())

        await context.close()
        await browser.close()

asyncio.run(main())
EOF
```

Expected:
```
position text: 1
feed shown: 1
after Next Feed, url: http://127.0.0.1:3939/digest?skip=1
position 2 of 3: 1
feed shown: 1
```

- [ ] **Step 3: Verify "Mark Read & Next" advances without changing the skip value**

```bash
cd /workspace
uv run --with playwright python3 - <<'EOF'
import asyncio
from playwright.async_api import async_playwright

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.launch()
        context = await browser.new_context(viewport={"width": 800, "height": 700})
        page = await context.new_page()
        page.on("dialog", lambda d: asyncio.ensure_future(d.accept()))

        # Start at position 1 (Rare Blog) and mark it read.
        await page.goto("http://127.0.0.1:3939/digest")
        await page.wait_for_load_state("networkidle")
        print("before mark-read, feed:", await page.locator("h2", has_text="Rare Blog").count())

        async with page.expect_navigation(wait_until="networkidle"):
            await page.get_by_text("Mark Read & Next").click()

        print("after mark-read, url:", page.url)
        # Rare Blog had its only article marked read, so it's gone from the queue.
        # Quarterly Journal now slides into position 1 - same URL as before (skip=0 or no param).
        print("feed now shown:", await page.locator("h2", has_text="Quarterly Journal").count())
        print("total now 2:", await page.locator("text=Feed 1 of 2").count())

        await context.close()
        await browser.close()

asyncio.run(main())
EOF
```

Expected:
```
before mark-read, feed: 1
after mark-read, url: http://127.0.0.1:3939/digest
feed now shown: 1
total now 2: 1
```

- [ ] **Step 4: Verify the caught-up screen once the queue is exhausted**

```bash
cd /workspace
uv run --with playwright python3 - <<'EOF'
import asyncio
from playwright.async_api import async_playwright

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.launch()
        context = await browser.new_context(viewport={"width": 800, "height": 700})
        page = await context.new_page()
        page.on("dialog", lambda d: asyncio.ensure_future(d.accept()))

        await page.goto("http://127.0.0.1:3939/digest")
        await page.wait_for_load_state("networkidle")

        # Mark-read-and-next through whatever remains in the queue.
        for _ in range(5):
            count = await page.get_by_text("Mark Read & Next").count()
            if count == 0:
                break
            async with page.expect_navigation(wait_until="networkidle"):
                await page.get_by_text("Mark Read & Next").click()

        print("caught up visible:", await page.get_by_text("caught up").count())
        await context.close()
        await browser.close()

asyncio.run(main())
EOF
```

Expected: `caught up visible: 1`

- [ ] **Step 5: Verify the swipe-left gesture on a narrow viewport**

Re-seed first (the DB from steps 2-4 is now fully read):

```bash
cd /workspace
python3 - <<'EOF'
import sqlite3
conn = sqlite3.connect('/tmp/fluxfeed-digest-e2e.db')
cur = conn.cursor()
cur.execute("UPDATE articles SET is_read = 0")
conn.commit()
conn.close()
EOF

uv run --with playwright python3 - <<'EOF'
import asyncio
from playwright.async_api import async_playwright

async def main():
    async with async_playwright() as p:
        browser = await p.chromium.launch()
        context = await browser.new_context(
            viewport={"width": 390, "height": 700},
            has_touch=True,
            is_mobile=True,
        )
        page = await context.new_page()

        await page.goto("http://127.0.0.1:3939/digest")
        await page.wait_for_load_state("networkidle")
        start_url = page.url

        await page.evaluate("""
            () => {
                const card = document.getElementById('digest-card');
                const rect = card.getBoundingClientRect();
                const y = rect.y + rect.height / 2;
                const start = new Touch({identifier: 1, target: card, clientX: rect.x + rect.width - 20, clientY: y});
                card.dispatchEvent(new TouchEvent('touchstart', {touches: [start], changedTouches: [start], bubbles: true}));
                const end = new Touch({identifier: 1, target: card, clientX: rect.x + 20, clientY: y});
                card.dispatchEvent(new TouchEvent('touchend', {touches: [], changedTouches: [end], bubbles: true}));
            }
        """)
        await page.wait_for_load_state("networkidle")
        print("start url:", start_url)
        print("after swipe url:", page.url)

        await context.close()
        await browser.close()

asyncio.run(main())
EOF
```

Expected: `after swipe url` has `skip=1` where `start url` had no `skip` param (or
`skip=0`) — confirming the swipe triggered the same navigation as "Next Feed".

- [ ] **Step 6: Verify nav placement**

```bash
curl -s http://127.0.0.1:3939/articles | grep -o '<a href="/digest"[^>]*>[^<]*<[^>]*>[^<]*' | head -1
```

Expected: output shows the `Digest` link appearing before the `Articles` link in
the nav markup (visually confirmed already in Tasks 8; this just double-checks the
built page serves it).

- [ ] **Step 7: Clean up**

```bash
kill "$(cat /tmp/fluxfeed-digest-e2e.pid)" 2>/dev/null
rm -f /tmp/fluxfeed-digest-e2e.db /tmp/fluxfeed-digest-e2e.pid /tmp/fluxfeed-digest-e2e.log /tmp/fluxfeed-digest-check.db
```

- [ ] **Step 8: Run the full test suite one last time**

Run: `just test`

Expected: all tests pass.

No commit for this task (no files changed) — if any step above surfaces a bug, fix
it, add/adjust a test if the bug was in testable logic, and commit that fix before
moving on.
