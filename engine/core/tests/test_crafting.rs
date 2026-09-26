//! Integration tests for the Crafting system (ROADMAP L57).
//!
//! Covers the M5 slice through the core `World` ops: tool gating (present,
//! consumed-once), material/input deduction, normative quality recomputation
//! from the same `(crafter_id, turn)` seed, skill gating with exact
//! deterministic XP, single Item+Material output with inventory fallback,
//! completion/cancel events and refund semantics, fixed validation order,
//! station reservation, stockpile-only rejection, tick progress through
//! `simulation_tick`, 50-tick two-world determinism, and save/load
//! round-trip. Ordering pin lives in `test_crafting_ordering.rs` and
//! `test_construction.rs`; the economic suite staying green proves
//! non-duplication.

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

#[path = "helpers/world_io.rs"]
mod world_io_helper;
use world_io_helper::save_and_load_roundtrip;

use engine_core::ecs::world::World;
use engine_core::systems::crafting::CraftingSystem;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use serde_json::{Value as JsonValue, json};
use std::cell::RefCell;
use std::rc::Rc;

const RECIPE_NAME: &str = "iron_sword";

/// Full-field craft recipe: duration 3, non-consumed hammer, iron
/// inputs+materials, crafting level 2 gate, item output, XP override 12.
fn sword_recipe() -> JsonValue {
    json!({
        "name": RECIPE_NAME,
        "inputs": [{"kind": "iron", "amount": 2}],
        "outputs": [],
        "duration": 3,
        "tools": [{"item": "hammer", "consumed": false}],
        "materials": [{"material": "iron", "amount": 2}],
        "required_skill": {"skill": "crafting", "level": 2},
        "output_item": {"id": "iron_sword", "name": "Iron Sword", "slot": "hand"},
        "station": null,
        "xp": 12
    })
}

/// Same shape with every gate field overridden by the caller.
fn custom_recipe(
    tools: JsonValue,
    materials: JsonValue,
    inputs: JsonValue,
    skill: Option<JsonValue>,
    station: JsonValue,
    outputs: JsonValue,
) -> JsonValue {
    json!({
        "name": RECIPE_NAME,
        "inputs": inputs,
        "outputs": outputs,
        "duration": 3,
        "tools": tools,
        "materials": materials,
        "required_skill": skill,
        "output_item": {"id": "iron_sword", "name": "Iron Sword", "slot": "hand"},
        "station": station,
        "xp": 12
    })
}

fn register_sword(world: &mut World) {
    world
        .register_craft_recipe(RECIPE_NAME.to_string(), sword_recipe())
        .unwrap();
}

/// Crafter with stocked iron, one hammer in a roomy inventory, crafting 2.
fn spawn_crafter(world: &mut World, iron: i64) -> u32 {
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Stockpile", json!({"resources": {"iron": iron}}))
        .unwrap();
    world
        .set_component(
            eid,
            "Inventory",
            json!({"slots": ["hammer"], "max_slots": 10, "weight": 0.0, "volume": 0.0}),
        )
        .unwrap();
    world
        .set_component(
            eid,
            "SkillLevels",
            json!({"skills": {"crafting": 2.0}, "total_xp": 0.0, "skill_xp": {}, "skill_levels": {}}),
        )
        .unwrap();
    eid
}

fn craft_world() -> World {
    let mut world = make_test_world();
    register_sword(&mut world);
    world
}

fn ticking_world(world: World) -> Rc<RefCell<World>> {
    let mut world = world;
    world.register_system(CraftingSystem);
    Rc::new(RefCell::new(world))
}

fn drain(world: &mut World, event_type: &str) -> Vec<JsonValue> {
    // `drain_events` reads the read buffer then swaps write→read, so a
    // second call surfaces events sent via direct op calls with no tick
    // since (tick-flushed events arrive on the first call). No explicit
    // `update_event_buses` here: `simulation_tick` already swaps at tick
    // end, and an extra swap would wipe the just-flushed read buffer when
    // the write side is empty.
    let mut events = world.drain_events::<JsonValue>(event_type);
    events.extend(world.drain_events::<JsonValue>(event_type));
    events
}

fn stockpile_iron(world: &World, eid: u32) -> i64 {
    world
        .get_component(eid, "Stockpile")
        .and_then(|s| s.get("resources"))
        .and_then(|r| r.get("iron"))
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .unwrap_or(-1)
}

/// Normative quality/XP recomputation from the SPEC formula: first stream
/// draw feeds quality, second feeds XP, seeded `(crafter_id, turn)`.
fn expected_quality_and_xp(
    crafter: u32,
    turn: u32,
    q_in: f64,
    skill: f64,
    base_xp: f64,
) -> (f64, i64) {
    let mut seed = [0u8; 32];
    seed[0..4].copy_from_slice(&crafter.to_le_bytes());
    seed[4..8].copy_from_slice(&turn.to_le_bytes());
    let mut rng = SmallRng::from_seed(seed);
    let quality = (q_in + 0.1 * skill + (rng.random::<f64>() - 0.5)).clamp(0.0, 10.0);
    let xp = ((base_xp + (rng.random::<f64>() - 0.5)).floor().max(1.0)) as i64;
    (quality, xp)
}

/// Start a craft and tick three times (the sword recipe duration); the third
/// tick completes while `world.turn` is 2 on a fresh world.
fn complete_one_craft(rc: &Rc<RefCell<World>>, crafter: u32) {
    rc.borrow_mut().start_craft(crafter, RECIPE_NAME).unwrap();
    for _ in 0..3 {
        World::tick(Rc::clone(rc));
    }
}

#[test]
fn rejects_craft_without_hammer_and_accepts_with_one() {
    let mut world = craft_world();
    let crafter = spawn_crafter(&mut world, 200);
    // Drop the hammer: the gate must name it.
    world
        .set_component(
            crafter,
            "Inventory",
            json!({"slots": [], "max_slots": 10, "weight": 0.0, "volume": 0.0}),
        )
        .unwrap();
    assert_eq!(
        world.can_craft(crafter, RECIPE_NAME).unwrap_err(),
        "missing_tool:hammer"
    );
    assert_eq!(
        world.start_craft(crafter, RECIPE_NAME).unwrap_err(),
        "missing_tool:hammer"
    );
    assert!(world.get_craft_state(crafter).is_none());

    // One hammer present satisfies the non-consumed tool requirement.
    world
        .set_component(
            crafter,
            "Inventory",
            json!({"slots": ["hammer"], "max_slots": 10, "weight": 0.0, "volume": 0.0}),
        )
        .unwrap();
    assert!(world.can_craft(crafter, RECIPE_NAME).is_ok());
    assert!(world.start_craft(crafter, RECIPE_NAME).is_ok());
    // Non-consumed tools are never removed.
    let slots = world.get_component(crafter, "Inventory").unwrap()["slots"].clone();
    assert_eq!(slots, json!(["hammer"]));
}

#[test]
fn removes_consumed_tool_exactly_once_at_start() {
    let mut world = craft_world();
    world
        .register_craft_recipe(
            RECIPE_NAME.to_string(),
            custom_recipe(
                json!([{"item": "hammer", "consumed": true}]),
                json!([]),
                json!([]),
                None,
                JsonValue::Null,
                json!([]),
            ),
        )
        .unwrap();
    let crafter = world.spawn_entity();
    world
        .set_component(
            crafter,
            "Inventory",
            json!({"slots": ["hammer", "hammer"], "max_slots": 10, "weight": 0.0, "volume": 0.0}),
        )
        .unwrap();
    world.start_craft(crafter, RECIPE_NAME).unwrap();
    let slots = world.get_component(crafter, "Inventory").unwrap()["slots"].clone();
    assert_eq!(slots, json!(["hammer"]));
    // Cancellation refunds stockpile inputs but never consumed tools.
    assert!(world.cancel_craft(crafter).unwrap());
    let slots = world.get_component(crafter, "Inventory").unwrap()["slots"].clone();
    assert_eq!(slots, json!(["hammer"]));
    assert!(world.get_craft_state(crafter).is_none());
}

#[test]
fn deducts_materials_at_start_and_matches_quality_formula() {
    let mut world = craft_world();
    let crafter = spawn_crafter(&mut world, 200);
    let rc = ticking_world(world);

    rc.borrow_mut().start_craft(crafter, RECIPE_NAME).unwrap();
    // Inputs (2) plus materials (2) leave from the same resource pool.
    assert_eq!(stockpile_iron(&rc.borrow(), crafter), 196);

    World::tick(Rc::clone(&rc));
    World::tick(Rc::clone(&rc));
    assert_eq!(
        rc.borrow().get_craft_state(crafter).unwrap()["progress"],
        json!(2)
    );
    World::tick(Rc::clone(&rc));

    // Completed on the third tick while turn was 2; fresh world turn is now 3.
    assert_eq!(rc.borrow().turn, 3);
    let (expected_quality, _) = expected_quality_and_xp(crafter, 2, 1.0, 2.0, 12.0);
    let state = rc.borrow().get_craft_state(crafter).unwrap().clone();
    assert_eq!(state["state"], json!("complete"));
    let output = state["output_entity"].as_u64().unwrap() as u32;
    let material = rc
        .borrow()
        .get_component(output, "Material")
        .unwrap()
        .clone();
    assert_eq!(material["material"], json!("iron"));
    assert_eq!(material["quality"], json!(expected_quality));
}

#[test]
fn gates_skill_level_and_awards_deterministic_xp() {
    let mut world = craft_world();
    let crafter = spawn_crafter(&mut world, 200);
    // Below the level-2 gate: rejection names the skill shortfall.
    world
        .set_component(
            crafter,
            "SkillLevels",
            json!({"skills": {"crafting": 1.0}, "total_xp": 0.0, "skill_xp": {}, "skill_levels": {}}),
        )
        .unwrap();
    assert_eq!(
        world.can_craft(crafter, RECIPE_NAME).unwrap_err(),
        "insufficient_skill"
    );
    // At and above the gate: success.
    for level in [2.0, 5.0] {
        world
            .set_component(
                crafter,
                "SkillLevels",
                json!({"skills": {"crafting": level}, "total_xp": 0.0, "skill_xp": {}, "skill_levels": {}}),
            )
            .unwrap();
        assert!(world.can_craft(crafter, RECIPE_NAME).is_ok());
    }

    let rc = ticking_world(world);
    complete_one_craft(&rc, crafter);
    let (_, expected_xp) = expected_quality_and_xp(crafter, 2, 1.0, 5.0, 12.0);
    let mut world = rc.borrow_mut();
    let events = drain(&mut world, "craft_completed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["xp_gained"], json!(expected_xp));
    let levels = world.get_component(crafter, "SkillLevels").unwrap().clone();
    assert_eq!(levels["total_xp"], json!(expected_xp as f64));
    assert_eq!(levels["skill_xp"]["crafting"], json!(expected_xp as f64));

    // Identical rerun earns the identical amount: XP is deterministic.
    let mut rerun = craft_world();
    let rerun_crafter = spawn_crafter(&mut rerun, 200);
    rerun
        .set_component(
            rerun_crafter,
            "SkillLevels",
            json!({"skills": {"crafting": 5.0}, "total_xp": 0.0, "skill_xp": {}, "skill_levels": {}}),
        )
        .unwrap();
    let rerun_rc = ticking_world(rerun);
    // Same spawn order keeps entity ids aligned across the two worlds.
    assert_eq!(rerun_crafter, crafter);
    complete_one_craft(&rerun_rc, rerun_crafter);
    let rerun_events = drain(&mut rerun_rc.borrow_mut(), "craft_completed");
    assert_eq!(rerun_events[0]["xp_gained"], json!(expected_xp));
}

#[test]
fn spawns_single_item_entity_into_inventory_with_matching_state() {
    let mut world = craft_world();
    let crafter = spawn_crafter(&mut world, 200);
    let before: Vec<u32> = {
        let mut ids = world.get_entities();
        ids.sort_unstable();
        ids
    };
    let rc = ticking_world(world);
    complete_one_craft(&rc, crafter);

    let world = rc.borrow();
    let mut after = world.get_entities();
    after.sort_unstable();
    assert_eq!(after.len(), before.len() + 1);
    let output = *after.iter().find(|id| !before.contains(id)).unwrap();

    let item = world.get_component(output, "Item").unwrap().clone();
    // Field-wise: `set_component` merges Item schema defaults
    // (`two_handed`, `requirements`, `effects`), so whole-object equality
    // would couple the test to the schema; assert the contractual fields.
    assert_eq!(item["id"], json!("iron_sword"));
    assert_eq!(item["name"], json!("Iron Sword"));
    assert_eq!(item["slot"], json!("hand"));
    assert_eq!(item["material"], json!("iron"));
    assert!(world.get_component(output, "Material").is_some());
    // Roomy inventory receives the output id through the normal write.
    let slots = world.get_component(crafter, "Inventory").unwrap()["slots"].clone();
    assert!(slots.as_array().unwrap().contains(&json!("iron_sword")));
    // State and event agree on the spawned entity.
    assert_eq!(
        world.get_craft_state(crafter).unwrap()["output_entity"],
        json!(output)
    );
}

#[test]
fn leaves_output_as_world_entity_when_inventory_full() {
    let mut world = craft_world();
    let crafter = spawn_crafter(&mut world, 200);
    // The hammer fills the single slot: gate passes, placement falls back.
    world
        .set_component(
            crafter,
            "Inventory",
            json!({"slots": ["hammer"], "max_slots": 1, "weight": 0.0, "volume": 0.0}),
        )
        .unwrap();
    let rc = ticking_world(world);
    complete_one_craft(&rc, crafter);

    let mut world = rc.borrow_mut();
    let events = drain(&mut world, "craft_completed");
    assert_eq!(events.len(), 1);
    let output = events[0]["output_entity"].as_u64().unwrap() as u32;
    assert!(world.entity_exists(output));
    assert!(world.get_component(output, "Item").is_some());
    let slots = world.get_component(crafter, "Inventory").unwrap()["slots"].clone();
    assert_eq!(slots, json!(["hammer"]));
}

#[test]
fn emits_completion_event_and_refunds_on_cancel() {
    let mut world = craft_world();
    let crafter = spawn_crafter(&mut world, 200);
    let rc = ticking_world(world);
    complete_one_craft(&rc, crafter);

    let mut world = rc.borrow_mut();
    let events = drain(&mut world, "craft_completed");
    assert_eq!(events.len(), 1);
    let payload = &events[0];
    for key in [
        "entity",
        "recipe",
        "output_entity",
        "output_item",
        "material",
        "quality",
        "xp_gained",
    ] {
        assert!(
            payload.get(key).is_some(),
            "craft_completed must carry {key}"
        );
    }
    assert_eq!(payload["entity"], json!(crafter));
    assert_eq!(payload["recipe"], json!(RECIPE_NAME));
    assert_eq!(payload["material"], json!("iron"));
    assert_eq!(
        payload["output_item"],
        json!({"id": "iron_sword", "name": "Iron Sword", "slot": "hand"})
    );

    // Completed orders are terminal: cancelling reports no order.
    assert_eq!(world.cancel_craft(crafter).unwrap_err(), "no_craft_order");

    // Fresh order cancelled mid-progress refunds stockpile, not tools.
    world.remove_component(crafter, "CraftOrder").unwrap();
    world
        .set_component(crafter, "Stockpile", json!({"resources": {"iron": 200}}))
        .unwrap();
    world.start_craft(crafter, RECIPE_NAME).unwrap();
    assert_eq!(stockpile_iron(&world, crafter), 196);
    assert!(world.cancel_craft(crafter).unwrap());
    assert_eq!(stockpile_iron(&world, crafter), 200);
    assert!(world.get_craft_state(crafter).is_none());
    let cancelled = drain(&mut world, "craft_cancelled");
    assert_eq!(cancelled.len(), 1);
    assert_eq!(
        cancelled[0],
        json!({"entity": crafter, "recipe": RECIPE_NAME, "refunded": true})
    );
    // Cancelling twice reports no order.
    assert_eq!(world.cancel_craft(crafter).unwrap_err(), "no_craft_order");
}

#[test]
fn rejects_double_start_unknown_recipe_and_completed_cancel() {
    let mut world = craft_world();
    let crafter = spawn_crafter(&mut world, 200);
    world.start_craft(crafter, RECIPE_NAME).unwrap();
    assert_eq!(
        world.start_craft(crafter, RECIPE_NAME).unwrap_err(),
        "already_crafting"
    );
    assert_eq!(
        world.get_craft_state(crafter).unwrap()["progress"],
        json!(0)
    );
    assert_eq!(
        world.can_craft(crafter, "ghost_recipe").unwrap_err(),
        "unknown_recipe"
    );
    assert_eq!(
        world.start_craft(crafter, "ghost_recipe").unwrap_err(),
        "unknown_recipe"
    );
    // Unknown recipes mutate nothing and emit nothing.
    let mut world = world;
    assert!(drain(&mut world, "craft_completed").is_empty());
    assert!(drain(&mut world, "craft_cancelled").is_empty());
}

#[test]
fn validates_inputs_before_materials_tools_skill_and_ignores_station() {
    let mut world = make_test_world();
    world
        .register_craft_recipe(
            RECIPE_NAME.to_string(),
            custom_recipe(
                json!([{"item": "hammer", "consumed": false}]),
                json!([{"material": "iron", "amount": 2}]),
                json!([{"kind": "coal", "amount": 1}]),
                Some(json!({"skill": "crafting", "level": 2})),
                json!("bogus_workbench"),
                json!([]),
            ),
        )
        .unwrap();
    // Bare crafter fails the input check first.
    let crafter = world.spawn_entity();
    assert_eq!(
        world.can_craft(crafter, RECIPE_NAME).unwrap_err(),
        "missing_input:coal"
    );
    world
        .set_component(crafter, "Stockpile", json!({"resources": {"coal": 1}}))
        .unwrap();
    assert_eq!(
        world.can_craft(crafter, RECIPE_NAME).unwrap_err(),
        "missing_material:iron"
    );
    world
        .set_component(
            crafter,
            "Stockpile",
            json!({"resources": {"coal": 1, "iron": 2}}),
        )
        .unwrap();
    assert_eq!(
        world.can_craft(crafter, RECIPE_NAME).unwrap_err(),
        "missing_tool:hammer"
    );
    world
        .set_component(
            crafter,
            "Inventory",
            json!({"slots": ["hammer"], "max_slots": 10, "weight": 0.0, "volume": 0.0}),
        )
        .unwrap();
    assert_eq!(
        world.can_craft(crafter, RECIPE_NAME).unwrap_err(),
        "insufficient_skill"
    );
    world
        .set_component(
            crafter,
            "SkillLevels",
            json!({"skills": {"crafting": 2.0}, "total_xp": 0.0, "skill_xp": {}, "skill_levels": {}}),
        )
        .unwrap();
    // The reserved station value never gates: bogus workbench still succeeds.
    assert!(world.can_craft(crafter, RECIPE_NAME).is_ok());
    assert!(world.start_craft(crafter, RECIPE_NAME).is_ok());
}

#[test]
fn rejects_stockpile_only_recipes_on_craft_path() {
    let mut world = make_test_world();
    world
        .register_craft_recipe(
            "plank_batch".to_string(),
            json!({
                "name": "plank_batch",
                "inputs": [{"kind": "wood", "amount": 1}],
                "outputs": [{"kind": "plank", "amount": 2}],
                "duration": 2
            }),
        )
        .unwrap();
    // Stored but invisible to the craft path and its listing.
    assert!(world.list_craft_recipes().is_empty());
    let crafter = world.spawn_entity();
    world
        .set_component(crafter, "Stockpile", json!({"resources": {"wood": 5}}))
        .unwrap();
    assert_eq!(
        world.can_craft(crafter, "plank_batch").unwrap_err(),
        "unknown_recipe"
    );
    assert_eq!(
        world.start_craft(crafter, "plank_batch").unwrap_err(),
        "unknown_recipe"
    );
    assert!(world.get_craft_state(crafter).is_none());
}

#[test]
fn advances_craft_through_simulation_tick() {
    let mut world = craft_world();
    let crafter = spawn_crafter(&mut world, 200);
    let rc = ticking_world(world);
    rc.borrow_mut().start_craft(crafter, RECIPE_NAME).unwrap();
    World::tick(Rc::clone(&rc));
    assert_eq!(
        rc.borrow().get_craft_state(crafter).unwrap()["progress"],
        json!(1)
    );
    assert_eq!(rc.borrow().turn, 1);
}

#[test]
fn never_writes_stockpile_outputs_for_item_recipes() {
    let mut world = make_test_world();
    // Outputs listed on an item recipe must not leak into stockpile counts.
    world
        .register_craft_recipe(
            RECIPE_NAME.to_string(),
            custom_recipe(
                json!([{"item": "hammer", "consumed": false}]),
                json!([]),
                json!([]),
                None,
                JsonValue::Null,
                json!([{"kind": "scrap", "amount": 5}]),
            ),
        )
        .unwrap();
    let crafter = world.spawn_entity();
    world
        .set_component(
            crafter,
            "Inventory",
            json!({"slots": ["hammer"], "max_slots": 10, "weight": 0.0, "volume": 0.0}),
        )
        .unwrap();
    let rc = ticking_world(world);
    complete_one_craft(&rc, crafter);
    // Scoped borrow: the `stockpile` reference keeps the guard alive through
    // the check, so the mutable drain borrow must come after the scope ends.
    {
        let world = rc.borrow();
        if let Some(stockpile) = world.get_component(crafter, "Stockpile") {
            assert!(
                stockpile
                    .get("resources")
                    .and_then(|r| r.as_object())
                    .is_none_or(|map| !map.contains_key("scrap")),
                "item recipes must not write stockpile outputs, got {stockpile}"
            );
        }
    }
    assert_eq!(drain(&mut rc.borrow_mut(), "craft_completed").len(), 1);
}

type CraftSnapshot = String;

fn snapshot_craft(world: &mut World) -> CraftSnapshot {
    let mut ids = world.get_entities();
    ids.sort_unstable();
    let mut parts: Vec<JsonValue> = Vec::new();
    for eid in ids {
        let mut entry = json!({"entity": eid});
        for name in [
            "Stockpile",
            "CraftOrder",
            "Item",
            "Material",
            "SkillLevels",
            "Inventory",
        ] {
            if let Some(value) = world.get_component(eid, name) {
                entry[name] = value.clone();
            }
        }
        parts.push(entry);
    }
    let events = world
        .drain_events::<JsonValue>("craft_completed")
        .into_iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>();
    json!({"turn": world.turn, "entities": parts, "events": events}).to_string()
}

fn deterministic_craft_world() -> (Rc<RefCell<World>>, u32) {
    let mut world = craft_world();
    let crafter = spawn_crafter(&mut world, 200);
    (ticking_world(world), crafter)
}

#[test]
fn two_worlds_match_over_fifty_ticks() {
    let (world_a, crafter_a) = deterministic_craft_world();
    let (world_b, crafter_b) = deterministic_craft_world();
    assert_eq!(crafter_a, crafter_b);

    for tick in 0..50 {
        for (rc, crafter) in [(&world_a, crafter_a), (&world_b, crafter_b)] {
            let needs_start = {
                let world = rc.borrow();
                world.get_craft_state(crafter).and_then(|order| {
                    order
                        .get("state")
                        .and_then(|s| s.as_str())
                        .map(str::to_string)
                }) != Some("in_progress".to_string())
            };
            if needs_start {
                rc.borrow_mut().start_craft(crafter, RECIPE_NAME).unwrap();
            }
            World::tick(Rc::clone(rc));
        }
        let snapshot_a = snapshot_craft(&mut world_a.borrow_mut());
        let snapshot_b = snapshot_craft(&mut world_b.borrow_mut());
        assert_eq!(
            snapshot_a, snapshot_b,
            "craft state diverged on tick {tick}"
        );
        assert_eq!(
            world_a.borrow().turn,
            world_b.borrow().turn,
            "turn diverged on tick {tick}"
        );
    }
}

#[test]
fn save_load_roundtrip_preserves_recipes_orders_entities_xp() {
    let mut world = craft_world();
    let crafter = spawn_crafter(&mut world, 200);
    let rc = ticking_world(world);
    rc.borrow_mut().start_craft(crafter, RECIPE_NAME).unwrap();
    World::tick(Rc::clone(&rc));

    let live_order = rc.borrow().get_craft_state(crafter).unwrap().clone();
    assert_eq!(live_order["progress"], json!(1));
    let registry = rc.borrow().registry.clone();
    let mut loaded = save_and_load_roundtrip(&rc.borrow(), registry);

    // Recipes, in-progress order, stockpile, and XP survive the trip.
    assert_eq!(loaded.list_craft_recipes(), vec![RECIPE_NAME.to_string()]);
    let loaded_order = loaded.get_craft_state(crafter).unwrap();
    assert_eq!(loaded_order, live_order);
    assert_eq!(stockpile_iron(&loaded, crafter), 196);
    assert_eq!(
        loaded.get_component(crafter, "SkillLevels").unwrap()["total_xp"],
        json!(0.0)
    );

    // The loaded world ticks to completion with identical payloads.
    loaded.register_system(CraftingSystem);
    let loaded_rc = Rc::new(RefCell::new(loaded));
    World::tick(Rc::clone(&loaded_rc));
    World::tick(Rc::clone(&loaded_rc));
    let (expected_quality, expected_xp) = expected_quality_and_xp(crafter, 2, 1.0, 2.0, 12.0);
    let mut loaded = loaded_rc.borrow_mut();
    let events = drain(&mut loaded, "craft_completed");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["xp_gained"], json!(expected_xp));
    let output = events[0]["output_entity"].as_u64().unwrap() as u32;
    assert_eq!(
        loaded.get_component(output, "Material").unwrap()["quality"],
        json!(expected_quality)
    );

    // Completed orders and their output entities survive a second trip.
    let registry = loaded.registry.clone();
    let reloaded = save_and_load_roundtrip(&loaded, registry);
    assert_eq!(
        reloaded.get_craft_state(crafter).unwrap()["state"],
        json!("complete")
    );
    assert_eq!(
        reloaded.get_craft_state(crafter).unwrap()["output_entity"],
        json!(output)
    );
    assert_eq!(
        reloaded.get_component(output, "Item").unwrap()["id"],
        json!("iron_sword")
    );
    assert_eq!(
        reloaded.get_component(crafter, "SkillLevels").unwrap()["total_xp"],
        json!(expected_xp as f64)
    );
}
