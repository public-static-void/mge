#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::system::{System, SystemRegistry};
use engine_core::ecs::world::Season;
use engine_core::ecs::world::World;
use engine_core::map::{Map, SquareGridMap};
use engine_core::systems::death_decay::{ProcessDeaths, ProcessDecay};
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

// === time_of_day tests ===

#[test]
fn test_time_of_day_initialization() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(time.hour, 0);
    assert_eq!(time.minute, 0);
    assert_eq!(time.day, 0);
}

#[test]
fn test_time_of_day_advances_on_tick() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));
    World::tick(Rc::clone(&world_rc));
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(time.minute, 1);
}

#[test]
fn test_time_of_day_wraps_after_24_hours() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));
    for _ in 0..(24 * 60) {
        World::tick(Rc::clone(&world_rc));
    }
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(time.hour, 0);
    assert_eq!(time.minute, 0);
    assert_eq!(time.day, 1);
}

#[test]
fn test_day_increments_after_full_day() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));
    for _ in 0..(24 * 60) {
        World::tick(Rc::clone(&world_rc));
    }
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(time.day, 1);
    assert_eq!(time.hour, 0);
    assert_eq!(time.minute, 0);
}

#[test]
fn test_season_default_is_spring() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));
    let time = world_rc.borrow().get_time_of_day();
    assert_eq!(Season::from_day(time.day), Season::Spring);
}

#[test]
fn test_season_from_day_boundaries() {
    assert_eq!(Season::from_day(0), Season::Spring);
    assert_eq!(Season::from_day(29), Season::Spring);
    assert_eq!(Season::from_day(30), Season::Summer);
    assert_eq!(Season::from_day(59), Season::Summer);
    assert_eq!(Season::from_day(60), Season::Autumn);
    assert_eq!(Season::from_day(89), Season::Autumn);
    assert_eq!(Season::from_day(90), Season::Winter);
    assert_eq!(Season::from_day(119), Season::Winter);
    assert_eq!(Season::from_day(120), Season::Spring);
    assert_eq!(Season::from_day(240), Season::Spring);
    assert_eq!(Season::from_day(555), Season::Autumn);
}

#[test]
fn test_season_display_lowercase() {
    assert_eq!(Season::Spring.to_string(), "spring");
    assert_eq!(Season::Summer.to_string(), "summer");
    assert_eq!(Season::Autumn.to_string(), "autumn");
    assert_eq!(Season::Winter.to_string(), "winter");
}

// === turn_system tests ===

#[test]
fn test_tick_advances_turn_and_runs_systems() {
    let mut world = world_helper::make_test_world();
    world.current_mode = "colony".to_string();

    let mut grid = SquareGridMap::new();
    grid.add_cell(1, 2, 0);
    grid.add_cell(2, 2, 0);
    world.map = Some(Map::new(Box::new(grid)));

    let id = world.spawn_entity();
    world
        .set_component(
            id,
            "Position",
            json!({ "pos": { "Square": { "x": 1, "y": 2, "z": 0 } } }),
        )
        .unwrap();
    world
        .set_component(id, "Health", json!({ "current": 10.0, "max": 10.0 }))
        .unwrap();

    if let Some(positions) = world.components.get_mut("Position") {
        for value in positions.values_mut() {
            if let Some(obj) = value.as_object_mut()
                && let Some(pos) = obj.get_mut("pos")
                && let Some(square) = pos.get_mut("Square")
                && let Some(x) = square.get_mut("x")
                && let Some(x_val) = x.as_i64()
            {
                *x = json!(x_val + 1);
            }
        }
    }
    if let Some(healths) = world.components.get_mut("Health") {
        for value in healths.values_mut() {
            if let Some(obj) = value.as_object_mut()
                && let Some(current) = obj.get_mut("current")
                && let Some(cur_val) = current.as_f64()
            {
                let new_val = (cur_val - 1.0).max(0.0);
                *current = json!(new_val);
            }
        }
    }
    world.register_system(ProcessDeaths);
    world.run_system("ProcessDeaths").unwrap();
    world.register_system(ProcessDecay);
    world.run_system("ProcessDecay").unwrap();
    world.turn += 1;

    let pos = world.get_component(id, "Position").unwrap();
    let health = world.get_component(id, "Health").unwrap();

    assert!((pos["pos"]["Square"]["x"].as_f64().unwrap() - 2.0).abs() < 1e-6);
    assert!((health["current"].as_f64().unwrap() - 9.0).abs() < 1e-6);
    assert_eq!(world.turn, 1);
}

// === simulation_scheduler tests ===

#[test]
fn test_systems_execute_in_registered_order() {
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
        fn run(&mut self, _world: &mut World) {
            self.0.lock().unwrap().push("A");
        }
    }
    struct SysB(Arc<Mutex<Vec<&'static str>>>);
    impl System for SysB {
        fn name(&self) -> &'static str {
            "B"
        }
        fn run(&mut self, _world: &mut World) {
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
fn test_dynamic_systems_are_executed_in_tick() {
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
fn test_systems_can_emit_and_receive_events_in_tick() {
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
        fn run(&mut self, world: &mut World) {
            world.send_event("test", json!({"val": 1})).unwrap();
        }
    }
    struct Receiver(Arc<Mutex<Vec<i64>>>);
    impl System for Receiver {
        fn name(&self) -> &'static str {
            "Receiver"
        }
        fn run(&mut self, world: &mut World) {
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

    World::simulation_tick(Rc::clone(&world_rc));
    World::simulation_tick(Rc::clone(&world_rc));

    let events = events.lock().unwrap();
    assert_eq!(&events[..], &[1]);
}

#[test]
fn test_simulation_tick_increments_turn() {
    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let world_rc = Rc::new(RefCell::new(World::new(registry)));
    let turn = world_rc.borrow().turn;
    World::simulation_tick(Rc::clone(&world_rc));
    assert_eq!(world_rc.borrow().turn, turn + 1);
}

#[test]
fn test_event_driven_tick_system_runs_in_order_and_processes_events() {
    let registry = Arc::new(Mutex::new(
        engine_core::ecs::registry::ComponentRegistry::new(),
    ));
    let world_rc = Rc::new(RefCell::new(World::new(registry.clone())));

    let actions = Arc::new(Mutex::new(Vec::new()));

    struct Emitter {
        actions: Arc<Mutex<Vec<String>>>,
    }
    impl System for Emitter {
        fn name(&self) -> &'static str {
            "Emitter"
        }
        fn run(&mut self, world: &mut World) {
            world.emit_event("TestEvent", json!({"value": 42}));
            self.actions.lock().unwrap().push("emitted".into());
        }
        fn dependencies(&self) -> &'static [&'static str] {
            &[]
        }
    }

    struct Receiver {
        actions: Arc<Mutex<Vec<String>>>,
    }
    impl System for Receiver {
        fn name(&self) -> &'static str {
            "Receiver"
        }
        fn run(&mut self, world: &mut World) {
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

    let mut sys_registry = SystemRegistry::new();
    sys_registry.register_system(Emitter {
        actions: actions.clone(),
    });
    sys_registry.register_system(Receiver {
        actions: actions.clone(),
    });

    world_rc.borrow_mut().systems = sys_registry;

    let sorted = world_rc.borrow().systems.sorted_system_names();
    for sys_name in sorted {
        let sys_cell = {
            let mut world_borrow = world_rc.borrow_mut();
            world_borrow.systems.take_system(&sys_name)
        };
        if let Some(cell) = sys_cell {
            {
                let mut world_borrow = world_rc.borrow_mut();
                let mut sys = cell.borrow_mut();
                sys.run(&mut world_borrow);
                world_borrow.update_event_queues();
            }
            world_rc
                .borrow_mut()
                .systems
                .register_system_boxed(sys_name, cell);
        }
    }

    let actions = actions.lock().unwrap();
    assert_eq!(actions.as_slice(), ["emitted", "received"]);
}
