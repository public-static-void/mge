//! Standing suite for procedural history and lore generation.
//!
//! Created in M1 alongside the core lore types and World persistence
//! wiring. Grows milestone by milestone (M2 extends in place with feed and
//! query coverage; M3 with rendering and backfill coverage).

#[path = "helpers/world.rs"]
mod world_helper;
#[path = "helpers/world_io.rs"]
mod world_io_helper;

use world_helper::make_test_world;
use world_io_helper::save_and_load_roundtrip;

use engine_core::lore::{
    ChronicleFilter, ChronicleKind, append_chronicle_entry, chronicle_len, get_chronicle_entry,
    list_chronicle,
};
use engine_core::narrative::{register_scenario, resolve_decision};
use engine_core::systems::narrative::NarrativeSystem;
use serde_json::json;

/// Saves a world holding mixed-kind chronicle entries and asserts the load
/// preserves entries, RNG state, and the id counter, with ids continuing
/// monotonically on the next append.
#[test]
fn roundtrip_preserves_entries_rng_state_and_id_sequence() {
    let mut world = make_test_world();
    world.lore.rng_state = [5u8; 32];

    let first = append_chronicle_entry(
        &mut world,
        2,
        "food_shortage",
        ChronicleKind::Fired,
        "Turn 2: Food Shortage -- fired".to_string(),
        None,
    );
    let second = append_chronicle_entry(
        &mut world,
        2,
        "food_shortage",
        ChronicleKind::Resolved,
        "Turn 2: Food Shortage -- resolved [ration]".to_string(),
        Some("ration".to_string()),
    );
    let third = append_chronicle_entry(
        &mut world,
        4,
        "founding:first_hearth",
        ChronicleKind::Founding,
        "Turn 4: Founding of the First Hearth -- founding".to_string(),
        None,
    );
    assert_eq!((first, second, third), (0, 1, 2));
    assert_eq!(chronicle_len(&world), 3);

    let registry = world.registry.clone();
    let mut loaded = save_and_load_roundtrip(&world, registry);

    assert_eq!(loaded.lore.entries, world.lore.entries);
    assert_eq!(loaded.lore.rng_state, world.lore.rng_state);
    assert_eq!(loaded.lore.next_entry_id, 3);

    let next = append_chronicle_entry(
        &mut loaded,
        6,
        "food_shortage",
        ChronicleKind::Expired,
        "Turn 6: Food Shortage -- expired".to_string(),
        None,
    );
    assert_eq!(next, 3, "ids continue monotonically after load");
    assert_eq!(chronicle_len(&loaded), 4);
    assert_eq!(loaded.lore.next_entry_id, 4);
}

/// A save written before lore existed loads with an empty chronicle.
#[test]
fn save_without_lore_fields_loads_with_empty_chronicle() {
    let world = make_test_world();

    let mut json: serde_json::Value = serde_json::to_value(&world).expect("world serializes");
    json.as_object_mut()
        .expect("world is a JSON object")
        .remove("lore");

    let loaded: engine_core::ecs::world::World =
        serde_json::from_value(json).expect("old save loads");
    assert_eq!(chronicle_len(&loaded), 0);
    assert!(loaded.lore.entries.is_empty());
    assert_eq!(loaded.lore.next_entry_id, 0);
}

// --- Live narrative feed: fired/resolved/expired records surface same-tick ---

/// A world with the narrative tick system registered.
fn lore_tick_world() -> engine_core::ecs::world::World {
    let mut world = make_test_world();
    world.register_system(NarrativeSystem);
    world
}

/// Runs one narrative tick through the registered system.
fn run_lore_tick(world: &mut engine_core::ecs::world::World) {
    world
        .run_system("NarrativeSystem")
        .expect("narrative tick must not fail");
}

/// Registers a turn-gated scenario with two effect-free choices.
fn register_gated_scenario(world: &mut engine_core::ecs::world::World, id: &str, turn: u64) {
    let def = json!({
        "id": id,
        "name": format!("Scenario {id}"),
        "description": "lore feed scenario",
        "triggers": [{"type": "turn_gte", "turn": turn}],
        "choices": [
            {"id": "ration", "label": "Ration", "effects": []},
            {"id": "feast", "label": "Feast", "effects": []},
        ],
        "cooldown_turns": 0,
        "once": false,
        "weight": 1.0,
    })
    .to_string();
    register_scenario(world, &def).expect("scenario registers");
}

/// A fired decision is queryable through the chronicle with no extra tick.
#[test]
fn fired_decision_appears_in_chronicle_without_extra_tick() {
    let mut world = lore_tick_world();
    register_gated_scenario(&mut world, "food_shortage", 5);
    world.turn = 5;
    run_lore_tick(&mut world);

    let entries = list_chronicle(&world, ChronicleFilter::default());
    assert_eq!(entries.len(), 1, "one chronicle entry after one tick");
    assert_eq!(entries[0].scenario_id, "food_shortage");
    assert_eq!(entries[0].kind, ChronicleKind::Fired);
    assert_eq!(entries[0].turn, 5);
    assert_eq!(entries[0].entry_id, 0);
    assert_eq!(chronicle_len(&world), 1);
}

/// A decision resolved in the same tick it fired is visible without more ticks.
#[test]
fn resolution_in_same_tick_is_visible_without_further_ticks() {
    let mut world = lore_tick_world();
    register_gated_scenario(&mut world, "food_shortage", 5);
    world.turn = 5;
    run_lore_tick(&mut world);
    let id = world
        .narrative
        .pending
        .values()
        .next()
        .expect("one pending decision")
        .id;
    resolve_decision(&mut world, id, "ration").expect("resolve succeeds");

    let entries = list_chronicle(&world, ChronicleFilter::default());
    assert_eq!(entries.len(), 2, "fired plus resolved with no extra tick");
    assert_eq!(entries[0].kind, ChronicleKind::Fired);
    assert_eq!(entries[1].kind, ChronicleKind::Resolved);
    assert_eq!(entries[1].choice_id.as_deref(), Some("ration"));
    let fetched = get_chronicle_entry(&world, entries[1].entry_id).expect("resolves by id");
    assert_eq!(fetched.kind, ChronicleKind::Resolved);
}

/// A timed-out decision surfaces its expiry record on the expiry tick.
#[test]
fn expired_decision_appears_in_chronicle_on_expiry_tick() {
    let mut world = lore_tick_world();
    let def = json!({
        "id": "supply_run",
        "name": "Supply Run",
        "triggers": [{"type": "turn_gte", "turn": 0}],
        "choices": [{"id": "wait", "label": "Wait", "effects": []}],
        "cooldown_turns": 0,
        "expires_in_turns": 2,
        "once": false,
        "weight": 1.0,
    })
    .to_string();
    register_scenario(&mut world, &def).expect("scenario registers");
    world.turn = 0;
    run_lore_tick(&mut world);
    world.turn = 2;
    run_lore_tick(&mut world);

    let entries = list_chronicle(&world, ChronicleFilter::default());
    assert_eq!(entries.len(), 2, "fired plus expired");
    assert_eq!(entries[1].kind, ChronicleKind::Expired);
    assert_eq!(entries[1].turn, 2);
}

// --- Chronicle query: conjunctive filters with stable turn/id ordering ---

/// Seeds six spread-turn entries across two scenarios and three kinds,
/// deliberately inserted out of turn order so the query must re-sort.
fn seed_spread_chronicle(world: &mut engine_core::ecs::world::World) {
    let seed = |world: &mut engine_core::ecs::world::World,
                turn: u64,
                scenario: &str,
                kind: ChronicleKind,
                choice: Option<&str>| {
        append_chronicle_entry(
            world,
            turn,
            scenario,
            kind,
            format!("t{turn} {scenario} {kind:?}"),
            choice.map(str::to_string),
        )
    };
    seed(world, 9, "omen", ChronicleKind::Expired, None); // id 0
    seed(world, 3, "food_shortage", ChronicleKind::Fired, None); // id 1
    seed(
        world,
        7,
        "food_shortage",
        ChronicleKind::Resolved,
        Some("ration"),
    ); // id 2
    seed(world, 3, "omen", ChronicleKind::Fired, None); // id 3
    seed(world, 5, "food_shortage", ChronicleKind::Expired, None); // id 4
    seed(world, 7, "omen", ChronicleKind::Resolved, Some("feast")); // id 5
}

/// Collects entry ids in listed order.
fn listed_ids(world: &engine_core::ecs::world::World, filter: ChronicleFilter) -> Vec<u64> {
    list_chronicle(world, filter)
        .iter()
        .map(|entry| entry.entry_id)
        .collect()
}

/// An empty filter returns every entry ordered by turn then entry id.
#[test]
fn empty_filter_returns_all_entries_in_turn_then_id_order() {
    let mut world = lore_tick_world();
    seed_spread_chronicle(&mut world);
    assert_eq!(
        listed_ids(&world, ChronicleFilter::default()),
        vec![1, 3, 4, 2, 5, 0]
    );
}

/// Each filter dimension narrows to exactly the matching entries in order.
#[test]
fn each_filter_dimension_narrows_to_matching_entries_in_order() {
    let mut world = lore_tick_world();
    seed_spread_chronicle(&mut world);

    assert_eq!(
        listed_ids(
            &world,
            ChronicleFilter {
                scenario_id: Some("food_shortage".to_string()),
                ..Default::default()
            }
        ),
        vec![1, 4, 2]
    );
    assert_eq!(
        listed_ids(
            &world,
            ChronicleFilter {
                kind: Some(ChronicleKind::Resolved),
                ..Default::default()
            }
        ),
        vec![2, 5]
    );
    assert_eq!(
        listed_ids(
            &world,
            ChronicleFilter {
                turn_from: Some(5),
                turn_to: Some(7),
                ..Default::default()
            }
        ),
        vec![4, 2, 5]
    );
    assert_eq!(
        listed_ids(
            &world,
            ChronicleFilter {
                turn_from: Some(7),
                ..Default::default()
            }
        ),
        vec![2, 5, 0]
    );
    assert_eq!(
        listed_ids(
            &world,
            ChronicleFilter {
                turn_to: Some(3),
                ..Default::default()
            }
        ),
        vec![1, 3]
    );
}

/// Scenario, kind, and turn bounds combine conjunctively.
#[test]
fn combined_filters_intersect_across_all_dimensions() {
    let mut world = lore_tick_world();
    seed_spread_chronicle(&mut world);

    let filter = ChronicleFilter {
        scenario_id: Some("omen".to_string()),
        kind: Some(ChronicleKind::Fired),
        turn_from: Some(0),
        turn_to: Some(3),
    };
    assert_eq!(listed_ids(&world, filter), vec![3]);

    let empty = ChronicleFilter {
        scenario_id: Some("omen".to_string()),
        kind: Some(ChronicleKind::Fired),
        turn_from: Some(4),
        ..Default::default()
    };
    assert!(list_chronicle(&world, empty).is_empty());
    assert!(get_chronicle_entry(&world, 99).is_none());
}
