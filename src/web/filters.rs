pub fn app_version(_: &str) -> askama::Result<String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}
