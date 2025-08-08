//! Job system orchestrator: topological job execution, conditional job spawning, agent cleanup.

use crate::ecs::world::World;
use crate::systems::job::core::dependencies;
use crate::systems::job::reservation::resource_reservation::ResourceReservationSystem;
use topo_sort::{SortResults, TopoSort};

/// Cleans up the agent's assignment and state when a job reaches a terminal state.
/// This always writes the agent back to ECS after any mutation.
pub fn cleanup_agent_on_job_state(world: &mut World, job: &serde_json::Value) {
    if let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64()) {
        let agent_id = agent_id as u32;
        let update_agent = world.get_component(agent_id, "Agent").cloned();
        if let Some(mut agent) = update_agent
            && agent.get("current_job").and_then(|v| v.as_u64())
                == job.get("id").and_then(|v| v.as_u64())
        {
            agent["current_job"] = serde_json::Value::Null;
            agent["state"] = serde_json::json!("idle");
            world.set_component(agent_id, "Agent", agent).unwrap();
        }
    }
    // Also clear assigned_to on the job itself
    if let Some(job_id) = job.get("id").and_then(|v| v.as_u64()) {
        let job_id = job_id as u32;
        if let Some(mut job_obj) = world.get_component(job_id, "Job").cloned() {
            job_obj["assigned_to"] = serde_json::Value::Null;
            world.set_component(job_id, "Job", job_obj).unwrap();
        }
    }
}

/// Returns true if the job should spawn a conditional child.
/// Supports field/equals, world_state, and entity_state conditions.
pub fn should_spawn_conditional_child(
    world: &World,
    parent_job: &serde_json::Value,
    spawn_if: &serde_json::Value,
) -> bool {
    if let Some(field) = spawn_if.get("field").and_then(|v| v.as_str())
        && let Some(equals) = spawn_if.get("equals")
    {
        return parent_job.get(field) == Some(equals);
    }
    if let Some(ws) = spawn_if.get("world_state") {
        return dependencies::evaluate_world_state(world, ws);
    }
    if let Some(es) = spawn_if.get("entity_state") {
        return dependencies::evaluate_entity_state(world, es);
    }
    false
}

/// Runs the job processing system for a single tick.
/// Processes each job (including children and assigned jobs) exactly once in topological order.
/// Spawns each conditional child at most once per parent job, robustly tracked in ECS.
/// Does not guarantee all jobs reach terminal state in a single call; call per tick for simulation.
pub fn run_job_system(world: &mut World, lua: Option<&mlua::Lua>) {
    use crate::systems::job::system::{effects, events, process};

    let job_entities: Vec<u32> = world.get_entities_with_component("Job");
    let mut sorter = TopoSort::new();

    // Create a ResourceReservationSystem instance for releasing reservations
    let reservation_system = ResourceReservationSystem::new();

    // Build dependency graph for topological sort
    for &eid in &job_entities {
        let job = world.get_component(eid, "Job").unwrap();
        fn collect_job_deps(dep: &serde_json::Value, out: &mut Vec<u32>) {
            if dep.is_string() {
                if let Ok(eid) = dep.as_str().unwrap().parse::<u32>() {
                    out.push(eid);
                }
            } else if dep.is_array() {
                for d in dep.as_array().unwrap() {
                    collect_job_deps(d, out);
                }
            } else if dep.is_object() {
                let obj = dep.as_object().unwrap();
                for key in &["all_of", "any_of", "not"] {
                    if let Some(arr) = obj.get(*key) {
                        for d in arr.as_array().unwrap() {
                            collect_job_deps(d, out);
                        }
                    }
                }
            }
        }

        let mut deps = Vec::new();
        if let Some(dep_val) = job.get("dependencies") {
            collect_job_deps(dep_val, &mut deps);
        }
        sorter.insert(eid, deps);
    }

    // Topologically sort jobs to respect dependencies
    let sorted_eids = match sorter.into_vec_nodes() {
        SortResults::Full(order) => order,
        SortResults::Partial(cycle) => {
            panic!("Cycle detected in job dependencies: {cycle:?}");
        }
    };

    for eid in sorted_eids.iter().copied() {
        let job = world.get_component(eid, "Job").unwrap().clone();
        let old_state = job
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        process::process_job(world, lua, eid, job.clone());
        // Fetch the latest job from ECS after processing
        let mut new_job = world.get_component(eid, "Job").unwrap().clone();
        let new_state = new_job
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let job_type = new_job
            .get("job_type")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let cond_children = new_job.get("conditional_children").cloned();

        if old_state != new_state {
            // Clean up agent state if job is now terminal or blocked.
            if matches!(
                new_state.as_str(),
                "complete" | "failed" | "cancelled" | "blocked"
            ) {
                cleanup_agent_on_job_state(world, &new_job);

                // Release resource reservation on terminal jobs
                reservation_system.release_reservation(world, eid);

                // Re-fetch the job after cleanup, as cleanup may have modified it in ECS
                if let Some(updated_job) = world.get_component(eid, "Job") {
                    new_job = updated_job.clone();
                }
            }

            // Emit job events and process job effects for terminal transitions.
            match new_state.as_str() {
                "complete" => {
                    events::emit_job_event(world, "job_completed", &new_job, None);
                    if let Some(mut obj) = new_job.as_object().cloned() {
                        effects::process_job_effects(world, eid, &job_type, &mut obj, false);
                        world
                            .set_component(eid, "Job", serde_json::Value::Object(obj))
                            .unwrap();
                    } else {
                        world.set_component(eid, "Job", new_job.clone()).unwrap();
                    }
                }
                "failed" => {
                    events::emit_job_event(world, "job_failed", &new_job, None);
                    if let Some(mut obj) = new_job.as_object().cloned() {
                        effects::process_job_effects(world, eid, &job_type, &mut obj, true);
                        world
                            .set_component(eid, "Job", serde_json::Value::Object(obj))
                            .unwrap();
                    } else {
                        world.set_component(eid, "Job", new_job.clone()).unwrap();
                    }
                }
                "cancelled" => {
                    events::emit_job_event(world, "job_cancelled", &new_job, None);
                    if let Some(mut obj) = new_job.as_object().cloned() {
                        effects::process_job_effects(world, eid, &job_type, &mut obj, true);
                        world
                            .set_component(eid, "Job", serde_json::Value::Object(obj))
                            .unwrap();
                    } else {
                        world.set_component(eid, "Job", new_job.clone()).unwrap();
                    }
                }
                _ => {
                    world.set_component(eid, "Job", new_job.clone()).unwrap();
                }
            }
        } else {
            world.set_component(eid, "Job", new_job.clone()).unwrap();
        }

        // Robust ECS-tracked conditional child spawn: only once per job_type/category per parent.
        if let Some(cond_children) = cond_children.and_then(|v| v.as_array().cloned()) {
            for entry in cond_children {
                let mut parent_job_obj = world.get_component(eid, "Job").unwrap().clone();

                let mut spawned_conditional_children = parent_job_obj
                    .get("spawned_conditional_children")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_else(Vec::new);

                let spawn_if = entry
                    .get("spawn_if")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                let child_job = entry.get("job").expect("conditional child must have job");
                let should_spawn =
                    should_spawn_conditional_child(world, &parent_job_obj, &spawn_if);

                let child_job_type = child_job
                    .get("job_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let child_category = child_job
                    .get("category")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let child_key = serde_json::json!({
                    "job_type": child_job_type,
                    "category": child_category
                });

                let already_spawned = spawned_conditional_children.iter().any(|entry| {
                    entry.get("job_type") == Some(&serde_json::json!(child_job_type))
                        && entry.get("category") == Some(&serde_json::json!(child_category))
                });

                let ecs_already_spawned =
                    world.get_entities_with_component("Job").iter().any(|&jid| {
                        if let Some(j) = world.get_component(jid, "Job") {
                            j.get("parent").and_then(|v| v.as_u64()) == Some(eid as u64)
                                && j.get("job_type") == child_job.get("job_type")
                                && j.get("category") == child_job.get("category")
                        } else {
                            false
                        }
                    });

                let effective_already_spawned = already_spawned || ecs_already_spawned;

                if should_spawn && !effective_already_spawned {
                    let new_child_id = world.spawn_entity();
                    let mut new_child = child_job.clone();
                    new_child["id"] = serde_json::json!(new_child_id);
                    new_child["parent"] = serde_json::json!(eid);

                    world
                        .set_component(new_child_id, "Job", new_child.clone())
                        .unwrap();

                    spawned_conditional_children.push(child_key);

                    parent_job_obj["spawned_conditional_children"] =
                        serde_json::Value::Array(spawned_conditional_children);
                    world
                        .set_component(eid, "Job", parent_job_obj.clone())
                        .unwrap();
                }
            }
        }
    }

    let job_entities: Vec<u32> = world.get_entities_with_component("Job");
    let mut terminal_jobs = Vec::new();
    for &eid in &job_entities {
        if let Some(job) = world.get_component(eid, "Job") {
            let job_type = job
                .get("job_type")
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string();
            let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
            terminal_jobs.push((eid, job_type, state.to_string()));
        }
    }
    for (eid, job_type, state) in terminal_jobs {
        if let Some(mut job_obj) = world.get_component(eid, "Job").cloned()
            && let Some(obj) = job_obj.as_object_mut()
        {
            match state.as_str() {
                "failed" | "cancelled" => {
                    effects::process_job_effects(world, eid, &job_type, obj, true);
                    world
                        .set_component(eid, "Job", serde_json::Value::Object(obj.clone()))
                        .unwrap();
                }
                "complete" => {
                    effects::process_job_effects(world, eid, &job_type, obj, false);
                    world
                        .set_component(eid, "Job", serde_json::Value::Object(obj.clone()))
                        .unwrap();
                }
                _ => {}
            }
        }
    }
}
