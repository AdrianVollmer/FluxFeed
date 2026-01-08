use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::LazyLock;

pub fn app_version(_: &str) -> askama::Result<String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

/// Format a DateTime as a friendly relative time string (e.g., "1m ago", "3h ago", "5d ago")
pub fn friendly_date(dt: &DateTime<Utc>) -> askama::Result<String> {
    let now = Utc::now();
    let duration = now.signed_duration_since(*dt);

    let result = if duration.num_seconds() < 60 {
        "just now".to_string()
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h ago", duration.num_hours())
    } else if duration.num_days() < 30 {
        format!("{}d ago", duration.num_days())
    } else if duration.num_days() < 365 {
        let months = duration.num_days() / 30;
        format!("{}mo ago", months)
    } else {
        let years = duration.num_days() / 365;
        format!("{}y ago", years)
    };

    Ok(result)
}

/// Check if an i64 value is in a slice
pub fn in_list(value: &i64, list: &[i64]) -> askama::Result<bool> {
    Ok(list.contains(value))
}

/// JS manifest mapping base names to hashed filenames
static JS_MANIFEST: LazyLock<HashMap<String, String>> = LazyLock::new(|| {
    let manifest_str = include_str!("../../static/js/dist/manifest.json");
    serde_json::from_str(manifest_str).unwrap_or_default()
});

/// Get the hashed JS filename for a given base name
/// Usage: {{ "articles.js"|js_path }}
pub fn js_path(name: &str) -> askama::Result<String> {
    let hashed = JS_MANIFEST
        .get(name)
        .cloned()
        .unwrap_or_else(|| name.to_string());
    Ok(format!("/static/js/dist/{}", hashed))
}
