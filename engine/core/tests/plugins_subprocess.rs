#[path = "helpers/plugins.rs"]
mod plugins_helper;
use plugins_helper::{plugin_bin_path, test_socket_path};

use engine_core::plugins::manager::PluginManager;
use engine_core::plugins::subprocess::{PluginRequest, PluginResponse};
use std::fs;
use std::thread;
use std::time::Duration;

const TEST_PLUGIN_NAME: &str = "rust_test_plugin";

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
