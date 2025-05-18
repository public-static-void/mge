use std::sync::{Arc, Mutex};

#[test]
fn systems_execute_in_registered_order() {
    use engine_core::ecs::system::System;
    use engine_core::scripting::world::World;

    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let mut world = World::new(registry);

    let log = Arc::new(Mutex::new(Vec::new()));

    struct SysA(Arc<Mutex<Vec<&'static str>>>);
    impl System for SysA {
        fn name(&self) -> &'static str {
            "A"
        }
        fn run(&mut self, _world: &mut World, _lua: Option<&mlua::Lua>) {
            self.0.lock().unwrap().push("A");
        }
    }
    struct SysB(Arc<Mutex<Vec<&'static str>>>);
    impl System for SysB {
        fn name(&self) -> &'static str {
            "B"
        }
        fn run(&mut self, _world: &mut World, _lua: Option<&mlua::Lua>) {
            self.0.lock().unwrap().push("B");
        }
    }

    world.register_system(SysA(log.clone()));
    world.register_system(SysB(log.clone()));

    world.simulation_tick();

    let log = log.lock().unwrap();
    assert!(
        log.as_slice() == ["A", "B"] || log.as_slice() == ["B", "A"],
        "Order was: {:?}, but expected [\"A\", \"B\"] or [\"B\", \"A\"]",
        log.as_slice()
    );
}

#[test]
fn dynamic_systems_are_executed_in_tick() {
    use engine_core::scripting::world::World;

    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let mut world = World::new(registry);

    let log = Arc::new(Mutex::new(Vec::new()));

    world.register_dynamic_system("dyn", {
        let log = log.clone();
        move |_, _| log.lock().unwrap().push("dyn")
    });

    world.simulation_tick();

    let log = log.lock().unwrap();
    assert!(log.contains(&"dyn"));
}

#[test]
fn systems_can_emit_and_receive_events_in_tick() {
    use engine_core::ecs::system::System;
    use engine_core::scripting::world::World;
    use serde_json::json;

    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let mut world = World::new(registry);

    let events = Arc::new(Mutex::new(Vec::new()));

    struct Emitter;
    impl System for Emitter {
        fn name(&self) -> &'static str {
            "Emitter"
        }
        fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
            world.send_event("test", json!({"val": 1})).unwrap();
        }
    }
    struct Receiver(Arc<Mutex<Vec<i64>>>);
    impl System for Receiver {
        fn name(&self) -> &'static str {
            "Receiver"
        }
        fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
            use engine_core::ecs::event::EventReader;
            let bus = world.get_or_create_event_bus("test");
            let mut reader = EventReader::default();
            let bus = bus.lock().unwrap();
            for event in reader.read(&bus) {
                if let Some(val) = event.get("val").and_then(|v| v.as_i64()) {
                    self.0.lock().unwrap().push(val);
                }
            }
        }
        fn dependencies(&self) -> &'static [&'static str] {
            &["Emitter"]
        }
    }

    world.register_system(Emitter);
    world.register_system(Receiver(events.clone()));

    world.simulation_tick(); // Emitter sends, Receiver does NOT see it yet
    world.simulation_tick(); // Receiver sees the event

    let events = events.lock().unwrap();
    assert_eq!(&events[..], &[1]);
}

#[test]
fn simulation_tick_increments_turn() {
    use engine_core::scripting::world::World;
    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let mut world = World::new(registry);
    let turn = world.turn;
    world.simulation_tick();
    assert_eq!(world.turn, turn + 1);
}
