-- Add composite index for article listing performance
-- The main articles query orders by (published_at DESC, created_at DESC)
-- This index allows SQLite to use index scan instead of sorting 100k+ rows

CREATE INDEX IF NOT EXISTS idx_articles_listing
ON articles(published_at DESC, created_at DESC);

-- Add composite index for common filter: unread articles sorted by date
CREATE INDEX IF NOT EXISTS idx_articles_unread_listing
ON articles(is_read, published_at DESC, created_at DESC)
WHERE is_read = 0;

-- Add composite index for starred articles listing
CREATE INDEX IF NOT EXISTS idx_articles_starred_listing
ON articles(is_starred, published_at DESC, created_at DESC)
WHERE is_starred = 1;

-- Covering index for article counts query (total, unread, read, starred)
-- This allows the COUNT queries to be answered entirely from the index
CREATE INDEX IF NOT EXISTS idx_articles_counts
ON articles(is_read, is_starred);
