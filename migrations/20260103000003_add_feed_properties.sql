-- Add new properties to feeds table

ALTER TABLE feeds ADD COLUMN color TEXT DEFAULT '#3B82F6';
ALTER TABLE feeds ADD COLUMN fetch_frequency TEXT DEFAULT 'smart';

-- Create tags table
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create feed_tags junction table for many-to-many relationship
CREATE TABLE IF NOT EXISTS feed_tags (
    feed_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (feed_id, tag_id),
    FOREIGN KEY (feed_id) REFERENCES feeds(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_feed_tags_feed_id ON feed_tags(feed_id);
CREATE INDEX IF NOT EXISTS idx_feed_tags_tag_id ON feed_tags(tag_id);
