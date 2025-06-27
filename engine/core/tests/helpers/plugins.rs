// engine/core/tests/helpers/plugins.rs
use std::env;
use std::path::{Path, PathBuf};

pub fn plugin_bin_path() -> PathBuf {
    // CARGO_MANIFEST_DIR is .../engine/core
    // workspace root is two levels up
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let bin = workspace_root.join("plugins/rust_test_plugin/rust_test_plugin");
    assert!(
        bin.exists(),
        "Plugin binary missing at {bin:?}. Build it with: cargo run -p xtask -- build-plugins"
    );
    bin
}

pub fn test_socket_path(suffix: &str) -> String {
    format!("/tmp/rust_test_plugin_{suffix}.sock")
}
