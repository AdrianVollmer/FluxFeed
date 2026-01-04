use crate::domain::models::{Article, Feed, LogWithFeed, Tag};
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate;

#[derive(Template)]
#[template(path = "feeds/list.html")]
pub struct FeedsListTemplate {
    pub feeds: Vec<Feed>,
}

#[derive(Template)]
#[template(path = "feeds/feed_row.html")]
pub struct FeedRowTemplate {
    pub feed: Feed,
}

#[derive(Template)]
#[template(path = "feeds/form.html")]
pub struct FeedFormTemplate;

#[derive(Template)]
#[template(path = "feeds/detail.html")]
pub struct FeedDetailTemplate {
    pub feed: Feed,
    pub tags: Vec<Tag>,
}

#[derive(Template)]
#[template(path = "feeds/edit_form.html")]
pub struct FeedEditFormTemplate {
    pub feed: Feed,
}

#[derive(Template)]
#[template(path = "articles/list.html")]
pub struct ArticlesListTemplate {
    pub articles: Vec<ArticleWithFeed>,
    pub feeds: Vec<Feed>,
    pub offset: i64,
    pub limit: i64,
    pub has_more: bool,
    pub filter_feed: Option<i64>,
    pub filter_read: Option<bool>,
    pub unread_count: i64,
    pub search_query: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Template)]
#[template(path = "articles/article_row.html")]
pub struct ArticleRowTemplate {
    pub article: Article,
    pub feed_title: String,
}

#[derive(Template)]
#[template(path = "articles/article_compact_row.html")]
pub struct ArticleCompactRowTemplate {
    pub article: Article,
    pub feed_title: String,
}

// Combined article + feed data for display
pub struct ArticleWithFeed {
    pub article: Article,
    pub feed_title: String,
    pub feed_color: String,
}

#[derive(Template)]
#[template(path = "articles/_article_rows.html")]
pub struct ArticleRowsTemplate {
    pub articles: Vec<ArticleWithFeed>,
}

#[derive(Template)]
#[template(path = "articles/_article_compact_rows.html")]
pub struct ArticleCompactRowsTemplate {
    pub articles: Vec<ArticleWithFeed>,
}

#[derive(Template)]
#[template(path = "articles/_load_more_button.html")]
pub struct LoadMoreButtonTemplate {
    pub next_offset: i64,
    pub filter_feed: Option<i64>,
    pub filter_read: Option<bool>,
    pub search_query: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Template)]
#[template(path = "articles/search.html")]
pub struct ArticleSearchTemplate {
    pub articles: Vec<ArticleWithFeed>,
    pub offset: i64,
    pub limit: i64,
    pub has_more: bool,
    pub search_query: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Template)]
#[template(path = "logs/list.html")]
pub struct LogsListTemplate {
    pub logs: Vec<LogWithFeed>,
    pub feeds: Vec<Feed>,
    pub offset: i64,
    pub limit: i64,
    pub has_more: bool,
    pub filter_feed: Option<i64>,
    pub filter_log_type: Option<String>,
}

#[derive(Template)]
#[template(path = "logs/_log_rows.html")]
pub struct LogRowsTemplate {
    pub logs: Vec<LogWithFeed>,
}

#[derive(Template)]
#[template(path = "logs/_load_more_button.html")]
pub struct LoadMoreButtonLogsTemplate {
    pub next_offset: i64,
    pub filter_feed: Option<i64>,
    pub filter_log_type: Option<String>,
}

#[derive(Template)]
#[template(path = "reader/reader_mode.html")]
pub struct ReaderModeTemplate {
    pub article_url: String,
    pub title: String,
    pub content: String,
    pub byline: Option<String>,
    pub excerpt: Option<String>,
}
