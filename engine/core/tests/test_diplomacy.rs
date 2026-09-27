//! Standing suite for the Diplomacy AI module (relationships, treaties, war).
//!
//! Created in M1 as a skeleton alongside the F-01 reputation-shape
//! precondition fix. Grows milestone by milestone (M2–M5 extend in place).

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::diplomacy::{
    RelationState, TreatyKind, TreatyStatus, accept_treaty, break_treaty, can_accept, can_break,
    can_propose, declare_peace, declare_war, expire_due_treaties, get_relation, get_standing,
    list_treaties, modify_standing, propose_treaty,
};
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::ComponentSchema;
use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use engine_core::map::{Map, SquareGridMap};
use std::collections::HashSet;

/// Build an open plane map with 8-directional adjacency.
fn open_plane(size: i32) -> Map {
    let mut grid = SquareGridMap::new();
    for x in -size..=size {
        for y in -size..=size {
            grid.add_cell(x, y, 0);
        }
    }
    for x in -size..=size {
        for y in -size..=size {
            for dx in [-1, 0, 1] {
                for dy in [-1, 0, 1] {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx >= -size && nx <= size && ny >= -size && ny <= size {
                        grid.add_neighbor((x, y, 0), (nx, ny, 0));
                    }
                }
            }
        }
    }
    Map::new(Box::new(grid))
}

/// Create a world with all schemas and a map, ready for diplomacy testing.
fn setup_world() -> World {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    world.map = Some(open_plane(10));
    {
        let mut reg = world.registry.lock().unwrap();
        register_diplomacy_schemas(&mut reg);
    }
    world
}

/// Registers the Diplomacy + Treaty component schemas from the on-disk files.
///
/// Kept as a helper so schema registration is explicit per test setup rather
/// than relying solely on the directory auto-load in `make_test_world`.
fn register_diplomacy_schemas(reg: &mut ComponentRegistry) {
    reg.register_external_schema(ComponentSchema {
        name: "Diplomacy".to_string(),
        schema: serde_json::from_str(include_str!("../../assets/schemas/diplomacy.json")).unwrap(),
        modes: vec![
            "colony".to_string(),
            "roguelike".to_string(),
            "simulation".to_string(),
        ],
    });
    reg.register_external_schema(ComponentSchema {
        name: "Treaty".to_string(),
        schema: serde_json::from_str(include_str!("../../assets/schemas/treaty.json")).unwrap(),
        modes: vec![
            "colony".to_string(),
            "roguelike".to_string(),
            "simulation".to_string(),
        ],
    });
}

/// Pair lookups behave identically regardless of argument order.
#[test]
fn pair_lookup_is_order_independent() {
    let mut world = setup_world();
    modify_standing(&mut world, "a", "b", 30).unwrap();
    assert_eq!(
        get_standing(&world, "b", "a"),
        get_standing(&world, "a", "b")
    );
    assert_eq!(get_standing(&world, "b", "a"), 30);
    assert_eq!(
        get_relation(&world, "b", "a"),
        get_relation(&world, "a", "b")
    );
}

/// A faction cannot hold a diplomatic relation with itself.
#[test]
fn self_pair_mutation_is_rejected() {
    let mut world = setup_world();
    assert!(modify_standing(&mut world, "a", "a", 10).is_err());
    world.update_event_buses::<serde_json::Value>();
    let events: Vec<serde_json::Value> = world.drain_events("relation_changed");
    assert!(events.is_empty(), "rejected mutation must emit no event");
}

/// Empty faction ids are invalid on every mutating path.
#[test]
fn empty_faction_id_is_rejected() {
    let mut world = setup_world();
    assert!(modify_standing(&mut world, "", "b", 10).is_err());
    assert!(modify_standing(&mut world, "a", "", 10).is_err());
    world.update_event_buses::<serde_json::Value>();
    let events: Vec<serde_json::Value> = world.drain_events("relation_changed");
    assert!(events.is_empty(), "rejected mutation must emit no event");
}

/// Standing clamps to ±100 on large deltas and on accumulation.
#[test]
fn standing_clamps_to_bounds() {
    let mut world = setup_world();
    modify_standing(&mut world, "a", "b", 200).unwrap();
    assert_eq!(get_standing(&world, "a", "b"), 100);
    modify_standing(&mut world, "c", "d", -200).unwrap();
    assert_eq!(get_standing(&world, "c", "d"), -100);
    modify_standing(&mut world, "e", "f", 90).unwrap();
    modify_standing(&mut world, "e", "f", 30).unwrap();
    assert_eq!(get_standing(&world, "e", "f"), 100);
}

/// Unknown pairs report neutral defaults without creating state.
#[test]
fn unknown_pair_returns_neutral_defaults() {
    let world = setup_world();
    assert_eq!(get_relation(&world, "a", "b"), RelationState::Neutral);
    assert_eq!(get_standing(&world, "a", "b"), 0);
}

/// Each standing mutation emits exactly one relation event.
#[test]
fn standing_mutation_emits_single_event() {
    let mut world = setup_world();
    modify_standing(&mut world, "a", "b", 30).unwrap();
    world.update_event_buses::<serde_json::Value>();
    let events: Vec<serde_json::Value> = world.drain_events("relation_changed");
    assert_eq!(events.len(), 1, "expected exactly one event per mutation");
    let event = &events[0];
    assert_eq!(event.get("a").and_then(|v| v.as_str()), Some("a"));
    assert_eq!(event.get("b").and_then(|v| v.as_str()), Some("b"));
    assert_eq!(event.get("old_standing").and_then(|v| v.as_i64()), Some(0));
    assert_eq!(event.get("new_standing").and_then(|v| v.as_i64()), Some(30));
    assert_eq!(event.get("state").and_then(|v| v.as_str()), Some("Neutral"));
}

/// Drain one event bus after flushing the bus registry.
fn drain_flushed(world: &mut World, name: &str) -> Vec<serde_json::Value> {
    world.update_event_buses::<serde_json::Value>();
    world.drain_events(name)
}

/// Spawn an EnemyAI actor of one faction plus a victim entity of another,
/// with positions two cells apart on the open plane. Returns (enemy, victim).
fn spawn_war_pair(world: &mut World, my_faction: &str, target_faction: &str) -> (u32, u32) {
    use serde_json::json;

    let enemy = world.spawn_entity();
    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "idle",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": target_faction,
                "target_entity": null
            }),
        )
        .unwrap();
    world
        .set_component(
            enemy,
            "Position",
            json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(enemy, "Health", json!({"current": 10.0, "max": 10.0}))
        .unwrap();
    world
        .set_component(enemy, "Sight", json!({"range": 10}))
        .unwrap();
    world
        .set_component(
            enemy,
            "Faction",
            json!({"faction_id": my_faction, "role": "enemy"}),
        )
        .unwrap();
    world
        .set_component(enemy, "Type", json!({"kind": "enemy"}))
        .unwrap();

    let victim = world.spawn_entity();
    world
        .set_component(
            victim,
            "Position",
            json!({"pos": {"Square": {"x": 3, "y": 0, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(victim, "Health", json!({"current": 10.0, "max": 10.0}))
        .unwrap();
    world
        .set_component(victim, "Type", json!({"kind": "villager"}))
        .unwrap();
    world
        .set_component(
            victim,
            "Faction",
            json!({"faction_id": target_faction, "role": "member"}),
        )
        .unwrap();
    (enemy, victim)
}

/// Make both war-pair cells visible to the observer.
fn make_pair_visible(world: &mut World, observer: u32) {
    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    visible.insert(CellKey::Square { x: 3, y: 0, z: 0 });
    world.set_visible_cells(observer, visible);
}

/// Read the EnemyAI state string for an entity.
fn ai_state(world: &World, enemy: u32) -> String {
    world
        .get_component(enemy, "EnemyAI")
        .and_then(|ai| ai.get("state"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .unwrap()
}

/// Proposing a treaty records it and emits exactly one proposal event.
#[test]
fn propose_treaty_records_and_emits_event() {
    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::NonAggression, Some(10)).unwrap();

    let listed = list_treaties(&world, None);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, id);
    assert_eq!(listed[0].status, TreatyStatus::Proposed);

    let events = drain_flushed(&mut world, "treaty_proposed");
    assert_eq!(events.len(), 1, "expected exactly one proposal event");
    assert_eq!(
        events[0].get("treaty_id").and_then(|v| v.as_u64()),
        Some(id)
    );
}

/// A duplicate same-kind treaty on one pair is rejected without an event.
#[test]
fn duplicate_propose_same_kind_is_rejected() {
    let mut world = setup_world();
    propose_treaty(&mut world, "a", "b", TreatyKind::NonAggression, Some(10)).unwrap();
    drain_flushed(&mut world, "treaty_proposed");

    assert!(can_propose(&world, "a", "b", &TreatyKind::NonAggression).is_err());
    assert!(propose_treaty(&mut world, "b", "a", TreatyKind::NonAggression, Some(5)).is_err());

    let events = drain_flushed(&mut world, "treaty_proposed");
    assert!(events.is_empty(), "rejected proposal must emit no event");

    let other_kind = propose_treaty(&mut world, "a", "b", TreatyKind::Alliance, Some(5));
    assert!(other_kind.is_ok(), "different kind on same pair is allowed");
}

/// Non-peace proposals are barred while the pair is at war; peace is allowed.
#[test]
fn propose_non_peace_during_war_is_rejected() {
    let mut world = setup_world();
    declare_war(&mut world, "a", "b").unwrap();
    drain_flushed(&mut world, "treaty_proposed");

    assert!(propose_treaty(&mut world, "a", "b", TreatyKind::NonAggression, Some(5)).is_err());
    assert!(propose_treaty(&mut world, "a", "b", TreatyKind::Alliance, Some(5)).is_err());
    assert!(propose_treaty(&mut world, "a", "b", TreatyKind::TradeStub, Some(5)).is_err());

    let events = drain_flushed(&mut world, "treaty_proposed");
    assert!(events.is_empty(), "rejected proposal must emit no event");

    let peace = propose_treaty(&mut world, "a", "b", TreatyKind::Peace, Some(5));
    assert!(peace.is_ok(), "peace proposal during war is allowed");
    declare_peace(&mut world, "a", "b").unwrap();
}

/// Accepting a proposal activates it and emits exactly one signed event.
#[test]
fn accept_treaty_activates_and_emits_event() {
    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::NonAggression, Some(10)).unwrap();
    accept_treaty(&mut world, id).unwrap();

    let listed = list_treaties(&world, None);
    assert_eq!(listed[0].status, TreatyStatus::Active);

    let events = drain_flushed(&mut world, "treaty_signed");
    assert_eq!(events.len(), 1, "expected exactly one signed event");
    assert_eq!(
        events[0].get("treaty_id").and_then(|v| v.as_u64()),
        Some(id)
    );
}

/// Alliance acceptance raises the pair to Allied.
#[test]
fn accept_alliance_moves_state_to_allied() {
    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::Alliance, Some(10)).unwrap();
    accept_treaty(&mut world, id).unwrap();
    assert_eq!(get_relation(&world, "a", "b"), RelationState::Allied);
}

/// Peace acceptance on a neutral pair keeps Neutral or better.
#[test]
fn accept_peace_on_neutral_keeps_neutral_or_better() {
    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::Peace, Some(10)).unwrap();
    accept_treaty(&mut world, id).unwrap();
    let state = get_relation(&world, "a", "b");
    assert!(
        state == RelationState::Neutral || state == RelationState::Allied,
        "peace must keep Neutral-or-better, got {state:?}"
    );
}

/// Non-aggression acceptance lifts a hostile pair to Neutral.
#[test]
fn accept_non_aggression_lifts_hostile_to_neutral() {
    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::NonAggression, Some(10)).unwrap();
    accept_treaty(&mut world, id).unwrap();
    assert_eq!(get_relation(&world, "a", "b"), RelationState::Neutral);
}

/// Double acceptance and acceptance of unknown ids are rejected silently.
#[test]
fn double_accept_is_rejected() {
    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::Alliance, Some(10)).unwrap();
    accept_treaty(&mut world, id).unwrap();
    drain_flushed(&mut world, "treaty_signed");

    assert!(can_accept(&world, id).is_err());
    assert!(accept_treaty(&mut world, id).is_err());
    assert!(accept_treaty(&mut world, 9999).is_err());

    let events = drain_flushed(&mut world, "treaty_signed");
    assert!(events.is_empty(), "rejected acceptance must emit no event");
}

/// Breaking an active treaty marks it broken, applies the penalty, and emits.
#[test]
fn break_active_treaty_applies_penalty_and_emits() {
    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::Alliance, Some(10)).unwrap();
    accept_treaty(&mut world, id).unwrap();
    break_treaty(&mut world, id).unwrap();

    let listed = list_treaties(&world, None);
    assert_eq!(listed[0].status, TreatyStatus::Broken);
    assert_eq!(get_standing(&world, "a", "b"), -25);

    let events = drain_flushed(&mut world, "treaty_broken");
    assert_eq!(events.len(), 1, "expected exactly one broken event");
    assert_eq!(
        events[0].get("treaty_id").and_then(|v| v.as_u64()),
        Some(id)
    );
    assert_eq!(events[0].get("penalty").and_then(|v| v.as_i64()), Some(-25));
}

/// Breaking an expired treaty or an unknown id is rejected silently.
#[test]
fn break_after_expire_is_rejected() {
    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::NonAggression, Some(0)).unwrap();
    accept_treaty(&mut world, id).unwrap();
    expire_due_treaties(&mut world).unwrap();
    assert_eq!(list_treaties(&world, None)[0].status, TreatyStatus::Expired);
    drain_flushed(&mut world, "treaty_broken");

    assert!(can_break(&world, id).is_err());
    assert!(break_treaty(&mut world, id).is_err());
    assert!(break_treaty(&mut world, 9999).is_err());

    let events = drain_flushed(&mut world, "treaty_broken");
    assert!(events.is_empty(), "rejected break must emit no event");
}

/// Active treaties reaching their expiry tick expire with one event each.
#[test]
fn expiry_at_turn_boundary_expires_with_event() {
    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::NonAggression, Some(5)).unwrap();
    accept_treaty(&mut world, id).unwrap();

    world.turn = 4;
    expire_due_treaties(&mut world).unwrap();
    assert_eq!(list_treaties(&world, None)[0].status, TreatyStatus::Active);
    assert!(drain_flushed(&mut world, "treaty_expired").is_empty());

    world.turn = 5;
    expire_due_treaties(&mut world).unwrap();
    assert_eq!(list_treaties(&world, None)[0].status, TreatyStatus::Expired);
    let events = drain_flushed(&mut world, "treaty_expired");
    assert_eq!(events.len(), 1, "expected exactly one expiry event");
    assert_eq!(
        events[0].get("treaty_id").and_then(|v| v.as_u64()),
        Some(id)
    );
}

/// Treaties without a duration never expire.
#[test]
fn none_duration_treaty_never_expires() {
    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::Alliance, None).unwrap();
    accept_treaty(&mut world, id).unwrap();

    world.turn = 100;
    expire_due_treaties(&mut world).unwrap();
    assert_eq!(list_treaties(&world, None)[0].status, TreatyStatus::Active);
    assert!(drain_flushed(&mut world, "treaty_expired").is_empty());
}

/// Trade treaties run the full lifecycle without touching pair state.
#[test]
fn trade_treaty_completes_lifecycle_without_state_change() {
    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::TradeStub, Some(5)).unwrap();
    accept_treaty(&mut world, id).unwrap();
    assert_eq!(get_relation(&world, "a", "b"), RelationState::Neutral);
    assert_eq!(get_standing(&world, "a", "b"), 0);

    world.turn = 5;
    expire_due_treaties(&mut world).unwrap();
    assert_eq!(list_treaties(&world, None)[0].status, TreatyStatus::Expired);
    assert_eq!(get_relation(&world, "a", "b"), RelationState::Neutral);
    assert_eq!(get_standing(&world, "a", "b"), 0);

    world.update_event_buses::<serde_json::Value>();
    let proposed: Vec<serde_json::Value> = world.drain_events("treaty_proposed");
    let signed: Vec<serde_json::Value> = world.drain_events("treaty_signed");
    let expired: Vec<serde_json::Value> = world.drain_events("treaty_expired");
    assert_eq!(proposed.len(), 1);
    assert_eq!(signed.len(), 1);
    assert_eq!(expired.len(), 1);
}

/// Treaty listings filter by faction membership.
#[test]
fn list_treaties_filters_by_faction() {
    let mut world = setup_world();
    propose_treaty(&mut world, "a", "b", TreatyKind::NonAggression, Some(10)).unwrap();
    propose_treaty(&mut world, "c", "d", TreatyKind::Alliance, Some(10)).unwrap();

    assert_eq!(list_treaties(&world, None).len(), 2);
    assert_eq!(list_treaties(&world, Some("a")).len(), 1);
    assert_eq!(list_treaties(&world, Some("d")).len(), 1);
    assert!(list_treaties(&world, Some("zzz")).is_empty());
}

/// Regression test for the F-01 precondition: hostile targeting reads the
/// production flat `values` shape written by `modify_reputation`.
#[test]
fn reputation_gated_targeting_uses_production_shape() {
    use engine_core::ecs::system::System;
    use engine_core::faction::modify_reputation;
    use engine_core::systems::enemy_behavior::EnemyBehaviorSystem;
    use serde_json::json;

    let mut world = setup_world();

    let enemy = world.spawn_entity();
    world
        .set_component(
            enemy,
            "EnemyAI",
            json!({
                "state": "idle",
                "alert_level": 0,
                "detection_range": 10,
                "attack_range": 1,
                "flee_threshold": 0.25,
                "target_faction": "players",
                "target_entity": null
            }),
        )
        .unwrap();
    world
        .set_component(
            enemy,
            "Position",
            json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(enemy, "Health", json!({"current": 10.0, "max": 10.0}))
        .unwrap();
    world
        .set_component(enemy, "Sight", json!({"range": 10}))
        .unwrap();
    world
        .set_component(
            enemy,
            "Faction",
            json!({"faction_id": "raiders", "role": "enemy"}),
        )
        .unwrap();
    world
        .set_component(enemy, "Type", json!({"kind": "enemy"}))
        .unwrap();

    let player = world.spawn_entity();
    world
        .set_component(
            player,
            "Position",
            json!({"pos": {"Square": {"x": 3, "y": 0, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(player, "Health", json!({"current": 10.0, "max": 10.0}))
        .unwrap();
    world
        .set_component(player, "Type", json!({"kind": "villager"}))
        .unwrap();
    world
        .set_component(
            player,
            "Faction",
            json!({"faction_id": "players", "role": "member"}),
        )
        .unwrap();
    modify_reputation(&mut world, player, "raiders", -50).unwrap();

    let mut visible = HashSet::new();
    visible.insert(CellKey::Square { x: 0, y: 0, z: 0 });
    visible.insert(CellKey::Square { x: 3, y: 0, z: 0 });
    world.set_visible_cells(enemy, visible);

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    let state = world
        .get_component(enemy, "EnemyAI")
        .and_then(|ai| ai.get("state"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_string())
        .unwrap();
    assert_eq!(
        state, "chase",
        "Enemy should detect and chase hostile faction member written via production shape"
    );
}

/// Declaring war sets the pair to War and emits exactly one war event.
#[test]
fn war_declaration_sets_war_and_emits_event() {
    let mut world = setup_world();
    declare_war(&mut world, "a", "b").unwrap();
    assert_eq!(get_relation(&world, "a", "b"), RelationState::War);

    let events = drain_flushed(&mut world, "war_declared");
    assert_eq!(events.len(), 1, "expected exactly one war event");
    assert_eq!(
        events[0].get("aggressor").and_then(|v| v.as_str()),
        Some("a")
    );
    assert_eq!(
        events[0].get("defender").and_then(|v| v.as_str()),
        Some("b")
    );
}

/// A second war declaration on a war pair is rejected without a new event.
#[test]
fn double_war_declaration_is_rejected() {
    use engine_core::diplomacy::can_war;

    let mut world = setup_world();
    declare_war(&mut world, "a", "b").unwrap();
    drain_flushed(&mut world, "war_declared");

    assert!(can_war(&world, "a", "b").is_err());
    assert!(declare_war(&mut world, "b", "a").is_err());

    let events = drain_flushed(&mut world, "war_declared");
    assert!(events.is_empty(), "rejected war must emit no event");
}

/// War auto-breaks live paper treaties on the pair, each with its own event.
#[test]
fn war_breaks_paper_treaties() {
    let mut world = setup_world();
    let pact = propose_treaty(&mut world, "a", "b", TreatyKind::NonAggression, Some(10)).unwrap();
    accept_treaty(&mut world, pact).unwrap();
    let pact2 = propose_treaty(&mut world, "a", "b", TreatyKind::Alliance, Some(10)).unwrap();
    accept_treaty(&mut world, pact2).unwrap();

    declare_war(&mut world, "a", "b").unwrap();
    assert_eq!(get_relation(&world, "a", "b"), RelationState::War);

    let listed = list_treaties(&world, None);
    let status_of = |id: u64| {
        listed
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.status)
            .unwrap()
    };
    assert_eq!(status_of(pact), TreatyStatus::Broken);
    assert_eq!(status_of(pact2), TreatyStatus::Broken);

    let events = drain_flushed(&mut world, "treaty_broken");
    assert_eq!(events.len(), 2, "one broken event per auto-broken treaty");
}

/// Trade treaties are not paper: war leaves an active trade treaty in force.
#[test]
fn war_leaves_trade_treaty_active() {
    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::TradeStub, Some(10)).unwrap();
    accept_treaty(&mut world, id).unwrap();

    declare_war(&mut world, "a", "b").unwrap();
    assert_eq!(list_treaties(&world, None)[0].status, TreatyStatus::Active);
}

/// Peace on a war pair resets to Neutral with zero standing and emits.
#[test]
fn peace_resets_war_pair_to_neutral_zero() {
    use engine_core::diplomacy::can_peace;

    let mut world = setup_world();
    declare_war(&mut world, "a", "b").unwrap();
    modify_standing(&mut world, "a", "b", 60).unwrap();
    drain_flushed(&mut world, "war_declared");
    drain_flushed(&mut world, "relation_changed");

    declare_peace(&mut world, "a", "b").unwrap();
    assert_eq!(get_relation(&world, "a", "b"), RelationState::Neutral);
    assert_eq!(get_standing(&world, "a", "b"), 0);

    let events = drain_flushed(&mut world, "peace_declared");
    assert_eq!(events.len(), 1, "expected exactly one peace event");

    assert!(can_peace(&world, "a", "b").is_err());
    assert!(declare_peace(&mut world, "a", "b").is_err());
    let again = drain_flushed(&mut world, "peace_declared");
    assert!(again.is_empty(), "rejected peace must emit no event");
}

/// Peace on a non-war pair is rejected without an event.
#[test]
fn peace_on_non_war_pair_is_rejected() {
    let mut world = setup_world();
    assert!(declare_peace(&mut world, "a", "b").is_err());
    let events = drain_flushed(&mut world, "peace_declared");
    assert!(events.is_empty(), "rejected peace must emit no event");
}

/// The treaty Peace path clears war with the same end state as declare_peace.
#[test]
fn treaty_peace_accept_on_war_pair_clears_war() {
    let mut world = setup_world();
    declare_war(&mut world, "a", "b").unwrap();
    modify_standing(&mut world, "a", "b", -40).unwrap();

    let id = propose_treaty(&mut world, "a", "b", TreatyKind::Peace, Some(10)).unwrap();
    accept_treaty(&mut world, id).unwrap();
    assert_eq!(get_relation(&world, "a", "b"), RelationState::Neutral);
    assert_eq!(get_standing(&world, "a", "b"), 0);
}

/// War-pair targeting ignores entity reputation: zero-rep victims are chased.
#[test]
fn war_pair_targeting_ignores_reputation() {
    use engine_core::ecs::system::System;
    use engine_core::systems::enemy_behavior::EnemyBehaviorSystem;

    let mut world = setup_world();
    let (enemy, _victim) = spawn_war_pair(&mut world, "raiders", "players");
    declare_war(&mut world, "raiders", "players").unwrap();
    make_pair_visible(&mut world, enemy);

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    assert_eq!(
        ai_state(&world, enemy),
        "chase",
        "war-pair victim with no negative reputation must still be chased"
    );
}

/// War never grants omniscience: a blind attacker finds no target.
#[test]
fn blind_attacker_finds_no_target_during_war() {
    use engine_core::ecs::system::System;
    use engine_core::systems::enemy_behavior::EnemyBehaviorSystem;

    let mut world = setup_world();
    let (enemy, _victim) = spawn_war_pair(&mut world, "raiders", "players");
    declare_war(&mut world, "raiders", "players").unwrap();

    let mut system = EnemyBehaviorSystem;
    system.run(&mut world);

    assert_eq!(
        ai_state(&world, enemy),
        "idle",
        "blind attacker must stay idle even during war"
    );
}

/// DiplomacySystem expires due treaties through the tick path with events.
#[test]
fn diplomacy_system_expires_due_treaties() {
    use engine_core::ecs::system::System;
    use engine_core::systems::diplomacy::DiplomacySystem;

    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::NonAggression, Some(5)).unwrap();
    accept_treaty(&mut world, id).unwrap();

    world.turn = 5;
    let mut system = DiplomacySystem;
    system.run(&mut world);

    assert_eq!(list_treaties(&world, None)[0].status, TreatyStatus::Expired);
    let events = drain_flushed(&mut world, "treaty_expired");
    assert_eq!(events.len(), 1, "expected exactly one expiry event");
    assert_eq!(
        events[0].get("treaty_id").and_then(|v| v.as_u64()),
        Some(id)
    );
}

/// DiplomacySystem leaves treaties without duration in force across 100 ticks.
#[test]
fn diplomacy_system_keeps_timeless_treaty_for_100_ticks() {
    use engine_core::ecs::system::System;
    use engine_core::systems::diplomacy::DiplomacySystem;

    let mut world = setup_world();
    let id = propose_treaty(&mut world, "a", "b", TreatyKind::Alliance, None).unwrap();
    accept_treaty(&mut world, id).unwrap();

    world.turn = 100;
    let mut system = DiplomacySystem;
    system.run(&mut world);

    assert_eq!(list_treaties(&world, None)[0].status, TreatyStatus::Active);
    assert!(drain_flushed(&mut world, "treaty_expired").is_empty());
}

/// DiplomacySystem sits strictly between reputation settlement and targeting.
#[test]
fn execution_order_places_diplomacy_between_reputation_and_enemy() {
    use engine_core::systems::SYSTEM_EXECUTION_ORDER;

    let pos = |name: &str| {
        SYSTEM_EXECUTION_ORDER
            .iter()
            .position(|n| *n == name)
            .unwrap_or_else(|| panic!("{name} missing from SYSTEM_EXECUTION_ORDER"))
    };
    assert!(
        pos("DiplomacySystem") > pos("FactionReputationSystem"),
        "DiplomacySystem must run after FactionReputationSystem"
    );
    assert!(
        pos("DiplomacySystem") < pos("EnemyBehaviorSystem"),
        "DiplomacySystem must run before EnemyBehaviorSystem"
    );
}

/// Diplomacy + Treaty schemas are registered in the test world registry.
#[test]
fn diplomacy_schemas_are_registered() {
    let world = setup_world();
    let reg = world.registry.lock().unwrap();
    let names = reg.all_component_names();
    assert!(
        names.contains(&"Diplomacy".to_string()),
        "Diplomacy schema must be registered"
    );
    assert!(
        names.contains(&"Treaty".to_string()),
        "Treaty schema must be registered"
    );
}

/// Whole-world save/load preserves relation state, standing, every treaty
/// record/status, and the monotonic id counter with no per-system save code.
#[test]
fn save_load_roundtrip_preserves_diplomacy_store() {
    use engine_core::ecs::system::System;
    use engine_core::systems::diplomacy::DiplomacySystem;

    let mut world = setup_world();

    modify_standing(&mut world, "a", "b", 45).unwrap();

    let active_id = propose_treaty(&mut world, "c", "d", TreatyKind::Alliance, Some(50)).unwrap();
    accept_treaty(&mut world, active_id).unwrap();

    let proposed_id =
        propose_treaty(&mut world, "e", "f", TreatyKind::NonAggression, Some(20)).unwrap();

    let broken_id = propose_treaty(&mut world, "g", "h", TreatyKind::TradeStub, Some(30)).unwrap();
    accept_treaty(&mut world, broken_id).unwrap();
    break_treaty(&mut world, broken_id).unwrap();

    let expired_id = propose_treaty(&mut world, "i", "j", TreatyKind::Peace, Some(5)).unwrap();
    accept_treaty(&mut world, expired_id).unwrap();
    world.turn = 5;
    let mut system = DiplomacySystem;
    system.run(&mut world);

    declare_war(&mut world, "k", "l").unwrap();

    let before_treaties = list_treaties(&world, None);

    let file = tempfile::NamedTempFile::new().unwrap();
    world.save_to_file(file.path()).unwrap();
    let mut loaded = World::load_from_file(file.path(), world.registry.clone()).unwrap();

    assert_eq!(get_relation(&loaded, "a", "b"), RelationState::Neutral);
    assert_eq!(get_standing(&loaded, "a", "b"), 45);
    assert_eq!(get_relation(&loaded, "c", "d"), RelationState::Allied);
    assert_eq!(get_relation(&loaded, "k", "l"), RelationState::War);
    assert_eq!(get_standing(&loaded, "g", "h"), -25);

    assert_eq!(list_treaties(&loaded, None), before_treaties);
    let status_of = |id: u64| {
        list_treaties(&loaded, None)
            .into_iter()
            .find(|t| t.id == id)
            .unwrap()
            .status
    };
    assert_eq!(status_of(active_id), TreatyStatus::Active);
    assert_eq!(status_of(proposed_id), TreatyStatus::Proposed);
    assert_eq!(status_of(broken_id), TreatyStatus::Broken);
    assert_eq!(status_of(expired_id), TreatyStatus::Expired);

    let next_id = propose_treaty(&mut world, "m", "n", TreatyKind::Peace, None).unwrap();
    let loaded_next_id = propose_treaty(&mut loaded, "m", "n", TreatyKind::Peace, None).unwrap();
    assert_eq!(loaded_next_id, next_id);
}
