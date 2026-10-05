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
use engine_core::ecs::world::World;
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::trade::{
    TradeError, TransferError, execute_treaty_trade, has_active_trade_treaty,
    transfer_stockpile_resource,
};
use serde_json::json;

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
