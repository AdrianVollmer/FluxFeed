use askama::Template;
use crate::domain::models::{Article, Feed};

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
}

#[derive(Template)]
#[template(path = "articles/article_row.html")]
pub struct ArticleRowTemplate {
    pub article: Article,
    pub feed_title: String,
}

// Combined article + feed data for display
pub struct ArticleWithFeed {
    pub article: Article,
    pub feed_title: String,
}
