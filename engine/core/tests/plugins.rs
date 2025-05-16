#[test]
fn test_ffi_spawn_entity_and_set_component() {
    // Setup: Create a new registry and register Position schema
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

    let registry = std::sync::Arc::new(registry);
    let mut world = engine_core::scripting::World::new(registry);

    // Call spawn_entity directly
    let entity_id = world.spawn_entity();
    assert!(entity_id > 0);

    // Prepare JSON component data
    let json_value = serde_json::json!({ "x": 10.0, "y": 20.0 });

    // Set component
    let result = world.set_component(entity_id, "Position", json_value);
    assert!(result.is_ok());

    // Verify component was set
    let component = world.get_component(entity_id, "Position");
    assert!(component.is_some());
}

#[test]
fn test_ffi_spawn_entity_and_set_component_via_ffi() {
    use std::ffi::CString;
    use std::os::raw::c_void;

    // Setup registry and world as before
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
    let registry = std::sync::Arc::new(registry);
    let mut world = engine_core::scripting::World::new(registry);

    // Prepare FFI call
    let world_ptr = &mut world as *mut _ as *mut c_void;

    // Call the FFI spawn_entity function (to be implemented)
    let entity_id = unsafe { engine_core::plugins::ffi_spawn_entity(world_ptr) };
    assert!(entity_id > 0);

    // Prepare FFI set_component call
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

    // Verify via native API
    let comp = world.get_component(entity_id, "Position");
    assert!(comp.is_some());
    let comp = comp.unwrap();
    assert_eq!(comp["x"], 42.0);
    assert_eq!(comp["y"], 99.0);
}

#[test]
fn test_loads_and_initializes_plugin() {
    use std::ffi::c_void;

    // Setup: create a world and registry as before
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
    let registry = std::sync::Arc::new(registry);
    let mut world = engine_core::scripting::World::new(registry);
    let world_ptr = &mut world as *mut _ as *mut c_void;

    // Prepare EngineApi struct
    let mut engine_api = engine_core::plugins::EngineApi {
        spawn_entity: engine_core::plugins::ffi_spawn_entity,
        set_component: engine_core::plugins::ffi_set_component,
    };

    let plugin_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // up to 'engine'
        .and_then(|p| p.parent()) // up to project root
        .expect("Failed to find project root")
        .join("plugins")
        .join("libtest_plugin.so");

    let _ = unsafe {
        engine_core::plugins::load_plugin(&plugin_path, &mut engine_api, world_ptr)
            .expect("Failed to load plugin")
    };

    // Optionally, call update/shutdown if you expose those
    // (loaded_plugin.vtable.update)(0.16);

    // Assert world state was changed by plugin
    let entities = world.get_entities();
    assert!(!entities.is_empty());
}

#[test]
fn test_plugin_registers_system() {
    use std::ffi::c_void;

    // Setup registry and world
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
    let registry = std::sync::Arc::new(registry);
    let mut world = engine_core::scripting::World::new(registry);
    let world_ptr = &mut world as *mut _ as *mut c_void;

    // Prepare EngineApi struct
    let mut engine_api = engine_core::plugins::EngineApi {
        spawn_entity: engine_core::plugins::ffi_spawn_entity,
        set_component: engine_core::plugins::ffi_set_component,
    };

    // This is the new loader you will implement!
    let plugin_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to find project root")
        .join("plugins")
        .join("libtest_plugin.so");

    let _ = unsafe {
        engine_core::plugins::load_plugin_and_register_systems(
            &plugin_path,
            &mut engine_api,
            world_ptr,
            &mut world,
        )
        .expect("Failed to load plugin and register systems")
    };

    // Assert the system is now registered
    let systems = world.dynamic_systems.list_systems();
    assert!(
        systems.contains(&"hello_system".to_string()),
        "System 'hello_system' should be registered by plugin"
    );

    // Optionally, run the system and check for a side effect
    // world.run_system("hello_system").expect("System run failed");
}
