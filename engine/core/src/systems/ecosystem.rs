//! Ecosystem simulation system: deterministic wildlife graze/wander loop.
//!
//! Gen-1 core (M2): herbivore/omnivore grazing with season-modulated nutrition,
//! wander movement over topology-generic neighbors, metabolism costs, and
//! saturating cooldown decay. Flee/rest/reproduce/predation arrive in M3.
//!
//! Follows the [`EnemyBehaviorSystem`](super::enemy_behavior) collect-then-apply
//! pattern: entity IDs are collected sorted ascending, each entity makes a
//! pure-read decision, and a single apply phase writes `Wildlife` + `Position`.
//! All stochastic choices draw from `deterministic_rng(entity_id, turn)` so the
//! shared weather stream is never disturbed.

use crate::ecs::system::System;
use crate::ecs::world::{Season, World};
use crate::map::cell_key::CellKey;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use serde_json::{Value as JsonValue, json};

/// Probability per tick of switching between graze and wander.
const GRAZE_WANDER_PROBABILITY: f64 = 0.3;

/// Deterministic FSM system for wildlife graze/wander behavior.
pub struct EcosystemSystem;

impl System for EcosystemSystem {
    fn name(&self) -> &'static str {
        "EcosystemSystem"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["FovUpdateSystem", "NoiseSystem"]
    }

    fn run(&mut self, world: &mut World) {
        if world.map.is_none() {
            return;
        }

        let mut entity_ids = world.get_entities_with_component("Wildlife");
        entity_ids.sort_unstable();

        let turn = world.turn;
        let season_modifier = season_modifier(Season::from_day(world.time_of_day.day));

        let mut decisions: Vec<EcosystemDecision> = Vec::new();
        for eid in entity_ids {
            let Some(wildlife) = world.get_component(eid, "Wildlife").cloned() else {
                continue;
            };
            let Some(position) = world.get_component(eid, "Position").cloned() else {
                continue;
            };
            let species = world.get_component(eid, "Species").cloned();
            let Some(cell) = CellKey::from_position(&position) else {
                continue;
            };
            if let Some(d) = decide(
                world,
                eid,
                turn,
                season_modifier,
                &cell,
                &wildlife,
                species.as_ref(),
            ) {
                decisions.push(d);
            }
        }

        for d in decisions {
            let _ = world.set_component(d.entity, "Wildlife", d.new_wildlife);
            if let Some(cell) = d.new_position {
                let _ = world.set_component(d.entity, "Position", cell_position_json(&cell));
            }
        }
    }
}

/// A pending per-entity update collected before mutation.
struct EcosystemDecision {
    entity: u32,
    new_wildlife: JsonValue,
    new_position: Option<CellKey>,
}

/// Pure-read decision for one wildlife entity: graze/wander core loop.
///
/// Non-graze/non-wander states are normalized to graze here; M3 refines them
/// into flee/rest/reproduce handling.
fn decide(
    world: &World,
    eid: u32,
    turn: u32,
    season_modifier: f64,
    cell: &CellKey,
    wildlife: &JsonValue,
    species: Option<&JsonValue>,
) -> Option<EcosystemDecision> {
    let diet = species
        .and_then(|s| s.get("diet"))
        .and_then(|v| v.as_str())
        .unwrap_or("herbivore");
    let graze_rate = species
        .and_then(|s| s.get("graze_nutrition_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.1);
    let metabolism = species
        .and_then(|s| s.get("metabolism_rate"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.05);

    let state = wildlife
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("graze");
    let satiety = wildlife
        .get("satiety")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    let cooldown = wildlife
        .get("reproduction_cooldown")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let mut rng = deterministic_rng(eid, turn);
    let mut new_wildlife = wildlife.clone();
    new_wildlife["reproduction_cooldown"] = json!(cooldown.saturating_sub(1));

    match state {
        "wander" => {
            let new_position = wander_step(world, cell, &mut rng);
            let mut decision = EcosystemDecision {
                entity: eid,
                new_wildlife,
                new_position,
            };
            decision.new_wildlife["satiety"] = json!((satiety - metabolism).max(0.0));
            decision.new_wildlife["state"] = if rng.random::<f64>() < GRAZE_WANDER_PROBABILITY {
                json!("graze")
            } else {
                json!("wander")
            };
            Some(decision)
        }
        _ => {
            let gain = match diet {
                "herbivore" | "omnivore" => graze_rate * season_modifier,
                _ => 0.0,
            };
            let mut decision = EcosystemDecision {
                entity: eid,
                new_wildlife,
                new_position: None,
            };
            decision.new_wildlife["satiety"] = json!((satiety + gain).clamp(0.0, 1.0));
            decision.new_wildlife["state"] =
                if satiety > 0.15 && rng.random::<f64>() < GRAZE_WANDER_PROBABILITY {
                    json!("wander")
                } else {
                    json!("graze")
                };
            Some(decision)
        }
    }
}

/// One deterministic wander step to a neighboring cell, or `None` when the
/// cell has no neighbors. Draws from the caller's per-entity RNG stream so
/// neighbor choice and state flips share one deterministic sequence.
///
/// Neighbor lists are sorted before indexing because grid backends store
/// adjacency in hash sets with nondeterministic iteration order; without the
/// sort identical seeds would diverge (NFR001).
fn wander_step(world: &World, cell: &CellKey, rng: &mut SmallRng) -> Option<CellKey> {
    let map = world.map.as_ref()?;
    let mut neighbors = map.neighbors(cell);
    if neighbors.is_empty() {
        return None;
    }
    neighbors.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    let idx = rng.random_range(0..neighbors.len());
    neighbors.into_iter().nth(idx)
}

/// Season nutrition multiplier: spring/summer full yield, autumn reduced,
/// winter scarce.
fn season_modifier(season: Season) -> f64 {
    match season {
        Season::Spring | Season::Summer => 1.0,
        Season::Autumn => 0.6,
        Season::Winter => 0.25,
    }
}

/// Deterministic RNG seeded from (entity_id, turn); never touches the shared
/// weather stream.
fn deterministic_rng(entity_id: u32, turn: u32) -> SmallRng {
    let mut seed = [0u8; 32];
    seed[0..4].copy_from_slice(&entity_id.to_le_bytes());
    seed[4..8].copy_from_slice(&turn.to_le_bytes());
    SmallRng::from_seed(seed)
}

/// Serialize a [`CellKey`] as a `Position`-shaped value, preserving the
/// entity's existing topology variant.
fn cell_position_json(cell: &CellKey) -> JsonValue {
    match cell {
        CellKey::Square { x, y, z } => json!({ "pos": { "Square": { "x": x, "y": y, "z": z } } }),
        CellKey::Hex { q, r, z } => json!({ "pos": { "Hex": { "q": q, "r": r, "z": z } } }),
        CellKey::Province { id } => json!({ "pos": { "Province": { "id": id } } }),
    }
}
