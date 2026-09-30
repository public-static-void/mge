//! Standing suite for inter-stockpile trade primitives and treaty queries.
//!
//! Created in M1 alongside the atomic transfer primitive and the read-only
//! trade-treaty query. Grows milestone by milestone (M2 gated execution,
//! M3 upkeep/consumption extend in place).

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::diplomacy::{
    TreatyKind, accept_treaty, break_treaty, expire_due_treaties, propose_treaty,
};
use engine_core::ecs::world::World;
use engine_core::trade::{TransferError, has_active_trade_treaty, transfer_stockpile_resource};
use serde_json::json;

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
