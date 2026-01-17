# Background Feed Import

## Problem

When importing many feeds at once via the import form, the UI was stuck
for a long time because feeds were processed synchronously. Each feed
creation involved:

1. URL validation
2. SSRF check
3. Database insert
4. **Immediate network fetch** to populate metadata (slow!)

The network fetch for each feed blocked the UI.

## Solution

Implemented background job processing for bulk feed imports:

1. **Deferred feed creation** (`create_feed_deferred`): Creates feeds in
   the database without immediately fetching them. The scheduler will
   fetch them automatically later.

2. **In-memory job store**: Added `ImportJobStore` to `AppState` using
   `Arc<RwLock<HashMap<String, ImportJob>>>` to track import jobs.

3. **Background processing**: The `import_feeds` endpoint now:
   - Parses all feed URLs
   - Creates a job with unique ID
   - Returns immediately with a progress UI
   - Spawns a background task to process feeds

4. **Polling UI**: New `import_progress.html` template uses HTMX to poll
   `/feeds/import/{job_id}` every 500ms, showing a progress bar and
   processed/total count.

5. **Job status endpoint**: New `get_import_job_status` endpoint returns
   either progress UI (if processing) or final results (if complete).

## Files Changed

- `src/api/feeds.rs`: Added job types, modified `import_feeds`, added
  `get_import_job_status`
- `src/domain/feed_service.rs`: Added `create_feed_deferred` function
- `src/web/templates.rs`: Added `FeedImportProgressTemplate`
- `src/web/templates/feeds/import_progress.html`: New progress UI template
- `src/main.rs`: Updated `AppState` init, added new route
- `Cargo.toml`: Added `uuid` dependency
- `tests/api_integration_tests.rs`: Updated for new `AppState` field

## User Experience

- Instant feedback: Progress UI appears immediately after clicking Import
- Live progress: Shows "Importing feeds... (X/Y)" with animated progress bar
- Informative: Message explains feeds will be fetched automatically
- Results: Final success/error status for each feed when complete
