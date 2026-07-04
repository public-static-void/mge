//! Core job processing logic for the job system.

use crate::ecs::world::World;
use rand::Rng;
use serde_json::{Map, Value as JsonValue};
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

/// Loaded skill registry data: skill name → (max_level, base_xp, stat_bonuses)
type SkillRegistryMap = HashMap<String, SkillEntry>;

struct SkillEntry {
    max_level: f64,
    base_xp_per_action: f64,
    derived_stat_bonus: HashMap<String, f64>,
}

/// Loads the skill registry from skill_registry.json on first access.
fn get_skill_registry() -> &'static SkillRegistryMap {
    static REGISTRY: OnceLock<SkillRegistryMap> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = SkillRegistryMap::new();
        let paths = [
            "engine/assets/schemas/skill_registry.json",
            "../engine/assets/schemas/skill_registry.json",
        ];
        for path_str in &paths {
            let path = Path::new(path_str);
            if path.exists() {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        if let Ok(json) = serde_json::from_str::<JsonValue>(&content)
                            && let Some(skills) = json.get("skills").and_then(|v| v.as_array())
                        {
                            for skill in skills {
                                let name = skill
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if name.is_empty() {
                                    continue;
                                }
                                let max_level = skill
                                    .get("max_level")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(100.0);
                                let base_xp = skill
                                    .get("base_xp_per_action")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(10.0);
                                let mut stat_bonus = HashMap::new();
                                if let Some(bonus_obj) =
                                    skill.get("derived_stat_bonus").and_then(|v| v.as_object())
                                {
                                    for (k, v) in bonus_obj {
                                        if let Some(val) = v.as_f64() {
                                            stat_bonus.insert(k.clone(), val);
                                        }
                                    }
                                }
                                registry.insert(
                                    name,
                                    SkillEntry {
                                        max_level,
                                        base_xp_per_action: base_xp,
                                        derived_stat_bonus: stat_bonus,
                                    },
                                );
                            }
                        }
                    }
                    Err(_) => continue,
                }
                break;
            }
        }
        registry
    })
}

/// Grants XP to an agent on job completion and handles level-up.
/// Returns the updated SkillLevels component if changes were made.
fn grant_xp_on_job_completion(
    world: &mut World,
    agent_id: u32,
    skill_name: &str,
) -> Option<JsonValue> {
    let registry = get_skill_registry();
    let entry = registry.get(skill_name);

    // If skill registry is empty or skill not found, use defaults
    let base_xp = entry.map(|e| e.base_xp_per_action).unwrap_or(10.0);
    let max_level = entry.map(|e| e.max_level).unwrap_or(100.0);

    // xp_gained = max(1, floor(base_xp * (1.0 + random_0_to_1)))
    let random_factor: f64 = rand::rng().random::<f64>();
    let xp_gained = (base_xp * (1.0 + random_factor)).floor().max(1.0);

    // Get or create SkillLevels component
    let mut skill_levels = world
        .get_component(agent_id, "SkillLevels")
        .cloned()
        .unwrap_or_else(|| {
            JsonValue::Object(Map::from_iter([
                ("skills".to_string(), JsonValue::Object(Map::new())),
                ("total_xp".to_string(), JsonValue::from(0.0)),
                ("skill_xp".to_string(), JsonValue::Object(Map::new())),
                ("skill_levels".to_string(), JsonValue::Object(Map::new())),
            ]))
        });

    // Update total_xp
    let current_total = skill_levels
        .get("total_xp")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    skill_levels["total_xp"] = JsonValue::from(current_total + xp_gained);

    // Update skill_xp
    let mut skill_xp = skill_levels
        .get("skill_xp")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let current_skill_xp = skill_xp
        .get(skill_name)
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    skill_xp.insert(
        skill_name.to_string(),
        JsonValue::from(current_skill_xp + xp_gained),
    );
    skill_levels["skill_xp"] = JsonValue::Object(skill_xp.clone());

    // Update skill (current level value) - read existing level, default to 1.0 if not set
    let mut skills = skill_levels
        .get("skills")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let current_skill = skills
        .get(skill_name)
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);
    skills.insert(skill_name.to_string(), JsonValue::from(current_skill));
    skill_levels["skills"] = JsonValue::Object(skills.clone());

    // Check for level-up
    let mut skill_levels_map = skill_levels
        .get("skill_levels")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let current_level = skill_levels_map
        .get(skill_name)
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    // Compute thresholds: xp_to_next_level = base_xp * (1.5 ^ current_level)
    let xp_to_next = base_xp * (1.5_f64).powf(current_level);
    let new_skill_xp = skill_xp
        .get(skill_name)
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let mut leveled_up = false;
    let mut new_level = current_level;

    while new_skill_xp >= xp_to_next && new_level < max_level {
        // Carry over excess XP
        let excess = new_skill_xp - xp_to_next;
        skill_xp.insert(skill_name.to_string(), JsonValue::from(excess));
        skill_levels["skill_xp"] = JsonValue::Object(skill_xp.clone());

        new_level += 1.0;
        let old_level = (new_level - 1.0) as u32;

        // Update skill_levels
        skill_levels_map.insert(skill_name.to_string(), JsonValue::from(new_level));
        skill_levels["skill_levels"] = JsonValue::Object(skill_levels_map.clone());

        // Update skills value to match level
        skills.insert(skill_name.to_string(), JsonValue::from(new_level));
        skill_levels["skills"] = JsonValue::Object(skills.clone());

        leveled_up = true;

        // Emit LeveledUpEvent
        let event_payload = serde_json::json!({
            "entity_id": agent_id,
            "skill_name": skill_name,
            "old_level": old_level,
            "new_level": new_level as u32,
        });
        world.emit_event("leveled_up", event_payload);

        // P016: Apply derived_stat_bonus from skill_registry
        if let Some(entry) = entry {
            for (stat_name, per_level_bonus) in &entry.derived_stat_bonus {
                // Apply bonus to BaseStats
                let base_stats = world
                    .get_component(agent_id, "BaseStats")
                    .cloned()
                    .unwrap_or_else(|| JsonValue::Object(Map::new()));
                let mut stats = base_stats;
                let current_val = stats.get(stat_name).and_then(|v| v.as_f64()).unwrap_or(0.0);
                stats[stat_name] = JsonValue::from(current_val + per_level_bonus);
                let _ = world.set_component(agent_id, "BaseStats", stats);
            }
        }
    }

    if leveled_up {
        // Write updated skill_levels
        skill_levels["skill_levels"] = JsonValue::Object(skill_levels_map);
    }

    let _ = world.set_component(agent_id, "SkillLevels", skill_levels.clone());
    Some(skill_levels)
}

/// Handles job progress and invokes custom or default logic as appropriate.
pub fn process_job_progress(
    world: &mut World,
    eid: u32,
    job_type: String,
    job: serde_json::Value,
) -> serde_json::Value {
    let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");

    if !matches!(state, "failed" | "complete" | "cancelled" | "interrupted") {
        let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let job_id = job.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        let handler_opt = {
            let registry = world.job_handler_registry.lock().unwrap();
            registry.get(&job_type).cloned()
        };
        if let Some(handler) = handler_opt {
            let prev_progress = job.get("progress").and_then(|v| v.as_f64());
            let result = handler(world, assigned_to, job_id, &job);
            let new_progress = result.get("progress").and_then(|v| v.as_f64());
            if new_progress != prev_progress {
                crate::systems::job::system::events::emit_job_event(
                    world,
                    "job_progressed",
                    &result,
                    None,
                );
            }
            let _ = world.set_component(eid, "Job", result.clone());

            // Handler is fully responsible for progress of this job_type and state.
            return result;
        }
    }

    match state {
        "pending" => crate::systems::job::states::handle_pending_state(world, eid, job),
        "going_to_site" => crate::systems::job::states::handle_going_to_site_state(world, eid, job),
        "fetching_resources" => {
            crate::systems::job::states::handle_fetching_resources_state(world, eid, job)
        }
        "delivering_resources" => {
            crate::systems::job::states::handle_delivering_resources_state(world, eid, job)
        }
        "at_site" => crate::systems::job::states::handle_at_site_state(world, eid, job),
        "in_progress" => default_job_progress(world, eid, job_type, job),
        _ => job,
    }
}

/// The default job progress logic for jobs in "in_progress" state.
///
/// This function ensures jobs make progress whether or not an agent is assigned,
/// so that effect-only jobs and agent-driven jobs both work properly.
/// When a job completes, XP is granted to the assigned agent (R012).
fn default_job_progress(
    world: &mut World,
    _eid: u32,
    job_type: String,
    mut job: JsonValue,
) -> JsonValue {
    let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

    let mut at_site = true;
    if assigned_to != 0
        && let Some(target_pos) = job.get("target_position")
        && let Some(agent_pos) = world.get_component(assigned_to, "Position")
    {
        let agent_cell = crate::map::CellKey::from_position(agent_pos);
        let target_cell = crate::map::CellKey::from_position(target_pos);
        if let (Some(agent_cell), Some(target_cell)) = (agent_cell, target_cell)
            && agent_cell != target_cell
        {
            at_site = false;
        }
    }
    if !at_site {
        return job;
    }

    // Always increment progress, even for jobs without an agent.
    let mut progress_increment = 1.0;
    if assigned_to != 0
        && let Some(agent) = world.get_component(assigned_to, "Agent")
    {
        // Try SkillLevels first (R014), fall back to deprecated agent.skills
        let skill_value =
            if let Some(skill_levels) = world.get_component(assigned_to, "SkillLevels") {
                skill_levels
                    .get("skills")
                    .and_then(|v| v.as_object())
                    .and_then(|map| map.get(&job_type))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0)
            } else {
                let skills = agent.get("skills").and_then(|v| v.as_object());
                skills
                    .and_then(|map| map.get(&job_type))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1.0)
            };
        let stamina = agent
            .get("stamina")
            .and_then(|v| v.as_f64())
            .unwrap_or(100.0);
        progress_increment = 1.0 * skill_value * (stamina / 100.0);
        if progress_increment < 0.1 {
            progress_increment = 0.1;
        }
    }
    let prev_progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let progress = prev_progress + progress_increment;
    job["progress"] = serde_json::json!(progress);
    if progress != prev_progress {
        crate::systems::job::system::events::emit_job_event(world, "job_progressed", &job, None);
    }
    let required_progress = job
        .get("required_progress")
        .and_then(|v| v.as_f64())
        .unwrap_or(3.0);
    if progress >= required_progress {
        job["progress"] = serde_json::json!(progress.max(required_progress));
        job["state"] = serde_json::json!("complete");

        // P014: Grant XP on job completion
        if assigned_to != 0 {
            grant_xp_on_job_completion(world, assigned_to, &job_type);
        }
    }
    job
}

/// Processes a single job, updating its state and handling dependencies, cancellation, and children.
pub fn process_job(world: &mut World, eid: u32, mut job: serde_json::Value) -> serde_json::Value {
    let job_type = job
        .get("job_type")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();
    let cancelled_cleanup_done = job
        .get("cancelled_cleanup_done")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // If job is in a terminal state, never allow it to transition out.
    {
        let state = job
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if matches!(
            state.as_str(),
            "cancelled" | "complete" | "failed" | "interrupted" | "blocked"
        ) {
            if state == "cancelled" && !cancelled_cleanup_done {
                if let Some(children_val) = job.get_mut("children") {
                    let (new_children, _all_children_complete) =
                        crate::systems::job::children::process_job_children(
                            world,
                            eid,
                            children_val,
                            true,
                        );
                    job["children"] = new_children;
                }
                crate::systems::job::states::handle_job_cancellation_cleanup(world, &job);
                crate::systems::job::system::orchestrator::cleanup_agent_on_job_state(world, &job);
                let _ = world.set_component(eid, "Job", job.clone());
                crate::systems::job::system::events::emit_job_event(
                    world,
                    "job_cancelled",
                    &job,
                    None,
                );
            }
            if state == "complete"
                && let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64())
                && let Some(mut agent) = world.get_component(agent_id as u32, "Agent").cloned()
            {
                agent["current_job"] = serde_json::Value::Null;
                agent["state"] = serde_json::json!("idle");
                let _ = world.set_component(agent_id as u32, "Agent", agent);
            }
            return job;
        }
    }

    // If agent is missing, mark job as interrupted and clean up assignment.
    if let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64()) {
        let agent_id = agent_id as u32;
        if world.get_component(agent_id, "Agent").is_none() {
            job["state"] = serde_json::json!("interrupted");
            let job_id = job.get("id").and_then(|v| v.as_u64()).unwrap_or(eid as u64) as u32;
            if let Some(obj) = job.as_object_mut() {
                crate::systems::job::system::effects::process_job_effects(
                    world, job_id, &job_type, obj, true,
                );
            }
            job["assigned_to"] = serde_json::Value::Null;
            let _ = world.set_component(eid, "Job", job.clone());
            return job;
        }
    }

    // Dependency failure handling.
    if let Some(dep_fail_state) =
        crate::systems::job::dependencies::dependency_failure_state(world, &job)
    {
        job["state"] = serde_json::json!(dep_fail_state);

        if dep_fail_state == "failed"
            && let Some(to_spawn) = job
                .get("on_dependency_failed_spawn")
                .and_then(|v| v.as_array())
        {
            let mut children = job
                .get("children")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for child in to_spawn {
                children.push(child.clone());
            }
            job["children"] = serde_json::Value::Array(children);
        }
        let _ = world.set_component(eid, "Job", job.clone());
        return job;
    }

    // If dependencies are not satisfied, set state to pending.
    if !crate::systems::job::dependencies::dependencies_satisfied(world, &job) {
        if job.get("state").and_then(|v| v.as_str()) != Some("pending") {
            job["state"] = serde_json::json!("pending");
        }
        let _ = world.set_component(eid, "Job", job.clone());
        return job;
    }

    // Promote job state from "pending" to "in_progress" if:
    // - the job is unassigned, OR
    // - the job does NOT have a target position
    if job.get("state").and_then(|v| v.as_str()) == Some("pending") {
        let has_target_position = job.get("target_position").is_some();
        let assigned_to_some = job.get("assigned_to").and_then(|v| v.as_u64()).is_some();

        if assigned_to_some && has_target_position {
            // Defer state advancement, let handle_pending_state decide
        } else {
            // For unassigned or no-target jobs, move to in_progress immediately
            job["state"] = serde_json::json!("in_progress");
        }
    }

    let state_is_cancelled = {
        let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
        state == "cancelled"
    };

    // Update children array if present.
    if let Some(children_val) = job.get_mut("children") {
        let (new_children, all_children_complete) =
            crate::systems::job::children::process_job_children(
                world,
                eid,
                children_val,
                state_is_cancelled,
            );
        job["children"] = new_children;
        if all_children_complete {
            job["state"] = serde_json::json!("complete");
        }
        let _ = world.set_component(eid, "Job", job.clone());
    } else {
        let _ = world.set_component(eid, "Job", job.clone());
    }

    // Fail job if should_fail flag is set.
    if job
        .get("should_fail")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        job["state"] = serde_json::json!("failed");
        let _ = world.set_component(eid, "Job", job.clone());
        return job;
    }

    // Allow effect-only jobs (with no assigned_to) to stay in "in_progress" state
    if job.get("state").and_then(|v| v.as_str()) == Some("in_progress") {
        let has_assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).is_some();
        let has_effects = world
            .job_types
            .get_data(job.get("job_type").and_then(|v| v.as_str()).unwrap_or(""))
            .map(|jt| !jt.effects.is_empty())
            .unwrap_or(false);

        if !has_assigned_to && !has_effects {
            job["state"] = serde_json::json!("pending");
            let _ = world.set_component(eid, "Job", job.clone());
            return job;
        }
    }

    // Track previous state to detect transition to "complete"
    let prev_state = job
        .get("state")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // State machine for job progress.
    let mut job = match job.get("state").and_then(|v| v.as_str()) {
        Some("pending") | None => {
            crate::systems::job::states::handle_pending_state(world, eid, job)
        }
        Some("going_to_site") => {
            crate::systems::job::states::handle_going_to_site_state(world, eid, job)
        }
        Some("fetching_resources") => {
            crate::systems::job::states::handle_fetching_resources_state(world, eid, job)
        }
        Some("delivering_resources") => {
            crate::systems::job::states::handle_delivering_resources_state(world, eid, job)
        }
        Some("at_site") => crate::systems::job::states::handle_at_site_state(world, eid, job),
        _ => job,
    };
    let _ = world.set_component(eid, "Job", job.clone());

    // If job just transitioned to "complete", cleanup agent assignment and unassign job
    let current_state = job.get("state").and_then(|v| v.as_str());
    if current_state == Some("complete") && prev_state.as_deref() != Some("complete") {
        if let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64())
            && let Some(mut agent) = world.get_component(agent_id as u32, "Agent").cloned()
        {
            agent["current_job"] = serde_json::Value::Null;
            agent["state"] = serde_json::json!("idle");
            let _ = world.set_component(agent_id as u32, "Agent", agent);
        }
        job["assigned_to"] = serde_json::Value::Null;
        let _ = world.set_component(eid, "Job", job.clone());
    }

    let job = process_job_progress(world, eid, job_type, job);
    let _ = world.set_component(eid, "Job", job.clone());

    job
}
