-- Create logs table for tracking feed fetch events
CREATE TABLE IF NOT EXISTS logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    feed_id INTEGER NOT NULL,
    log_type TEXT NOT NULL,
    status_code INTEGER,
    error_message TEXT,
    retry_after TEXT,
    fetched_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE
);

-- Index for filtering by feed
CREATE INDEX IF NOT EXISTS idx_logs_feed_id ON logs(feed_id);

-- Index for chronological sorting (most recent first)
CREATE INDEX IF NOT EXISTS idx_logs_fetched_at ON logs(fetched_at DESC);

-- Index for filtering by log type
CREATE INDEX IF NOT EXISTS idx_logs_log_type ON logs(log_type);

-- Composite index for efficient feed-specific timeline queries
CREATE INDEX IF NOT EXISTS idx_logs_feed_fetched ON logs(feed_id, fetched_at DESC);
