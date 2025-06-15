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
