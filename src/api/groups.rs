use crate::api::articles::AppError;
use crate::api::feeds::AppState;
use crate::domain::group_service;
use crate::infrastructure::repository;
use crate::web::templates::{
    AssignFeedTemplate, FeedFilterModalTemplate, GroupFormTemplate, GroupListContentTemplate,
    GroupsListTemplate,
};
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    Form,
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
    // Get all groups, feeds, and unread counts
    let groups = repository::list_groups(&state.db_pool).await?;
    let feeds = repository::list_feeds(&state.db_pool).await?;
    let unread_counts = repository::get_feed_unread_counts(&state.db_pool).await?;

    // Build group tree with unread counts
    let (group_tree, ungrouped_feeds) = group_service::build_group_tree(groups, feeds);
    let group_tree = group_service::add_unread_counts_to_tree(group_tree, &unread_counts);
    let ungrouped_feeds =
        group_service::add_unread_counts_to_feeds(ungrouped_feeds, &unread_counts);

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

// ============ Group CRUD Handlers ============

/// List all groups (GET /groups)
pub async fn list_groups(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let groups = repository::list_groups(&state.db_pool).await?;
    let feeds = repository::list_feeds(&state.db_pool).await?;

    let (group_tree, ungrouped_feeds) = group_service::build_group_tree(groups, feeds);
    let tree_items = group_service::flatten_group_tree(&group_tree);

    let template = GroupsListTemplate {
        tree_items,
        ungrouped_feeds,
    };

    Ok(Html(template.render()?))
}

/// Returns just the group list content (for HTMX partial updates)
async fn render_group_list_content(state: &AppState) -> Result<Html<String>, AppError> {
    let groups = repository::list_groups(&state.db_pool).await?;
    let feeds = repository::list_feeds(&state.db_pool).await?;

    let (group_tree, ungrouped_feeds) = group_service::build_group_tree(groups, feeds);
    let tree_items = group_service::flatten_group_tree(&group_tree);

    let template = GroupListContentTemplate {
        tree_items,
        ungrouped_feeds,
    };

    Ok(Html(template.render()?))
}

/// Show new group form (GET /groups/new)
pub async fn show_new_group_form(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let available_groups = repository::list_groups(&state.db_pool).await?;

    let template = GroupFormTemplate {
        group: None,
        available_groups,
    };

    Ok(Html(template.render()?))
}

#[derive(Deserialize)]
pub struct CreateGroupForm {
    pub name: String,
    pub parent_id: Option<String>,
}

/// Create a new group (POST /groups)
pub async fn create_group(
    State(state): State<AppState>,
    Form(form): Form<CreateGroupForm>,
) -> Result<Html<String>, AppError> {
    let parent_id =
        form.parent_id
            .as_ref()
            .and_then(|s| if s.is_empty() { None } else { s.parse().ok() });

    repository::create_group(&state.db_pool, &form.name, parent_id).await?;

    // Return the updated group list content (partial for HTMX)
    render_group_list_content(&state).await
}

/// Show edit group form (GET /groups/:id/edit)
pub async fn show_edit_group_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let group = repository::get_group(&state.db_pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Group not found".to_string()))?;

    let available_groups = repository::list_groups(&state.db_pool).await?;

    let template = GroupFormTemplate {
        group: Some(group),
        available_groups,
    };

    Ok(Html(template.render()?))
}

#[derive(Deserialize)]
pub struct UpdateGroupForm {
    pub name: String,
    pub parent_id: Option<String>,
}

/// Update a group (PUT /groups/:id)
pub async fn update_group(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<UpdateGroupForm>,
) -> Result<Html<String>, AppError> {
    let parent_id =
        form.parent_id
            .as_ref()
            .and_then(|s| if s.is_empty() { None } else { s.parse().ok() });

    repository::update_group(&state.db_pool, id, &form.name, parent_id).await?;

    // Return the updated group list content (partial for HTMX)
    render_group_list_content(&state).await
}

/// Delete a group (DELETE /groups/:id)
pub async fn delete_group(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    repository::delete_group(&state.db_pool, id).await?;

    // Return the updated group list content (partial for HTMX)
    render_group_list_content(&state).await
}

/// Show assign feed to group form (GET /groups/assign-feed/:feed_id)
pub async fn show_assign_feed_form(
    State(state): State<AppState>,
    Path(feed_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let feed = repository::get_feed_by_id(&state.db_pool, feed_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Feed not found".to_string()))?;

    let groups = repository::list_groups(&state.db_pool).await?;

    let template = AssignFeedTemplate { feed, groups };

    Ok(Html(template.render()?))
}

#[derive(Deserialize)]
pub struct AssignFeedForm {
    pub group_id: Option<String>,
}

/// Assign feed to group (PUT /feeds/:id/group)
pub async fn assign_feed_to_group(
    State(state): State<AppState>,
    Path(feed_id): Path<i64>,
    Form(form): Form<AssignFeedForm>,
) -> Result<Html<String>, AppError> {
    let group_id =
        form.group_id
            .as_ref()
            .and_then(|s| if s.is_empty() { None } else { s.parse().ok() });

    repository::update_feed_group(&state.db_pool, feed_id, group_id).await?;

    // Return the updated group list content (partial for HTMX)
    render_group_list_content(&state).await
}

#[derive(Deserialize)]
pub struct MoveGroupForm {
    pub parent_id: Option<String>,
}

/// Move a group to a new parent (PUT /groups/:id/parent)
pub async fn move_group(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<MoveGroupForm>,
) -> Result<Html<String>, AppError> {
    let parent_id =
        form.parent_id
            .as_ref()
            .and_then(|s| if s.is_empty() { None } else { s.parse().ok() });

    // Prevent moving a group into itself or its descendants
    if let Some(new_parent_id) = parent_id {
        if new_parent_id == id {
            return Err(AppError::NotFound(
                "Cannot move a group into itself".to_string(),
            ));
        }
        // Check if new_parent_id is a descendant of id
        let descendants = repository::get_descendant_group_ids(&state.db_pool, id).await?;
        if descendants.contains(&new_parent_id) {
            return Err(AppError::NotFound(
                "Cannot move a group into its own descendant".to_string(),
            ));
        }
    }

    repository::update_group_parent(&state.db_pool, id, parent_id).await?;

    // Return the updated group list content (partial for HTMX)
    render_group_list_content(&state).await
}
