pub fn app_version(_: &str) -> askama::Result<String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

/// Check if an i64 value is in a slice
pub fn in_list(value: &i64, list: &[i64]) -> askama::Result<bool> {
    Ok(list.contains(value))
}
