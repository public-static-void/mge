//! Ecosystem simulation system: deterministic wildlife entity-FSM.
//!
//! Gen-1 full FSM (M3): graze/wander core loop plus flee preemption,
//! activity-phase rest, temperature-gated reproduction, adjacent predation,
//! and `wildlife_state_changed` / `wildlife_born` events.
//!
//! Follows the [`EnemyBehaviorSystem`](super::enemy_behavior) collect-then-apply
//! pattern: entity IDs are collected sorted ascending, each entity makes a
//! pure-read decision, and a single apply phase writes `Wildlife` + `Position`,
//! spawns births, applies predation damage, and emits events. All stochastic
//! choices draw from `deterministic_rng(entity_id, turn)` so the shared
//! weather stream is never disturbed.
//!
//! Per-tick priority for one entity: threat stimulus (flee) outranks
//! activity-phase mismatch (rest), which outranks the reproduction guard,
//! which outranks the graze/wander core loop.

use crate::ecs::system::System;
use crate::ecs::world::{Season, World};
use crate::map::cell_key::CellKey;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use serde_json::{Value as JsonValue, json};

/// Probability per tick of switching between graze and wander.
const GRAZE_WANDER_PROBABILITY: f64 = 0.3;

/// Consecutive stimulus-free ticks before a fleeing entity returns to graze.
const FLEE_RECOVERY_TICKS: u64 = 3;

/// Cell-temperature window (°C) permitting reproduction.
const REPRO_MIN_TEMP: f64 = 0.0;
const REPRO_MAX_TEMP: f64 = 45.0;

/// Satiety granted to a predator when its prey dies from the hit.
const PREDATION_SATIETY_GAIN: f64 = 0.5;

/// Day hours are 06:00–17:59; night is 18:00–05:59. Diurnal entities rest at
/// night, nocturnal entities rest during the day.
const DAY_START_HOUR: u8 = 6;
const DAY_END_HOUR: u8 = 18;

/// Deterministic FSM system for wildlife behavior.
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
            let emitted = d.emitted_state_change();
            let _ = world.set_component(d.entity, "Wildlife", d.new_wildlife);
            if let Some(cell) = &d.new_position {
                let _ = world.set_component(d.entity, "Position", cell_position_json(cell));
            }
            let mut children: Vec<u32> = Vec::new();
            for spawn in &d.spawn_requests {
                let child = world.spawn_entity();
                let _ = world.set_component(child, "Wildlife", spawn.wildlife.clone());
                let _ = world.set_component(child, "Species", spawn.species.clone());
                let _ = world.set_component(child, "Position", cell_position_json(&spawn.position));
                children.push(child);
            }
            if let Some(target) = d.attack_target
                && world.is_entity_alive(target)
            {
                world.damage_entity(target, 1.0);
                if !world.is_entity_alive(target)
                    && let Some(predator) = world.get_component(d.entity, "Wildlife").cloned()
                {
                    let satiety = predator
                        .get("satiety")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let mut updated = predator.clone();
                    updated["satiety"] = json!((satiety + PREDATION_SATIETY_GAIN).clamp(0.0, 1.0));
                    let _ = world.set_component(d.entity, "Wildlife", updated);
                }
            }
            if let Some(new_state) = emitted {
                let _ = world.send_event(
                    "wildlife_state_changed",
                    json!({
                        "entity": d.entity,
                        "old_state": d.old_state,
                        "new_state": new_state,
                    }),
                );
            }
            for child in children {
                let _ = world.send_event(
                    "wildlife_born",
                    json!({
                        "parent": d.entity,
                        "child": child,
                        "species_diet": d.species_diet,
                    }),
                );
            }
        }
    }
}

/// A pending per-entity update collected before mutation.
struct EcosystemDecision {
    entity: u32,
    old_state: String,
    new_wildlife: JsonValue,
    new_position: Option<CellKey>,
    spawn_requests: Vec<SpawnRequest>,
    attack_target: Option<u32>,
    species_diet: String,
    /// State name reported in `wildlife_state_changed`. Reproduction resolves
    /// in-tick back to graze, so the stored state and the emitted transition
    /// target differ there; `None` means "derive from the stored state".
    emitted_new_state: Option<String>,
}

impl EcosystemDecision {
    /// State name for the transition event, or `None` when the stored state
    /// is unchanged from the previous tick (no transition to report).
    fn emitted_state_change(&self) -> Option<String> {
        if let Some(emitted) = &self.emitted_new_state {
            return Some(emitted.clone());
        }
        let stored = self
            .new_wildlife
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("graze");
        if stored == self.old_state {
            None
        } else {
            Some(stored.to_string())
        }
    }
}

/// One birth request collected in the read phase and spawned in apply.
struct SpawnRequest {
    wildlife: JsonValue,
    species: JsonValue,
    position: CellKey,
}

/// Static per-species parameters with schema defaults applied.
struct SpeciesParams {
    diet: String,
    graze_rate: f64,
    metabolism: f64,
    reproduction_threshold: f64,
    reproduction_cooldown_ticks: u64,
    litter_size: u64,
    detection_range: u64,
    noise_flee_threshold: f64,
    activity: String,
}

impl SpeciesParams {
    fn from_json(species: Option<&JsonValue>) -> Self {
        Self {
            diet: species
                .and_then(|s| s.get("diet"))
                .and_then(|v| v.as_str())
                .unwrap_or("herbivore")
                .to_string(),
            graze_rate: species
                .and_then(|s| s.get("graze_nutrition_rate"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.1),
            metabolism: species
                .and_then(|s| s.get("metabolism_rate"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.05),
            reproduction_threshold: species
                .and_then(|s| s.get("reproduction_threshold"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.8),
            reproduction_cooldown_ticks: species
                .and_then(|s| s.get("reproduction_cooldown_ticks"))
                .and_then(|v| v.as_u64())
                .unwrap_or(10),
            litter_size: species
                .and_then(|s| s.get("litter_size"))
                .and_then(|v| v.as_u64())
                .unwrap_or(1)
                .clamp(1, 3),
            detection_range: species
                .and_then(|s| s.get("detection_range"))
                .and_then(|v| v.as_u64())
                .unwrap_or(6)
                .max(1),
            noise_flee_threshold: species
                .and_then(|s| s.get("noise_flee_threshold"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5),
            activity: species
                .and_then(|s| s.get("activity"))
                .and_then(|v| v.as_str())
                .unwrap_or("diurnal")
                .to_string(),
        }
    }

    /// Full `Species` component value: the parent's copy when present, else
    /// the schema defaults. Used for newborn offspring.
    fn component_json(species: Option<&JsonValue>) -> JsonValue {
        species.cloned().unwrap_or(json!({
            "diet": "herbivore",
            "graze_nutrition_rate": 0.1,
            "metabolism_rate": 0.05,
            "reproduction_threshold": 0.8,
            "reproduction_cooldown_ticks": 10,
            "litter_size": 1,
            "detection_range": 6,
            "noise_flee_threshold": 0.5,
            "activity": "diurnal",
        }))
    }
}

/// Pure-read decision for one wildlife entity covering the full FSM.
///
/// Unknown stored states are normalized to graze. Every path decrements
/// `reproduction_cooldown` saturating; satiety stays clamped to [0, 1].
fn decide(
    world: &World,
    eid: u32,
    turn: u32,
    season_modifier: f64,
    cell: &CellKey,
    wildlife: &JsonValue,
    species: Option<&JsonValue>,
) -> Option<EcosystemDecision> {
    let params = SpeciesParams::from_json(species);

    let raw_state = wildlife
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("graze");
    let state = match raw_state {
        "graze" | "wander" | "flee" | "rest" => raw_state,
        _ => "graze",
    };
    let satiety = wildlife
        .get("satiety")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    let cooldown = wildlife
        .get("reproduction_cooldown")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let flee_ticks = wildlife
        .get("flee_ticks")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let rest_ticks = wildlife
        .get("rest_ticks")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let mut rng = deterministic_rng(eid, turn);
    let mut new_wildlife = wildlife.clone();
    new_wildlife["reproduction_cooldown"] = json!(cooldown.saturating_sub(1));
    let decremented_cooldown = cooldown.saturating_sub(1);

    // Threat stimulus outranks every other transition.
    let threat_cell = find_threat_cell(world, eid, cell, params.detection_range);
    let noise = world.get_noise_at(cell).unwrap_or(0.0);
    let stimulated = threat_cell.is_some() || noise > params.noise_flee_threshold;

    // --- Flee state: recover after consecutive stimulus-free ticks. ---
    if state == "flee" {
        if stimulated {
            new_wildlife["state"] = json!("flee");
            new_wildlife["flee_ticks"] = json!(0);
            new_wildlife["rest_ticks"] = json!(0);
            let new_position = flee_step(world, cell, threat_cell.as_ref());
            return Some(EcosystemDecision {
                entity: eid,
                old_state: state.to_string(),
                new_wildlife,
                new_position,
                spawn_requests: Vec::new(),
                attack_target: None,
                species_diet: params.diet,
                emitted_new_state: None,
            });
        }
        let quiet_ticks = flee_ticks + 1;
        if quiet_ticks < FLEE_RECOVERY_TICKS {
            new_wildlife["state"] = json!("flee");
            new_wildlife["flee_ticks"] = json!(quiet_ticks);
            new_wildlife["rest_ticks"] = json!(0);
            return Some(EcosystemDecision {
                entity: eid,
                old_state: state.to_string(),
                new_wildlife,
                new_position: None,
                spawn_requests: Vec::new(),
                attack_target: None,
                species_diet: params.diet,
                emitted_new_state: None,
            });
        }
        // Recovered: fall through to the graze handling below with counters reset.
        new_wildlife["flee_ticks"] = json!(0);
        new_wildlife["rest_ticks"] = json!(0);
        return Some(graze_decision(
            &params,
            &mut rng,
            eid,
            state,
            satiety,
            season_modifier,
            new_wildlife,
        ));
    }

    // --- Threat preemption from any other state. ---
    if stimulated {
        new_wildlife["state"] = json!("flee");
        new_wildlife["flee_ticks"] = json!(0);
        new_wildlife["rest_ticks"] = json!(0);
        let new_position = flee_step(world, cell, threat_cell.as_ref());
        return Some(EcosystemDecision {
            entity: eid,
            old_state: state.to_string(),
            new_wildlife,
            new_position,
            spawn_requests: Vec::new(),
            attack_target: None,
            species_diet: params.diet,
            emitted_new_state: None,
        });
    }

    let resting_phase = should_rest(&params.activity, world.time_of_day.hour);

    // --- Rest state: sleep through the inactive phase, wake into graze. ---
    if state == "rest" {
        if resting_phase {
            new_wildlife["state"] = json!("rest");
            new_wildlife["rest_ticks"] = json!(rest_ticks + 1);
            new_wildlife["flee_ticks"] = json!(0);
            new_wildlife["satiety"] = json!(rest_satiety(satiety, &params, season_modifier));
            return Some(EcosystemDecision {
                entity: eid,
                old_state: state.to_string(),
                new_wildlife,
                new_position: None,
                spawn_requests: Vec::new(),
                attack_target: None,
                species_diet: params.diet,
                emitted_new_state: None,
            });
        }
        new_wildlife["rest_ticks"] = json!(0);
        new_wildlife["flee_ticks"] = json!(0);
        return Some(graze_decision(
            &params,
            &mut rng,
            eid,
            state,
            satiety,
            season_modifier,
            new_wildlife,
        ));
    }

    // --- Activity-phase mismatch sends graze/wander entities to rest. ---
    if resting_phase {
        new_wildlife["state"] = json!("rest");
        new_wildlife["rest_ticks"] = json!(rest_ticks + 1);
        new_wildlife["flee_ticks"] = json!(0);
        new_wildlife["satiety"] = json!(rest_satiety(satiety, &params, season_modifier));
        return Some(EcosystemDecision {
            entity: eid,
            old_state: state.to_string(),
            new_wildlife,
            new_position: None,
            spawn_requests: Vec::new(),
            attack_target: None,
            species_diet: params.diet,
            emitted_new_state: None,
        });
    }

    // --- Reproduction: temperature-gated, resolves in-tick to graze. ---
    if satiety >= params.reproduction_threshold
        && decremented_cooldown == 0
        && cell_temperature(world, cell) >= REPRO_MIN_TEMP
        && cell_temperature(world, cell) <= REPRO_MAX_TEMP
    {
        let spawn_requests = birth_requests(world, cell, &params, species);
        new_wildlife["state"] = json!("graze");
        new_wildlife["satiety"] = json!(satiety / 2.0);
        new_wildlife["reproduction_cooldown"] = json!(params.reproduction_cooldown_ticks);
        new_wildlife["flee_ticks"] = json!(0);
        new_wildlife["rest_ticks"] = json!(0);
        return Some(EcosystemDecision {
            entity: eid,
            old_state: state.to_string(),
            new_wildlife,
            new_position: None,
            spawn_requests,
            attack_target: None,
            species_diet: params.diet,
            emitted_new_state: Some("reproduce".to_string()),
        });
    }

    // --- Graze/wander core loop. ---
    if state == "wander" {
        let new_position = wander_step(world, cell, &mut rng);
        let attack_target = match params.diet.as_str() {
            "carnivore" | "omnivore" => find_adjacent_prey(world, eid, cell),
            _ => None,
        };
        let mut decision = EcosystemDecision {
            entity: eid,
            old_state: state.to_string(),
            new_wildlife,
            new_position,
            spawn_requests: Vec::new(),
            attack_target,
            species_diet: params.diet,
            emitted_new_state: None,
        };
        decision.new_wildlife["satiety"] = json!((satiety - params.metabolism).max(0.0));
        decision.new_wildlife["state"] = if rng.random::<f64>() < GRAZE_WANDER_PROBABILITY {
            json!("graze")
        } else {
            json!("wander")
        };
        return Some(decision);
    }

    Some(graze_decision(
        &params,
        &mut rng,
        eid,
        state,
        satiety,
        season_modifier,
        new_wildlife,
    ))
}

/// Shared graze-tick update: stationary full-rate nutrition with the
/// graze/wander switch roll. Used for graze state, flee recovery, and waking.
#[allow(clippy::too_many_arguments)]
fn graze_decision(
    params: &SpeciesParams,
    rng: &mut SmallRng,
    eid: u32,
    old_state: &str,
    satiety: f64,
    season_modifier: f64,
    new_wildlife: JsonValue,
) -> EcosystemDecision {
    let gain = match params.diet.as_str() {
        "herbivore" | "omnivore" => params.graze_rate * season_modifier,
        _ => 0.0,
    };
    let mut decision = EcosystemDecision {
        entity: eid,
        old_state: old_state.to_string(),
        new_wildlife,
        new_position: None,
        spawn_requests: Vec::new(),
        attack_target: None,
        species_diet: params.diet.clone(),
        emitted_new_state: None,
    };
    decision.new_wildlife["satiety"] = json!((satiety + gain).clamp(0.0, 1.0));
    decision.new_wildlife["state"] =
        if satiety > 0.15 && rng.random::<f64>() < GRAZE_WANDER_PROBABILITY {
            json!("wander")
        } else {
            json!("graze")
        };
    decision
}

/// Rest-tick nutrition: half-rate grazing at quarter metabolism, clamped.
fn rest_satiety(satiety: f64, params: &SpeciesParams, season_modifier: f64) -> f64 {
    let gain = match params.diet.as_str() {
        "herbivore" | "omnivore" => params.graze_rate * season_modifier * 0.5,
        _ => 0.0,
    };
    let fed = (satiety + gain).clamp(0.0, 1.0);
    (fed - params.metabolism * 0.25).clamp(0.0, 1.0)
}

/// True when the entity's activity pattern says it should be resting now:
/// diurnal entities rest at night, nocturnal entities rest during the day.
fn should_rest(activity: &str, hour: u8) -> bool {
    let is_day = (DAY_START_HOUR..DAY_END_HOUR).contains(&hour);
    match activity {
        "nocturnal" => is_day,
        _ => !is_day,
    }
}

/// Nearest live threat cell within `detection_range` path steps, or `None`.
///
/// A threat is any other living entity whose `EnemyAI.state` is chase or
/// attack. Distance is the topology-generic A* step count from `find_path`;
/// unreachable cells are skipped.
fn find_threat_cell(
    world: &World,
    eid: u32,
    cell: &CellKey,
    detection_range: u64,
) -> Option<CellKey> {
    let mut best: Option<(CellKey, usize)> = None;
    for other in world.get_entities_with_component("EnemyAI") {
        if other == eid || !world.is_entity_alive(other) {
            continue;
        }
        let is_threat = world
            .get_component(other, "EnemyAI")
            .and_then(|ai| ai.get("state"))
            .and_then(|v| v.as_str())
            .is_some_and(|s| s == "chase" || s == "attack");
        if !is_threat {
            continue;
        }
        let Some(other_cell) = world
            .get_component(other, "Position")
            .and_then(CellKey::from_position)
        else {
            continue;
        };
        let Some(steps) = path_steps(world, cell, &other_cell) else {
            continue;
        };
        if steps <= detection_range as usize && best.as_ref().is_none_or(|(_, d)| steps < *d) {
            best = Some((other_cell, steps));
        }
    }
    best.map(|(cell, _)| cell)
}

/// A* step count between two cells, or `None` when unreachable.
fn path_steps(world: &World, from: &CellKey, to: &CellKey) -> Option<usize> {
    world
        .find_path(from, to)
        .map(|result| result.path.len().saturating_sub(1))
}

/// One flee step away from the nearest threat, or down the noise gradient
/// when the stimulus is noise only. Deterministic: neighbors are compared in
/// sorted order and the first optimum wins, so no RNG draw is consumed.
fn flee_step(world: &World, cell: &CellKey, threat: Option<&CellKey>) -> Option<CellKey> {
    let map = world.map.as_ref()?;
    let mut neighbors = map.neighbors(cell);
    if neighbors.is_empty() {
        return None;
    }
    neighbors.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
    match threat {
        Some(threat_cell) => {
            let here = path_steps(world, threat_cell, cell).unwrap_or(usize::MAX);
            let mut best: Option<(CellKey, usize)> = None;
            for neighbor in neighbors {
                let steps = path_steps(world, threat_cell, &neighbor).unwrap_or(usize::MAX);
                if steps > here && best.as_ref().is_none_or(|(_, d)| steps > *d) {
                    best = Some((neighbor, steps));
                }
            }
            best.map(|(cell, _)| cell)
        }
        None => {
            let here_noise = world.get_noise_at(cell).unwrap_or(0.0);
            let mut best: Option<(CellKey, f64)> = None;
            for neighbor in neighbors {
                let noise = world.get_noise_at(&neighbor).unwrap_or(0.0);
                if noise < here_noise && best.as_ref().is_none_or(|(_, n)| noise < *n) {
                    best = Some((neighbor, noise));
                }
            }
            best.map(|(cell, _)| cell)
        }
    }
}

/// Lowest-ID living herbivore-Wildlife entity in a neighboring cell.
///
/// Prey is any other living entity carrying `Wildlife` whose `Species.diet`
/// is herbivore (entities without `Species` use the herbivore default).
fn find_adjacent_prey(world: &World, eid: u32, cell: &CellKey) -> Option<u32> {
    let map = world.map.as_ref()?;
    let neighbors = map.neighbors(cell);
    let mut best: Option<u32> = None;
    for neighbor in &neighbors {
        for other in world.entities_in_cell(neighbor) {
            if other == eid || !world.is_entity_alive(other) {
                continue;
            }
            if !world.has_component(other, "Wildlife") {
                continue;
            }
            let diet = world
                .get_component(other, "Species")
                .and_then(|s| s.get("diet"))
                .and_then(|v| v.as_str())
                .unwrap_or("herbivore");
            if diet != "herbivore" {
                continue;
            }
            if best.is_none_or(|b| other < b) {
                best = Some(other);
            }
        }
    }
    best
}

/// Cell temperature through the transient diffusion map with ambient fallback,
/// mirroring the temperature system's own drift target. Map-keyed so it stays
/// topology-generic (the `get_cell_temperature(x, y, z)` helper is Square-only).
fn cell_temperature(world: &World, cell: &CellKey) -> f64 {
    world
        .temperature_map
        .get(cell)
        .copied()
        .unwrap_or(world.temperature.ambient)
}

/// Birth requests for one reproduction: `litter_size` children with zeroed
/// counters, a copy of the parent species, and placement at the parent cell
/// or the first free sorted neighbor (all-occupied falls back to the parent
/// cell and never fails the tick).
fn birth_requests(
    world: &World,
    cell: &CellKey,
    params: &SpeciesParams,
    species: Option<&JsonValue>,
) -> Vec<SpawnRequest> {
    let child_wildlife = json!({
        "state": "graze",
        "satiety": 0.5,
        "reproduction_cooldown": 0,
        "flee_ticks": 0,
        "rest_ticks": 0,
    });
    let child_species = SpeciesParams::component_json(species);
    let mut spots: Vec<CellKey> = Vec::new();
    if let Some(map) = world.map.as_ref() {
        let mut neighbors = map.neighbors(cell);
        neighbors.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        for neighbor in neighbors {
            if spots.len() >= params.litter_size as usize {
                break;
            }
            if world.entities_in_cell(&neighbor).is_empty() {
                spots.push(neighbor);
            }
        }
    }
    while spots.len() < params.litter_size as usize {
        spots.push(cell.clone());
    }
    spots
        .into_iter()
        .map(|position| SpawnRequest {
            wildlife: child_wildlife.clone(),
            species: child_species.clone(),
            position,
        })
        .collect()
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
