use engine_core::plugins::manager::PluginManager;
use engine_core::plugins::subprocess::{PluginRequest, PluginResponse};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

/// Returns a unique socket path for each test (using the test name)
fn test_socket_path(suffix: &str) -> String {
    format!("/tmp/rust_test_plugin_{}.sock", suffix)
}

/// Returns the absolute path to the plugin binary, robust to test CWD
fn plugin_bin_path() -> PathBuf {
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
        "Plugin binary missing at {:?}. Build it with: cargo run -p xtask -- build-plugins",
        bin
    );
    bin
}

const TEST_PLUGIN_NAME: &str = "rust_test_plugin";

#[test]
fn show_cwd() {
    println!("CWD: {:?}", std::env::current_dir().unwrap());
    println!("Plugin bin path: {:?}", plugin_bin_path());
}

#[test]
fn test_subprocess_plugin_lifecycle() {
    let bin = plugin_bin_path();
    let socket_path = test_socket_path("lifecycle");
    let _ = fs::remove_file(&socket_path);
    let mut manager = PluginManager::new();

    manager
        .launch_plugin(TEST_PLUGIN_NAME.to_string(), &bin, &socket_path)
        .expect("Failed to launch plugin");

    thread::sleep(Duration::from_millis(100));

    let resp = manager
        .send(TEST_PLUGIN_NAME, &PluginRequest::Initialize)
        .expect("Failed to send Initialize");
    assert!(matches!(resp, PluginResponse::Initialized));

    let resp = manager
        .send(
            TEST_PLUGIN_NAME,
            &PluginRequest::RunCommand {
                command: "echo".to_string(),
                data: serde_json::json!({"foo": 42}),
            },
        )
        .expect("Failed to send RunCommand");
    match resp {
        PluginResponse::CommandResult { result } => {
            assert_eq!(result["echo"]["foo"], 42);
        }
        _ => panic!("Unexpected response: {:?}", resp),
    }

    let resp = manager
        .send(TEST_PLUGIN_NAME, &PluginRequest::Reload)
        .expect("Failed to send Reload");
    assert!(matches!(resp, PluginResponse::Reloaded));

    manager
        .reload_plugin(TEST_PLUGIN_NAME, &bin, &socket_path)
        .expect("Failed to hot reload plugin");
    thread::sleep(Duration::from_millis(100));
    let resp = manager
        .send(TEST_PLUGIN_NAME, &PluginRequest::Initialize)
        .expect("Failed to send Initialize after reload");
    assert!(matches!(resp, PluginResponse::Initialized));

    manager
        .shutdown_plugin(TEST_PLUGIN_NAME)
        .expect("Failed to shutdown plugin");
    let _ = fs::remove_file(&socket_path);
}

#[test]
fn test_plugin_shutdown_all() {
    let bin = plugin_bin_path();
    let socket_path = test_socket_path("shutdown_all");
    let _ = fs::remove_file(&socket_path);
    let mut manager = PluginManager::new();

    manager
        .launch_plugin(TEST_PLUGIN_NAME.to_string(), &bin, &socket_path)
        .expect("Failed to launch plugin");
    thread::sleep(Duration::from_millis(100));

    let resp = manager
        .send(TEST_PLUGIN_NAME, &PluginRequest::Initialize)
        .expect("Failed to send Initialize");
    assert!(matches!(resp, PluginResponse::Initialized));

    manager.shutdown_all();
    let _ = fs::remove_file(&socket_path);
}

#[test]
fn test_plugin_error_on_duplicate_launch() {
    let bin = plugin_bin_path();
    let socket_path = test_socket_path("duplicate");
    let _ = fs::remove_file(&socket_path);
    let mut manager = PluginManager::new();

    manager
        .launch_plugin(TEST_PLUGIN_NAME.to_string(), &bin, &socket_path)
        .expect("Failed to launch plugin");
    thread::sleep(Duration::from_millis(100));

    let err = manager
        .launch_plugin(TEST_PLUGIN_NAME.to_string(), &bin, &socket_path)
        .unwrap_err();
    assert!(err.contains("already running"));

    manager.shutdown_all();
    let _ = fs::remove_file(&socket_path);
}

#[test]
fn test_plugin_error_on_send_to_missing() {
    let mut manager = PluginManager::new();
    let err = manager
        .send(TEST_PLUGIN_NAME, &PluginRequest::Initialize)
        .unwrap_err();
    assert!(err.contains("Plugin not found"));
}
