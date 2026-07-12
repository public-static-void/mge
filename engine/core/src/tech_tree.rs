//! Tech tree and research system definitions.
//!
//! Provides loading of tech tree definitions from JSON, per-entity tech progress
//! management, prerequisite checking, and research queue manipulation.

use crate::ecs::world::World;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::path::Path;
use std::sync::OnceLock;

/// A prerequisite for unlocking a tech node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prerequisite {
    /// Either "tech" or "skill"
    #[serde(rename = "type")]
    pub prereq_type: String,
    /// ID of the tech or skill
    pub id: String,
    /// Required skill level (only for skill-type prerequisites)
    #[serde(default)]
    pub level: Option<f64>,
}

/// An effect that fires when a tech is unlocked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    /// Action identifier
    pub action: String,
    /// Action-specific data payload
    #[serde(default)]
    pub data: Option<JsonValue>,
}

/// A single tech tree node definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechNode {
    /// Unique tech identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Description text
    #[serde(default)]
    pub description: String,
    /// Research point cost to complete
    #[serde(default = "default_cost")]
    pub cost: f64,
    /// Tech tier (for display / sorting)
    #[serde(default = "default_tier")]
    pub tier: u32,
    /// Category grouping
    #[serde(default = "default_category")]
    pub category: String,
    /// Icon identifier (optional)
    #[serde(default)]
    pub icon: String,
    /// Prerequisites that must be met before researching
    #[serde(default)]
    pub prerequisites: Vec<Prerequisite>,
    /// Effects triggered when tech is unlocked
    #[serde(default)]
    pub effects: Vec<Effect>,
}

fn default_cost() -> f64 {
    100.0
}

fn default_tier() -> u32 {
    1
}

fn default_category() -> String {
    "general".to_string()
}

/// Loaded tech tree data: map of tech ID → TechNode.
type TechTreeMap = Vec<TechNode>;

/// Loads the tech tree from tech_tree.json on first access.
fn get_tech_tree_inner() -> &'static TechTreeMap {
    static TECH_TREE: OnceLock<TechTreeMap> = OnceLock::new();
    TECH_TREE.get_or_init(|| {
        let paths = [
            "engine/assets/schemas/tech_tree.json", // from workspace root
            "../engine/assets/schemas/tech_tree.json", // from engine/ subdir
            "../../engine/assets/schemas/tech_tree.json", // from engine/core/ subdir (tests)
        ];
        for path_str in &paths {
            let path = Path::new(path_str);
            if path.exists() {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        if let Ok(json) = serde_json::from_str::<JsonValue>(&content)
                            && let Some(techs) = json.get("techs").and_then(|v| v.as_array())
                        {
                            let nodes: Vec<TechNode> = techs
                                .iter()
                                .filter_map(|t| {
                                    let mut node: TechNode =
                                        serde_json::from_value(t.clone()).ok()?;
                                    // Ensure cost is at least 1 to avoid division by zero
                                    if node.cost < 1.0 {
                                        node.cost = 1.0;
                                    }
                                    Some(node)
                                })
                                .collect();
                            if !nodes.is_empty() {
                                return nodes;
                            }
                        }
                    }
                    Err(_) => continue,
                }
                break;
            }
        }
        // Return empty vec if file not found or parse error
        Vec::new()
    })
}

/// Returns all tech tree nodes.
pub fn get_tech_tree() -> &'static Vec<TechNode> {
    get_tech_tree_inner()
}

/// Returns a specific tech node by ID, or None if not found.
pub fn get_tech_node(id: &str) -> Option<&'static TechNode> {
    get_tech_tree_inner().iter().find(|n| n.id == id)
}

/// Reads the TechProgress component for an entity, returning the raw JSON or None.
pub fn get_tech_progress(world: &World, entity: u32) -> Option<JsonValue> {
    world.get_component(entity, "TechProgress").cloned()
}

/// Returns a list of completed tech IDs for an entity.
pub fn get_completed_techs(world: &World, entity: u32) -> Vec<String> {
    get_tech_progress(world, entity)
        .and_then(|p| {
            p.get("completed")
                .and_then(|c| c.as_object())
                .map(|m| m.keys().cloned().collect())
        })
        .unwrap_or_default()
}

/// Checks if a specific tech has been completed by an entity.
pub fn is_tech_completed(world: &World, entity: u32, tech_id: &str) -> bool {
    get_tech_progress(world, entity)
        .and_then(|p| {
            p.get("completed")
                .and_then(|c| c.as_object())
                .map(|m| m.contains_key(tech_id))
        })
        .unwrap_or(false)
}

/// Returns the current research queue for an entity.
pub fn get_research_queue(world: &World, entity: u32) -> Vec<String> {
    get_tech_progress(world, entity)
        .and_then(|p| {
            p.get("queue").and_then(|q| q.as_array()).map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
        })
        .unwrap_or_default()
}

/// Returns the queue progress map for an entity (tech_id → accumulated points).
pub fn get_research_queue_progress(world: &World, entity: u32) -> JsonValue {
    get_tech_progress(world, entity)
        .and_then(|p| p.get("queue_progress").cloned())
        .unwrap_or(json!({}))
}

// ── Prerequisite checking ────────────────────────────────────────────────

/// Checks if all prerequisites for a tech are satisfied by the given entity.
///
/// Returns `Ok(true)` if all prerequisites are met. Returns `Err(reason)` with
/// a human-readable reason if any prerequisite is not satisfied.
fn check_prerequisites(world: &World, entity: u32, node: &TechNode) -> Result<bool, String> {
    for prereq in &node.prerequisites {
        match prereq.prereq_type.as_str() {
            "tech" => {
                if !is_tech_completed(world, entity, &prereq.id) {
                    let name = get_tech_node(&prereq.id)
                        .map(|n| n.name.as_str())
                        .unwrap_or(&prereq.id);
                    return Err(format!("Requires tech '{}' ({})", prereq.id, name));
                }
            }
            "skill" => {
                let required_level = prereq.level.unwrap_or(1.0);
                let current_level = world
                    .get_component(entity, "SkillLevels")
                    .and_then(|sl| {
                        sl.get("skill_levels")
                            .and_then(|slm| slm.as_object())
                            .and_then(|m| m.get(&prereq.id))
                            .and_then(|v| v.as_f64())
                    })
                    .unwrap_or(0.0);
                if current_level < required_level {
                    return Err(format!(
                        "Requires skill '{}' level {} (current: {})",
                        prereq.id, required_level, current_level
                    ));
                }
            }
            other => {
                return Err(format!("Unknown prerequisite type: {other}"));
            }
        }
    }
    Ok(true)
}

// ── Cycle detection ─────────────────────────────────────────────────────

/// Checks if adding `tech_id` to the queue would create a dependency cycle.
///
/// A cycle occurs when the target tech depends (directly or transitively)
/// on a tech already in the queue, and that queued tech transitively depends
/// back on the target tech. We walk forward from the target's prerequisites.
fn would_create_cycle(node: &TechNode, queue: &[String]) -> bool {
    // Collect all tech IDs in the queue as Strings for HashSet lookup
    let queued_ids: std::collections::HashSet<String> = queue.iter().cloned().collect();

    // BFS/DFS through the target's prerequisite chain
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![node.id.clone()];

    while let Some(current) = stack.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        // If any prerequisite of the current node is in the queue, that's a cycle
        if queued_ids.contains(&current) {
            return true;
        }
        if let Some(current_node) = get_tech_node(&current) {
            for prereq in &current_node.prerequisites {
                if prereq.prereq_type == "tech" {
                    stack.push(prereq.id.clone());
                }
            }
        }
    }

    false
}

// ── Research action functions ───────────────────────────────────────────

/// Checks whether an entity has met all prerequisites and conditions to research a tech.
///
/// Returns `Ok(true)` if the tech can be researched, or `Err(reason)` with a
/// human-readable reason if it cannot.
pub fn can_research_tech(world: &World, entity: u32, tech_id: &str) -> Result<bool, String> {
    let node = get_tech_node(tech_id).ok_or_else(|| format!("Unknown tech '{}'", tech_id))?;

    // Already completed?
    if is_tech_completed(world, entity, tech_id) {
        return Err(format!("Tech '{}' already completed", tech_id));
    }

    // Already in queue?
    let queue = get_research_queue(world, entity);
    if queue.iter().any(|t| t == tech_id) {
        return Err(format!("Tech '{}' already in research queue", tech_id));
    }

    // Check prerequisites
    check_prerequisites(world, entity, node)?;

    Ok(true)
}

/// Adds a tech to the research queue if prerequisites are met and it's not
/// already completed or queued. Fires a `research_started` event on success.
pub fn research_tech(world: &mut World, entity: u32, tech_id: &str) -> Result<(), String> {
    let node = get_tech_node(tech_id).ok_or_else(|| format!("Unknown tech '{}'", tech_id))?;

    // Validate via can_research_tech
    can_research_tech(world, entity, tech_id)?;

    // Cycle detection: check that queuing this tech does not create a cycle
    let queue = get_research_queue(world, entity);
    if would_create_cycle(node, &queue) {
        return Err(format!(
            "Researching '{}' would create a dependency cycle",
            tech_id
        ));
    }

    // Read or create TechProgress component
    let progress = get_or_create_tech_progress(world, entity);

    let completed = progress
        .get("completed")
        .and_then(|c| c.as_object())
        .cloned()
        .unwrap_or_default();
    let mut queue: Vec<JsonValue> = progress
        .get("queue")
        .and_then(|q| q.as_array())
        .cloned()
        .unwrap_or_default();
    let mut queue_progress = progress
        .get("queue_progress")
        .and_then(|qp| qp.as_object())
        .cloned()
        .unwrap_or_default();
    let research_points = progress
        .get("research_points")
        .and_then(|rp| rp.as_f64())
        .unwrap_or(0.0);

    // Add to queue
    queue.push(json!(tech_id));
    queue_progress.insert(tech_id.to_string(), json!(0.0));

    let updated = json!({
        "completed": JsonValue::Object(completed),
        "queue": JsonValue::Array(queue),
        "queue_progress": JsonValue::Object(queue_progress),
        "research_points": research_points,
    });

    world.set_component(entity, "TechProgress", updated)?;

    // Fire research_started event
    world.send_event(
        "research_started",
        json!({
            "entity": entity,
            "tech_id": tech_id,
            "tech_name": node.name,
        }),
    )?;

    Ok(())
}

/// Removes a tech from the research queue. Fires a `research_cancelled` event.
pub fn cancel_research(world: &mut World, entity: u32, tech_id: &str) -> Result<(), String> {
    let progress = get_or_create_tech_progress(world, entity);

    let completed = progress
        .get("completed")
        .and_then(|c| c.as_object())
        .cloned()
        .unwrap_or_default();
    let mut queue: Vec<JsonValue> = progress
        .get("queue")
        .and_then(|q| q.as_array())
        .cloned()
        .unwrap_or_default();
    let mut queue_progress = progress
        .get("queue_progress")
        .and_then(|qp| qp.as_object())
        .cloned()
        .unwrap_or_default();
    let research_points = progress
        .get("research_points")
        .and_then(|rp| rp.as_f64())
        .unwrap_or(0.0);

    // Find and remove from queue
    let pos = queue.iter().position(|v| v.as_str() == Some(tech_id));
    match pos {
        Some(idx) => {
            queue.remove(idx);
            queue_progress.remove(tech_id);
        }
        None => {
            return Err(format!("Tech '{}' is not in the research queue", tech_id));
        }
    }

    let updated = json!({
        "completed": JsonValue::Object(completed),
        "queue": JsonValue::Array(queue),
        "queue_progress": JsonValue::Object(queue_progress),
        "research_points": research_points,
    });

    world.set_component(entity, "TechProgress", updated)?;

    // Fire research_cancelled event
    let node = get_tech_node(tech_id);
    world.send_event(
        "research_cancelled",
        json!({
            "entity": entity,
            "tech_id": tech_id,
            "tech_name": node.map(|n| n.name.as_str()).unwrap_or(tech_id),
        }),
    )?;

    Ok(())
}

/// Empties the research queue and clears queue progress.
pub fn clear_research_queue(world: &mut World, entity: u32) -> Result<(), String> {
    let progress = get_or_create_tech_progress(world, entity);

    let completed = progress
        .get("completed")
        .and_then(|c| c.as_object())
        .cloned()
        .unwrap_or_default();
    let research_points = progress
        .get("research_points")
        .and_then(|rp| rp.as_f64())
        .unwrap_or(0.0);

    let updated = json!({
        "completed": JsonValue::Object(completed),
        "queue": JsonValue::Array(vec![]),
        "queue_progress": json!({}),
        "research_points": research_points,
    });

    world.set_component(entity, "TechProgress", updated)?;

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Returns the TechProgress component value for an entity, creating a default
/// one with empty fields if it doesn't exist yet.
fn get_or_create_tech_progress(world: &World, entity: u32) -> JsonValue {
    world
        .get_component(entity, "TechProgress")
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "completed": {},
                "queue": [],
                "queue_progress": {},
                "research_points": 0.0,
            })
        })
}
