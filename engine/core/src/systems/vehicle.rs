//! Vehicle carrier system: embark/disembark state plus mounted co-movement.
//!
//! Vehicles carry riders listed in the `Vehicle.occupants` array (the single
//! source of mount truth). Each tick a vehicle advances up to `speed` steps
//! along its `Vehicle.move_path`, validating every step against
//! `blocked_terrains` plus `walkable:false` cell metadata; a failing step
//! truncates the remaining path and emits `vehicle_move_blocked`. Every
//! surviving occupant's `Position` is then synced to the vehicle's resulting
//! cell, so mounted riders never move independently. Province vehicles hold
//! position (documented no-step).
//!
//! Follows the [`EcosystemSystem`](super::ecosystem) collect-then-apply
//! pattern: vehicle IDs are collected sorted ascending, decisions are made on
//! pure reads, and a single apply phase writes `Vehicle` + `Position` and
//! emits events. No randomness is used, so ticks are deterministic.

use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::map::cell_key::CellKey;
use serde_json::{Value as JsonValue, json};

/// Default rider capacity when the component field is null or missing.
const DEFAULT_CAPACITY: u64 = 2;
/// Default steps-per-tick when the component field is null or missing.
const DEFAULT_SPEED: u64 = 1;

/// Deterministic FSM-free carrier system for vehicle movement.
pub struct VehicleSystem;

impl System for VehicleSystem {
    fn name(&self) -> &'static str {
        "VehicleSystem"
    }

    fn run(&mut self, world: &mut World) {
        let mut entity_ids = world.get_entities_with_component("Vehicle");
        entity_ids.sort_unstable();

        let mut decisions: Vec<VehicleDecision> = Vec::new();
        for eid in entity_ids {
            let Some(vehicle) = world.get_component(eid, "Vehicle").cloned() else {
                continue;
            };
            let position = world.get_component(eid, "Position").cloned();
            let start_cell = position.as_ref().and_then(CellKey::from_position);
            if let Some(d) = decide(world, eid, &vehicle, start_cell.as_ref()) {
                decisions.push(d);
            }
        }

        for d in decisions {
            let _ = world.set_component(d.entity, "Vehicle", d.new_vehicle);
            if let Some(cell) = &d.new_position {
                let _ = world.set_component(d.entity, "Position", cell_position_json(cell));
            }
            for (rider, cell) in &d.rider_sync {
                let _ = world.set_component(*rider, "Position", cell_position_json(cell));
            }
            if let Some(blocked) = &d.blocked_cell {
                let _ = world.send_event(
                    "vehicle_move_blocked",
                    json!({
                        "vehicle": d.entity,
                        "cell": cell_to_step_json(blocked),
                    }),
                );
            }
        }
    }
}

/// A pending per-vehicle update collected before mutation.
struct VehicleDecision {
    entity: u32,
    new_vehicle: JsonValue,
    new_position: Option<CellKey>,
    blocked_cell: Option<CellKey>,
    rider_sync: Vec<(u32, CellKey)>,
}

/// Pure-read decision for one vehicle: occupant cleanup, up-to-`speed`
/// stepping with per-step terrain guards, and rider co-location targets.
fn decide(
    world: &World,
    eid: u32,
    vehicle: &JsonValue,
    start_cell: Option<&CellKey>,
) -> Option<VehicleDecision> {
    let (capacity, speed, blocked_terrains) = vehicle_params(vehicle);
    let _ = capacity;

    let mut occupants = occupants_of(vehicle);
    occupants.retain(|rider| world.entity_exists(*rider));

    let mut new_vehicle = vehicle.clone();
    new_vehicle["occupants"] = json!(&occupants);

    let Some(start) = start_cell else {
        return Some(VehicleDecision {
            entity: eid,
            new_vehicle,
            new_position: None,
            blocked_cell: None,
            rider_sync: Vec::new(),
        });
    };

    // Province vehicles hold position; riders stay synced to the held cell.
    if matches!(start, CellKey::Province { .. }) {
        let rider_sync = occupants
            .iter()
            .map(|rider| (*rider, start.clone()))
            .collect();
        return Some(VehicleDecision {
            entity: eid,
            new_vehicle,
            new_position: None,
            blocked_cell: None,
            rider_sync,
        });
    }

    let steps = move_path_of(vehicle);
    let mut current = start.clone();
    let mut consumed = 0usize;
    let mut blocked_cell: Option<CellKey> = None;
    for step in steps.iter().take(speed as usize) {
        let Some(cell) = step_to_cell(step) else {
            break;
        };
        if cell_blocked(world, &blocked_terrains, &cell) {
            blocked_cell = Some(cell);
            break;
        }
        current = cell;
        consumed += 1;
    }

    let remaining: Vec<JsonValue> = if blocked_cell.is_some() {
        Vec::new()
    } else {
        steps.into_iter().skip(consumed).collect()
    };
    if remaining.is_empty() {
        if let Some(obj) = new_vehicle.as_object_mut() {
            obj.remove("move_path");
        }
    } else {
        new_vehicle["move_path"] = json!(remaining);
    }

    let new_position = if current != *start {
        Some(current.clone())
    } else {
        None
    };
    let rider_sync = occupants
        .iter()
        .map(|rider| (*rider, current.clone()))
        .collect();

    Some(VehicleDecision {
        entity: eid,
        new_vehicle,
        new_position,
        blocked_cell,
        rider_sync,
    })
}

/// Capacity, per-tick speed, and terrain block list with schema defaults applied.
pub(crate) fn vehicle_params(vehicle: &JsonValue) -> (u64, u64, Vec<String>) {
    let capacity = vehicle
        .get("capacity")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_CAPACITY)
        .max(1);
    let speed = vehicle
        .get("speed")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_SPEED)
        .max(1);
    let blocked = vehicle
        .get("blocked_terrains")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    (capacity, speed, blocked)
}

/// Occupant entity IDs from the `occupants` array; ignores non-integer entries.
pub(crate) fn occupants_of(vehicle: &JsonValue) -> Vec<u32> {
    vehicle
        .get("occupants")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_u64().map(|id| id as u32))
                .collect()
        })
        .unwrap_or_default()
}

/// Raw `move_path` step values in stored order.
pub(crate) fn move_path_of(vehicle: &JsonValue) -> Vec<JsonValue> {
    vehicle
        .get("move_path")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

/// True when the cell is impassable for the vehicle: `walkable:false` metadata
/// always blocks; a `terrain` metadata string blocks when listed in
/// `blocked_terrains`. Cells without metadata never block on terrain.
pub(crate) fn cell_blocked(world: &World, blocked_terrains: &[String], cell: &CellKey) -> bool {
    let meta = world.get_cell_metadata(cell);
    match meta {
        None => false,
        Some(m) => {
            if m.get("walkable").and_then(|v| v.as_bool()) == Some(false) {
                return true;
            }
            match m.get("terrain").and_then(|v| v.as_str()) {
                Some(terrain) => blocked_terrains.iter().any(|t| t == terrain),
                None => false,
            }
        }
    }
}

/// Parse one stored path step into a [`CellKey`], accepting both the bare
/// `{"Square": ...}` shape and the `{"pos": {...}}`-wrapped shape.
pub(crate) fn step_to_cell(step: &JsonValue) -> Option<CellKey> {
    CellKey::from_position(step)
}

/// Serialize a [`CellKey`] as one stored path step (bare shape, matching the
/// shape `MovementSystem` consumes).
pub(crate) fn cell_to_step_json(cell: &CellKey) -> JsonValue {
    match cell {
        CellKey::Square { x, y, z } => json!({ "Square": { "x": x, "y": y, "z": z } }),
        CellKey::Hex { q, r, z } => json!({ "Hex": { "q": q, "r": r, "z": z } }),
        CellKey::Province { id } => json!({ "Province": { "id": id } }),
    }
}

/// Serialize a [`CellKey`] as a `Position`-shaped value, preserving the
/// entity's existing topology variant.
pub(crate) fn cell_position_json(cell: &CellKey) -> JsonValue {
    match cell {
        CellKey::Square { x, y, z } => json!({ "pos": { "Square": { "x": x, "y": y, "z": z } } }),
        CellKey::Hex { q, r, z } => json!({ "pos": { "Hex": { "q": q, "r": r, "z": z } } }),
        CellKey::Province { id } => json!({ "pos": { "Province": { "id": id } } }),
    }
}
