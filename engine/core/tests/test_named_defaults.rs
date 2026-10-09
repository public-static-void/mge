//! Pinning tests for per-domain named defaults (issue 13 + F-04).
//!
//! Each domain owns its fallback consts following the `DungeonConfig::DEFAULT_*`
//! precedent. No central `Defaults` god-object.
//!
//! Assertions compare runtime behavior against the consts (never const against
//! literal, per the repo's `test_dungeon_gen.rs` convention), except for three
//! registry-side skill/tech values with no public behavioral path — those carry
//! a targeted allow with this rationale: the literal comparison is the pin.

#![allow(clippy::assertions_on_constants)]

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::system::System;
use engine_core::faction::{DEFAULT_DECAY_RATE, DEFAULT_REPUTATION, get_reputation};
use engine_core::map::{CellKey, Map, SquareGridMap};
use engine_core::systems::construction::system::construction_defaults;
use engine_core::systems::construction::{get_construction_state, place_blueprint};
use engine_core::systems::crafting::crafting_defaults;
use engine_core::systems::death_decay::{ProcessDeaths, health_defaults};
use engine_core::systems::economic::recipe::{Recipe, ResourceAmount};
use engine_core::systems::economic::system::{EconomicSystem, economic_defaults};
use engine_core::systems::faction_reputation::FactionReputationSystem;
use engine_core::systems::job::system::process::{
    DEFAULT_BASE_XP_PER_ACTION, DEFAULT_MAX_SKILL_LEVEL, DEFAULT_SKILL_LEVEL, DEFAULT_STAMINA,
    process_job_progress,
};
use engine_core::systems::noise::{NoiseSystem, noise_defaults};
use engine_core::tech_tree::{DEFAULT_REQUIRED_SKILL_LEVEL, MIN_TECH_COST, get_tech_tree};
use engine_core::trade::{DEFAULT_MISSING_BALANCE, TransferError, transfer_stockpile_resource};
use serde_json::json;

/// Line map of 13 cells so the shared radius default (5) has room to show
/// both its falloff edge (distance 5 reads 0.0) and its depth limit
/// (distance 6 is unvisited).
fn line_world() -> engine_core::ecs::world::World {
    let mut world = make_test_world();
    world.current_mode = "roguelike".to_string();
    let mut grid = SquareGridMap::new();
    for x in -6..=6 {
        grid.add_cell(x, 0, 0);
    }
    for x in -6..=5 {
        grid.add_neighbor((x, 0, 0), (x + 1, 0, 0));
    }
    world.map = Some(Map::new(Box::new(grid)));
    world
}

#[test]
fn test_noise_bare_emitter_uses_shared_defaults() {
    let mut world = line_world();
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "Position",
            json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
        )
        .unwrap();
    world.set_component(eid, "NoiseEmitter", json!({})).unwrap();
    world.set_component(eid, "Stealth", json!({})).unwrap();
    let mut system = NoiseSystem;
    system.run(&mut world);
    let at = |x: i32| CellKey::Square { x, y: 0, z: 0 };
    assert_eq!(world.get_noise_at(&at(0)), Some(noise_defaults::INTENSITY));
    assert_eq!(
        world.get_noise_at(&at(5)),
        Some(0.0),
        "linear falloff reaches exactly zero at the shared radius edge"
    );
    assert_eq!(
        world.get_noise_at(&at(6)),
        None,
        "propagation stops at the shared radius depth"
    );
}

#[test]
fn test_job_progress_skill_fallbacks_use_named_defaults() {
    let mut world = make_test_world();
    let agent = world.spawn_entity();
    world
        .set_component(agent, "Agent", json!({"entity_id": agent}))
        .unwrap();
    let job_eid = world.spawn_entity();
    let job = json!({
        "id": 1,
        "job_type": "dig",
        "category": "digging",
        "state": "in_progress",
        "assigned_to": agent,
        "progress": 0.0,
        "required_progress": 100.0,
    });
    let out = process_job_progress(&mut world, job_eid, "dig".to_string(), job);
    let expected = DEFAULT_SKILL_LEVEL * (DEFAULT_STAMINA / 100.0);
    assert_eq!(out.get("progress").and_then(|v| v.as_f64()), Some(expected));
}

#[test]
fn test_skill_registry_fallback_values() {
    assert_eq!(DEFAULT_BASE_XP_PER_ACTION, 10.0);
    assert_eq!(DEFAULT_MAX_SKILL_LEVEL, 100.0);
}

#[test]
fn test_tech_tree_costs_respect_named_minimum() {
    let tree = get_tech_tree();
    assert!(
        !tree.is_empty(),
        "tech_tree.json loads in the test layout so the clamp has data to guard"
    );
    for node in tree {
        assert!(
            node.cost >= MIN_TECH_COST,
            "tech {} cost {} honors the named minimum",
            node.id,
            node.cost
        );
    }
    assert_eq!(DEFAULT_REQUIRED_SKILL_LEVEL, 1.0);
}

#[test]
fn test_construction_state_missing_fields_use_named_defaults() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = make_test_world();
    world.set_mode("colony");
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    world.map = Some(Map::new(Box::new(grid)));
    let site = place_blueprint(
        &mut world,
        "hut",
        &CellKey::Square { x: 0, y: 0, z: 0 },
        &[("wood".to_string(), 1)],
        1,
    )
    .expect("blueprint places on a free in-bounds cell");
    let state = get_construction_state(&world, site).expect("fresh site reads");
    assert_eq!(
        state.get("state").and_then(|v| v.as_str()),
        Some(construction_defaults::PENDING_STATE),
        "spawn default state equals the named const"
    );
    assert_eq!(
        state.get("progress").and_then(|v| v.as_i64()),
        Some(construction_defaults::NO_PROGRESS)
    );
    assert_eq!(
        state.get("required_work").and_then(|v| v.as_i64()),
        Some(construction_defaults::UNIT_WORK)
    );
    assert_eq!(
        state.get("building_type").and_then(|v| v.as_str()),
        Some("hut")
    );
    assert_eq!(construction_defaults::MISSING_STATE, "");
    assert_eq!(construction_defaults::DEFAULT_BUILDING_TYPE, "");
    assert_eq!(construction_defaults::DEFAULT_KIND, "");
    assert_eq!(construction_defaults::NO_AMOUNT, 0);
    assert_eq!(construction_defaults::DEFAULT_INTEGRITY, 0);
}

#[test]
fn test_production_job_missing_fields_use_named_defaults() {
    let mut world = make_test_world();
    world.set_mode("colony");
    let mut system = EconomicSystem::with_recipes(vec![Recipe {
        name: "planks".to_string(),
        inputs: vec![],
        outputs: vec![ResourceAmount {
            kind: "plank".to_string(),
            amount: 1,
        }],
        duration: 1,
        tools: vec![],
        materials: vec![],
        required_skill: None,
        output_item: None,
        station: None,
        xp: None,
    }]);
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "ProductionJob",
            json!({"recipe": "planks", "progress": 0, "state": "pending"}),
        )
        .unwrap();
    world
        .set_component(eid, "Stockpile", json!({"resources": {}}))
        .unwrap();
    system.run(&mut world);
    let job = world.get_component(eid, "ProductionJob").unwrap().clone();
    assert_eq!(
        job.get("state").and_then(|v| v.as_str()),
        Some("complete"),
        "duration-1 job completes on the first tick"
    );
    let stock = world.get_component(eid, "Stockpile").unwrap()["resources"].clone();
    assert_eq!(
        stock.get("plank").and_then(|v| v.as_i64()),
        Some(economic_defaults::UNIT_BATCH),
        "absent batch_size falls back to the named unit batch"
    );
    assert_eq!(
        job.get("progress").and_then(|v| v.as_i64()),
        Some(economic_defaults::NO_PROGRESS + 1)
    );
}

#[test]
fn test_trade_missing_balance_uses_named_default() {
    let mut world = make_test_world();
    let from = world.spawn_entity();
    world
        .set_component(from, "Stockpile", json!({"resources": {}}))
        .unwrap();
    let to = world.spawn_entity();
    world
        .set_component(to, "Stockpile", json!({"resources": {}}))
        .unwrap();
    let err = transfer_stockpile_resource(&mut world, from, to, "wood", 1.0).unwrap_err();
    match err {
        TransferError::InsufficientFunds { available, .. } => {
            assert_eq!(available, DEFAULT_MISSING_BALANCE);
        }
        other => panic!("expected InsufficientFunds, got {other:?}"),
    }
}

#[test]
fn test_reputation_fallbacks_use_named_defaults() {
    let mut world = make_test_world();
    let eid = world.spawn_entity();
    assert_eq!(get_reputation(&world, eid, "brigands"), DEFAULT_REPUTATION);
    world
        .set_component(
            eid,
            "Reputation",
            json!({"values": {"brigands": 10}, "decay_rate": null}),
        )
        .unwrap();
    let mut system = FactionReputationSystem;
    system.run(&mut world);
    let values = world.get_component(eid, "Reputation").unwrap()["values"].clone();
    assert_eq!(
        values.get("brigands").and_then(|v| v.as_i64()),
        Some(10),
        "absent decay rate falls back to the named zero: no decay"
    );
    assert_eq!(DEFAULT_DECAY_RATE, 0.0);
}

#[test]
fn test_health_current_floor_uses_named_default() {
    let mut world = make_test_world();
    let dead = world.spawn_entity();
    world
        .set_component(dead, "Health", json!({"current": 0.0, "max": 100.0}))
        .unwrap();
    let mut system = ProcessDeaths;
    system.run(&mut world);
    assert!(
        world.has_component(dead, "Corpse"),
        "control: zero current still kills"
    );
    assert_eq!(health_defaults::DEFAULT_CURRENT, 1.0);
    assert_eq!(crafting_defaults::DEFAULT_INPUT_QUALITY, 1.0);
}
