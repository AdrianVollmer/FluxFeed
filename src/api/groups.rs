use crate::api::articles::AppError;
use crate::api::feeds::AppState;
use crate::domain::group_service;
use crate::infrastructure::repository;
use crate::web::templates::FeedFilterModalTemplate;
use askama::Template;
use axum::{
    extract::{Query, State},
    response::Html,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct FilterModalParams {
    pub feed_ids: Option<String>,
    pub group_ids: Option<String>,
    pub is_read: Option<bool>,
    pub is_starred: Option<bool>,
}

/// Parse comma-separated IDs from query parameter
fn parse_ids(ids_str: Option<&str>) -> Vec<i64> {
    ids_str
        .map(|s| {
            s.split(',')
                .filter_map(|id| id.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Render the feed filter modal with hierarchical tree
pub async fn show_feed_filter_modal(
    State(state): State<AppState>,
    Query(params): Query<FilterModalParams>,
) -> Result<Html<String>, AppError> {
    // Get all groups and feeds
    let groups = repository::list_groups(&state.db_pool).await?;
    let feeds = repository::list_feeds(&state.db_pool).await?;

    // Build group tree
    let (group_tree, ungrouped_feeds) = group_service::build_group_tree(groups, feeds);

    // Parse currently selected IDs from query params
    let selected_feed_ids = parse_ids(params.feed_ids.as_deref());
    let selected_group_ids = parse_ids(params.group_ids.as_deref());

    let template = FeedFilterModalTemplate {
        group_tree,
        ungrouped_feeds,
        selected_feed_ids,
        selected_group_ids,
        filter_read: params.is_read,
        filter_starred: params.is_starred,
    };

    Ok(Html(template.render()?))
}
