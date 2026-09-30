//! Standing suite for the event-driven narrative engine (incident director).
//!
//! Created in M1 alongside scenario registration. Grows milestone by
//! milestone (M2–M4 extend in place with trigger, lifecycle, and
//! persistence coverage; M5 adds bridge suites).

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::world::World;
use engine_core::narrative::{NarrativeRecord, NarrativeRecordKind};
use engine_core::narrative::{get_scenario, list_scenarios, register_scenario};
use engine_core::systems::narrative::NarrativeSystem;
use serde_json::{Value as JsonValue, json};

/// Builds a scenario definition JSON string with the given id and choices.
fn def_json(id: &str, choices: JsonValue) -> String {
    json!({
        "id": id,
        "name": format!("Scenario {id}"),
        "description": "test scenario",
        "triggers": [{"type": "turn_gte", "turn": 5}],
        "choices": choices,
        "cooldown_turns": 0,
        "once": false,
        "weight": 1.0,
    })
    .to_string()
}

/// A single effect-free choice payload.
fn single_choice() -> JsonValue {
    json!([{"id": "accept", "label": "Accept", "effects": []}])
}

/// A registered scenario is retrievable by id and appears in listings.
#[test]
fn registers_scenario_and_retrieves_it_by_id() {
    let mut world = make_test_world();
    register_scenario(&mut world, &def_json("harvest_omen", single_choice()))
        .expect("valid definition should register");
    let listed = list_scenarios(&world);
    assert_eq!(listed.len(), 1, "one scenario should be listed");
    assert_eq!(listed[0].id, "harvest_omen");
    let fetched = get_scenario(&world, "harvest_omen").expect("scenario should be found");
    assert_eq!(fetched.name, "Scenario harvest_omen");
    assert!(get_scenario(&world, "missing").is_none());
}

/// Registering a duplicate id replaces the stored definition.
#[test]
fn duplicate_registration_replaces_stored_definition() {
    let mut world = make_test_world();
    register_scenario(&mut world, &def_json("omen", single_choice())).expect("first register");
    let updated = json!({
        "id": "omen",
        "name": "Omen Revised",
        "triggers": [{"type": "turn_gte", "turn": 9}],
        "choices": [{"id": "wait", "label": "Wait"}],
    })
    .to_string();
    register_scenario(&mut world, &updated).expect("duplicate register should replace");
    let listed = list_scenarios(&world);
    assert_eq!(
        listed.len(),
        1,
        "duplicate id must not duplicate evaluation entries"
    );
    assert_eq!(listed[0].name, "Omen Revised");
}

/// A definition with zero choices is rejected.
#[test]
fn rejects_definition_with_no_choices() {
    let mut world = make_test_world();
    let err = register_scenario(&mut world, &def_json("empty", json!([])))
        .expect_err("choices: [] must be rejected");
    assert!(
        err.contains("choice"),
        "error should mention choices: {err}"
    );
    assert!(list_scenarios(&world).is_empty());
}

/// Malformed definitions are rejected and leave prior state untouched.
#[test]
fn rejects_malformed_definition_without_touching_state() {
    let mut world = make_test_world();
    register_scenario(&mut world, &def_json("keeper", single_choice())).expect("seed register");
    register_scenario(&mut world, "{not json").expect_err("invalid JSON must be rejected");
    register_scenario(
        &mut world,
        &json!({"name": "No Id", "choices": single_choice()}).to_string(),
    )
    .expect_err("missing id must be rejected");
    let listed = list_scenarios(&world);
    assert_eq!(listed.len(), 1, "failed registrations must not alter state");
    assert_eq!(listed[0].id, "keeper");
}

/// The shipped scenario content file parses and carries the scenarios array.
#[test]
fn scenario_content_file_parses_with_scenarios_array() {
    let content = std::fs::read_to_string("../../engine/assets/schemas/scenarios.json")
        .expect("scenarios.json should exist alongside the other schemas");
    let json: JsonValue = serde_json::from_str(&content).expect("content file must be valid JSON");
    assert!(
        json.get("title").is_some(),
        "schema validation requires a title"
    );
    assert!(
        json.get("scenarios").and_then(|v| v.as_array()).is_some(),
        "content file must carry a scenarios array"
    );
    let _ = engine_core::narrative::load_scenario_definitions();
}

/// An empty scenario id is rejected.
#[test]
fn rejects_definition_with_empty_id() {
    let mut world = make_test_world();
    register_scenario(&mut world, &def_json("", single_choice()))
        .expect_err("empty id must be rejected");
    assert!(list_scenarios(&world).is_empty());
}

/// Unknown predicate types still register; they gate firing, not loading.
#[test]
fn accepts_unknown_predicate_types_at_registration() {
    let mut world = make_test_world();
    let def = json!({
        "id": "strange",
        "name": "Strange",
        "triggers": [{"type": "prophecy", "doom": 7}],
        "choices": single_choice(),
    })
    .to_string();
    register_scenario(&mut world, &def).expect("unknown predicates must not block registration");
    assert_eq!(list_scenarios(&world).len(), 1);
}

// --- M2: trigger predicates, tick evaluation, RNG selection, determinism ---

/// A world with the narrative tick system registered.
fn narrative_world() -> World {
    let mut world = make_test_world();
    world.register_system(NarrativeSystem);
    world
}

/// Runs one narrative tick through the registered system.
fn run_narrative_tick(world: &mut World) {
    world
        .run_system("NarrativeSystem")
        .expect("narrative tick must not fail");
}

/// Drains one event bus after flushing the bus registry.
fn drain_flushed(world: &mut World, name: &str) -> Vec<JsonValue> {
    world.update_event_buses::<JsonValue>();
    world.drain_events(name)
}

/// Builds a scenario definition JSON string with custom triggers.
fn custom_def(id: &str, triggers: JsonValue) -> String {
    json!({
        "id": id,
        "name": format!("Scenario {id}"),
        "triggers": triggers,
        "choices": single_choice(),
    })
    .to_string()
}

/// Builds a scenario definition JSON string with custom triggers and weight.
fn weighted_def(id: &str, triggers: JsonValue, weight: f64) -> String {
    json!({
        "id": id,
        "name": format!("Scenario {id}"),
        "triggers": triggers,
        "choices": single_choice(),
        "weight": weight,
    })
    .to_string()
}

/// Builds a scenario definition JSON string with a chance trigger.
fn chance_def(id: &str, p: f64) -> String {
    custom_def(id, json!([{"type": "chance", "p": p}]))
}

/// A turn-gated scenario fires only once its turn arrives.
#[test]
fn predicate_turn_gte_gates_firing() {
    let mut world = narrative_world();
    register_scenario(
        &mut world,
        &custom_def("omen", json!([{"type": "turn_gte", "turn": 5}])),
    )
    .expect("register");
    for turn in 0..5 {
        world.turn = turn;
        run_narrative_tick(&mut world);
        assert!(
            drain_flushed(&mut world, "narrative_fired").is_empty(),
            "no firing before turn 5 (turn {turn})"
        );
    }
    world.turn = 5;
    run_narrative_tick(&mut world);
    let events = drain_flushed(&mut world, "narrative_fired");
    assert_eq!(events.len(), 1, "exactly one firing at turn 5");
    assert_eq!(
        events[0].get("scenario_id").and_then(|v| v.as_str()),
        Some("omen")
    );
    assert_eq!(
        events[0].get("fired_tick").and_then(|v| v.as_u64()),
        Some(5)
    );
    assert!(
        events[0]
            .get("decision_id")
            .and_then(|v| v.as_u64())
            .is_some(),
        "fired payload carries the pending decision id"
    );
}

/// An entity-count gate fires only while enough entities carry the component.
#[test]
fn predicate_entity_count_gte_gates_firing() {
    use engine_core::ecs::registry::ComponentRegistry;
    use engine_core::ecs::schema::ComponentSchema;
    use engine_core::faction::set_faction;
    use std::sync::{Arc, Mutex};

    let registry = Arc::new(Mutex::new(ComponentRegistry::new()));
    {
        let mut reg = registry.lock().unwrap();
        reg.register_external_schema(ComponentSchema {
            name: "Faction".to_string(),
            schema: serde_json::from_str(include_str!("../../assets/schemas/faction.json"))
                .unwrap(),
            modes: vec!["colony".to_string()],
        });
    }
    let mut world = World::new(registry);
    world.current_mode = "colony".to_string();
    world.register_system(NarrativeSystem);
    register_scenario(
        &mut world,
        &custom_def(
            "muster",
            json!([{"type": "entity_count_gte", "component": "Faction", "count": 2}]),
        ),
    )
    .expect("register");

    let e = world.spawn_entity();
    set_faction(&mut world, e, "goblins", "member").expect("set faction");
    run_narrative_tick(&mut world);
    assert!(
        drain_flushed(&mut world, "narrative_fired").is_empty(),
        "one bearer is below the count gate"
    );

    let e2 = world.spawn_entity();
    set_faction(&mut world, e2, "goblins", "member").expect("set faction");
    run_narrative_tick(&mut world);
    let events = drain_flushed(&mut world, "narrative_fired");
    assert_eq!(events.len(), 1, "two bearers meet the count gate");
    assert_eq!(
        events[0].get("scenario_id").and_then(|v| v.as_str()),
        Some("muster")
    );
}

/// Standing bounds gate firing from above and below.
#[test]
fn predicate_standing_bounds_gate_firing() {
    use engine_core::diplomacy::modify_standing;

    let mut low_world = narrative_world();
    register_scenario(
        &mut low_world,
        &custom_def(
            "grievance",
            json!([{"type": "faction_standing_lte", "a": "a", "b": "b", "value": 10}]),
        ),
    )
    .expect("register");
    run_narrative_tick(&mut low_world);
    assert_eq!(
        drain_flushed(&mut low_world, "narrative_fired").len(),
        1,
        "standing 0 satisfies lte 10"
    );

    let mut high_world = narrative_world();
    register_scenario(
        &mut high_world,
        &custom_def(
            "accord",
            json!([{"type": "faction_standing_gte", "a": "a", "b": "b", "value": 10}]),
        ),
    )
    .expect("register");
    run_narrative_tick(&mut high_world);
    assert!(
        drain_flushed(&mut high_world, "narrative_fired").is_empty(),
        "standing 0 does not satisfy gte 10"
    );
    modify_standing(&mut high_world, "a", "b", 20).expect("raise standing");
    run_narrative_tick(&mut high_world);
    assert_eq!(
        drain_flushed(&mut high_world, "narrative_fired").len(),
        1,
        "standing 20 satisfies gte 10"
    );
}

/// A relation gate fires only while the pair holds the named relation.
#[test]
fn predicate_relation_is_matches_pair_state() {
    use engine_core::diplomacy::declare_war;

    let mut world = narrative_world();
    register_scenario(
        &mut world,
        &custom_def(
            "war_omens",
            json!([{"type": "faction_relation_is", "a": "a", "b": "b", "relation": "war"}]),
        ),
    )
    .expect("register");
    run_narrative_tick(&mut world);
    assert!(
        drain_flushed(&mut world, "narrative_fired").is_empty(),
        "neutral pair does not satisfy relation war"
    );
    declare_war(&mut world, "a", "b").expect("declare war");
    run_narrative_tick(&mut world);
    let events = drain_flushed(&mut world, "narrative_fired");
    assert_eq!(events.len(), 1, "warring pair satisfies relation war");
}

/// A prior-resolution gate stays shut until its scenario resolves.
#[test]
fn predicate_scenario_resolved_requires_prior_resolution() {
    let mut world = narrative_world();
    register_scenario(
        &mut world,
        &custom_def(
            "aftermath",
            json!([{"type": "scenario_resolved", "scenario_id": "prelude"}]),
        ),
    )
    .expect("register");
    run_narrative_tick(&mut world);
    assert!(
        drain_flushed(&mut world, "narrative_fired").is_empty(),
        "no prior resolution means no firing"
    );
    world.narrative.history.push(NarrativeRecord {
        scenario_id: "prelude".to_string(),
        decision_id: 0,
        kind: NarrativeRecordKind::Resolved,
        turn: 0,
        choice_id: Some("accept".to_string()),
    });
    run_narrative_tick(&mut world);
    assert_eq!(
        drain_flushed(&mut world, "narrative_fired").len(),
        1,
        "recorded resolution opens the gate"
    );
}

/// Unknown predicate types keep their scenario ineligible while the tick continues.
#[test]
fn unknown_predicate_keeps_scenario_ineligible_without_aborting_tick() {
    let mut world = narrative_world();
    register_scenario(
        &mut world,
        &custom_def("strange", json!([{"type": "prophecy", "doom": 7}])),
    )
    .expect("register");
    register_scenario(
        &mut world,
        &custom_def("plain", json!([{"type": "turn_gte", "turn": 0}])),
    )
    .expect("register");
    run_narrative_tick(&mut world);
    let events = drain_flushed(&mut world, "narrative_fired");
    assert_eq!(events.len(), 1, "eligible scenario still fires");
    assert_eq!(
        events[0].get("scenario_id").and_then(|v| v.as_str()),
        Some("plain")
    );
    assert!(
        world
            .narrative
            .pending
            .values()
            .all(|d| d.scenario_id != "strange"),
        "unknown-predicate scenario never gains a pending decision"
    );
}

/// Non-positive weights never win selection.
#[test]
fn weight_zero_scenario_never_fires() {
    let mut world = narrative_world();
    register_scenario(
        &mut world,
        &weighted_def("weightless", json!([{"type": "turn_gte", "turn": 0}]), 0.0),
    )
    .expect("register");
    for turn in 0..3 {
        world.turn = turn;
        run_narrative_tick(&mut world);
    }
    assert!(
        drain_flushed(&mut world, "narrative_fired").is_empty(),
        "weight 0.0 stays ineligible"
    );
    assert!(world.narrative.pending.is_empty());
}

/// Chance boundaries are absolute: 0.0 never fires, 1.0 always fires.
#[test]
fn chance_boundary_zero_never_one_always() {
    let mut world = narrative_world();
    register_scenario(&mut world, &chance_def("never", 0.0)).expect("register");
    register_scenario(&mut world, &chance_def("always", 1.0)).expect("register");
    run_narrative_tick(&mut world);
    let events = drain_flushed(&mut world, "narrative_fired");
    assert_eq!(events.len(), 1, "only the certain scenario fires");
    assert_eq!(
        events[0].get("scenario_id").and_then(|v| v.as_str()),
        Some("always")
    );
    for turn in 1..5 {
        world.turn = turn;
        run_narrative_tick(&mut world);
    }
    assert!(
        drain_flushed(&mut world, "narrative_fired").is_empty(),
        "chance 0.0 never fires across ticks"
    );
    assert!(
        world
            .narrative
            .pending
            .values()
            .all(|d| d.scenario_id != "never"),
        "chance 0.0 never gains a pending decision"
    );
}

/// A held trigger yields exactly one pending decision and one fired event.
#[test]
fn held_trigger_fires_exactly_once_while_pending() {
    let mut world = narrative_world();
    register_scenario(
        &mut world,
        &custom_def("omen", json!([{"type": "turn_gte", "turn": 0}])),
    )
    .expect("register");
    let mut total_events = 0;
    for _ in 0..5 {
        run_narrative_tick(&mut world);
        total_events += drain_flushed(&mut world, "narrative_fired").len();
    }
    assert_eq!(total_events, 1, "exactly one fired event across held ticks");
    assert_eq!(
        world.narrative.pending.len(),
        1,
        "exactly one pending decision for the scenario"
    );
}

/// Identical seeds and scenario sets reproduce identical fire order.
#[test]
fn seeded_two_world_replay_produces_identical_fire_order() {
    fn seeded_world() -> World {
        let mut world = narrative_world();
        world.narrative.rng_state = [7u8; 32];
        register_scenario(
            &mut world,
            &weighted_def("heavy", json!([{"type": "turn_gte", "turn": 0}]), 3.0),
        )
        .expect("register");
        register_scenario(
            &mut world,
            &weighted_def("light", json!([{"type": "turn_gte", "turn": 0}]), 1.0),
        )
        .expect("register");
        world
    }
    fn fire_sequence(world: &mut World) -> Vec<String> {
        let mut seq = Vec::new();
        for turn in 0..2 {
            world.turn = turn;
            run_narrative_tick(world);
            for event in drain_flushed(world, "narrative_fired") {
                seq.push(
                    event
                        .get("scenario_id")
                        .and_then(|v| v.as_str())
                        .expect("fired payload carries scenario_id")
                        .to_string(),
                );
            }
        }
        seq
    }
    let mut first = seeded_world();
    let mut second = seeded_world();
    let first_seq = fire_sequence(&mut first);
    let second_seq = fire_sequence(&mut second);
    assert_eq!(first_seq.len(), 2, "both scenarios fire across two ticks");
    assert_eq!(
        first_seq, second_seq,
        "seeded replay reproduces the fire order"
    );
    assert_eq!(first.narrative.history, second.narrative.history);
}

/// Chance draws on the shared stream replay identically across worlds.
#[test]
fn chance_consumption_is_deterministic_across_worlds() {
    fn seeded_world() -> World {
        let mut world = narrative_world();
        world.narrative.rng_state = [9u8; 32];
        register_scenario(&mut world, &chance_def("gamble", 0.5)).expect("register");
        world
    }
    fn fire_ticks(world: &mut World) -> Vec<u64> {
        let mut ticks = Vec::new();
        for turn in 0..6 {
            world.turn = turn;
            run_narrative_tick(world);
            // Resolve-free soak: drain, then clear pending so the gate re-opens
            // (resolve path arrives in M3; here we re-arm directly).
            if !drain_flushed(world, "narrative_fired").is_empty() {
                ticks.push(u64::from(turn));
                world.narrative.pending.clear();
            }
        }
        ticks
    }
    let mut first = seeded_world();
    let mut second = seeded_world();
    assert_eq!(
        fire_ticks(&mut first),
        fire_ticks(&mut second),
        "chance draws replay identically"
    );
}

/// The narrative slot sits between diplomacy and fluid simulation.
#[test]
fn execution_order_slots_narrative_between_diplomacy_and_fluid() {
    use engine_core::systems::SYSTEM_EXECUTION_ORDER;

    let position = |name: &str| {
        SYSTEM_EXECUTION_ORDER
            .iter()
            .position(|slot| *slot == name)
            .unwrap_or_else(|| panic!("{name} must be in SYSTEM_EXECUTION_ORDER"))
    };
    assert!(
        position("DiplomacySystem") < position("NarrativeSystem"),
        "narrative runs after diplomacy"
    );
    assert!(
        position("NarrativeSystem") < position("FluidSimulationSystem"),
        "narrative runs before fluid simulation"
    );
}

/// Zero registered scenarios keep the tick a field-access no-op.
#[test]
fn zero_scenario_tick_is_noop() {
    let mut world = narrative_world();
    run_narrative_tick(&mut world);
    assert!(world.narrative.pending.is_empty());
    assert!(world.narrative.history.is_empty());
    assert!(
        drain_flushed(&mut world, "narrative_fired").is_empty(),
        "no scenarios means no events"
    );
}
