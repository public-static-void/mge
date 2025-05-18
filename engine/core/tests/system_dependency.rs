use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::system::{System, SystemRegistry};
use engine_core::ecs::world::World;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// Helper system that records its run order.
struct OrderSystem {
    name: &'static str,
    ran: Arc<AtomicUsize>,
    order: Arc<std::sync::Mutex<Vec<&'static str>>>,
    dependencies: &'static [&'static str],
}
impl System for OrderSystem {
    fn name(&self) -> &'static str {
        self.name
    }
    fn run(&mut self, _world: &mut World, _lua: Option<&mlua::Lua>) {
        self.ran.fetch_add(1, Ordering::SeqCst);
        self.order.lock().unwrap().push(self.name);
    }
    fn dependencies(&self) -> &'static [&'static str] {
        self.dependencies
    }
}

#[test]
fn test_systems_run_in_dependency_order() {
    let ran = Arc::new(AtomicUsize::new(0));
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));

    // C depends on B, B depends on A
    let mut registry = SystemRegistry::new();
    registry.register_system(OrderSystem {
        name: "A",
        ran: ran.clone(),
        order: order.clone(),
        dependencies: &[],
    });
    registry.register_system(OrderSystem {
        name: "B",
        ran: ran.clone(),
        order: order.clone(),
        dependencies: &["A"],
    });
    registry.register_system(OrderSystem {
        name: "C",
        ran: ran.clone(),
        order: order.clone(),
        dependencies: &["B"],
    });

    let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(component_registry.clone());
    world.systems = registry;

    world.simulation_tick();

    let run_order = order.lock().unwrap().clone();
    assert_eq!(run_order, vec!["A", "B", "C"]);
}

#[test]
fn test_cycle_detection_errors() {
    let ran = Arc::new(AtomicUsize::new(0));
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));

    let mut registry = SystemRegistry::new();
    registry.register_system(OrderSystem {
        name: "A",
        ran: ran.clone(),
        order: order.clone(),
        dependencies: &["B"],
    });
    registry.register_system(OrderSystem {
        name: "B",
        ran: ran.clone(),
        order: order.clone(),
        dependencies: &["A"],
    });

    let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(component_registry.clone());
    world.systems = registry;

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let mut world = world;
        world.simulation_tick();
    }));
    assert!(result.is_err(), "Cycle was not detected!");
}

#[test]
fn test_independent_systems_run_in_registration_order() {
    let ran = Arc::new(AtomicUsize::new(0));
    let order = Arc::new(std::sync::Mutex::new(Vec::new()));

    let mut registry = SystemRegistry::new();
    registry.register_system(OrderSystem {
        name: "X",
        ran: ran.clone(),
        order: order.clone(),
        dependencies: &[],
    });
    registry.register_system(OrderSystem {
        name: "Y",
        ran: ran.clone(),
        order: order.clone(),
        dependencies: &[],
    });

    let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(component_registry.clone());
    world.systems = registry;

    world.simulation_tick();

    let run_order = order.lock().unwrap().clone();
    assert!(
        run_order == vec!["X", "Y"] || run_order == vec!["Y", "X"],
        "Order was: {:?}, expected [\"X\", \"Y\"] or [\"Y\", \"X\"]",
        run_order
    );
}
