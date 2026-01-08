use crate::api::articles::AppError;
use crate::api::feeds::AppState;
use crate::infrastructure::repository;
use crate::web::templates::{
    TagFilterModalTemplate, TagFormTemplate, TagListContentTemplate, TagsListTemplate,
};
use askama::Template;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    Form,
};
use serde::Deserialize;

/// List all tags (GET /tags)
pub async fn list_tags(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let tags = repository::list_tags(&state.db_pool).await?;

    let template = TagsListTemplate { tags };

    Ok(Html(template.render()?))
}

/// Returns just the tag list content (for HTMX partial updates)
async fn render_tag_list_content(state: &AppState) -> Result<Html<String>, AppError> {
    let tags = repository::list_tags(&state.db_pool).await?;

    let template = TagListContentTemplate { tags };

    Ok(Html(template.render()?))
}

/// Show new tag form (GET /tags/new)
pub async fn show_new_tag_form() -> Result<Html<String>, AppError> {
    let template = TagFormTemplate { tag: None };

    Ok(Html(template.render()?))
}

#[derive(Deserialize)]
pub struct CreateTagForm {
    pub name: String,
    pub color: String,
    pub style: String,
}

/// Create a new tag (POST /tags)
pub async fn create_tag(
    State(state): State<AppState>,
    Form(form): Form<CreateTagForm>,
) -> Result<Html<String>, AppError> {
    // Validate color format
    if !form.color.starts_with('#') || form.color.len() != 7 {
        return Err(AppError::NotFound(
            "Invalid color format. Use #RRGGBB".to_string(),
        ));
    }

    // Validate style
    if !["solid", "outline", "striped"].contains(&form.style.as_str()) {
        return Err(AppError::NotFound(
            "Invalid style. Use solid, outline, or striped".to_string(),
        ));
    }

    repository::create_tag(&state.db_pool, &form.name, &form.color, &form.style).await?;

    // Return the updated tag list content (partial for HTMX)
    render_tag_list_content(&state).await
}

/// Show edit tag form (GET /tags/:id/edit)
pub async fn show_edit_tag_form(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    let tag = repository::get_tag(&state.db_pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Tag not found".to_string()))?;

    let template = TagFormTemplate { tag: Some(tag) };

    Ok(Html(template.render()?))
}

#[derive(Deserialize)]
pub struct UpdateTagForm {
    pub name: String,
    pub color: String,
    pub style: String,
}

/// Update a tag (PUT /tags/:id)
pub async fn update_tag(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(form): Form<UpdateTagForm>,
) -> Result<Html<String>, AppError> {
    // Validate color format
    if !form.color.starts_with('#') || form.color.len() != 7 {
        return Err(AppError::NotFound(
            "Invalid color format. Use #RRGGBB".to_string(),
        ));
    }

    // Validate style
    if !["solid", "outline", "striped"].contains(&form.style.as_str()) {
        return Err(AppError::NotFound(
            "Invalid style. Use solid, outline, or striped".to_string(),
        ));
    }

    repository::update_tag(&state.db_pool, id, &form.name, &form.color, &form.style).await?;

    // Return the updated tag list content (partial for HTMX)
    render_tag_list_content(&state).await
}

/// Delete a tag (DELETE /tags/:id)
pub async fn delete_tag(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    repository::delete_tag(&state.db_pool, id).await?;

    // Return the updated tag list content (partial for HTMX)
    render_tag_list_content(&state).await
}

#[derive(Deserialize)]
pub struct TagFilterModalParams {
    pub tag_ids: Option<String>,
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

/// Show tag filter modal (GET /articles/tag-filter-modal)
pub async fn show_tag_filter_modal(
    State(state): State<AppState>,
    Query(params): Query<TagFilterModalParams>,
) -> Result<Html<String>, AppError> {
    let tags = repository::list_tags(&state.db_pool).await?;

    let selected_tag_ids = parse_ids(params.tag_ids.as_deref());
    let filter_feed_ids = parse_ids(params.feed_ids.as_deref());
    let filter_group_ids = parse_ids(params.group_ids.as_deref());

    let template = TagFilterModalTemplate {
        tags,
        selected_tag_ids,
        filter_read: params.is_read,
        filter_starred: params.is_starred,
        filter_feed_ids,
        filter_group_ids,
    };

    Ok(Html(template.render()?))
}
