//! Standing suite for the supply and logistics network MVP.
//!
//! Created in M1 alongside the `SupplyLink` lifecycle, the core `SupplySystem`
//! tick, the `SYSTEM_EXECUTION_ORDER` slot, and all-harness registration.
//! Grows milestone by milestone (M2 war/path gates, determinism, save/load
//! extend in place).

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::ecs::world::wasm::WasmWorld;
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
