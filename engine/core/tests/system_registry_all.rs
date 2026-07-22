use engine_core::ecs::World;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::system::{System, SystemRegistry};
use engine_core::plugins::dynamic_systems::DynamicSystemRegistry;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// === system_registry tests ===

struct DummySystem;
impl System for DummySystem {
    fn name(&self) -> &'static str {
        "DummySystem"
    }
    fn run(&mut self, _world: &mut World) {}
}

#[test]
fn test_register_and_list_systems() {
    let mut system_registry = SystemRegistry::new();
    system_registry.register_system(DummySystem);
    let systems = system_registry.list_systems();
    assert!(systems.contains(&"DummySystem".to_string()));
}

#[test]
fn test_run_system() {
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

    let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(component_registry.clone());

    world.register_system(TestSystem {
        called: called.clone(),
    });

    world.run_system("TestSystem").unwrap();
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_run_nonexistent_system_errors() {
    let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(component_registry.clone());
    let result = world.run_system("NoSuchSystem");
    assert!(result.is_err());
}

#[test]
fn test_register_and_unregister_system() {
    let mut registry = SystemRegistry::new();
    registry.register_system(DummySystem);
    assert!(registry.list_systems().contains(&"DummySystem".to_string()));

    registry.unregister_system("DummySystem");
    assert!(!registry.list_systems().contains(&"DummySystem".to_string()));
}

struct A;
impl System for A {
    fn name(&self) -> &'static str {
        "A"
    }
    fn run(&mut self, _world: &mut World) {}
    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }
}

struct B;
impl System for B {
    fn name(&self) -> &'static str {
        "B"
    }
    fn run(&mut self, _world: &mut World) {}
    fn dependencies(&self) -> &'static [&'static str] {
        &["A"]
    }
}

#[test]
fn test_register_and_query_systems() {
    let mut reg = SystemRegistry::new();
    reg.register_system(A);
    reg.register_system(B);

    assert!(reg.is_registered("A"));
    assert!(reg.is_registered("B"));
    assert!(!reg.is_registered("C"));

    assert!(reg.get_system("A").is_some());
    assert!(reg.get_system_mut("A").is_some());
}

#[test]
fn test_unregister_system() {
    let mut reg = SystemRegistry::new();
    reg.register_system(A);
    assert!(reg.is_registered("A"));
    reg.unregister_system("A");
    assert!(!reg.is_registered("A"));
}

#[test]
fn test_sorted_system_names() {
    let mut reg = SystemRegistry::new();
    reg.register_system(A);
    reg.register_system(B);
    let sorted = reg.sorted_system_names();
    assert_eq!(sorted, vec!["A".to_string(), "B".to_string()]);
}

#[test]
#[should_panic]
fn test_cycle_detection() {
    struct C;
    impl System for C {
        fn name(&self) -> &'static str {
            "C"
        }
        fn run(&mut self, _world: &mut World) {}
        fn dependencies(&self) -> &'static [&'static str] {
            &["D"]
        }
    }
    struct D;
    impl System for D {
        fn name(&self) -> &'static str {
            "D"
        }
        fn run(&mut self, _world: &mut World) {}
        fn dependencies(&self) -> &'static [&'static str] {
            &["C"]
        }
    }
    let mut reg = SystemRegistry::new();
    reg.register_system(C);
    reg.register_system(D);
    reg.sorted_system_names();
}

// === system_dependencies tests ===

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
    fn run(&mut self, _world: &mut World) {
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
    let world_rc = Rc::new(RefCell::new(World::new(component_registry.clone())));
    world_rc.borrow_mut().systems = registry;

    engine_core::ecs::world::World::simulation_tick(Rc::clone(&world_rc));

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
    let world_rc = Rc::new(RefCell::new(World::new(component_registry.clone())));
    world_rc.borrow_mut().systems = registry;

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        engine_core::ecs::world::World::simulation_tick(Rc::clone(&world_rc));
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
    let world_rc = Rc::new(RefCell::new(World::new(component_registry.clone())));
    world_rc.borrow_mut().systems = registry;

    engine_core::ecs::world::World::simulation_tick(Rc::clone(&world_rc));

    let run_order = order.lock().unwrap().clone();
    assert!(
        run_order == vec!["X", "Y"] || run_order == vec!["Y", "X"],
        "Order was: {run_order:?}, expected [\"X\", \"Y\"] or [\"Y\", \"X\"]"
    );
}

// === system_registry_dependencies tests ===

#[test]
fn test_system_dependency_ordering() {
    let mut registry = DynamicSystemRegistry::new();
    let world_rc = Rc::new(RefCell::new(World::new(Arc::new(Mutex::new(
        ComponentRegistry::new(),
    )))));

    let order = Arc::new(Mutex::new(Vec::new()));

    {
        let order = Arc::clone(&order);
        registry.register_system_with_deps(
            "A".to_string(),
            vec![],
            Box::new(move |_, _| {
                order.lock().unwrap().push("A");
            }),
        );
    }

    {
        let order = Arc::clone(&order);
        registry.register_system_with_deps(
            "B".to_string(),
            vec!["A".to_string()],
            Box::new(move |_, _| {
                order.lock().unwrap().push("B");
            }),
        );
    }

    {
        let order = Arc::clone(&order);
        registry.register_system_with_deps(
            "C".to_string(),
            vec!["B".to_string()],
            Box::new(move |_, _| {
                order.lock().unwrap().push("C");
            }),
        );
    }

    registry.run_all_systems(Rc::clone(&world_rc), 0.0).unwrap();

    let result = order.lock().unwrap().clone();
    assert_eq!(result, vec!["A", "B", "C"]);

    registry.register_system_with_deps("D".to_string(), vec!["C".to_string()], Box::new(|_, _| {}));
    registry
        .update_system_dependencies("C", vec!["B".to_string(), "D".to_string()])
        .unwrap();

    let run_result = registry.run_all_systems(Rc::clone(&world_rc), 0.0);
    assert!(run_result.is_err());
}

// === system_registry_hot_reload tests ===

// Use static mut for test state (not thread-safe, but fine for single-threaded test)
static mut CALLED: bool = false;
static mut CALLED2: bool = false;

#[test]
fn test_system_register_update_unregister() {
    let mut registry = DynamicSystemRegistry::new();
    let ecs_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world_rc = Rc::new(RefCell::new(World::new(ecs_registry)));

    registry.register_system(
        "test".to_string(),
        Box::new(|_world_rc: Rc<RefCell<World>>, _dt: f32| unsafe {
            CALLED = true;
        }),
    );

    unsafe {
        CALLED = false;
    }
    registry
        .run_system(Rc::clone(&world_rc), "test", 0.0)
        .unwrap();
    unsafe {
        assert!(CALLED);
    }

    registry.register_system(
        "test".to_string(),
        Box::new(|_world_rc: Rc<RefCell<World>>, _dt: f32| unsafe {
            CALLED2 = true;
        }),
    );

    unsafe {
        CALLED2 = false;
    }
    registry
        .run_system(Rc::clone(&world_rc), "test", 0.0)
        .unwrap();
    unsafe {
        assert!(CALLED2);
    }

    let _ = registry.unregister_system("test");
    assert!(
        registry
            .run_system(Rc::clone(&world_rc), "test", 0.0)
            .is_err()
    );
}
