# Digest page design

**Date:** 2026-11-14
**Status:** Approved, ready for implementation plan

## Problem

The articles page already ranks feeds by "publish rarity" for the smart-pills row
(`article_service::rank_feeds_by_rarity`), surfacing feeds that post rarely but have
unread articles — these are easy to miss among high-frequency feeds. That ranking is
useful enough to deserve its own dedicated page: a focused, one-feed-at-a-time triage
flow for working through unread articles, rarest publishers first.

## Goals

- A new page, `/digest`, first in the nav (before Articles).
- Shows exactly one feed at a time, from a queue of every feed with unread articles,
  ordered rarest-publisher-first.
- Two ways to move on:
  - **Next Feed** — skip this feed, no state change.
  - **Mark Read & Next** — marks every article in this feed read, then advances.
    Must never be triggerable by a single tap or swipe alone (it's irreversible).
- On mobile, swiping left on the feed card does the same thing as "Next Feed".
- When the queue is exhausted (or empty), show a "caught up" end screen.

## Non-goals (v1)

- No "previous feed" navigation.
- No auto-advance when a feed's last unread article is individually marked read.
- No swipe gesture for "mark read" (tap + confirm dialog only).
- No slide-out/transition animation on swipe.

## Queue & position model

The queue is computed fresh on every request: every feed with at least one unread
article, sorted rarest-first using the existing rarity heuristic. There is no
server-side session state for "where you are" — position is encoded entirely in the
URL as a `skip` query parameter.

`GET /digest?skip=N` means: "sort the queue fresh, skip the first N entries, show me
the one after that." `skip` defaults to `0`.

- **Next Feed** is a plain link to `/digest?skip={N+1}`. The current feed is left
  untouched (still unread), so it's still in the queue — we have to explicitly step
  past it.
- **Mark Read & Next** reuses the *existing* `POST /articles/mark-all-read?feed_ids={id}`
  endpoint unmodified. That handler already marks the feed read and responds with
  `HX-Refresh: true`, which tells htmx to reload the current URL. Since marking the
  feed read removes it from the queue, reloading the *same* `skip=N` now lands on
  whatever used to be at `N+1`. No new backend mutation logic is needed for this
  action.
- If `skip` is past the end of the (possibly now-shorter) queue — including the
  empty-queue case — render the "caught up" end screen instead of a feed.

This mirrors the existing `loaded` query param trick used for article-list pagination
state: a refresh or a shared link reproduces the exact same view, and leaving/
returning to `/digest` always restarts at the top of the freshly-sorted queue.

## Per-feed content

For the feed at the current queue position:

- Feed name, color swatch, and unread count in the header.
- "Feed X of Y" position indicator (`skip + 1` of queue length).
- Up to 20 unread articles for that feed, reusing the existing article-card
  component and its endpoints verbatim (star / mark-read / reader-mode icons per
  article — same as the articles page, so individual articles stay fully
  actionable within the digest).
- If the feed has more than 20 unread articles, a "View all N unread in Articles"
  link to `/articles?feed_ids={id}&is_read=false` below the capped list.
- "Next Feed" link and "Mark Read & Next" button (with `hx-confirm`, same confirm-
  dialog pattern already used by the existing mark-all-read button elsewhere in the
  app).

## Backend changes

### `src/domain/article_service.rs`

Split the existing `rank_feeds_by_rarity(stats, limit)` into:

- `sort_feeds_by_rarity(stats: Vec<FeedPublishStats>) -> Vec<FeedPublishStats>` — the
  actual sort, no truncation.
- `rank_feeds_by_rarity(stats, limit)` — thin wrapper: sort, then `.truncate(limit)`.
  Existing callers (smart pills) are unaffected.

Add:

```rust
pub async fn get_digest_queue(
    pool: &SqlitePool,
) -> Result<Vec<FeedPublishStats>, ArticleServiceError> {
    let stats = repository::get_feed_publish_stats_for_unread(pool).await?;
    Ok(sort_feeds_by_rarity(stats))
}
```

Reuses the existing `repository::get_feed_publish_stats_for_unread` query — no new
SQL needed.

### `src/api/digest.rs` (new)

One handler, `show_digest`:

1. Parse `skip: Option<usize>` from query (default `0`).
2. `queue = article_service::get_digest_queue(pool)`.
3. If `skip >= queue.len()` (this covers both "queue is empty" and "ran past the
   end of a non-empty queue" — both render the same end screen): render
   `DigestTemplate` in "caught up" mode.
4. Otherwise, take `queue[skip]`, fetch its unread articles via the existing
   `repository::list_articles_with_feeds(pool, Some(vec![feed_id]), Some(false), None, None, None, None, 21, 0)`
   (fetch 21 to detect "more than 20" — same `has_more` pattern already used on the
   articles page), attach tags via `attach_tags_to_articles`, and render
   `DigestTemplate` with the feed in view.

Note: `attach_tags_to_articles` is currently private to `src/api/articles.rs`.
Change its visibility to `pub(crate)` so `api::digest` can call it directly — don't
duplicate the batch-fetch-and-attach logic.

No new mutation routes. Register `GET /digest` in `main.rs`; add `pub mod digest;`
to `src/api/mod.rs`.

### Template struct (`src/web/templates.rs`)

```rust
#[derive(Template)]
#[template(path = "digest/show.html")]
pub struct DigestTemplate {
    pub caught_up: bool,
    pub queue_position: usize,   // 1-based; meaningless when caught_up
    pub queue_total: usize,
    pub feed: Option<FeedPublishStats>, // already carries title/color/feed_id/unread_count;
                                         // reusing it avoids a redundant Feed fetch
    pub articles: Vec<ArticleWithFeed>,
    pub has_more_articles: bool,
    pub next_skip: usize,
}
```

## Frontend changes

- `src/web/templates/digest/show.html` — extends `base.html`. Branches on
  `caught_up` for the end screen (reusing the existing `.card.empty-state` /
  `icons/newspaper.html` pattern already used elsewhere); otherwise renders the feed
  header, includes the existing `articles/_article_rows.html` partial for the
  capped article list, and the Next Feed / Mark Read & Next controls.
- `static/ts/digest.ts` (new) — touch handlers scoped to the digest card only:
  swipe left triggers the same navigation as the "Next Feed" link
  (`window.location.href`). Modeled directly on the existing swipe-gesture code in
  `base.html` (touchstart/touchend, horizontal-dominant swipe, distance threshold),
  but scoped to this page instead of global nav.
- `src/web/templates/components/header.html` — add "Digest" as the first nav item,
  both the desktop nav and the mobile slide-in panel, linking to `/digest`.

## Error handling

Reuses the existing `AppError` / `ErrorTemplate` pattern already in
`src/api/articles.rs` (template errors, database errors) — no new error cases;
`show_digest` only reads data and never mutates.

## Testing

- Unit tests for `sort_feeds_by_rarity` (rename of existing `rank_feeds_by_rarity`
  tests, now calling the split function) and a new thin test confirming
  `rank_feeds_by_rarity` still truncates correctly after the split.
- Unit test (or integration test, depending on how `list_articles_with_feeds` is
  already tested) confirming a feed with 0 unread articles doesn't appear in
  `get_digest_queue`'s output.
- Manual verification via Playwright (as done for prior UI changes in this project):
  queue ordering, `skip` pagination, caught-up screen, Mark Read & Next advancing
  correctly, swipe-left on a narrow viewport.
