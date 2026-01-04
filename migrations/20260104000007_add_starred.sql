-- Add starred column to articles table
ALTER TABLE articles ADD COLUMN is_starred BOOLEAN NOT NULL DEFAULT 0;

-- Create index for filtering starred articles
CREATE INDEX idx_articles_starred ON articles(is_starred);
