//! Enemy AI behavior system with deterministic finite state machine.
//!
//! Implements R003–R018: idle/patrol/investigate/chase/attack/flee states, target selection,
//! A* pathfinding movement, deterministic RNG, noise detection via Hearing, and
//! collect-then-apply pattern.

use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::map::cell_key::CellKey;
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;
use serde_json::{Value as JsonValue, json};

/// Deterministic finite-state-machine system for enemy AI behaviors.
///
/// Each tick iterates entities with an `EnemyAI` component, reads their state,
/// evaluates transitions, and writes back the updated state. Movement is applied
/// directly to `Position` (matching the demo pattern), bypassing `MovementSystem`.
pub struct EnemyBehaviorSystem;

impl System for EnemyBehaviorSystem {
    fn name(&self) -> &'static str {
        "EnemyBehaviorSystem"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["FovUpdateSystem", "NoiseSystem"]
    }

    fn run(&mut self, world: &mut World) {
        // Collect entity IDs with EnemyAI to avoid borrow conflicts
        let entity_ids: Vec<u32> = world
            .get_entities_with_component("EnemyAI")
            .into_iter()
            .collect();

        // Collect-then-apply: gather all decisions first, then apply
        let mut decisions: Vec<EnemyDecision> = Vec::new();

        for eid in entity_ids {
            let Some(ai) = world.get_component(eid, "EnemyAI").cloned() else {
                continue;
            };
            let Some(position) = world.get_component(eid, "Position").cloned() else {
                continue;
            };
            let health = world.get_component(eid, "Health").cloned();
            let _hearing = world.get_component(eid, "Hearing").cloned();

            let state = ai
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("idle")
                .to_string();

            let decision = match state.as_str() {
                "idle" => tick_idle(world, eid, &ai, &position, health.as_ref()),
                "patrol" => tick_patrol(world, eid, &ai, &position, health.as_ref()),
                "chase" => tick_chase(world, eid, &ai, &position, health.as_ref()),
                "investigate" => tick_investigate(world, eid, &ai, &position, health.as_ref()),
                "attack" => tick_attack(world, eid, &ai, &position, health.as_ref()),
                "flee" => tick_flee(world, eid, &ai, &position, health.as_ref()),
                _ => None,
            };

            if let Some(d) = decision {
                decisions.push(d);
            }
        }

        // Apply all collected decisions
        for d in decisions {
            let _ = world.set_component(d.entity, "EnemyAI", d.new_ai);
            if let Some((x, y, z)) = d.new_position {
                let _ = world.set_component(
                    d.entity,
                    "Position",
                    json!({"pos": {"Square": {"x": x, "y": y, "z": z}}}),
                );
            }
            if let Some(route) = d.new_patrol_route {
                let _ = world.set_component(d.entity, "PatrolRoute", route);
            }
            if let Some(target) = d.attack_damage_target {
                world.damage_entity(target, 1.0);
            }
        }
    }
}

/// A pending state change for one entity, collected before mutation.
struct EnemyDecision {
    entity: u32,
    new_ai: JsonValue,
    new_position: Option<(i32, i32, i32)>,
    new_patrol_route: Option<JsonValue>,
    attack_damage_target: Option<u32>,
}

// ---------------------------------------------------------------------------
// Target selection
// ---------------------------------------------------------------------------

/// Priority: target_entity → target_faction → Type.kind == "player" (R014).
fn select_target(
    world: &World,
    eid: u32,
    ai: &JsonValue,
    visible: Option<&std::collections::HashSet<CellKey>>,
) -> Option<u32> {
    let visible = visible?;

    // 1. Explicit target_entity (R014.1)
    if let Some(target_id) = ai
        .get("target_entity")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        && world.is_entity_alive(target_id)
        && entity_visible_to(world, target_id, visible)
    {
        return Some(target_id);
    }

    // 2. target_faction with reputation filter (R014.2)
    if let Some(target_faction) = ai.get("target_faction").and_then(|v| v.as_str()) {
        let my_faction = world
            .get_component(eid, "Faction")
            .and_then(|f| f.get("faction_id"))
            .and_then(|v| v.as_str());

        if let Some(best) =
            find_nearest_hostile_in_faction(world, eid, target_faction, my_faction, visible)
        {
            return Some(best);
        }
    }

    // 3. Fallback: nearest visible player (R014.3)
    find_nearest_visible_player(world, eid, visible)
}

/// Check if an entity's cell is in the observer's visible set.
fn entity_visible_to(
    world: &World,
    target: u32,
    visible: &std::collections::HashSet<CellKey>,
) -> bool {
    world
        .get_component(target, "Position")
        .and_then(CellKey::from_position)
        .is_some_and(|cell| visible.contains(&cell))
}

/// Find the nearest visible entity whose Faction.faction_id matches and whose
/// Reputation for the observer's faction is negative.
fn find_nearest_hostile_in_faction(
    world: &World,
    eid: u32,
    target_faction: &str,
    my_faction: Option<&str>,
    visible: &std::collections::HashSet<CellKey>,
) -> Option<u32> {
    let my_pos = world
        .get_component(eid, "Position")
        .and_then(CellKey::from_position)?;

    let mut best: Option<(u32, i32)> = None;

    for other in visible.iter().flat_map(|cell| world.entities_in_cell(cell)) {
        if other == eid {
            continue;
        }
        let other_faction = world
            .get_component(other, "Faction")
            .and_then(|f| f.get("faction_id"))
            .and_then(|v| v.as_str());
        if other_faction != Some(target_faction) {
            continue;
        }

        if let Some(my_fid) = my_faction {
            let rep_negative = world
                .get_component(other, "Reputation")
                .and_then(|r| r.get("relations"))
                .and_then(|r| r.get(my_fid))
                .and_then(|r| r.get("score"))
                .and_then(|s| s.as_f64())
                .map(|s| s < 0.0)
                .unwrap_or(false);
            if !rep_negative {
                continue;
            }
        }

        let other_pos = world
            .get_component(other, "Position")
            .and_then(CellKey::from_position)?;
        let dist = manhattan_distance(&my_pos, &other_pos);
        if best.is_none_or(|(_, d)| dist < d) {
            best = Some((other, dist));
        }
    }

    best.map(|(id, _)| id)
}

/// Find the nearest visible entity with Type.kind == "player".
fn find_nearest_visible_player(
    world: &World,
    eid: u32,
    visible: &std::collections::HashSet<CellKey>,
) -> Option<u32> {
    let my_pos = world
        .get_component(eid, "Position")
        .and_then(CellKey::from_position)?;

    let mut best: Option<(u32, i32)> = None;

    for other in visible.iter().flat_map(|cell| world.entities_in_cell(cell)) {
        if other == eid {
            continue;
        }
        let is_player = world
            .get_component(other, "Type")
            .and_then(|t| t.get("kind"))
            .and_then(|k| k.as_str())
            == Some("player");
        if !is_player {
            continue;
        }

        let other_pos = world
            .get_component(other, "Position")
            .and_then(CellKey::from_position)?;
        let dist = manhattan_distance(&my_pos, &other_pos);
        if best.is_none_or(|(_, d)| dist < d) {
            best = Some((other, dist));
        }
    }

    best.map(|(id, _)| id)
}

// ---------------------------------------------------------------------------
// Movement helpers
// ---------------------------------------------------------------------------

/// Manhattan distance between two CellKeys (Square topology).
fn manhattan_distance(a: &CellKey, b: &CellKey) -> i32 {
    match (a, b) {
        (
            CellKey::Square {
                x: ax,
                y: ay,
                z: az,
            },
            CellKey::Square {
                x: bx,
                y: by,
                z: bz,
            },
        ) => (ax - bx).abs() + (ay - by).abs() + (az - bz).abs(),
        _ => 0,
    }
}

/// Compute the next step along a path from `from` toward `goal`.
fn move_toward(world: &World, from: &CellKey, goal: &CellKey) -> Option<(i32, i32, i32)> {
    let result = world.find_path(from, goal)?;
    if result.path.len() < 2 {
        return None;
    }
    cell_to_xyz(&result.path[1])
}

/// Convert a CellKey to (x, y, z).
fn cell_to_xyz(cell: &CellKey) -> Option<(i32, i32, i32)> {
    match cell {
        CellKey::Square { x, y, z } => Some((*x, *y, *z)),
        _ => None,
    }
}

/// Deterministic RNG seeded from (entity_id, turn) — R013.
fn deterministic_rng(entity_id: u32, turn: u32) -> SmallRng {
    let mut seed = [0u8; 32];
    seed[0..4].copy_from_slice(&entity_id.to_le_bytes());
    seed[4..8].copy_from_slice(&turn.to_le_bytes());
    SmallRng::from_seed(seed)
}

/// Create a default EnemyDecision for an entity.
fn default_decision(eid: u32, ai: JsonValue) -> EnemyDecision {
    EnemyDecision {
        entity: eid,
        new_ai: ai,
        new_position: None,
        new_patrol_route: None,
        attack_damage_target: None,
    }
}

// ---------------------------------------------------------------------------
// State tick functions
// ---------------------------------------------------------------------------

/// R006/R015 — idle: visual contact → chase; noise detection → investigate/chase;
/// has PatrolRoute → patrol; else stay idle.
fn tick_idle(
    world: &World,
    eid: u32,
    ai: &JsonValue,
    position: &JsonValue,
    _health: Option<&JsonValue>,
) -> Option<EnemyDecision> {
    let my_pos = CellKey::from_position(position)?;
    let visible = world.get_visible_cells(eid);

    // R006/R016: Visual contact within detection_range → chase (alert_level = 1.0)
    if let Some(target) = select_target(world, eid, ai, visible) {
        let detection_range = ai
            .get("detection_range")
            .and_then(|v| v.as_u64())
            .unwrap_or(8) as i32;

        let target_pos = world
            .get_component(target, "Position")
            .and_then(CellKey::from_position)?;
        let dist = manhattan_distance(&my_pos, &target_pos);

        if dist <= detection_range {
            let mut d = default_decision(eid, ai.clone());
            d.new_ai["state"] = json!("chase");
            d.new_ai["current_target"] = json!(target);
            d.new_ai["alert_level"] = json!(1.0);
            if let Some((x, y, z)) = cell_to_xyz(&my_pos) {
                d.new_ai["home_position"] = json!({"Square": {"x": x, "y": y, "z": z}});
            }
            d.new_ai["lost_target_ticks"] = json!(0);
            return Some(d);
        }
    }

    // R015: Noise detection via Hearing — escalate alert_level, transition to investigate/chase
    if let Some(d) = noise_detection(world, eid, ai, &my_pos) {
        return Some(d);
    }

    // R006: Has PatrolRoute → patrol
    if world.has_component(eid, "PatrolRoute") {
        let mut d = default_decision(eid, ai.clone());
        d.new_ai["state"] = json!("patrol");
        return Some(d);
    }

    None
}

/// R007/R015 — patrol: move toward waypoints, advance index, detect hostile → chase,
/// noise detection → investigate/chase.
fn tick_patrol(
    world: &World,
    eid: u32,
    ai: &JsonValue,
    position: &JsonValue,
    _health: Option<&JsonValue>,
) -> Option<EnemyDecision> {
    let my_pos = CellKey::from_position(position)?;
    let visible = world.get_visible_cells(eid);

    // R007/R016: Visual contact → chase (alert_level = 1.0, priority over patrol movement)
    if let Some(target) = select_target(world, eid, ai, visible) {
        let detection_range = ai
            .get("detection_range")
            .and_then(|v| v.as_u64())
            .unwrap_or(8) as i32;
        let target_pos = world
            .get_component(target, "Position")
            .and_then(CellKey::from_position)?;
        let dist = manhattan_distance(&my_pos, &target_pos);
        if dist <= detection_range {
            let mut d = default_decision(eid, ai.clone());
            d.new_ai["state"] = json!("chase");
            d.new_ai["current_target"] = json!(target);
            d.new_ai["alert_level"] = json!(1.0);
            if let Some((x, y, z)) = cell_to_xyz(&my_pos) {
                d.new_ai["home_position"] = json!({"Square": {"x": x, "y": y, "z": z}});
            }
            d.new_ai["lost_target_ticks"] = json!(0);
            return Some(d);
        }
    }

    // R015: Noise detection via Hearing — escalate alert_level, transition to investigate/chase
    if let Some(d) = noise_detection(world, eid, ai, &my_pos) {
        return Some(d);
    }

    let route = world.get_component(eid, "PatrolRoute")?.clone();
    let waypoints = route.get("waypoints").and_then(|w| w.as_array())?;
    if waypoints.is_empty() {
        return None;
    }

    let current_index = route
        .get("current_index")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    let loop_patrol = route.get("loop").and_then(|v| v.as_bool()).unwrap_or(true);
    let wait_ticks = route
        .get("wait_ticks")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    // Check wait counter (runtime field on EnemyAI)
    let wait_counter = ai.get("wait_counter").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    if wait_counter > 0 {
        let mut d = default_decision(eid, ai.clone());
        d.new_ai["wait_counter"] = json!(wait_counter - 1);
        return Some(d);
    }

    let wp = waypoints.get(current_index)?;
    let wp_cell = CellKey::from_position(wp)?;

    let dist = manhattan_distance(&my_pos, &wp_cell);
    if dist <= 1 {
        let next_index = current_index + 1;
        if next_index >= waypoints.len() {
            if loop_patrol {
                let mut d = default_decision(eid, ai.clone());
                let mut new_route = route.clone();
                new_route["current_index"] = json!(0);
                d.new_patrol_route = Some(new_route);
                if wait_ticks > 0 {
                    let mut rng = deterministic_rng(eid, world.turn);
                    d.new_ai["wait_counter"] = json!(rng.random_range(0..=wait_ticks));
                }
                return Some(d);
            } else {
                let mut d = default_decision(eid, ai.clone());
                d.new_ai["state"] = json!("idle");
                return Some(d);
            }
        } else {
            let mut d = default_decision(eid, ai.clone());
            let mut new_route = route.clone();
            new_route["current_index"] = json!(next_index);
            d.new_patrol_route = Some(new_route);
            if wait_ticks > 0 {
                let mut rng = deterministic_rng(eid, world.turn);
                d.new_ai["wait_counter"] = json!(rng.random_range(0..=wait_ticks));
            }
            return Some(d);
        }
    }

    // Move toward the waypoint
    if let Some(new_pos) = move_toward(world, &my_pos, &wp_cell) {
        let mut d = default_decision(eid, ai.clone());
        d.new_position = Some(new_pos);
        return Some(d);
    }

    None
}

/// R015 — investigate: visual contact → chase; noise escalation → chase at >0.7;
/// no movement; decay to idle when alert_level reaches 0.
fn tick_investigate(
    world: &World,
    eid: u32,
    ai: &JsonValue,
    position: &JsonValue,
    _health: Option<&JsonValue>,
) -> Option<EnemyDecision> {
    let my_pos = CellKey::from_position(position)?;
    let visible = world.get_visible_cells(eid);

    // R015/R016: Visual contact → chase (alert_level = 1.0)
    if let Some(target) = select_target(world, eid, ai, visible) {
        let detection_range = ai
            .get("detection_range")
            .and_then(|v| v.as_u64())
            .unwrap_or(8) as i32;
        let target_pos = world
            .get_component(target, "Position")
            .and_then(CellKey::from_position)?;
        let dist = manhattan_distance(&my_pos, &target_pos);
        if dist <= detection_range {
            let mut d = default_decision(eid, ai.clone());
            d.new_ai["state"] = json!("chase");
            d.new_ai["current_target"] = json!(target);
            d.new_ai["alert_level"] = json!(1.0);
            if let Some((x, y, z)) = cell_to_xyz(&my_pos) {
                d.new_ai["home_position"] = json!({"Square": {"x": x, "y": y, "z": z}});
            }
            d.new_ai["lost_target_ticks"] = json!(0);
            return Some(d);
        }
    }

    // R015: Noise detection — escalate to chase at >0.7, otherwise stay investigating
    if let Some(mut d) = noise_detection(world, eid, ai, &my_pos) {
        let alert = d
            .new_ai
            .get("alert_level")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let state = d
            .new_ai
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("investigate");
        // R015: No stimulus + alert_level == 0 → idle
        if state == "investigate" && alert == 0.0 {
            d.new_ai["state"] = json!("idle");
        }
        return Some(d);
    }

    // No Hearing component or already calm — settle to idle
    let mut d = default_decision(eid, ai.clone());
    let alert = d
        .new_ai
        .get("alert_level")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    if alert == 0.0 {
        d.new_ai["state"] = json!("idle");
        return Some(d);
    }
    None
}

/// R008 — chase: move toward target, transition to attack/flee/idle.
fn tick_chase(
    world: &World,
    eid: u32,
    ai: &JsonValue,
    position: &JsonValue,
    health: Option<&JsonValue>,
) -> Option<EnemyDecision> {
    let my_pos = CellKey::from_position(position)?;
    let visible = world.get_visible_cells(eid);

    let target_id = ai
        .get("current_target")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    // R008: If no target or target dead, chase noise if present, else return home
    let target_id = match target_id {
        Some(id) if world.is_entity_alive(id) => id,
        _ => {
            // R015: Target lost but noise present — continue chase toward the noise source
            if world.get_noise_at(&my_pos).unwrap_or(0.0) > 0.0 {
                let mut d = default_decision(eid, ai.clone());
                d.new_ai["lost_target_ticks"] = json!(0);
                if let Some(new_pos) = move_toward_noise(world, &my_pos) {
                    d.new_position = Some(new_pos);
                }
                return Some(d);
            }
            return return_to_home(world, eid, ai);
        }
    };

    let target_visible = visible
        .as_ref()
        .is_some_and(|v| entity_visible_to(world, target_id, v));

    let mut d = default_decision(eid, ai.clone());

    if !target_visible {
        // R008: Increment lost target counter
        let lost_ticks = d
            .new_ai
            .get("lost_target_ticks")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            + 1;
        d.new_ai["lost_target_ticks"] = json!(lost_ticks);
        if lost_ticks >= 3 {
            return return_to_home(world, eid, &d.new_ai);
        }
        return Some(d);
    }

    d.new_ai["lost_target_ticks"] = json!(0);

    // R008: Flee check
    if should_flee(&d.new_ai, health) {
        d.new_ai["state"] = json!("flee");
        d.new_ai["flee_ticks"] = json!(0);
        return Some(d);
    }

    // R008: Attack range check
    let target_pos = world
        .get_component(target_id, "Position")
        .and_then(CellKey::from_position)?;
    let dist = manhattan_distance(&my_pos, &target_pos);
    let attack_range = d
        .new_ai
        .get("attack_range")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as i32;

    if dist <= attack_range {
        d.new_ai["state"] = json!("attack");
        return Some(d);
    }

    // R011: Move toward target via pathfinding
    if let Some(new_pos) = move_toward(world, &my_pos, &target_pos) {
        d.new_position = Some(new_pos);
    }

    Some(d)
}

/// R009 — attack: deal damage, transition to chase/flee/idle.
fn tick_attack(
    world: &World,
    eid: u32,
    ai: &JsonValue,
    position: &JsonValue,
    health: Option<&JsonValue>,
) -> Option<EnemyDecision> {
    let my_pos = CellKey::from_position(position)?;

    let target_id = ai
        .get("current_target")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    // R009: If no target or target dead → idle
    let target_id = match target_id {
        Some(id) if world.is_entity_alive(id) => id,
        _ => {
            let mut d = default_decision(eid, ai.clone());
            d.new_ai["state"] = json!("idle");
            d.new_ai["current_target"] = json!(null);
            return Some(d);
        }
    };

    // R009: Flee check
    if should_flee(ai, health) {
        let mut d = default_decision(eid, ai.clone());
        d.new_ai["state"] = json!("flee");
        d.new_ai["flee_ticks"] = json!(0);
        return Some(d);
    }

    // R009: Attack range check
    let target_pos = world
        .get_component(target_id, "Position")
        .and_then(CellKey::from_position)?;
    let dist = manhattan_distance(&my_pos, &target_pos);
    let attack_range = ai.get("attack_range").and_then(|v| v.as_u64()).unwrap_or(1) as i32;

    if dist > attack_range {
        let mut d = default_decision(eid, ai.clone());
        d.new_ai["state"] = json!("chase");
        return Some(d);
    }

    // R009: Deal 1 damage to target (applied in run()'s apply phase)
    let mut d = default_decision(eid, ai.clone());
    d.attack_damage_target = Some(target_id);
    Some(d)
}

/// R010 — flee: move away from target, recover after 3 ticks.
fn tick_flee(
    world: &World,
    eid: u32,
    ai: &JsonValue,
    position: &JsonValue,
    health: Option<&JsonValue>,
) -> Option<EnemyDecision> {
    let my_pos = CellKey::from_position(position)?;

    let target_id = ai
        .get("current_target")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let mut d = default_decision(eid, ai.clone());
    let flee_ticks = d
        .new_ai
        .get("flee_ticks")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        + 1;
    d.new_ai["flee_ticks"] = json!(flee_ticks);

    // R010: After 3 ticks, if health above threshold → idle
    if flee_ticks >= 3 && !should_flee(&d.new_ai, health) {
        d.new_ai["state"] = json!("idle");
        return Some(d);
    }

    // R012: Move away from target
    if let Some(target_id) = target_id
        && let Some(target_pos) = world
            .get_component(target_id, "Position")
            .and_then(CellKey::from_position)
        && let Some(new_pos) = flee_from(world, &my_pos, &target_pos)
    {
        d.new_position = Some(new_pos);
        return Some(d);
    }

    // R012: Deterministic random walk as fallback
    if let Some(pos) = random_walk(world, &my_pos, eid, world.turn) {
        d.new_position = Some(pos);
    }

    Some(d)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// R008/R009: Check if health fraction is at or below flee threshold.
fn should_flee(ai: &JsonValue, health: Option<&JsonValue>) -> bool {
    let threshold = ai
        .get("flee_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.25);
    match health {
        Some(h) => {
            let current = h.get("current").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let max = h.get("max").and_then(|v| v.as_f64()).unwrap_or(1.0);
            if max <= 0.0 {
                return true;
            }
            current / max <= threshold
        }
        None => false,
    }
}

/// R015: Evaluate noise stimulus for an entity with Hearing.
///
/// Returns a decision when the entity has Hearing and either noise is detected
/// (alert_level incremented by `noise × sensitivity`, capped at 1.0, with state
/// transitions at `noise_detection_threshold` → investigate and 0.7 → chase) or
/// alert_level decays by `alert_decay_rate` when no noise is present (floored at 0).
/// Returns None when the entity has no Hearing or is already calm with no noise.
fn noise_detection(
    world: &World,
    eid: u32,
    ai: &JsonValue,
    my_pos: &CellKey,
) -> Option<EnemyDecision> {
    let hearing = world.get_component(eid, "Hearing").cloned()?;
    let sensitivity = hearing
        .get("sensitivity")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);
    let threshold = hearing
        .get("threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.1);

    let noise_level = world.get_noise_at(my_pos).unwrap_or(0.0);
    let mut d = default_decision(eid, ai.clone());
    let current_alert = d
        .new_ai
        .get("alert_level")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    if noise_level >= threshold * sensitivity {
        let new_alert = (current_alert + noise_level * sensitivity).min(1.0);
        d.new_ai["alert_level"] = json!(new_alert);
        if new_alert > 0.7 {
            d.new_ai["state"] = json!("chase");
        } else if new_alert
            > ai.get("noise_detection_threshold")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.3)
        {
            d.new_ai["state"] = json!("investigate");
        }
        return Some(d);
    }

    // R017: No noise stimulus — decay alert_level by alert_decay_rate, floored at 0
    if current_alert > 0.0 {
        let decay_rate = ai
            .get("alert_decay_rate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.1);
        d.new_ai["alert_level"] = json!((current_alert - decay_rate).max(0.0));
        return Some(d);
    }

    None
}

/// Return to home_position after losing target.
fn return_to_home(world: &World, eid: u32, ai: &JsonValue) -> Option<EnemyDecision> {
    let my_pos = world
        .get_component(eid, "Position")
        .and_then(CellKey::from_position)?;

    if let Some(home) = ai.get("home_position").and_then(CellKey::from_position) {
        let dist = manhattan_distance(&my_pos, &home);
        if dist <= 1 {
            // Already at home — transition to idle
            let mut d = default_decision(eid, ai.clone());
            d.new_ai["state"] = json!("idle");
            d.new_ai["current_target"] = json!(null);
            d.new_ai["lost_target_ticks"] = json!(0);
            return Some(d);
        }
        if let Some(new_pos) = move_toward(world, &my_pos, &home) {
            let mut d = default_decision(eid, ai.clone());
            d.new_position = Some(new_pos);
            return Some(d);
        }
    }

    let mut d = default_decision(eid, ai.clone());
    d.new_ai["state"] = json!("idle");
    d.new_ai["current_target"] = json!(null);
    d.new_ai["lost_target_ticks"] = json!(0);
    Some(d)
}

/// R012: Move one step away from the target.
fn flee_from(world: &World, from: &CellKey, target: &CellKey) -> Option<(i32, i32, i32)> {
    let map = world.map.as_ref()?;
    let neighbors = map.neighbors(from);
    let target_xyz = cell_to_xyz(target)?;

    let mut best: Option<((i32, i32, i32), i32)> = None;
    for n in &neighbors {
        if let Some((nx, ny, nz)) = cell_to_xyz(n) {
            let dist =
                (nx - target_xyz.0).abs() + (ny - target_xyz.1).abs() + (nz - target_xyz.2).abs();
            if best.is_none_or(|(_, d)| dist > d) {
                best = Some(((nx, ny, nz), dist));
            }
        }
    }

    best.map(|(pos, _)| pos)
}

/// R015: Move one step toward the neighbor with the highest noise level.
fn move_toward_noise(world: &World, from: &CellKey) -> Option<(i32, i32, i32)> {
    let map = world.map.as_ref()?;
    let neighbors = map.neighbors(from);
    let current_noise = world.get_noise_at(from).unwrap_or(0.0);

    let mut best: Option<((i32, i32, i32), f64)> = None;
    for n in &neighbors {
        let noise = world.get_noise_at(n).unwrap_or(0.0);
        if noise > current_noise
            && best.is_none_or(|(_, n)| noise > n)
            && let Some(xyz) = cell_to_xyz(n)
        {
            best = Some((xyz, noise));
        }
    }

    best.map(|(pos, _)| pos)
}

/// R013: Deterministic random walk — picks a random neighbor.
fn random_walk(
    world: &World,
    from: &CellKey,
    entity_id: u32,
    turn: u32,
) -> Option<(i32, i32, i32)> {
    let map = world.map.as_ref()?;
    let neighbors = map.neighbors(from);
    if neighbors.is_empty() {
        return None;
    }

    let mut rng = deterministic_rng(entity_id, turn);
    let idx = rng.random_range(0..neighbors.len());
    cell_to_xyz(&neighbors[idx])
}
