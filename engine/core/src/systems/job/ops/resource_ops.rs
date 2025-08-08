//! Resource and inventory operations for the job system.
//!
//! This module centralizes all resource pickup, delivery, and inventory calculations
//! for jobs and agents. It is designed for reuse, testing, and clarity.

use crate::ecs::world::World;
use serde_json::{Value as JsonValue, json};
use std::collections::HashMap;

/// Calculates the maximum pickup for each resource kind, considering agent inventory limits,
/// job requirements, and stockpile availability.
///
/// Returns a vector of `{ "kind": ..., "amount": ... }` values representing the pickup.
pub fn calculate_pickup(
    world: &World,
    agent_id: u32,
    requirements: &[JsonValue],
    job: &JsonValue,
    stock_resources: &serde_json::Map<String, JsonValue>,
) -> Vec<JsonValue> {
    let mut max_weight = f64::INFINITY;
    let mut max_slots = usize::MAX;
    let mut max_volume = f64::INFINITY;
    let mut cur_weight = 0.0;
    let mut cur_slots = 0;
    let mut cur_volume = 0.0;
    if let Some(inv) = world.get_component(agent_id, "Inventory") {
        max_weight = inv
            .get("max_weight")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY);
        max_slots = inv
            .get("max_slots")
            .and_then(|v| v.as_u64())
            .unwrap_or(u64::MAX) as usize;
        max_volume = inv
            .get("max_volume")
            .and_then(|v| v.as_f64())
            .unwrap_or(f64::INFINITY);
        cur_weight = inv.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
        cur_slots = inv
            .get("slots")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        cur_volume = inv.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0);
    }

    let mut pickup: Vec<JsonValue> = Vec::new();

    // Use a shadow copy for calculation, never modify the actual map in this function
    let mut simulated_resources = stock_resources.clone();

    for req in requirements {
        let kind = req.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        let amount_needed = req.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);

        // How much already delivered for this kind?
        let delivered_so_far = job
            .get("delivered_resources")
            .and_then(|arr| arr.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|r| r.get("kind") == Some(&JsonValue::String(kind.to_string())))
            })
            .and_then(|r| r.get("amount").and_then(|v| v.as_i64()))
            .unwrap_or(0);

        let amount_remaining = amount_needed - delivered_so_far;
        if amount_remaining <= 0 {
            continue;
        }

        // How much available in stockpile?
        let available = simulated_resources
            .get(kind)
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        if available <= 0 {
            continue;
        }

        let (unit_weight, unit_volume) =
            crate::systems::economic::resource::get_resource_unit_properties(world, kind);
        let can_carry_by_weight = ((max_weight - cur_weight) / unit_weight).floor() as i64;
        let can_carry_by_volume = ((max_volume - cur_volume) / unit_volume).floor() as i64;
        let can_carry_by_slots = max_slots as i64 - cur_slots as i64;

        let mut can_carry = amount_remaining.min(available);
        can_carry = can_carry.min(can_carry_by_weight);
        can_carry = can_carry.min(can_carry_by_volume);
        can_carry = can_carry.min(can_carry_by_slots);

        if can_carry > 0 {
            pickup.push(json!({"kind": kind, "amount": can_carry}));
            cur_weight += unit_weight * can_carry as f64;
            cur_volume += unit_volume * can_carry as f64;
            cur_slots += 1;
            // Only update shadow copy, never the actual resource map
            let entry = simulated_resources
                .entry(kind.to_string())
                .or_insert(json!(0));
            *entry = json!(entry.as_i64().unwrap_or(0) - can_carry);
        }
    }

    pickup
}

/// Applies a pickup to the agent's inventory and updates the stockpile component.
pub fn apply_pickup(
    world: &mut World,
    agent_id: u32,
    pickup: &[JsonValue],
    stockpile_id: u32,
    stockpile: &serde_json::Map<String, JsonValue>,
) {
    // Update agent's carried_resources
    let mut agent = world.get_component(agent_id, "Agent").cloned().unwrap();
    agent["carried_resources"] = json!(pickup);
    let _ = world.set_component(agent_id, "Agent", agent);

    // Update stockpile resources
    let mut stockpile_val = world
        .get_component(stockpile_id, "Stockpile")
        .cloned()
        .unwrap();
    stockpile_val["resources"] = json!(stockpile);
    let _ = world.set_component(stockpile_id, "Stockpile", stockpile_val);
}

/// Updates delivered_resources on a job after delivery, combining with any existing delivered resources.
pub fn accumulate_delivery(
    requirements: &[JsonValue],
    delivered: &[JsonValue],
    carried: &[JsonValue],
) -> Vec<JsonValue> {
    let mut delivered_map: HashMap<String, i64> = HashMap::new();
    for res in delivered {
        if let (Some(kind), Some(amount)) = (
            res.get("kind").and_then(|v| v.as_str()),
            res.get("amount").and_then(|v| v.as_i64()),
        ) {
            delivered_map.insert(kind.to_string(), amount);
        }
    }
    for res in carried {
        if let (Some(kind), Some(amount)) = (
            res.get("kind").and_then(|v| v.as_str()),
            res.get("amount").and_then(|v| v.as_i64()),
        ) {
            *delivered_map.entry(kind.to_string()).or_insert(0) += amount;
        }
    }
    let mut new_delivered: Vec<JsonValue> = Vec::new();
    for req in requirements {
        if let Some(kind) = req.get("kind").and_then(|v| v.as_str()) {
            let amount = delivered_map.get(kind).cloned().unwrap_or(0);
            new_delivered.push(json!({"kind": kind, "amount": amount}));
        }
    }
    new_delivered
}
