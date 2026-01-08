-- Add color and style columns to tags for visual customization
ALTER TABLE tags ADD COLUMN color TEXT NOT NULL DEFAULT '#3B82F6';
ALTER TABLE tags ADD COLUMN style TEXT NOT NULL DEFAULT 'solid';
