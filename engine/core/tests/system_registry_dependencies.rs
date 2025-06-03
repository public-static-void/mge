use engine_core::ecs::World;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::plugins::dynamic_systems::DynamicSystemRegistry;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[test]
fn test_system_dependency_ordering() {
    let mut registry = DynamicSystemRegistry::new();
    let world_rc = Rc::new(RefCell::new(World::new(Arc::new(Mutex::new(
        ComponentRegistry::new(),
    )))));

    let order = Arc::new(Mutex::new(Vec::new()));

    // System A (no dependencies)
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

    // System B (depends on A)
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

    // System C (depends on B)
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

    // Run all systems
    registry.run_all_systems(Rc::clone(&world_rc), 0.0).unwrap();

    let result = order.lock().unwrap().clone();
    assert_eq!(result, vec!["A", "B", "C"]);

    // Add a cycle: D depends on C, C depends on D
    registry.register_system_with_deps("D".to_string(), vec!["C".to_string()], Box::new(|_, _| {}));
    // Now update C to depend on D as well, forming a cycle
    registry
        .update_system_dependencies("C", vec!["B".to_string(), "D".to_string()])
        .unwrap();

    let run_result = registry.run_all_systems(Rc::clone(&world_rc), 0.0);
    assert!(run_result.is_err());
}
