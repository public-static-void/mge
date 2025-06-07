use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use std::sync::{Arc, Mutex};

#[test]
fn test_entity_and_component_existence() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry.clone());
    // Register schema for Foo
    registry
        .lock()
        .unwrap()
        .register_external_schema(engine_core::ecs::schema::ComponentSchema {
            name: "Foo".to_string(),
            schema: serde_json::json!({
                "title": "Foo",
                "modes": ["colony"],
                "type": "object"
            }),
            modes: vec!["colony".to_string()],
        });
    let eid = world.spawn_entity();
    assert!(world.entity_exists(eid));
    assert!(!world.has_component(eid, "Foo"));
    world
        .set_component(eid, "Foo", serde_json::json!({"bar": 1}))
        .unwrap();
    assert!(world.has_component(eid, "Foo"));
}

#[test]
fn test_system_existence() {
    use engine_core::ecs::system::System;
    struct Dummy;
    impl System for Dummy {
        fn name(&self) -> &'static str {
            "Dummy"
        }
        fn run(&mut self, _: &mut World, _: Option<&mlua::Lua>) {}
    }
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);
    world.register_system(Dummy);
    assert!(world.has_system("Dummy"));
}

#[test]
fn test_mode_switching_removes_disallowed_components() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry.clone());
    // Register a schema for "Foo" allowed only in "colony"
    registry
        .lock()
        .unwrap()
        .register_external_schema(engine_core::ecs::schema::ComponentSchema {
            name: "Foo".to_string(),
            schema: serde_json::json!({
                "title": "Foo",
                "modes": ["colony"],
                "type": "object"
            }),
            modes: vec!["colony".to_string()],
        });
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Foo", serde_json::json!({"x": 1}))
        .unwrap();
    world.set_mode("roguelike");
    assert!(!world.has_component(eid, "Foo"));
}
