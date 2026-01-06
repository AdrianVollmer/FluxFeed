-- Add groups table for organizing feeds hierarchically
CREATE TABLE IF NOT EXISTS groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    parent_id INTEGER REFERENCES groups(id) ON DELETE CASCADE,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Add group_id to feeds table (nullable for ungrouped feeds)
-- ON DELETE SET NULL: deleting a group ungroups the feeds rather than deleting them
ALTER TABLE feeds ADD COLUMN group_id INTEGER REFERENCES groups(id) ON DELETE SET NULL;

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_groups_parent_id ON groups(parent_id);
CREATE INDEX IF NOT EXISTS idx_groups_position ON groups(parent_id, position);
CREATE INDEX IF NOT EXISTS idx_feeds_group_id ON feeds(group_id);
