use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::{Value as JsonValue, json};

/// Default hp for body parts missing hp/max_hp fields (backward compat for pre-migration data).
const DEFAULT_PART_HP: f64 = 25.0;

/// Returns the hp of a part, defaulting to DEFAULT_PART_HP if absent.
fn part_hp(part: &JsonValue) -> f64 {
    part.get("hp")
        .and_then(|v| v.as_f64())
        .unwrap_or(DEFAULT_PART_HP)
}

/// Returns the max_hp of a part, defaulting to DEFAULT_PART_HP if absent.
fn part_max_hp(part: &JsonValue) -> f64 {
    part.get("max_hp")
        .and_then(|v| v.as_f64())
        .unwrap_or(DEFAULT_PART_HP)
}

/// Returns the status of a part, defaulting to "healthy" if absent.
fn part_status(part: &JsonValue) -> &str {
    part.get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("healthy")
}

/// Updates part status based on hp thresholds per spec R007.
fn update_part_status(part: &mut JsonValue) {
    let hp = part_hp(part);
    let max_hp = part_max_hp(part);
    let current_status = part_status(part).to_string();

    // Already missing stays missing
    let new_status = if current_status == "missing" {
        "missing"
    } else if current_status == "broken" && hp <= 0.0 {
        // broken → missing on subsequent damage that keeps hp ≤ 0
        "missing"
    } else if hp <= 0.0 {
        "broken"
    } else if hp <= 0.5 * max_hp {
        "wounded"
    } else {
        "healthy"
    };

    part["status"] = json!(new_status);
}

/// Applies a single damage amount to a part. Returns the actual damage dealt.
fn apply_damage_to_part(part: &mut JsonValue, amount: f64) -> f64 {
    if part_status(part) == "missing" {
        return 0.0;
    }

    let hp = part_hp(part);
    let actual_damage = amount.min(hp);
    let new_hp = (hp - actual_damage).max(0.0);

    part["hp"] = json!(new_hp);
    update_part_status(part);
    actual_damage
}

/// Applies damage to a named part recursively through the hierarchy.
fn apply_damage_to_named_part(parts: &mut [JsonValue], target: &str, amount: f64) {
    for part in parts.iter_mut() {
        if part.get("name").and_then(|n| n.as_str()) == Some(target) {
            apply_damage_to_part(part, amount);
            return;
        }
        if let Some(children) = part.get_mut("children").and_then(|v| v.as_array_mut()) {
            apply_damage_to_named_part(children, target, amount);
        }
    }
}

/// Recursively applies damage to all parts by name (for distributing to each).
fn apply_damage_by_name(parts: &mut [JsonValue], name: &str, amount: f64) {
    apply_damage_to_named_part(parts, name, amount);
}

/// Collects (name, max_hp, status) tuples for all parts recursively.
fn collect_part_info(parts: &[JsonValue], result: &mut Vec<(String, f64, String)>) {
    for part in parts {
        let name = part
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        result.push((name, part_max_hp(part), part_status(part).to_string()));
        if let Some(children) = part.get("children").and_then(|v| v.as_array()) {
            collect_part_info(children, result);
        }
    }
}

/// System that processes PendingDamage components and distributes damage across body parts.
///
/// For each entity with both Body and PendingDamage:
/// - Targeted damage (`target_part` specified) goes directly to that part.
/// - Untargeted damage (`target_part` null) distributes proportionally by max_hp across non-missing parts.
/// - After all damages are processed, entity Health.current is recomputed as sum of all part HPs.
/// - PendingDamage is removed after processing.
pub struct BodyPartDamageSystem;

impl System for BodyPartDamageSystem {
    fn name(&self) -> &'static str {
        "BodyPartDamageSystem"
    }

    fn run(&mut self, world: &mut World) {
        // Collect entities that have both Body and PendingDamage
        let entities: Vec<u32> = {
            let body_entities: std::collections::HashSet<u32> = world
                .get_entities_with_component("Body")
                .into_iter()
                .collect();
            let pending_entities: std::collections::HashSet<u32> = world
                .get_entities_with_component("PendingDamage")
                .into_iter()
                .collect();
            body_entities
                .intersection(&pending_entities)
                .copied()
                .collect()
        };

        for entity in entities {
            let pending = match world.get_component(entity, "PendingDamage").cloned() {
                Some(p) => p,
                None => continue,
            };

            let damages = match pending.get("damages").and_then(|v| v.as_array()) {
                Some(d) if !d.is_empty() => d.clone(),
                _ => {
                    // Empty damages array → remove component, no state change
                    let _ = world.remove_component(entity, "PendingDamage");
                    continue;
                }
            };

            let mut body = match world.get_component(entity, "Body").cloned() {
                Some(b) => b,
                None => {
                    let _ = world.remove_component(entity, "PendingDamage");
                    continue;
                }
            };

            if let Some(parts) = body.get_mut("parts").and_then(|v| v.as_array_mut()) {
                // Phase 1: Collect damage amounts per part name
                let mut damage_per_part: std::collections::HashMap<String, f64> =
                    std::collections::HashMap::new();

                for damage_entry in &damages {
                    let amount = damage_entry
                        .get("amount")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    if amount <= 0.0 {
                        continue;
                    }

                    let target_part = damage_entry.get("target_part").and_then(|v| v.as_str());

                    if let Some(target) = target_part {
                        // Targeted: accumulate for the named part
                        *damage_per_part.entry(target.to_string()).or_insert(0.0) += amount;
                    } else {
                        // Untargeted: distribute proportionally across non-missing parts
                        let mut part_info = Vec::new();
                        collect_part_info(parts, &mut part_info);

                        let total_max_hp: f64 = part_info
                            .iter()
                            .filter(|(_, _, status)| status != "missing")
                            .map(|(_, max_hp, _)| max_hp)
                            .sum();

                        if total_max_hp > 0.0 {
                            for (name, max_hp, status) in &part_info {
                                if status == "missing" {
                                    continue;
                                }
                                let weight = max_hp / total_max_hp;
                                let share = amount * weight;
                                *damage_per_part.entry(name.clone()).or_insert(0.0) += share;
                            }
                        }
                    }
                }

                // Phase 2: Apply accumulated damage to each part
                for (name, amount) in &damage_per_part {
                    apply_damage_by_name(parts, name, *amount);
                }
            }

            // Recompute Health.current as sum of all parts' hp
            let total_hp = {
                let mut part_info = Vec::new();
                if let Some(parts) = body.get("parts").and_then(|v| v.as_array()) {
                    collect_part_info_for_hp(parts, &mut part_info);
                }
                part_info.iter().copied().sum::<f64>().max(0.0)
            };

            let _ = world.set_component(entity, "Body", body);

            // Update Health.current
            if let Some(healths) = world.components.get_mut("Health")
                && let Some(value) = healths.get_mut(&entity)
                && let Some(obj) = value.as_object_mut()
                && let Some(current) = obj.get_mut("current")
            {
                *current = json!(total_hp);
            }

            let _ = world.remove_component(entity, "PendingDamage");
        }
    }
}

/// Collects hp values from all parts recursively.
fn collect_part_info_for_hp(parts: &[JsonValue], result: &mut Vec<f64>) {
    for part in parts {
        result.push(part_hp(part));
        if let Some(children) = part.get("children").and_then(|v| v.as_array()) {
            collect_part_info_for_hp(children, result);
        }
    }
}
