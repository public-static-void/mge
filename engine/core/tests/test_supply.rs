//! Standing suite for the supply and logistics network MVP.
//!
//! Created in M1 alongside the `SupplyLink` lifecycle, the core `SupplySystem`
//! tick, the `SYSTEM_EXECUTION_ORDER` slot, and all-harness registration.
//! Grows milestone by milestone (M2 war/path gates, determinism, save/load
//! extend in place).

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

#[path = "helpers/world_io.rs"]
mod world_io_helper;
use world_io_helper::save_and_load_roundtrip;

use engine_core::diplomacy::{declare_peace, declare_war};
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::faction::set_faction;
use engine_core::map::{Map, SquareGridMap};
use engine_core::supply::{
    SupplyError, SupplyLink, create_supply_link, get_supply_link, list_supply_links,
    remove_supply_link, set_supply_link_active,
};
use engine_core::systems::order_systems;
use engine_core::systems::supply::SupplySystem;
use engine_core::systems::{SYSTEM_EXECUTION_ORDER, consumption::ConsumptionSystem};
use serde_json::{Value as JsonValue, json};
use std::cell::RefCell;
use std::rc::Rc;

/// Spawn an entity carrying a Stockpile with the given resources map.
fn spawn_stockpile(world: &mut World, resources: serde_json::Value) -> u32 {
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Stockpile", json!({ "resources": resources }))
        .unwrap();
    eid
}

/// Read one resource balance from an entity's Stockpile (missing entry = 0.0).
fn stockpile_amount(world: &World, entity: u32, kind: &str) -> f64 {
    world
        .get_component(entity, "Stockpile")
        .and_then(|s| s.get("resources"))
        .and_then(|r| r.get(kind))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}

/// Snapshot an entity's full Stockpile value for byte-identical comparison.
fn stockpile_snapshot(world: &World, entity: u32) -> serde_json::Value {
    world
        .get_component(entity, "Stockpile")
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

/// Drain all events of the given type emitted so far.
///
/// Callers sync once with [`sync_events`] before draining each type:
/// every sync swaps the double buffers, so syncing between drains would
/// discard events still waiting on other buses.
fn drain_events(world: &mut World, event_type: &str) -> Vec<JsonValue> {
    world.take_events(event_type)
}

/// Make emitted events visible to readers (single buffer swap per phase).
fn sync_events(world: &mut World) {
    world.update_event_buses::<JsonValue>();
}

/// Run one supply tick on the world.
fn run_supply(world: &mut World) {
    SupplySystem.run(world);
}

/// A fresh link between distinct stockpiles stores its record verbatim.
#[test]
fn stores_link_record_verbatim_between_distinct_stockpiles() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));

    let link = create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();

    assert_eq!(
        get_supply_link(&world, link),
        Some(SupplyLink {
            source,
            target,
            kind: "grain".to_string(),
            amount_per_tick: 10.0,
            capacity_per_tick: 25.0,
            active: true,
        })
    );
    assert_eq!(list_supply_links(&world), vec![link]);
}

/// Self-links, empty kinds, bad amounts, and bare endpoints fail cleanly.
#[test]
fn rejects_bad_links_without_leaving_link_entities_behind() {
    let mut world = make_test_world();
    let stocked = spawn_stockpile(&mut world, json!({ "grain": 50.0 }));
    let other = spawn_stockpile(&mut world, json!({ "grain": 50.0 }));
    let bare = world.spawn_entity();
    let entities_before = world.get_entities().len();

    assert_eq!(
        create_supply_link(&mut world, stocked, stocked, "grain", 10.0, 10.0).unwrap_err(),
        SupplyError::SameEndpoint
    );
    assert_eq!(
        create_supply_link(&mut world, stocked, other, "", 10.0, 10.0).unwrap_err(),
        SupplyError::UnknownKind(String::new())
    );
    for bad in [0.0, -5.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            create_supply_link(&mut world, stocked, other, "grain", bad, 10.0).unwrap_err(),
            SupplyError::NonPositiveAmount,
            "amount {bad} must be rejected"
        );
        assert_eq!(
            create_supply_link(&mut world, stocked, other, "grain", 10.0, bad).unwrap_err(),
            SupplyError::NonPositiveAmount,
            "capacity {bad} must be rejected"
        );
    }
    assert_eq!(
        create_supply_link(&mut world, bare, other, "grain", 10.0, 10.0).unwrap_err(),
        SupplyError::NoStockpile(bare)
    );
    assert_eq!(
        create_supply_link(&mut world, stocked, bare, "grain", 10.0, 10.0).unwrap_err(),
        SupplyError::NoStockpile(bare)
    );

    assert_eq!(world.get_entities().len(), entities_before);
    assert!(list_supply_links(&world).is_empty());
}

/// One tick moves the requested amount and reports the delivery.
#[test]
fn moves_requested_amount_per_tick_and_reports_delivery() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    let link = create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();

    run_supply(&mut world);

    assert_eq!(stockpile_amount(&world, source, "grain"), 90.0);
    assert_eq!(stockpile_amount(&world, target, "grain"), 10.0);
    sync_events(&mut world);
    let delivered = drain_events(&mut world, "supply_delivered");
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0]["link"].as_u64(), Some(link as u64));
    assert_eq!(delivered[0]["kind"].as_str(), Some("grain"));
    assert_eq!(delivered[0]["amount"].as_f64(), Some(10.0));
    assert_eq!(delivered[0]["turn"].as_u64(), Some(0));
    assert!(drain_events(&mut world, "supply_shortfall").is_empty());
    assert!(drain_events(&mut world, "supply_blocked").is_empty());
}

/// The capacity cap binds when the requested amount exceeds it.
#[test]
fn delivers_capacity_ceiling_when_request_exceeds_it() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    create_supply_link(&mut world, source, target, "grain", 30.0, 12.0).unwrap();

    run_supply(&mut world);

    assert_eq!(stockpile_amount(&world, source, "grain"), 88.0);
    assert_eq!(stockpile_amount(&world, target, "grain"), 12.0);
    sync_events(&mut world);
    let delivered = drain_events(&mut world, "supply_delivered");
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0]["amount"].as_f64(), Some(12.0));
}

/// A shortfall moves nothing, keeps both stockpiles identical, and is reported.
#[test]
fn leaves_stockpiles_identical_and_reports_shortfall_when_source_is_short() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 5.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 20.0 }));
    let link = create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();
    let before_source = stockpile_snapshot(&world, source);
    let before_target = stockpile_snapshot(&world, target);

    run_supply(&mut world);

    assert_eq!(stockpile_snapshot(&world, source), before_source);
    assert_eq!(stockpile_snapshot(&world, target), before_target);
    sync_events(&mut world);
    assert!(drain_events(&mut world, "supply_delivered").is_empty());
    let shortfalls = drain_events(&mut world, "supply_shortfall");
    assert_eq!(shortfalls.len(), 1);
    assert_eq!(shortfalls[0]["link"].as_u64(), Some(link as u64));
    assert_eq!(shortfalls[0]["kind"].as_str(), Some("grain"));
    assert_eq!(shortfalls[0]["requested"].as_f64(), Some(10.0));
    assert_eq!(shortfalls[0]["available"].as_f64(), Some(5.0));
}

/// Inactive links transfer nothing and stay silent until reactivated.
#[test]
fn skips_inactive_links_silently_and_resumes_on_reactivation() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    let link = create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();
    set_supply_link_active(&mut world, link, false).unwrap();

    run_supply(&mut world);

    assert_eq!(stockpile_amount(&world, source, "grain"), 100.0);
    assert_eq!(stockpile_amount(&world, target, "grain"), 0.0);
    sync_events(&mut world);
    assert!(drain_events(&mut world, "supply_delivered").is_empty());
    assert!(drain_events(&mut world, "supply_shortfall").is_empty());
    assert!(drain_events(&mut world, "supply_blocked").is_empty());

    set_supply_link_active(&mut world, link, true).unwrap();
    run_supply(&mut world);

    assert_eq!(stockpile_amount(&world, source, "grain"), 90.0);
    assert_eq!(stockpile_amount(&world, target, "grain"), 10.0);
    sync_events(&mut world);
    assert_eq!(drain_events(&mut world, "supply_delivered").len(), 1);
}

/// Endpoints that lose their stockpile block the leg with a reason.
#[test]
fn blocks_legs_whose_endpoint_lost_its_stockpile() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    let link = create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();
    world.remove_component(target, "Stockpile").unwrap();
    let before_source = stockpile_snapshot(&world, source);

    run_supply(&mut world);

    assert_eq!(stockpile_snapshot(&world, source), before_source);
    sync_events(&mut world);
    assert!(drain_events(&mut world, "supply_delivered").is_empty());
    let blocked = drain_events(&mut world, "supply_blocked");
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0]["link"].as_u64(), Some(link as u64));
    assert_eq!(blocked[0]["reason"].as_str(), Some("missing_stockpile"));
}

/// A despawned endpoint blocks the leg the same way a stripped one does.
#[test]
fn blocks_legs_whose_endpoint_was_despawned() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();
    world.despawn_entity(target);

    run_supply(&mut world);

    assert_eq!(stockpile_amount(&world, source, "grain"), 100.0);
    sync_events(&mut world);
    let blocked = drain_events(&mut world, "supply_blocked");
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0]["reason"].as_str(), Some("missing_stockpile"));
}

/// Link removal and unknown-link errors round-trip through the helpers.
#[test]
fn removes_links_and_rejects_unknown_link_ids() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 10.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    let link = create_supply_link(&mut world, source, target, "grain", 5.0, 5.0).unwrap();

    remove_supply_link(&mut world, link).unwrap();

    assert_eq!(get_supply_link(&world, link), None);
    assert!(list_supply_links(&world).is_empty());
    assert_eq!(
        remove_supply_link(&mut world, link).unwrap_err(),
        SupplyError::UnknownLink(link)
    );
    assert_eq!(
        set_supply_link_active(&mut world, link, false).unwrap_err(),
        SupplyError::UnknownLink(link)
    );
    assert_eq!(get_supply_link(&world, 999_999), None);
}

/// The supply slot sits immediately after consumption and before crafting.
#[test]
fn pins_supply_between_consumption_and_crafting_in_execution_order() {
    let position = |name: &str| {
        SYSTEM_EXECUTION_ORDER
            .iter()
            .position(|entry| *entry == name)
            .unwrap_or_else(|| panic!("{name} must be pinned in SYSTEM_EXECUTION_ORDER"))
    };
    let consumption = position("ConsumptionSystem");
    let supply = position("SupplySystem");
    let crafting = position("CraftingSystem");
    assert_eq!(supply, consumption + 1);
    assert_eq!(crafting, supply + 1);

    let ordered = order_systems(&[
        "CraftingSystem".to_string(),
        "SupplySystem".to_string(),
        "ConsumptionSystem".to_string(),
    ]);
    assert_eq!(
        ordered,
        vec![
            "ConsumptionSystem".to_string(),
            "SupplySystem".to_string(),
            "CraftingSystem".to_string(),
        ]
    );
    let checker = SupplySystem;
    assert_eq!(checker.name(), "SupplySystem");
    assert_eq!(checker.dependencies(), &["ConsumptionSystem"]);
    let consumption_checker = ConsumptionSystem;
    assert_eq!(consumption_checker.name(), "ConsumptionSystem");
}

/// A registered supply system delivers on a plain world tick.
#[test]
fn delivers_on_world_tick_once_registered() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();
    world.register_system(SupplySystem);
    assert!(world.has_system("SupplySystem"));
    let world_rc = Rc::new(RefCell::new(world));

    World::tick(Rc::clone(&world_rc));

    let world = world_rc.borrow();
    assert_eq!(stockpile_amount(&world, source, "grain"), 90.0);
    assert_eq!(stockpile_amount(&world, target, "grain"), 10.0);
    drop(world);
    let mut world = Rc::try_unwrap(world_rc).ok().unwrap().into_inner();
    // No sync here: World::tick already swapped the buffers at end of tick.
    let delivered = drain_events(&mut world, "supply_delivered");
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0]["amount"].as_f64(), Some(10.0));
}

/// The WASM host mirror delivers on tick with the same events.
#[test]
fn wasm_mirror_delivers_on_tick_with_matching_events() {
    fn wasm_amount(world: &WasmWorld, entity: u32, kind: &str) -> f64 {
        world
            .get_component(entity, "Stockpile")
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|stock| {
                stock
                    .get("resources")
                    .and_then(|r| r.get(kind))
                    .and_then(|v| v.as_f64())
            })
            .unwrap_or(0.0)
    }

    let mut world = WasmWorld::new();
    let source = world.spawn_entity();
    world
        .set_component(
            source,
            "Stockpile",
            &json!({ "resources": { "grain": 100.0 } }).to_string(),
        )
        .unwrap();
    let target = world.spawn_entity();
    world
        .set_component(
            target,
            "Stockpile",
            &json!({ "resources": { "grain": 0.0 } }).to_string(),
        )
        .unwrap();
    let link = world.spawn_entity();
    world
        .set_component(
            link,
            "SupplyLink",
            &json!({
                "source": source,
                "target": target,
                "kind": "grain",
                "amount_per_tick": 10.0,
                "capacity_per_tick": 25.0,
                "active": true,
            })
            .to_string(),
        )
        .unwrap();

    world.tick();

    assert_eq!(wasm_amount(&world, source, "grain"), 90.0);
    assert_eq!(wasm_amount(&world, target, "grain"), 10.0);
    let delivered: Vec<JsonValue> =
        serde_json::from_str(&world.take_events("supply_delivered")).unwrap();
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0]["link"].as_u64(), Some(link as u64));
    assert_eq!(delivered[0]["amount"].as_f64(), Some(10.0));
}

/// Place an entity on a square cell.
fn set_position(world: &mut World, entity: u32, x: i32, y: i32) {
    world
        .set_component(
            entity,
            "Position",
            json!({"pos": {"Square": {"x": x, "y": y, "z": 0}}}),
        )
        .unwrap();
}

/// Map holding two cells with no adjacency between them.
fn disconnected_map() -> Map {
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(2, 2, 0);
    Map::new(Box::new(grid))
}

/// Two adjacent cells whose goal cell is marked impassable.
fn blocked_goal_map() -> Map {
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    grid.add_neighbor((0, 0, 0), (1, 0, 0));
    grid.add_neighbor((1, 0, 0), (0, 0, 0));
    Map::new(Box::new(grid))
}

/// Build one leg of the determinism pair: identical construction order gives
/// identical entity ids, so balances and event payloads must match exactly.
fn deterministic_supply_world() -> (World, u32, u32, u32) {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let first = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    let second = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    create_supply_link(&mut world, source, first, "grain", 10.0, 25.0).unwrap();
    create_supply_link(&mut world, source, second, "grain", 30.0, 12.0).unwrap();
    (world, source, first, second)
}

/// Legs between factions at war move nothing and report the war.
#[test]
fn blocks_cross_faction_legs_while_at_war_and_resumes_after_peace() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    set_faction(&mut world, source, "red", "member").unwrap();
    set_faction(&mut world, target, "blue", "member").unwrap();
    let link = create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();
    declare_war(&mut world, "red", "blue").unwrap();
    let before_source = stockpile_snapshot(&world, source);
    let before_target = stockpile_snapshot(&world, target);

    run_supply(&mut world);

    assert_eq!(stockpile_snapshot(&world, source), before_source);
    assert_eq!(stockpile_snapshot(&world, target), before_target);
    sync_events(&mut world);
    assert!(drain_events(&mut world, "supply_delivered").is_empty());
    assert!(drain_events(&mut world, "supply_shortfall").is_empty());
    let blocked = drain_events(&mut world, "supply_blocked");
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0]["link"].as_u64(), Some(link as u64));
    assert_eq!(blocked[0]["reason"].as_str(), Some("war"));

    declare_peace(&mut world, "red", "blue").unwrap();
    run_supply(&mut world);

    assert_eq!(stockpile_amount(&world, source, "grain"), 90.0);
    assert_eq!(stockpile_amount(&world, target, "grain"), 10.0);
    sync_events(&mut world);
    let delivered = drain_events(&mut world, "supply_delivered");
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0]["link"].as_u64(), Some(link as u64));
}

/// A war elsewhere never blocks legs inside one faction.
#[test]
fn lets_same_faction_legs_deliver_during_war_elsewhere() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    set_faction(&mut world, source, "red", "member").unwrap();
    set_faction(&mut world, target, "red", "member").unwrap();
    create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();
    declare_war(&mut world, "red", "blue").unwrap();

    run_supply(&mut world);

    assert_eq!(stockpile_amount(&world, source, "grain"), 90.0);
    assert_eq!(stockpile_amount(&world, target, "grain"), 10.0);
    sync_events(&mut world);
    assert_eq!(drain_events(&mut world, "supply_delivered").len(), 1);
    assert!(drain_events(&mut world, "supply_blocked").is_empty());
}

/// Endpoints with no route between them move nothing and report the block.
#[test]
fn blocks_legs_without_a_route_between_stockpiles() {
    let mut world = make_test_world();
    world.map = Some(disconnected_map());
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    set_position(&mut world, source, 0, 0);
    set_position(&mut world, target, 2, 2);
    let link = create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();
    let before_source = stockpile_snapshot(&world, source);

    run_supply(&mut world);

    assert_eq!(stockpile_snapshot(&world, source), before_source);
    assert_eq!(stockpile_amount(&world, target, "grain"), 0.0);
    sync_events(&mut world);
    assert!(drain_events(&mut world, "supply_delivered").is_empty());
    let blocked = drain_events(&mut world, "supply_blocked");
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0]["link"].as_u64(), Some(link as u64));
    assert_eq!(blocked[0]["reason"].as_str(), Some("no_path"));
}

/// An impassable destination cell blocks the leg the same way.
#[test]
fn blocks_legs_into_impassable_cells() {
    use engine_core::map::CellKey;

    let mut world = make_test_world();
    let mut map = blocked_goal_map();
    map.set_cell_metadata(
        &CellKey::Square { x: 1, y: 0, z: 0 },
        json!({"walkable": false}),
    );
    world.map = Some(map);
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    set_position(&mut world, source, 0, 0);
    set_position(&mut world, target, 1, 0);
    create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();

    run_supply(&mut world);

    assert_eq!(stockpile_amount(&world, source, "grain"), 100.0);
    assert_eq!(stockpile_amount(&world, target, "grain"), 0.0);
    sync_events(&mut world);
    let blocked = drain_events(&mut world, "supply_blocked");
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0]["reason"].as_str(), Some("no_path"));
}

/// Positions off every known map leave the link unblocked.
#[test]
fn treats_off_map_positions_as_unblocked() {
    let mut world = make_test_world();
    world.map = Some(disconnected_map());
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    set_position(&mut world, source, 9, 9);
    set_position(&mut world, target, 8, 8);
    create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();

    run_supply(&mut world);

    assert_eq!(stockpile_amount(&world, source, "grain"), 90.0);
    assert_eq!(stockpile_amount(&world, target, "grain"), 10.0);
    sync_events(&mut world);
    assert_eq!(drain_events(&mut world, "supply_delivered").len(), 1);
    assert!(drain_events(&mut world, "supply_blocked").is_empty());
}

/// A despawned link entity ticks as gone: no transfer and no event.
#[test]
fn stays_silent_when_the_link_itself_is_gone() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    let link = create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();
    world.despawn_entity(link);

    run_supply(&mut world);

    assert_eq!(stockpile_amount(&world, source, "grain"), 100.0);
    assert_eq!(stockpile_amount(&world, target, "grain"), 0.0);
    sync_events(&mut world);
    assert!(drain_events(&mut world, "supply_delivered").is_empty());
    assert!(drain_events(&mut world, "supply_shortfall").is_empty());
    assert!(drain_events(&mut world, "supply_blocked").is_empty());
}

/// Legs sharing one source run in ascending link-id order.
#[test]
fn processes_shared_sources_in_link_order() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 10.0 }));
    let first = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    let second = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    let link_one = create_supply_link(&mut world, source, first, "grain", 10.0, 10.0).unwrap();
    let link_two = create_supply_link(&mut world, source, second, "grain", 10.0, 10.0).unwrap();
    assert!(link_one < link_two);

    run_supply(&mut world);

    assert_eq!(stockpile_amount(&world, source, "grain"), 0.0);
    assert_eq!(stockpile_amount(&world, first, "grain"), 10.0);
    assert_eq!(stockpile_amount(&world, second, "grain"), 0.0);
    sync_events(&mut world);
    let delivered = drain_events(&mut world, "supply_delivered");
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0]["link"].as_u64(), Some(link_one as u64));
    let shortfalls = drain_events(&mut world, "supply_shortfall");
    assert_eq!(shortfalls.len(), 1);
    assert_eq!(shortfalls[0]["link"].as_u64(), Some(link_two as u64));
    assert!(drain_events(&mut world, "supply_blocked").is_empty());
}

/// Two identically built worlds tick to identical balances and events.
#[test]
fn reproduces_balances_and_events_across_identical_worlds() {
    let (mut first_world, first_source, first_a, first_b) = deterministic_supply_world();
    let (mut second_world, second_source, second_a, second_b) = deterministic_supply_world();

    run_supply(&mut first_world);
    run_supply(&mut second_world);

    for (entity_a, entity_b) in [
        (first_source, second_source),
        (first_a, second_a),
        (first_b, second_b),
    ] {
        assert_eq!(
            stockpile_snapshot(&first_world, entity_a),
            stockpile_snapshot(&second_world, entity_b)
        );
    }
    sync_events(&mut first_world);
    sync_events(&mut second_world);
    let first_events: Vec<String> = drain_events(&mut first_world, "supply_delivered")
        .iter()
        .map(|event| event.to_string())
        .collect();
    let second_events: Vec<String> = drain_events(&mut second_world, "supply_delivered")
        .iter()
        .map(|event| event.to_string())
        .collect();
    assert_eq!(first_events.len(), 2);
    assert_eq!(first_events, second_events);
}

/// Save/load keeps every link and flag, and delivery resumes after load.
#[test]
fn preserves_links_across_save_and_load_and_resumes_delivery() {
    let mut world = make_test_world();
    let source = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let target = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    let other = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    let live = create_supply_link(&mut world, source, target, "grain", 10.0, 25.0).unwrap();
    let quiet = create_supply_link(&mut world, source, other, "grain", 5.0, 5.0).unwrap();
    set_supply_link_active(&mut world, quiet, false).unwrap();

    let registry = world.registry.clone();
    let mut loaded = save_and_load_roundtrip(&world, registry);

    assert_eq!(list_supply_links(&loaded), vec![live, quiet]);
    assert_eq!(
        get_supply_link(&loaded, live),
        get_supply_link(&world, live)
    );
    assert_eq!(
        get_supply_link(&loaded, quiet),
        get_supply_link(&world, quiet)
    );

    run_supply(&mut loaded);

    assert_eq!(stockpile_amount(&loaded, source, "grain"), 90.0);
    assert_eq!(stockpile_amount(&loaded, target, "grain"), 10.0);
    assert_eq!(stockpile_amount(&loaded, other, "grain"), 0.0);
    sync_events(&mut loaded);
    assert_eq!(drain_events(&mut loaded, "supply_delivered").len(), 1);
}

/// The WASM host tick enforces the war gate like the core system.
#[test]
fn wasm_mirror_blocks_cross_faction_legs_while_at_war() {
    let mut world = WasmWorld::new();
    let source = world.spawn_entity();
    world
        .set_component(
            source,
            "Stockpile",
            &json!({ "resources": { "grain": 100.0 } }).to_string(),
        )
        .unwrap();
    world
        .set_component(
            source,
            "Faction",
            &json!({"faction_id": "red", "role": "member", "joined_tick": 0}).to_string(),
        )
        .unwrap();
    let target = world.spawn_entity();
    world
        .set_component(
            target,
            "Stockpile",
            &json!({ "resources": { "grain": 0.0 } }).to_string(),
        )
        .unwrap();
    world
        .set_component(
            target,
            "Faction",
            &json!({"faction_id": "blue", "role": "member", "joined_tick": 0}).to_string(),
        )
        .unwrap();
    let link = world.spawn_entity();
    world
        .set_component(
            link,
            "SupplyLink",
            &json!({
                "source": source,
                "target": target,
                "kind": "grain",
                "amount_per_tick": 10.0,
                "capacity_per_tick": 25.0,
                "active": true,
            })
            .to_string(),
        )
        .unwrap();
    world.diplomacy.apply_declare_war("red", "blue", 0).unwrap();

    world.tick();

    let amount = |entity: u32| {
        world
            .get_component(entity, "Stockpile")
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|stock| {
                stock
                    .get("resources")
                    .and_then(|r| r.get("grain"))
                    .and_then(|v| v.as_f64())
            })
            .unwrap_or(0.0)
    };
    assert_eq!(amount(source), 100.0);
    assert_eq!(amount(target), 0.0);
    let blocked: Vec<JsonValue> =
        serde_json::from_str(&world.take_events("supply_blocked")).unwrap();
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0]["link"].as_u64(), Some(link as u64));
    assert_eq!(blocked[0]["reason"].as_str(), Some("war"));
    let delivered: Vec<JsonValue> =
        serde_json::from_str(&world.take_events("supply_delivered")).unwrap();
    assert!(delivered.is_empty());
}

/// The WASM host world runs the same validated link lifecycle as core.
#[test]
fn wasm_world_runs_the_supply_link_lifecycle() {
    let mut world = WasmWorld::new();
    let source = world.spawn_entity();
    world
        .set_component(
            source,
            "Stockpile",
            &json!({ "resources": { "grain": 100.0 } }).to_string(),
        )
        .unwrap();
    let target = world.spawn_entity();
    world
        .set_component(
            target,
            "Stockpile",
            &json!({ "resources": { "grain": 0.0 } }).to_string(),
        )
        .unwrap();

    let link = world
        .create_supply_link(source, target, "grain", 10.0, 25.0)
        .unwrap();
    let record = world.get_supply_link(link).unwrap();
    assert_eq!(record.source, source);
    assert_eq!(record.target, target);
    assert_eq!(record.kind, "grain");
    assert_eq!(record.amount_per_tick, 10.0);
    assert_eq!(record.capacity_per_tick, 25.0);
    assert!(record.active);
    assert_eq!(world.list_supply_links(), vec![link]);

    let bare = world.spawn_entity();
    assert!(
        world
            .create_supply_link(source, source, "grain", 10.0, 10.0)
            .unwrap_err()
            .starts_with("SameEndpoint")
    );
    assert!(
        world
            .create_supply_link(source, target, "", 10.0, 10.0)
            .unwrap_err()
            .starts_with("UnknownKind")
    );
    for bad in [0.0, -5.0, f64::NAN, f64::INFINITY] {
        assert!(
            world
                .create_supply_link(source, target, "grain", bad, 10.0)
                .unwrap_err()
                .starts_with("NonPositiveAmount")
        );
    }
    assert!(
        world
            .create_supply_link(bare, target, "grain", 1.0, 1.0)
            .unwrap_err()
            .starts_with("NoStockpile")
    );
    assert!(world.get_supply_link(9999).is_none());
    assert!(
        world
            .remove_supply_link(9999)
            .unwrap_err()
            .starts_with("UnknownLink")
    );
    assert!(
        world
            .set_supply_link_active(9999, true)
            .unwrap_err()
            .starts_with("UnknownLink")
    );
    assert_eq!(world.list_supply_links(), vec![link]);

    let amount = |world: &WasmWorld, entity: u32| {
        world
            .get_component(entity, "Stockpile")
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|stock| {
                stock
                    .get("resources")
                    .and_then(|r| r.get("grain"))
                    .and_then(|v| v.as_f64())
            })
            .unwrap_or(0.0)
    };
    world.set_supply_link_active(link, false).unwrap();
    world.tick();
    assert_eq!(amount(&world, source), 100.0);
    assert_eq!(amount(&world, target), 0.0);
    world.set_supply_link_active(link, true).unwrap();
    world.tick();
    assert_eq!(amount(&world, source), 90.0);
    assert_eq!(amount(&world, target), 10.0);
    let delivered: Vec<JsonValue> =
        serde_json::from_str(&world.take_events("supply_delivered")).unwrap();
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0]["link"].as_u64(), Some(link as u64));

    world.remove_supply_link(link).unwrap();
    assert!(world.list_supply_links().is_empty());
    assert!(world.get_supply_link(link).is_none());
}

/// Run one game-loop pass the way `tick()` orders it: consumption, then supply.
fn run_game_pass(world: &mut World) {
    ConsumptionSystem.run(world);
    SupplySystem.run(world);
}

/// A multi-depot strategy loop: harvest, capped pushes, consumption, war, peace.
#[test]
fn runs_a_multi_depot_strategy_loop_from_field_to_table() {
    let mut world = make_test_world();
    let farm = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let granary = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));
    let town = world.spawn_entity();
    world
        .set_component(town, "Stockpile", json!({ "resources": { "grain": 10.0 } }))
        .unwrap();
    world
        .set_component(
            town,
            "Upkeep",
            json!({ "drains": [{ "kind": "grain", "amount_per_tick": 4.0 }] }),
        )
        .unwrap();
    set_faction(&mut world, farm, "valoria", "member").unwrap();
    set_faction(&mut world, granary, "valoria", "member").unwrap();
    set_faction(&mut world, town, "drakmor", "member").unwrap();
    let field_road = create_supply_link(&mut world, farm, granary, "grain", 20.0, 8.0).unwrap();
    let town_road = create_supply_link(&mut world, granary, town, "grain", 10.0, 5.0).unwrap();

    // Peaceful harvest: the town eats first, then each capped leg pushes.
    run_game_pass(&mut world);
    assert_eq!(stockpile_amount(&world, farm, "grain"), 92.0);
    assert_eq!(stockpile_amount(&world, granary, "grain"), 3.0);
    assert_eq!(stockpile_amount(&world, town, "grain"), 11.0);
    sync_events(&mut world);
    assert!(drain_events(&mut world, "consumption_shortage").is_empty());
    let delivered = drain_events(&mut world, "supply_delivered");
    assert_eq!(delivered.len(), 2);

    // A bumper harvest lands at the farm; both caps still bind.
    let ripe = stockpile_amount(&world, farm, "grain") + 40.0;
    let mut stock = world.get_component(farm, "Stockpile").cloned().unwrap();
    stock["resources"]["grain"] = json!(ripe);
    world.set_component(farm, "Stockpile", stock).unwrap();
    run_game_pass(&mut world);
    assert_eq!(stockpile_amount(&world, farm, "grain"), 124.0);
    assert_eq!(stockpile_amount(&world, granary, "grain"), 6.0);
    assert_eq!(stockpile_amount(&world, town, "grain"), 12.0);
    sync_events(&mut world);
    assert_eq!(drain_events(&mut world, "supply_delivered").len(), 2);

    // War with the town's nation blocks only the cross-border leg.
    declare_war(&mut world, "valoria", "drakmor").unwrap();
    run_game_pass(&mut world);
    assert_eq!(stockpile_amount(&world, farm, "grain"), 116.0);
    assert_eq!(stockpile_amount(&world, granary, "grain"), 14.0);
    assert_eq!(stockpile_amount(&world, town, "grain"), 8.0);
    sync_events(&mut world);
    let blocked = drain_events(&mut world, "supply_blocked");
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0]["link"].as_u64(), Some(town_road as u64));
    assert_eq!(blocked[0]["reason"].as_str(), Some("war"));
    let delivered = drain_events(&mut world, "supply_delivered");
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0]["link"].as_u64(), Some(field_road as u64));

    // Peace reopens the road; both legs deliver again.
    declare_peace(&mut world, "valoria", "drakmor").unwrap();
    run_game_pass(&mut world);
    assert_eq!(stockpile_amount(&world, farm, "grain"), 108.0);
    assert_eq!(stockpile_amount(&world, granary, "grain"), 17.0);
    assert_eq!(stockpile_amount(&world, town, "grain"), 9.0);
    sync_events(&mut world);
    assert!(drain_events(&mut world, "supply_blocked").is_empty());
    assert_eq!(drain_events(&mut world, "supply_delivered").len(), 2);
}
