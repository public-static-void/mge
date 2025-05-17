use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::system::System;
use engine_core::scripting::world::World;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};

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
