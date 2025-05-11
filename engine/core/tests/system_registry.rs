use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::system::{System, SystemRegistry};
use engine_core::scripting::world::World;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// Dummy system for registration/listing test
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
    let mut system_registry = SystemRegistry::new();
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
    system_registry.register_system(TestSystem {
        called: called.clone(),
    });

    let component_registry = Arc::new(ComponentRegistry::new());
    let mut world = World::new(component_registry);

    system_registry
        .run_system("TestSystem", &mut world)
        .unwrap();
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_run_nonexistent_system_errors() {
    let mut system_registry = SystemRegistry::new();
    let component_registry = Arc::new(ComponentRegistry::new());
    let mut world = World::new(component_registry);
    let result = system_registry.run_system("NoSuchSystem", &mut world);
    assert!(result.is_err());
}
