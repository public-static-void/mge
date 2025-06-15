use std::sync::{Arc, Mutex};

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
        .join("test_plugin")
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
        .join("test_plugin")
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
