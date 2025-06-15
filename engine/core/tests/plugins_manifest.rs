use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_load_plugin_manifest() {
    use engine_core::plugins::loader::load_plugin_manifest;

    let dir = tempdir().unwrap();
    let manifest_path = dir.path().join("plugin.json");
    let manifest_content = r#"
    {
        "name": "Test Plugin",
        "version": "1.2.3",
        "description": "A test plugin.",
        "authors": ["Alice", "Bob"],
        "dependencies": ["foo", "bar"],
        "dynamic_library": "libtest_plugin.so"
    }
    "#;
    let mut file = File::create(&manifest_path).unwrap();
    file.write_all(manifest_content.as_bytes()).unwrap();

    let manifest = load_plugin_manifest(&manifest_path).unwrap();
    assert_eq!(manifest.name, "Test Plugin");
    assert_eq!(manifest.version, "1.2.3");
    assert_eq!(manifest.description, "A test plugin.");
    assert_eq!(manifest.authors, vec!["Alice", "Bob"]);
    assert_eq!(manifest.dependencies, vec!["foo", "bar"]);
    assert_eq!(manifest.dynamic_library, "libtest_plugin.so");
}

#[test]
fn test_load_plugin_with_manifest_and_metadata() {
    use engine_core::plugins::loader::load_plugin_with_manifest;
    use std::ffi::c_void;

    let mut registry = engine_core::ecs::registry::ComponentRegistry::new();
    let schema_json = r#"
    {
        "title": "Position",
        "type": "object",
        "properties": {
            "x": { "type": "number" },
            "y": { "type": "number" }
        },
        "required": ["x", "y"],
        "modes": ["colony", "roguelike"]
    }
    "#;
    registry
        .register_external_schema_from_json(schema_json)
        .unwrap();
    let registry = std::sync::Arc::new(std::sync::Mutex::new(registry));
    let mut world = engine_core::ecs::World::new(registry.clone());
    let world_ptr = &mut world as *mut _ as *mut c_void;

    let plugin_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find project root")
        .join("plugins")
        .join("test_plugin");
    let manifest_path = plugin_dir.join("plugin.json");

    // Write a test manifest file pointing to the test plugin
    let manifest_content = r#"
    {
        "name": "Test Plugin",
        "version": "1.0.0",
        "description": "A test plugin for the Modular Game Engine.",
        "authors": ["Test Author"],
        "dependencies": [],
        "dynamic_library": "libtest_plugin.so"
    }
    "#;
    std::fs::write(&manifest_path, manifest_content).unwrap();

    let mut engine_api = engine_core::plugins::EngineApi {
        spawn_entity: engine_core::plugins::ffi_spawn_entity,
        set_component: engine_core::plugins::ffi_set_component,
    };

    // This should not panic or return an error
    let result = unsafe { load_plugin_with_manifest(&manifest_path, &mut engine_api, world_ptr) };
    assert!(result.is_ok());
}

#[test]
fn test_plugin_dependency_resolution_simple() {
    use engine_core::plugins::loader::resolve_plugin_load_order;
    use engine_core::plugins::types::PluginManifest;

    let a = (
        "a.json".to_string(),
        PluginManifest {
            name: "A".to_string(),
            version: "1.0.0".to_string(),
            description: "".to_string(),
            authors: vec![],
            dependencies: vec!["B".to_string()],
            dynamic_library: "liba.so".to_string(),
        },
    );
    let b = (
        "b.json".to_string(),
        PluginManifest {
            name: "B".to_string(),
            version: "1.0.0".to_string(),
            description: "".to_string(),
            authors: vec![],
            dependencies: vec![],
            dynamic_library: "libb.so".to_string(),
        },
    );
    let order = resolve_plugin_load_order(&[a.clone(), b.clone()]).unwrap();
    assert_eq!(order, vec!["b.json".to_string(), "a.json".to_string()]);
}
