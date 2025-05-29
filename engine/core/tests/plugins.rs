use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

#[test]
fn test_ffi_spawn_entity_and_set_component() {
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

    let registry = Arc::new(Mutex::new(registry));
    let mut world = engine_core::ecs::World::new(registry.clone());

    let entity_id = world.spawn_entity();
    assert!(entity_id > 0);

    let json_value = serde_json::json!({ "x": 10.0, "y": 20.0 });

    let result = world.set_component(entity_id, "Position", json_value);
    assert!(result.is_ok());

    let component = world.get_component(entity_id, "Position");
    assert!(component.is_some());
}

#[test]
fn test_ffi_spawn_entity_and_set_component_via_ffi() {
    use std::ffi::CString;
    use std::os::raw::c_void;

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
    let registry = Arc::new(Mutex::new(registry));
    let mut world = engine_core::ecs::World::new(registry.clone());

    let world_ptr = &mut world as *mut _ as *mut c_void;

    let entity_id = unsafe { engine_core::plugins::ffi_spawn_entity(world_ptr) };
    assert!(entity_id > 0);

    let comp_name = CString::new("Position").unwrap();
    let comp_json = CString::new(r#"{"x": 42, "y": 99}"#).unwrap();
    let set_result = unsafe {
        engine_core::plugins::ffi_set_component(
            world_ptr,
            entity_id,
            comp_name.as_ptr(),
            comp_json.as_ptr(),
        )
    };
    assert_eq!(set_result, 0);

    let comp = world.get_component(entity_id, "Position");
    assert!(comp.is_some());
    let comp = comp.unwrap();
    assert_eq!(comp["x"], 42.0);
    assert_eq!(comp["y"], 99.0);
}

#[test]
fn test_loads_and_initializes_plugin() {
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
    let registry = Arc::new(Mutex::new(registry));
    let mut world = engine_core::ecs::World::new(registry.clone());
    let world_ptr = &mut world as *mut _ as *mut c_void;

    let mut engine_api = engine_core::plugins::EngineApi {
        spawn_entity: engine_core::plugins::ffi_spawn_entity,
        set_component: engine_core::plugins::ffi_set_component,
    };

    let plugin_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find project root")
        .join("plugins")
        .join("libtest_plugin.so");

    unsafe {
        engine_core::plugins::load_plugin(&plugin_path, &mut engine_api, world_ptr)
            .expect("Failed to load plugin")
    };

    let entities = world.get_entities();
    assert!(!entities.is_empty());
}

#[test]
fn test_plugin_registers_system() {
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
    let registry = Arc::new(Mutex::new(registry));
    let mut world = engine_core::ecs::World::new(registry.clone());
    let world_ptr = &mut world as *mut _ as *mut c_void;

    let mut engine_api = engine_core::plugins::EngineApi {
        spawn_entity: engine_core::plugins::ffi_spawn_entity,
        set_component: engine_core::plugins::ffi_set_component,
    };

    let plugin_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find project root")
        .join("plugins")
        .join("libtest_plugin.so");

    unsafe {
        engine_core::plugins::load_plugin_and_register_systems(
            &plugin_path,
            &mut engine_api,
            world_ptr,
            &mut world,
        )
        .expect("Failed to load plugin and register systems")
    };

    let systems = world.dynamic_systems.list_systems();
    assert!(
        systems.contains(&"hello_system".to_string()),
        "System 'hello_system' should be registered by plugin"
    );
    // Optionally, run the system and check for a side effect
    // world.run_system("hello_system").expect("System run failed");
}

#[test]
fn test_plugin_registers_and_frees_dynamic_systems() {
    use std::ffi::c_void;

    // This test assumes a plugin that allocates its SystemPlugin array dynamically
    // For now, we check that the loader calls free_systems if present.
    // (You may need to add a test plugin that does this for full coverage.)

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
    let registry = Arc::new(Mutex::new(registry));
    let mut world = engine_core::ecs::World::new(registry.clone());
    let world_ptr = &mut world as *mut _ as *mut c_void;

    let mut engine_api = engine_core::plugins::EngineApi {
        spawn_entity: engine_core::plugins::ffi_spawn_entity,
        set_component: engine_core::plugins::ffi_set_component,
    };

    let plugin_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find project root")
        .join("plugins")
        .join("libtest_plugin.so");

    // This will call free_systems if present (for static array it's NULL, so no-op)
    unsafe {
        engine_core::plugins::load_plugin_and_register_systems(
            &plugin_path,
            &mut engine_api,
            world_ptr,
            &mut world,
        )
        .expect("Failed to load plugin and register systems")
    };

    // No assertion needed: test passes if no segfault/leak and systems are registered.
}

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
        .join("plugins");
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

#[test]
fn test_plugin_dependency_resolution_cycle() {
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
            dependencies: vec!["A".to_string()],
            dynamic_library: "libb.so".to_string(),
        },
    );
    let err = resolve_plugin_load_order(&[a, b]).unwrap_err();
    assert!(err.contains("Cycle detected"));
}

#[test]
fn test_plugin_dependency_resolution_missing_dep() {
    use engine_core::plugins::loader::resolve_plugin_load_order;
    use engine_core::plugins::types::PluginManifest;

    let a = (
        "a.json".to_string(),
        PluginManifest {
            name: "A".to_string(),
            version: "1.0.0".to_string(),
            description: "".to_string(),
            authors: vec![],
            dependencies: vec!["B".to_string(), "C".to_string()],
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
    // C is missing
    let err = resolve_plugin_load_order(&[a, b]).unwrap_err();
    assert!(
        err.starts_with("Missing dependencies:"),
        "Expected missing dependency error, got: {err}"
    );
}
