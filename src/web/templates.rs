use askama::Template;
use crate::domain::models::Feed;

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
