//! Central resolver for file-backed asset lookup paths.
//!
//! The schema/registry loaders used to hard-code their own relative candidate
//! lists, which made them untestable without the exact workspace layout on
//! disk. They now build their candidate lists through [`resolve_asset_paths`],
//! which prepends the `MGE_SCHEMA_DIR`-style override directory (when set)
//! in front of each loader's compiled-in defaults. Loaders keep first-hit
//! iteration, so production behavior is unchanged.

use std::path::PathBuf;

/// Resolve candidate paths for an asset file such as `tech_tree.json`.
///
/// The override directory (typically [`schema_dir_override`]) wins when set;
/// the compiled-in `defaults` follow as fallback so callers keep working when
/// the override is absent or does not contain the file. Pure over its inputs
/// to stay deterministic under parallel tests — callers read the environment
/// at the boundary.
pub fn resolve_asset_paths(
    file_name: &str,
    env_override: Option<PathBuf>,
    defaults: &[&str],
) -> Vec<PathBuf> {
    let mut resolved = Vec::with_capacity(defaults.len() + 1);
    if let Some(dir) = env_override {
        resolved.push(dir.join(file_name));
    }
    resolved.extend(defaults.iter().map(PathBuf::from));
    resolved
}

/// Read the `MGE_SCHEMA_DIR` schema-directory override, if set.
pub fn schema_dir_override() -> Option<PathBuf> {
    std::env::var("MGE_SCHEMA_DIR").ok().map(PathBuf::from)
}
