use std::sync::{Arc, Mutex};

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

    let _ = unsafe {
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

    let _ = unsafe {
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
