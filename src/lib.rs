pub mod api;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod web;

/// Returns the FluxFeed user agent string with the current version
///
/// Format: "FluxFeed/X.Y.Z"
///
/// The version is read from Cargo.toml at compile time, ensuring it's
/// always in sync with the package version.
pub fn user_agent() -> String {
    format!("FluxFeed/{}", env!("CARGO_PKG_VERSION"))
}
