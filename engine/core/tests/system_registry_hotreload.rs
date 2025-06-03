use engine_core::ecs::World;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::plugins::dynamic_systems::DynamicSystemRegistry;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[test]
fn test_system_register_update_unregister() {
    let mut registry = DynamicSystemRegistry::new();
    let ecs_registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let world_rc = Rc::new(RefCell::new(World::new(ecs_registry)));

    // Register initial system
    registry.register_system(
        "test".to_string(),
        Box::new(|_world_rc: Rc<RefCell<World>>, _dt: f32| {
            // Mark as called
            unsafe {
                CALLED = true;
            }
        }),
    );

    // Run the system
    unsafe {
        CALLED = false;
    }
    registry
        .run_system(Rc::clone(&world_rc), "test", 0.0)
        .unwrap();
    unsafe {
        assert!(CALLED);
    }

    // Hot-reload: update system implementation
    registry.register_system(
        "test".to_string(),
        Box::new(|_world_rc: Rc<RefCell<World>>, _dt: f32| unsafe {
            CALLED2 = true;
        }),
    );

    // Run again: should use the new system
    unsafe {
        CALLED2 = false;
    }
    registry
        .run_system(Rc::clone(&world_rc), "test", 0.0)
        .unwrap();
    unsafe {
        assert!(CALLED2);
    }

    // Unregister system
    let _ = registry.unregister_system("test");
    assert!(
        registry
            .run_system(Rc::clone(&world_rc), "test", 0.0)
            .is_err()
    );
}

// Use static mut for test state (not thread-safe, but fine for single-threaded test)
static mut CALLED: bool = false;
static mut CALLED2: bool = false;
