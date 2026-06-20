use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::Value as JsonValue;

#[test]
fn test_wasm_world_reservation_flow() {
    let mut world = WasmWorld::new();

    // Create a stockpile entity
    let stockpile_eid = world.spawn_entity();
    let stockpile_data = serde_json::json!({"resources": {"iron_ore": 100.0}});
    world
        .set_component(
            stockpile_eid,
            "Stockpile",
            &serde_json::to_string(&stockpile_data).unwrap(),
        )
        .unwrap();

    // Verify stockpile was stored
    let stockpile_str = world.get_component(stockpile_eid, "Stockpile").unwrap();
    let stockpile_val: JsonValue = serde_json::from_str(&stockpile_str).unwrap();
    assert_eq!(
        stockpile_val["resources"]["iron_ore"], 100.0,
        "Stockpile should have iron_ore 100.0"
    );

    // Create a job entity
    let job_eid = world.spawn_entity();
    let job_data = serde_json::json!({
        "state": "pending",
        "resource_requirements": [{"kind": "iron_ore", "amount": 10}]
    });
    world
        .set_component(job_eid, "Job", &serde_json::to_string(&job_data).unwrap())
        .unwrap();

    // Verify job was stored
    let job_str = world.get_component(job_eid, "Job").unwrap();
    let job_val: JsonValue = serde_json::from_str(&job_str).unwrap();
    assert_eq!(job_val["state"], "pending", "Job state should be pending");
    assert!(
        job_val.get("resource_requirements").is_some(),
        "Job should have resource_requirements"
    );

    // Verify entity lists
    let stockpile_entities = world.get_entities_with_component("Stockpile");
    assert_eq!(
        stockpile_entities,
        vec![stockpile_eid],
        "Should find stockpile entity"
    );
    let job_entities = world.get_entities_with_component("Job");
    assert_eq!(job_entities, vec![job_eid], "Should find job entity");

    // Check: no reservations yet
    let reservations = world.get_job_resource_reservations(job_eid);
    assert!(
        reservations.is_none(),
        "Should have no reservations before reserve"
    );

    // Run reservation
    world.reserve_job_resources();

    // Check: should have reservations
    let reservations = world.get_job_resource_reservations(job_eid);
    assert!(
        reservations.is_some(),
        "Should have reservations after reserve"
    );
    let reservations_str = reservations.unwrap();
    let res_value: JsonValue = serde_json::from_str(&reservations_str).unwrap();
    assert!(
        res_value.is_array(),
        "Reserved resources should be an array"
    );
    assert!(
        !res_value.as_array().unwrap().is_empty(),
        "Reserved resources should not be empty"
    );

    // Release reservation
    world.release_job_resource_reservations(job_eid);

    // Check: reservations cleared
    let reservations = world.get_job_resource_reservations(job_eid);
    assert!(
        reservations.is_none(),
        "Should have no reservations after release"
    );
}

#[test]
fn test_wasm_world_reservation_insufficient_resources() {
    let mut world = WasmWorld::new();

    // Stockpile with insufficient resources
    let stockpile_eid = world.spawn_entity();
    let stockpile_data = serde_json::json!({"resources": {"iron_ore": 5.0}});
    world
        .set_component(
            stockpile_eid,
            "Stockpile",
            &serde_json::to_string(&stockpile_data).unwrap(),
        )
        .unwrap();

    // Job needs 10 iron_ore
    let job_eid = world.spawn_entity();
    let job_data = serde_json::json!({
        "state": "pending",
        "resource_requirements": [{"kind": "iron_ore", "amount": 10}]
    });
    world
        .set_component(job_eid, "Job", &serde_json::to_string(&job_data).unwrap())
        .unwrap();

    world.reserve_job_resources();

    // Should still have no reservations (insufficient resources)
    let reservations = world.get_job_resource_reservations(job_eid);
    assert!(
        reservations.is_none(),
        "Should have no reservations with insufficient resources"
    );
}
