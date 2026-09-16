use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::Value as JsonValue;

fn spawn_stockpile(world: &mut WasmWorld, resources: JsonValue) -> u32 {
    let eid = world.spawn_entity();
    let data = serde_json::json!({ "resources": resources });
    world
        .set_component(eid, "Stockpile", &serde_json::to_string(&data).unwrap())
        .unwrap();
    eid
}

fn spawn_pending_job(world: &mut WasmWorld, requirements: JsonValue) -> u32 {
    let eid = world.spawn_entity();
    let data = serde_json::json!({
        "state": "pending",
        "resource_requirements": requirements,
    });
    world
        .set_component(eid, "Job", &serde_json::to_string(&data).unwrap())
        .unwrap();
    eid
}

#[test]
fn test_float_stockpile_satisfies_integer_requirement() {
    let mut world = WasmWorld::new();
    spawn_stockpile(&mut world, serde_json::json!({ "iron_ore": 100.0 }));
    let job_eid = spawn_pending_job(
        &mut world,
        serde_json::json!([{ "kind": "iron_ore", "amount": 10 }]),
    );

    world.reserve_job_resources();

    let reservations = world.get_job_resource_reservations(job_eid);
    assert!(
        reservations.is_some(),
        "Float stockpile 100.0 should reserve for integer requirement 10"
    );
    let value: JsonValue = serde_json::from_str(&reservations.unwrap()).unwrap();
    assert!(
        value.as_array().is_some_and(|a| !a.is_empty()),
        "Reserved resources should be a non-empty array"
    );
}

#[test]
fn test_fractional_stockpile_does_not_round_up() {
    let mut world = WasmWorld::new();
    spawn_stockpile(&mut world, serde_json::json!({ "iron_ore": 9.7 }));
    let job_eid = spawn_pending_job(
        &mut world,
        serde_json::json!([{ "kind": "iron_ore", "amount": 10 }]),
    );

    world.reserve_job_resources();

    assert!(
        world.get_job_resource_reservations(job_eid).is_none(),
        "Stockpile 9.7 truncates to 9 and must not satisfy requirement 10"
    );
}

#[test]
fn test_negative_and_string_stockpiles_do_not_reserve() {
    let mut world = WasmWorld::new();
    spawn_stockpile(
        &mut world,
        serde_json::json!({ "iron_ore": -5, "copper_ore": "10" }),
    );
    let job_eid = spawn_pending_job(
        &mut world,
        serde_json::json!([{ "kind": "iron_ore", "amount": 10 }]),
    );
    let job2_eid = spawn_pending_job(
        &mut world,
        serde_json::json!([{ "kind": "copper_ore", "amount": 10 }]),
    );

    world.reserve_job_resources();

    assert!(
        world.get_job_resource_reservations(job_eid).is_none(),
        "Negative stockpile must read as zero and not reserve"
    );
    assert!(
        world.get_job_resource_reservations(job2_eid).is_none(),
        "String stockpile must read as zero and not reserve"
    );
}
