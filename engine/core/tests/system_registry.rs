use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::system::{System, SystemRegistry};
use engine_core::ecs::world::World;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// Dummy system for registration/listing test
struct DummySystem;
impl System for DummySystem {
    fn name(&self) -> &'static str {
        "DummySystem"
    }
    fn run(&mut self, _world: &mut World, _lua: Option<&mlua::Lua>) {}
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
        fn run(&mut self, _world: &mut World, _lua: Option<&mlua::Lua>) {
            self.called.store(true, Ordering::SeqCst);
        }
    }

    let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(component_registry.clone());

    world.register_system(TestSystem {
        called: called.clone(),
    });

    world.run_system("TestSystem", None).unwrap();
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_run_nonexistent_system_errors() {
    let component_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(component_registry.clone());
    let result = world.run_system("NoSuchSystem", None);
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
