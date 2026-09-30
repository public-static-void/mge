//! Standing suite for the event-driven narrative engine (incident director).
//!
//! Created in M1 alongside scenario registration. Grows milestone by
//! milestone (M2–M4 extend in place with trigger, lifecycle, and
//! persistence coverage; M5 adds bridge suites).

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::narrative::{get_scenario, list_scenarios, register_scenario};
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
        json.get("scenarios")
            .and_then(|v| v.as_array())
            .is_some(),
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
