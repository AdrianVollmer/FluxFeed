use std::collections::HashMap;
use std::sync::LazyLock;

pub fn app_version(_: &str) -> askama::Result<String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
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
