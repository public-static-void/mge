//! Standing suite for inter-stockpile trade primitives and treaty queries.
//!
//! Created in M1 alongside the atomic transfer primitive and the read-only
//! trade-treaty query. Grows milestone by milestone (M2 gated execution,
//! M3 upkeep/consumption extend in place).

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::diplomacy::RelationState;
use engine_core::diplomacy::{
    TreatyKind, accept_treaty, break_treaty, declare_war, expire_due_treaties, get_relation,
    propose_treaty,
};
use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::systems::consumption::ConsumptionSystem;
use engine_core::systems::economic::{EconomicSystem, Recipe, ResourceAmount};
use engine_core::trade::{
    TradeError, TransferError, execute_treaty_trade, has_active_trade_treaty,
    transfer_stockpile_resource,
};
use serde_json::{Value as JsonValue, json};

/// Spawn an entity carrying a Stockpile with the given resources map.
fn spawn_stockpile(world: &mut World, resources: serde_json::Value) -> u32 {
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Stockpile", json!({ "resources": resources }))
        .unwrap();
    eid
}

/// Spawn an entity carrying a Stockpile plus a Faction membership.
fn spawn_faction_stockpile(
    world: &mut World,
    faction_id: &str,
    resources: serde_json::Value,
) -> u32 {
    let eid = spawn_stockpile(world, resources);
    world
        .set_component(eid, "Faction", json!({ "faction_id": faction_id }))
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

/// Moving grain between two stockpiles debits one and credits the other.
#[test]
fn moves_grain_between_stockpiles_in_one_call() {
    let mut world = make_test_world();
    let from = spawn_stockpile(&mut world, json!({ "grain": 100.0 }));
    let to = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));

    transfer_stockpile_resource(&mut world, from, to, "grain", 30.0).unwrap();

    assert_eq!(stockpile_amount(&world, from, "grain"), 70.0);
    assert_eq!(stockpile_amount(&world, to, "grain"), 30.0);
}

/// A transfer that exceeds available funds fails and mutates nothing.
#[test]
fn leaves_both_stockpiles_unchanged_when_funds_are_insufficient() {
    let mut world = make_test_world();
    let from = spawn_stockpile(&mut world, json!({ "grain": 20.0 }));
    let to = spawn_stockpile(&mut world, json!({ "grain": 5.0 }));
    let before_from = stockpile_snapshot(&world, from);
    let before_to = stockpile_snapshot(&world, to);

    let err = transfer_stockpile_resource(&mut world, from, to, "grain", 30.0).unwrap_err();

    assert_eq!(
        err,
        TransferError::InsufficientFunds {
            kind: "grain".to_string(),
            required: 30.0,
            available: 20.0,
        }
    );
    assert_eq!(stockpile_snapshot(&world, from), before_from);
    assert_eq!(stockpile_snapshot(&world, to), before_to);
}

/// Missing stockpiles, unknown kinds, and bad amounts fail without mutation.
#[test]
fn rejects_missing_stockpile_unknown_kind_and_bad_amounts_without_mutating() {
    let mut world = make_test_world();
    let stocked = spawn_stockpile(&mut world, json!({ "grain": 50.0 }));
    let bare = world.spawn_entity();
    let before = stockpile_snapshot(&world, stocked);

    assert_eq!(
        transfer_stockpile_resource(&mut world, bare, stocked, "grain", 10.0).unwrap_err(),
        TransferError::NoStockpile(bare)
    );
    assert_eq!(
        transfer_stockpile_resource(&mut world, stocked, bare, "grain", 10.0).unwrap_err(),
        TransferError::NoStockpile(bare)
    );
    assert_eq!(
        transfer_stockpile_resource(&mut world, stocked, stocked, "", 10.0).unwrap_err(),
        TransferError::UnknownKind(String::new())
    );
    for bad in [0.0, -5.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            transfer_stockpile_resource(&mut world, stocked, stocked, "grain", bad).unwrap_err(),
            TransferError::NonPositiveAmount,
            "amount {bad} must be rejected"
        );
    }
    assert_eq!(stockpile_snapshot(&world, stocked), before);
    assert!(world.get_component(bare, "Stockpile").is_none());
}

/// Transferring to self validates the amount and otherwise changes nothing.
#[test]
fn treats_self_transfer_as_validated_noop() {
    let mut world = make_test_world();
    let entity = spawn_stockpile(&mut world, json!({ "grain": 40.0 }));
    let before = stockpile_snapshot(&world, entity);

    transfer_stockpile_resource(&mut world, entity, entity, "grain", 10.0).unwrap();
    assert_eq!(stockpile_snapshot(&world, entity), before);

    assert_eq!(
        transfer_stockpile_resource(&mut world, entity, entity, "grain", 0.0).unwrap_err(),
        TransferError::NonPositiveAmount
    );
    assert_eq!(stockpile_snapshot(&world, entity), before);
}

/// Dust-level shortfalls within epsilon succeed and floor at zero.
#[test]
fn succeeds_when_shortfall_is_float_dust_within_epsilon() {
    let mut world = make_test_world();
    let from = spawn_stockpile(&mut world, json!({ "grain": 5.0 - 0.5e-9 }));
    let to = spawn_stockpile(&mut world, json!({ "grain": 0.0 }));

    transfer_stockpile_resource(&mut world, from, to, "grain", 5.0).unwrap();

    assert_eq!(stockpile_amount(&world, from, "grain"), 0.0);
    assert_eq!(stockpile_amount(&world, to, "grain"), 5.0);

    let short = spawn_stockpile(&mut world, json!({ "grain": 5.0 - 2e-9 }));
    let dest = spawn_stockpile(&mut world, json!({ "grain": 1.0 }));
    let before_short = stockpile_snapshot(&world, short);
    let before_dest = stockpile_snapshot(&world, dest);
    assert!(
        transfer_stockpile_resource(&mut world, short, dest, "grain", 5.0).is_err(),
        "shortfall beyond epsilon must fail"
    );
    assert_eq!(stockpile_snapshot(&world, short), before_short);
    assert_eq!(stockpile_snapshot(&world, dest), before_dest);
}

/// The treaty query tracks accept/break/expire without touching diplomacy state.
#[test]
fn reports_treaty_status_after_accept_break_and_expiry() {
    let mut world = make_test_world();
    assert!(!has_active_trade_treaty(&world, "a", "b"));

    let id = propose_treaty(&mut world, "a", "b", TreatyKind::TradeStub, None).unwrap();
    accept_treaty(&mut world, id).unwrap();
    assert!(has_active_trade_treaty(&world, "a", "b"));

    let before = world.diplomacy.clone();
    assert!(has_active_trade_treaty(&world, "b", "a"));
    assert!(!has_active_trade_treaty(&world, "a", "zzz"));
    assert_eq!(
        world.diplomacy, before,
        "read-only query must not touch the diplomacy store"
    );

    break_treaty(&mut world, id).unwrap();
    assert!(!has_active_trade_treaty(&world, "a", "b"));

    let expiring = propose_treaty(&mut world, "c", "d", TreatyKind::TradeStub, Some(5)).unwrap();
    accept_treaty(&mut world, expiring).unwrap();
    assert!(has_active_trade_treaty(&world, "c", "d"));
    world.turn = 5;
    expire_due_treaties(&mut world).unwrap();
    assert!(!has_active_trade_treaty(&world, "c", "d"));
}

/// A proposed-but-unaccepted trade treaty does not count as active.
#[test]
fn stays_false_for_proposed_but_unaccepted_treaty() {
    let mut world = make_test_world();
    propose_treaty(&mut world, "a", "b", TreatyKind::TradeStub, None).unwrap();
    assert!(!has_active_trade_treaty(&world, "a", "b"));

    let other = propose_treaty(&mut world, "a", "b", TreatyKind::Alliance, None).unwrap();
    accept_treaty(&mut world, other).unwrap();
    assert!(
        !has_active_trade_treaty(&world, "a", "b"),
        "non-trade treaties must not satisfy the query"
    );
}

/// Pair lookups behave identically regardless of argument order.
#[test]
fn matches_treaty_pairs_regardless_of_argument_order() {
    let mut world = make_test_world();
    let id = propose_treaty(&mut world, "alpha", "beta", TreatyKind::TradeStub, None).unwrap();
    accept_treaty(&mut world, id).unwrap();
    assert!(has_active_trade_treaty(&world, "alpha", "beta"));
    assert!(has_active_trade_treaty(&world, "beta", "alpha"));
}

/// A live trade treaty in peacetime authorizes gated execution.
#[test]
fn executes_gated_trade_when_treaty_is_live_and_pair_is_at_peace() {
    let mut world = make_test_world();
    let from = spawn_faction_stockpile(&mut world, "a", json!({ "grain": 100.0 }));
    let to = spawn_faction_stockpile(&mut world, "b", json!({ "grain": 0.0 }));
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::TradeStub, None).unwrap();
    accept_treaty(&mut world, id).unwrap();

    execute_treaty_trade(&mut world, from, to, "grain", 30.0).unwrap();

    assert_eq!(stockpile_amount(&world, from, "grain"), 70.0);
    assert_eq!(stockpile_amount(&world, to, "grain"), 30.0);
}

/// War blocks gated execution while the pre-war treaty stays listed, and the
/// ungated primitive keeps working between the same entities.
#[test]
fn blocks_gated_trade_during_war_but_leaves_ungated_transfer_open() {
    let mut world = make_test_world();
    let from = spawn_faction_stockpile(&mut world, "a", json!({ "grain": 100.0 }));
    let to = spawn_faction_stockpile(&mut world, "b", json!({ "grain": 0.0 }));
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::TradeStub, None).unwrap();
    accept_treaty(&mut world, id).unwrap();
    execute_treaty_trade(&mut world, from, to, "grain", 30.0).unwrap();

    declare_war(&mut world, "a", "b").unwrap();
    assert_eq!(get_relation(&world, "a", "b"), RelationState::War);

    let before_from = stockpile_snapshot(&world, from);
    let before_to = stockpile_snapshot(&world, to);
    assert_eq!(
        execute_treaty_trade(&mut world, from, to, "grain", 10.0).unwrap_err(),
        TradeError::RelationIsWar
    );
    assert_eq!(stockpile_snapshot(&world, from), before_from);
    assert_eq!(stockpile_snapshot(&world, to), before_to);
    assert!(
        has_active_trade_treaty(&world, "a", "b"),
        "pre-war TradeStub treaties survive declare_war yet must not authorize"
    );

    transfer_stockpile_resource(&mut world, from, to, "grain", 10.0).unwrap();
    assert_eq!(stockpile_amount(&world, from, "grain"), 60.0);
    assert_eq!(stockpile_amount(&world, to, "grain"), 40.0);
}

/// Gated execution without any live treaty fails and mutates nothing.
#[test]
fn rejects_gated_trade_without_a_live_treaty() {
    let mut world = make_test_world();
    let from = spawn_faction_stockpile(&mut world, "a", json!({ "grain": 50.0 }));
    let to = spawn_faction_stockpile(&mut world, "b", json!({ "grain": 5.0 }));
    let before_from = stockpile_snapshot(&world, from);
    let before_to = stockpile_snapshot(&world, to);

    assert_eq!(
        execute_treaty_trade(&mut world, from, to, "grain", 10.0).unwrap_err(),
        TradeError::NoLiveTreaty
    );
    assert_eq!(stockpile_snapshot(&world, from), before_from);
    assert_eq!(stockpile_snapshot(&world, to), before_to);

    let id = propose_treaty(&mut world, "a", "b", TreatyKind::TradeStub, None).unwrap();
    accept_treaty(&mut world, id).unwrap();
    break_treaty(&mut world, id).unwrap();
    assert_eq!(
        execute_treaty_trade(&mut world, from, to, "grain", 10.0).unwrap_err(),
        TradeError::NoLiveTreaty
    );
    assert_eq!(stockpile_snapshot(&world, from), before_from);
    assert_eq!(stockpile_snapshot(&world, to), before_to);
}

/// Entities without faction membership cannot authorize gated execution.
#[test]
fn rejects_gated_trade_when_either_entity_has_no_faction() {
    let mut world = make_test_world();
    let factioned = spawn_faction_stockpile(&mut world, "a", json!({ "grain": 50.0 }));
    let bare = spawn_stockpile(&mut world, json!({ "grain": 50.0 }));
    let other = spawn_faction_stockpile(&mut world, "b", json!({ "grain": 0.0 }));
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::TradeStub, None).unwrap();
    accept_treaty(&mut world, id).unwrap();

    assert_eq!(
        execute_treaty_trade(&mut world, bare, other, "grain", 10.0).unwrap_err(),
        TradeError::NoLiveTreaty
    );
    assert_eq!(
        execute_treaty_trade(&mut world, factioned, bare, "grain", 10.0).unwrap_err(),
        TradeError::NoLiveTreaty
    );
    assert_eq!(stockpile_amount(&world, bare, "grain"), 50.0);
    assert_eq!(stockpile_amount(&world, other, "grain"), 0.0);
}

/// An expired treaty stops authorizing gated execution; completed transfers stand.
#[test]
fn rejects_gated_trade_after_treaty_expiry_while_completed_transfers_stand() {
    let mut world = make_test_world();
    let from = spawn_faction_stockpile(&mut world, "c", json!({ "grain": 100.0 }));
    let to = spawn_faction_stockpile(&mut world, "d", json!({ "grain": 0.0 }));
    let id = propose_treaty(&mut world, "c", "d", TreatyKind::TradeStub, Some(5)).unwrap();
    accept_treaty(&mut world, id).unwrap();
    execute_treaty_trade(&mut world, from, to, "grain", 30.0).unwrap();

    world.turn = 5;
    expire_due_treaties(&mut world).unwrap();
    assert!(!has_active_trade_treaty(&world, "c", "d"));
    assert_eq!(
        execute_treaty_trade(&mut world, from, to, "grain", 10.0).unwrap_err(),
        TradeError::NoLiveTreaty
    );
    assert_eq!(stockpile_amount(&world, from, "grain"), 70.0);
    assert_eq!(stockpile_amount(&world, to, "grain"), 30.0);
}

/// Transfer failures inside gated execution surface as transfer errors.
#[test]
fn surfaces_transfer_failures_from_gated_execution() {
    let mut world = make_test_world();
    let from = spawn_faction_stockpile(&mut world, "a", json!({ "grain": 5.0 }));
    let to = spawn_faction_stockpile(&mut world, "b", json!({ "grain": 0.0 }));
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::TradeStub, None).unwrap();
    accept_treaty(&mut world, id).unwrap();

    let err = execute_treaty_trade(&mut world, from, to, "grain", 30.0).unwrap_err();
    assert!(
        matches!(
            err,
            TradeError::Transfer(TransferError::InsufficientFunds { .. })
        ),
        "expected a wrapped insufficient-funds error, got {err:?}"
    );
    assert_eq!(stockpile_amount(&world, from, "grain"), 5.0);
    assert_eq!(stockpile_amount(&world, to, "grain"), 0.0);
}

/// The WASM trade mirror preserves the war gate while leaving the ungated
/// primitive open on the same world state.
#[test]
fn wasm_mirror_preserves_war_gate_while_leaving_ungated_transfer_open() {
    fn spawn_wasm_stockpile(
        world: &mut WasmWorld,
        faction: &str,
        resources: serde_json::Value,
    ) -> u32 {
        let eid = world.spawn_entity();
        world
            .set_component(
                eid,
                "Stockpile",
                &json!({ "resources": resources }).to_string(),
            )
            .unwrap();
        world
            .set_component(
                eid,
                "Faction",
                &json!({ "faction_id": faction }).to_string(),
            )
            .unwrap();
        eid
    }

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
    let from = spawn_wasm_stockpile(&mut world, "a", json!({ "grain": 100.0 }));
    let to = spawn_wasm_stockpile(&mut world, "b", json!({ "grain": 0.0 }));
    let (id, _) = world
        .diplomacy
        .apply_propose_treaty("a", "b", TreatyKind::TradeStub, 0, None)
        .unwrap();
    world.diplomacy.apply_accept_treaty(id, 0).unwrap();
    assert!(world.has_active_trade_treaty("a", "b"));
    world.execute_treaty_trade(from, to, "grain", 30.0).unwrap();

    world.diplomacy.apply_declare_war("a", "b", 0).unwrap();
    let err = world
        .execute_treaty_trade(from, to, "grain", 10.0)
        .unwrap_err();
    assert!(
        err.contains("RelationIsWar"),
        "expected the war gate, got {err}"
    );
    assert!(
        world.has_active_trade_treaty("a", "b"),
        "pre-war TradeStub treaties survive declare_war yet must not authorize"
    );
    assert_eq!(wasm_amount(&world, from, "grain"), 70.0);
    assert_eq!(wasm_amount(&world, to, "grain"), 30.0);

    world
        .transfer_stockpile_resource(from, to, "grain", 10.0)
        .unwrap();
    assert_eq!(wasm_amount(&world, from, "grain"), 60.0);
    assert_eq!(wasm_amount(&world, to, "grain"), 40.0);
}

/// Spawn an entity carrying a Stockpile plus an Upkeep drain list.
fn spawn_upkeep(world: &mut World, resources: serde_json::Value, drains: serde_json::Value) -> u32 {
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Stockpile", json!({ "resources": resources }))
        .unwrap();
    world
        .set_component(eid, "Upkeep", json!({ "drains": drains }))
        .unwrap();
    eid
}

/// Run one consumption tick on the world.
fn run_consumption(world: &mut World) {
    ConsumptionSystem.run(world);
}

/// Drain all `consumption_shortage` events emitted so far.
fn shortage_events(world: &mut World) -> Vec<JsonValue> {
    world.update_event_buses::<JsonValue>();
    world.take_events("consumption_shortage")
}

/// Upkeep validates its drain shape and round-trips through get_component.
#[test]
fn validates_upkeep_drains_and_round_trips_through_component_access() {
    let mut world = make_test_world();
    let eid = world.spawn_entity();

    world
        .set_component(
            eid,
            "Upkeep",
            json!({ "drains": [{ "kind": "grain", "amount_per_tick": 2.0 }] }),
        )
        .unwrap();
    let stored = world.get_component(eid, "Upkeep").unwrap();
    assert_eq!(
        stored["drains"][0]["kind"].as_str(),
        Some("grain"),
        "drains must round-trip through get_component"
    );
    assert_eq!(
        stored["drains"][0]["amount_per_tick"].as_f64(),
        Some(2.0),
        "drain amounts must round-trip through get_component"
    );

    assert!(
        world
            .set_component(
                eid,
                "Upkeep",
                json!({ "drains": [{ "kind": "", "amount_per_tick": 2.0 }] })
            )
            .is_err(),
        "empty kind must be rejected"
    );
    for bad in [0.0, -1.0] {
        assert!(
            world
                .set_component(
                    eid,
                    "Upkeep",
                    json!({ "drains": [{ "kind": "grain", "amount_per_tick": bad }] })
                )
                .is_err(),
            "non-positive amount {bad} must be rejected"
        );
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(
            world
                .set_component(
                    eid,
                    "Upkeep",
                    json!({ "drains": [{ "kind": "grain", "amount_per_tick": bad }] })
                )
                .is_err(),
            "non-finite amount {bad} must be rejected"
        );
    }
    assert!(
        world.set_component(eid, "Upkeep", json!({})).is_ok(),
        "a missing drain list defaults to an empty no-op list"
    );
    let stored = world.get_component(eid, "Upkeep").unwrap();
    assert_eq!(
        stored["drains"],
        json!([]),
        "the defaulted drain list stores empty"
    );
    world
        .set_component(
            eid,
            "Upkeep",
            json!({ "drains": [{ "kind": "grain", "amount_per_tick": 2.0 }] }),
        )
        .unwrap();
    let stored = world.get_component(eid, "Upkeep").unwrap();
    assert_eq!(
        stored["drains"][0]["amount_per_tick"].as_f64(),
        Some(2.0),
        "rejected writes must leave the stored Upkeep untouched"
    );
}

/// A steady drain subtracts the per-tick amount on every tick.
#[test]
fn drains_steady_upkeep_across_consecutive_ticks() {
    let mut world = make_test_world();
    let eid = spawn_upkeep(
        &mut world,
        json!({ "grain": 10.0 }),
        json!([{ "kind": "grain", "amount_per_tick": 2.0 }]),
    );

    run_consumption(&mut world);
    assert_eq!(stockpile_amount(&world, eid, "grain"), 8.0);

    run_consumption(&mut world);
    run_consumption(&mut world);
    assert_eq!(stockpile_amount(&world, eid, "grain"), 4.0);
    assert!(
        shortage_events(&mut world).is_empty(),
        "fully covered drains emit no shortage events"
    );
}

/// An uncovered drain leaves the stockpile untouched and emits a shortage event.
#[test]
fn keeps_stockpile_and_emits_shortage_when_drain_is_uncovered() {
    let mut world = make_test_world();
    let eid = spawn_upkeep(
        &mut world,
        json!({ "grain": 1.0 }),
        json!([{ "kind": "grain", "amount_per_tick": 2.0 }]),
    );

    run_consumption(&mut world);

    assert_eq!(
        stockpile_amount(&world, eid, "grain"),
        1.0,
        "a shorted entity keeps its balance"
    );
    let events = shortage_events(&mut world);
    assert_eq!(events.len(), 1, "one missing kind emits one event");
    let event = &events[0];
    assert_eq!(event["type"].as_str(), Some("consumption_shortage"));
    assert_eq!(event["entity"].as_u64(), Some(eid as u64));
    assert_eq!(event["kind"].as_str(), Some("grain"));
    assert_eq!(event["required"].as_f64(), Some(2.0));
    assert_eq!(event["available"].as_f64(), Some(1.0));
    assert!(
        event["turn"].as_u64().is_some(),
        "shortage carries its tick"
    );
}

/// One uncovered drain blocks every drain on the same entity for that tick.
#[test]
fn applies_no_drains_for_entity_with_any_shortfall() {
    let mut world = make_test_world();
    let eid = spawn_upkeep(
        &mut world,
        json!({ "grain": 10.0, "wood": 1.0 }),
        json!([
            { "kind": "grain", "amount_per_tick": 2.0 },
            { "kind": "wood", "amount_per_tick": 2.0 },
        ]),
    );

    run_consumption(&mut world);

    assert_eq!(stockpile_amount(&world, eid, "grain"), 10.0);
    assert_eq!(stockpile_amount(&world, eid, "wood"), 1.0);
    let events = shortage_events(&mut world);
    assert_eq!(events.len(), 1, "only the missing kind emits an event");
    assert_eq!(events[0]["kind"].as_str(), Some("wood"));
    assert_eq!(events[0]["required"].as_f64(), Some(2.0));
    assert_eq!(events[0]["available"].as_f64(), Some(1.0));
}

/// Repeated identical tick sequences produce identical balances and event order.
#[test]
fn repeats_identical_balances_and_event_order_across_runs() {
    type TickOutcome = (Vec<(f64, f64)>, Vec<Vec<(u64, String)>>);
    fn run_two_ticks() -> TickOutcome {
        let mut world = make_test_world();
        let low = spawn_upkeep(
            &mut world,
            json!({ "grain": 3.0 }),
            json!([{ "kind": "grain", "amount_per_tick": 2.0 }]),
        );
        let high = spawn_upkeep(
            &mut world,
            json!({ "grain": 1.0 }),
            json!([{ "kind": "grain", "amount_per_tick": 2.0 }]),
        );
        let mut per_tick = Vec::new();
        for _ in 0..2 {
            run_consumption(&mut world);
            let batch: Vec<(u64, String)> = shortage_events(&mut world)
                .iter()
                .map(|e| {
                    (
                        e["entity"].as_u64().unwrap(),
                        e["kind"].as_str().unwrap().to_string(),
                    )
                })
                .collect();
            per_tick.push(batch);
        }
        let balances = vec![
            (stockpile_amount(&world, low, "grain"), low as f64),
            (stockpile_amount(&world, high, "grain"), high as f64),
        ];
        (balances, per_tick)
    }

    let (first_balances, first_ticks) = run_two_ticks();
    let (second_balances, second_ticks) = run_two_ticks();
    assert_eq!(first_balances, second_balances);
    assert_eq!(first_balances[0].0, 1.0);
    assert_eq!(first_balances[1].0, 1.0);
    assert_eq!(
        first_ticks, second_ticks,
        "repeat runs must match tick by tick"
    );
    assert_eq!(first_ticks.len(), 2);
    assert_eq!(
        first_ticks[0].len(),
        1,
        "only the low-balance carrier shorts first"
    );
    for (n, batch) in first_ticks.iter().enumerate() {
        let mut sorted = batch.clone();
        sorted.sort();
        assert_eq!(
            *batch, sorted,
            "tick {n} events follow ascending entity-id order"
        );
    }
    assert_eq!(
        first_ticks[1].len(),
        2,
        "both carriers short once the low balance runs out"
    );
}

/// A shorted entity persists with no side effects from consumption alone.
#[test]
fn leaves_shorted_entity_alive_without_side_effects() {
    let mut world = make_test_world();
    let eid = spawn_upkeep(
        &mut world,
        json!({ "grain": 1.0 }),
        json!([{ "kind": "grain", "amount_per_tick": 2.0 }]),
    );
    let before = stockpile_snapshot(&world, eid);

    run_consumption(&mut world);

    assert!(
        world.entities.contains(&eid),
        "shortage must not despawn the entity"
    );
    assert!(
        world.get_component(eid, "Health").is_none(),
        "consumption alone adds no health side effect"
    );
    assert_eq!(stockpile_snapshot(&world, eid), before);
    assert_eq!(shortage_events(&mut world).len(), 1);
}

/// Same-tick production lands before the upkeep drain.
#[test]
fn applies_production_before_consumption_within_one_tick() {
    let mut world = make_test_world();
    let eid = spawn_upkeep(
        &mut world,
        json!({ "grain": 0.0 }),
        json!([{ "kind": "grain", "amount_per_tick": 2.0 }]),
    );
    world
        .set_component(
            eid,
            "ProductionJob",
            json!({ "recipe": "grain_bounty", "progress": 0, "state": "pending" }),
        )
        .unwrap();
    let recipe = Recipe {
        name: "grain_bounty".to_string(),
        inputs: vec![],
        outputs: vec![ResourceAmount {
            kind: "grain".to_string(),
            amount: 5,
        }],
        duration: 1,
        tools: vec![],
        materials: vec![],
        required_skill: None,
        output_item: None,
        station: None,
        xp: None,
    };
    let mut economic = EconomicSystem::with_recipes(vec![recipe]);

    economic.run(&mut world);
    run_consumption(&mut world);

    assert_eq!(
        stockpile_amount(&world, eid, "grain"),
        3.0,
        "production (+5) must land before upkeep (-2) in the same tick"
    );

    let order: Vec<&str> = engine_core::systems::SYSTEM_EXECUTION_ORDER.to_vec();
    let economic_pos = order
        .iter()
        .position(|name| *name == "EconomicSystem")
        .unwrap();
    let consumption_pos = order
        .iter()
        .position(|name| *name == "ConsumptionSystem")
        .unwrap();
    let crafting_pos = order
        .iter()
        .position(|name| *name == "CraftingSystem")
        .unwrap();
    assert_eq!(consumption_pos, economic_pos + 1);
    assert_eq!(crafting_pos, consumption_pos + 1);
}

/// Dust-level shortfalls within epsilon succeed and floor at zero.
#[test]
fn floors_dust_remainder_to_zero_without_shortage_event() {
    let mut world = make_test_world();
    let eid = spawn_upkeep(
        &mut world,
        json!({ "grain": 1.0 }),
        json!([{ "kind": "grain", "amount_per_tick": 1.0 + 5e-10 }]),
    );

    run_consumption(&mut world);

    assert_eq!(stockpile_amount(&world, eid, "grain"), 0.0);
    assert!(
        shortage_events(&mut world).is_empty(),
        "a shortfall within epsilon counts as paid"
    );
}

/// Entities without Upkeep are never drained; Upkeep without Stockpile is skipped.
#[test]
fn spares_entities_without_upkeep_and_reports_stockpile_less_carriers() {
    let mut world = make_test_world();
    let plain = spawn_stockpile(&mut world, json!({ "grain": 5.0 }));
    let bare = world.spawn_entity();
    world
        .set_component(
            bare,
            "Upkeep",
            json!({ "drains": [{ "kind": "grain", "amount_per_tick": 2.0 }] }),
        )
        .unwrap();

    run_consumption(&mut world);

    assert_eq!(
        stockpile_amount(&world, plain, "grain"),
        5.0,
        "an entity without Upkeep is never drained"
    );
    assert!(
        world.get_component(bare, "Stockpile").is_none(),
        "a stockpile-less carrier gains no stockpile"
    );
    let events = shortage_events(&mut world);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["entity"].as_u64(), Some(bare as u64));
    assert_eq!(events[0]["kind"].as_str(), Some("grain"));
    assert_eq!(events[0]["required"].as_f64(), Some(2.0));
    assert_eq!(events[0]["available"].as_f64(), Some(0.0));
}
