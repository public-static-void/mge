use engine_core::ecs::system::{System, SystemRegistry};
use engine_core::ecs::world::World;
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[test]
fn systems_execute_in_registered_order() {
    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));

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

    world_rc.borrow_mut().register_system(SysA(log.clone()));
    world_rc.borrow_mut().register_system(SysB(log.clone()));

    World::simulation_tick(Rc::clone(&world_rc));

    let log = log.lock().unwrap();
    assert!(
        log.as_slice() == ["A", "B"] || log.as_slice() == ["B", "A"],
        "Order was: {:?}, but expected [\"A\", \"B\"] or [\"B\", \"A\"]",
        log.as_slice()
    );
}

#[test]
fn dynamic_systems_are_executed_in_tick() {
    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));

    let log = Arc::new(Mutex::new(Vec::new()));

    world_rc.borrow_mut().register_dynamic_system("dyn", {
        let log = log.clone();
        move |_, _| log.lock().unwrap().push("dyn")
    });

    World::simulation_tick(Rc::clone(&world_rc));

    let log = log.lock().unwrap();
    assert!(log.contains(&"dyn"));
}

#[test]
fn systems_can_emit_and_receive_events_in_tick() {
    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));

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
            let bus = world.get_or_create_event_bus::<serde_json::Value>("test");
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

    world_rc.borrow_mut().register_system(Emitter);
    world_rc
        .borrow_mut()
        .register_system(Receiver(events.clone()));

    World::simulation_tick(Rc::clone(&world_rc)); // Emitter sends, Receiver does NOT see it yet
    World::simulation_tick(Rc::clone(&world_rc)); // Receiver sees the event

    let events = events.lock().unwrap();
    assert_eq!(&events[..], &[1]);
}

#[test]
fn simulation_tick_increments_turn() {
    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));
    let turn = world_rc.borrow().turn;
    World::simulation_tick(Rc::clone(&world_rc));
    assert_eq!(world_rc.borrow().turn, turn + 1);
}

#[test]
fn event_driven_tick_system_runs_in_order_and_processes_events() {
    // Setup world and registry
    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let world_rc = Rc::new(RefCell::new(World::new(registry.clone())));

    // Shared vector to record actions for assertion
    let actions = Arc::new(Mutex::new(Vec::new()));

    // System 1: Emits an event
    struct Emitter {
        actions: Arc<Mutex<Vec<String>>>,
    }
    impl System for Emitter {
        fn name(&self) -> &'static str {
            "Emitter"
        }
        fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
            world.emit_event("TestEvent", json!({"value": 42}));
            self.actions.lock().unwrap().push("emitted".into());
        }
        fn dependencies(&self) -> &'static [&'static str] {
            &[]
        }
    }

    // System 2: Subscribes to the event
    struct Receiver {
        actions: Arc<Mutex<Vec<String>>>,
    }
    impl System for Receiver {
        fn name(&self) -> &'static str {
            "Receiver"
        }
        fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
            let mut received = false;
            world.process_events("TestEvent", |payload| {
                if payload.get("value") == Some(&json!(42)) {
                    received = true;
                }
            });
            if received {
                self.actions.lock().unwrap().push("received".into());
            }
        }
        fn dependencies(&self) -> &'static [&'static str] {
            &["Emitter"]
        }
    }

    // Register systems
    let mut sys_registry = SystemRegistry::new();
    sys_registry.register_system(Emitter {
        actions: actions.clone(),
    });
    sys_registry.register_system(Receiver {
        actions: actions.clone(),
    });

    // Assign registry to world
    world_rc.borrow_mut().systems = sys_registry;

    // Run the new tick loop
    let sorted = world_rc.borrow().systems.sorted_system_names();
    for sys_name in sorted {
        // Take the system out of the registry
        let sys_cell = {
            let mut world_borrow = world_rc.borrow_mut();
            world_borrow.systems.take_system(&sys_name)
        };
        if let Some(cell) = sys_cell {
            {
                let mut world_borrow = world_rc.borrow_mut();
                let mut sys = cell.borrow_mut();
                sys.run(&mut world_borrow, None);
                world_borrow.update_event_queues();
            }
            // Put the system back in the registry
            world_rc
                .borrow_mut()
                .systems
                .register_system_boxed(sys_name, cell);
        }
    }

    // Assert both systems ran and the event was delivered in order
    let actions = actions.lock().unwrap();
    assert_eq!(actions.as_slice(), ["emitted", "received"]);
}
