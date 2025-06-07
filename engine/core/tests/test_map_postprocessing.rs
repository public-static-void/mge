use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::world::World;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn test_postprocessor_runs_after_map_application() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    // Flag to check if hook was called
    let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let called_clone = called.clone();

    world.register_map_postprocessor(move |_world| {
        called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    });

    let map_json = json!({
        "topology": "square",
        "cells": [
            { "x": 0, "y": 0, "z": 0, "biome": "Default", "terrain": "default", "neighbors": [] }
        ]
    });

    world.apply_generated_map(&map_json).expect("map applied");
    assert!(
        called.load(std::sync::atomic::Ordering::SeqCst),
        "postprocessor was called"
    );
}

#[test]
fn test_postprocessor_error_causes_failure() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    world.register_map_postprocessor(|_world| Err("validation failed".to_string()));

    let map_json = json!({
        "topology": "square",
        "cells": [
            { "x": 0, "y": 0, "z": 0, "biome": "Default", "terrain": "default", "neighbors": [] }
        ]
    });

    let result = world.apply_generated_map(&map_json);
    assert!(
        result.is_err(),
        "apply_generated_map should fail if postprocessor returns Err"
    );
    assert!(result.unwrap_err().contains("validation failed"));
}

#[test]
fn test_multiple_postprocessors_all_run() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    let call_order = Arc::new(Mutex::new(vec![]));
    let order1 = call_order.clone();
    let order2 = call_order.clone();

    world.register_map_postprocessor(move |_world| {
        order1.lock().unwrap().push(1);
        Ok(())
    });
    world.register_map_postprocessor(move |_world| {
        order2.lock().unwrap().push(2);
        Ok(())
    });

    let map_json = json!({
        "topology": "square",
        "cells": [
            { "x": 0, "y": 0, "z": 0, "biome": "Default", "terrain": "default", "neighbors": [] }
        ]
    });

    world.apply_generated_map(&map_json).expect("map applied");
    let order = call_order.lock().unwrap().clone();
    assert_eq!(order, vec![1, 2], "both postprocessors ran in order");
}

#[test]
fn test_postprocessor_can_mutate_world() {
    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    let mut world = World::new(registry);

    world.register_map_postprocessor(|world| {
        world.current_mode = "postprocessed".to_string();
        Ok(())
    });

    let map_json = json!({
        "topology": "square",
        "cells": [
            { "x": 0, "y": 0, "z": 0, "biome": "Default", "terrain": "default", "neighbors": [] }
        ]
    });

    world.apply_generated_map(&map_json).expect("map applied");
    assert_eq!(world.current_mode, "postprocessed");
}
