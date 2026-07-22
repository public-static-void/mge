use engine_core::ecs::ComponentSchema;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use schemars::Schema;
use serde_json::json;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Debug, Clone, PartialEq)]
struct FooEvent(pub i32);

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
    struct Dummy;
    impl System for Dummy {
        fn name(&self) -> &'static str {
            "Dummy"
        }
        fn run(&mut self, _: &mut World) {}
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

#[test]
fn test_get_and_set_mode() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    assert_eq!(world.get_mode(), "colony");
    world.set_mode("roguelike");
    assert_eq!(world.get_mode(), "roguelike");
}

#[test]
fn test_world_can_register_and_run_system() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    let called = Arc::new(AtomicBool::new(false));
    struct TestSystem {
        called: Arc<AtomicBool>,
    }
    impl System for TestSystem {
        fn name(&self) -> &'static str {
            "TestSystem"
        }
        fn run(&mut self, _world: &mut World) {
            self.called.store(true, Ordering::SeqCst);
        }
    }

    world.register_system(TestSystem {
        called: called.clone(),
    });
    world.run_system("TestSystem").unwrap();
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_world_lists_systems() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    struct DummySystem;
    impl System for DummySystem {
        fn name(&self) -> &'static str {
            "DummySystem"
        }

        fn run(&mut self, _world: &mut World) {}
    }

    world.register_system(DummySystem);
    let systems = world.list_systems();
    assert!(systems.contains(&"DummySystem".to_string()));
}

#[test]
fn test_component_data_cleanup_on_unregister() {
    let mut registry = ComponentRegistry::new();
    let schema = ComponentSchema {
        name: "CleanupComponent".to_string(),
        schema: Schema::default().into(),
        modes: vec!["colony".to_string()],
    };
    registry.register_external_schema(schema);

    let registry = Arc::new(Mutex::new(registry));
    let mut world = World::new(registry.clone());
    let eid = world.spawn_entity();
    world
        .set_component(eid, "CleanupComponent", serde_json::json!({"foo": 1}))
        .unwrap();
    assert!(world.get_component(eid, "CleanupComponent").is_some());

    world.unregister_component_and_cleanup("CleanupComponent");
    assert!(world.get_component(eid, "CleanupComponent").is_none());
}

#[test]
fn test_world_eventbus_integration() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    // Register event bus via World
    let bus = world.get_or_create_event_bus("TestBus");
    bus.lock().unwrap().send(json!({"foo": 1}));
    bus.lock().unwrap().update();
    let events = world.take_events("TestBus");
    assert_eq!(events, vec![json!({"foo": 1})]);

    // Hot-reload: update event bus via registry
    let new_bus = Arc::new(Mutex::new(engine_core::ecs::event::EventBus::default()));
    world
        .event_buses
        .register_event_bus::<serde_json::Value>("TestBus".to_string(), new_bus.clone());
    assert!(Arc::ptr_eq(
        &world.get_event_bus("TestBus").unwrap(),
        &new_bus
    ));

    // Unregister event bus via registry
    assert!(
        world
            .event_buses
            .unregister_event_bus::<serde_json::Value>("TestBus")
    );
}

#[test]
fn test_world_type_safe_event_buses() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    let bus = world.get_or_create_event_bus::<FooEvent>("foo");
    bus.lock().unwrap().send(FooEvent(123));
    bus.lock().unwrap().update();

    let mut reader = engine_core::ecs::event::EventReader::default();
    let events: Vec<_> = reader.read(&*bus.lock().unwrap()).cloned().collect();
    assert_eq!(events, vec![FooEvent(123)]);

    // Update all FooEvent buses in the world
    world.update_event_buses::<FooEvent>();
}
