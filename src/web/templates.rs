use crate::domain::models::{Article, Feed, GroupNode, LogWithFeed, Tag};
use crate::infrastructure::repository::ArticleCounts;
use crate::web::filters;
use askama::Template;

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub status_code: u16,
    pub status_text: String,
    pub message: String,
}

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
    pub all_tags: Vec<Tag>,
    pub feed_tag_ids: Vec<i64>,
}

#[derive(Template)]
#[template(path = "feeds/import_form.html")]
pub struct FeedImportFormTemplate;

pub struct ImportResult {
    pub url: String,
    pub title: Option<String>,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "feeds/import_results.html")]
pub struct FeedImportResultsTemplate {
    pub results: Vec<ImportResult>,
    pub success_count: usize,
}

#[derive(Template)]
#[template(path = "feeds/import_progress.html")]
pub struct FeedImportProgressTemplate {
    pub job_id: String,
    pub total: usize,
    pub processed: usize,
}

#[derive(Template)]
#[template(path = "articles/list.html")]
#[allow(dead_code)]
pub struct ArticlesListTemplate {
    pub articles: Vec<ArticleWithFeed>,
    pub feeds: Vec<Feed>,
    pub group_tree: Vec<GroupNode>,
    pub ungrouped_feeds: Vec<Feed>,
    pub offset: i64,
    pub limit: i64,
    pub has_more: bool,
    pub filter_feed_ids: Vec<i64>,
    pub filter_group_ids: Vec<i64>,
    pub filter_tag_ids: Vec<i64>,
    pub filter_read: Option<bool>,
    pub filter_starred: Option<bool>,
    pub article_counts: ArticleCounts,
    pub active_filter: String,
    pub search_query: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub all_tags: Vec<Tag>,
    pub view_mode: String,
}

#[derive(Template)]
#[template(path = "articles/article_row.html")]
pub struct ArticleRowTemplate {
    pub item: ArticleWithFeed,
}

#[derive(Template)]
#[template(path = "articles/article_compact_row.html")]
pub struct ArticleCompactRowTemplate {
    pub item: ArticleWithFeed,
}

// Combined article + feed data for display
#[derive(Clone)]
pub struct ArticleWithFeed {
    pub article: Article,
    pub feed_title: String,
    pub feed_color: String,
    pub tags: Vec<Tag>,
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
#[template(path = "articles/article_fullscreen_row.html")]
pub struct ArticleFullscreenRowTemplate {
    pub item: ArticleWithFeed,
}

#[derive(Template)]
#[template(path = "articles/_article_fullscreen_rows.html")]
#[allow(dead_code)]
pub struct ArticleFullscreenRowsTemplate {
    pub articles: Vec<ArticleWithFeed>,
}

#[derive(Template)]
#[template(path = "articles/_list_footer.html")]
pub struct ArticleListFooterTemplate {
    pub has_more: bool,
    pub show_mark_all_read: bool,
    pub next_offset: i64,
    pub filter_feed_ids: Option<String>,
    pub filter_group_ids: Option<String>,
    pub filter_tag_ids: Option<String>,
    pub filter_read: Option<bool>,
    pub filter_starred: Option<bool>,
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
    pub filter_feed_name: Option<String>,
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
    pub filter_feed_name: Option<String>,
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

#[derive(Template)]
#[template(path = "reader/reader_content.html")]
pub struct ReaderContentTemplate {
    pub article_url: String,
    pub title: String,
    pub content: String,
    pub byline: Option<String>,
    pub excerpt: Option<String>,
}

#[derive(Template)]
#[template(path = "articles/feed_filter_modal.html")]
#[allow(dead_code)]
pub struct FeedFilterModalTemplate {
    pub group_tree: Vec<GroupNode>,
    pub ungrouped_feeds: Vec<Feed>,
    pub selected_feed_ids: Vec<i64>,
    pub selected_group_ids: Vec<i64>,
    pub filter_read: Option<bool>,
    pub filter_starred: Option<bool>,
}

#[derive(Template)]
#[template(path = "articles/tag_filter_modal.html")]
#[allow(dead_code)]
pub struct TagFilterModalTemplate {
    pub tags: Vec<Tag>,
    pub selected_tag_ids: Vec<i64>,
    pub filter_read: Option<bool>,
    pub filter_starred: Option<bool>,
    pub filter_feed_ids: Vec<i64>,
    pub filter_group_ids: Vec<i64>,
}

// Group templates
use crate::domain::models::{FlatTreeItem, Group};

#[derive(Template)]
#[template(path = "groups/list.html")]
pub struct GroupsListTemplate {
    pub tree_items: Vec<FlatTreeItem>,
    pub ungrouped_feeds: Vec<Feed>,
}

#[derive(Template)]
#[template(path = "groups/_group_list_content.html")]
pub struct GroupListContentTemplate {
    pub tree_items: Vec<FlatTreeItem>,
    pub ungrouped_feeds: Vec<Feed>,
}

#[derive(Template)]
#[template(path = "groups/form.html")]
pub struct GroupFormTemplate {
    pub group: Option<Group>,
    pub available_groups: Vec<Group>,
}

#[derive(Template)]
#[template(path = "groups/assign_feed.html")]
pub struct AssignFeedTemplate {
    pub feed: Feed,
    pub groups: Vec<Group>,
}

// Tag templates

#[derive(Template)]
#[template(path = "tags/list.html")]
pub struct TagsListTemplate {
    pub tags: Vec<Tag>,
}

#[derive(Template)]
#[template(path = "tags/_list_content.html")]
pub struct TagListContentTemplate {
    pub tags: Vec<Tag>,
}

#[derive(Template)]
#[template(path = "tags/form.html")]
pub struct TagFormTemplate {
    pub tag: Option<Tag>,
}
