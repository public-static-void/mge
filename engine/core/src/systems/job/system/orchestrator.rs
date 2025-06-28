use crate::ecs::world::World;
use topo_sort::{SortResults, TopoSort};

pub fn cleanup_agent_on_job_state(world: &mut World, job: &serde_json::Value) {
    if let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64()) {
        let agent_id = agent_id as u32;
        if let Some(agent) = world
            .components
            .get_mut("Agent")
            .and_then(|m| m.get_mut(&agent_id))
        {
            if agent.get("current_job").and_then(|v| v.as_u64())
                == job.get("id").and_then(|v| v.as_u64())
            {
                agent.as_object_mut().unwrap().remove("current_job");
                agent["state"] = serde_json::json!("idle");
            }
        }
    }
}

pub fn should_spawn_conditional_child(
    world: &World,
    parent_job: &serde_json::Value,
    spawn_if: &serde_json::Value,
) -> bool {
    if let Some(field) = spawn_if.get("field").and_then(|v| v.as_str()) {
        if let Some(equals) = spawn_if.get("equals") {
            return parent_job.get(field) == Some(equals);
        }
    }
    if let Some(ws) = spawn_if.get("world_state") {
        return crate::systems::job::core::dependencies::evaluate_world_state(world, ws);
    }
    if let Some(es) = spawn_if.get("entity_state") {
        return crate::systems::job::core::dependencies::evaluate_entity_state(world, es);
    }
    false
}

pub fn run_job_system(world: &mut World, lua: Option<&mlua::Lua>) {
    use crate::systems::job::system::{effects, events, process};
    let job_entities: Vec<u32> = world.get_entities_with_component("Job");
    let mut sorter = TopoSort::new();

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

    let sorted_eids = match sorter.into_vec_nodes() {
        SortResults::Full(order) => order,
        SortResults::Partial(cycle) => {
            panic!("Cycle detected in job dependencies: {cycle:?}");
        }
    };

    for eid in sorted_eids.iter().copied() {
        let old_job = world.get_component(eid, "Job").unwrap().clone();
        let mut new_job = process::process_job(world, lua, eid, old_job.clone());
        let old_state = old_job
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let new_state = new_job.get("state").and_then(|v| v.as_str()).unwrap_or("");
        let job_type = new_job
            .get("job_type")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let cond_children =
            if old_state != new_state && matches!(new_state, "complete" | "failed" | "cancelled") {
                new_job.get("conditional_children").cloned()
            } else {
                None
            };

        if old_state != new_state {
            if matches!(new_state, "complete" | "failed" | "cancelled") {
                cleanup_agent_on_job_state(world, &new_job);
            }
            match new_state {
                "complete" => {
                    events::emit_job_event(world, "job_completed", &new_job, None);
                    if let Some(obj) = new_job.as_object_mut() {
                        effects::process_job_effects(world, eid, &job_type, obj, false);
                    }
                }
                "failed" => {
                    events::emit_job_event(world, "job_failed", &new_job, None);
                    if let Some(obj) = new_job.as_object_mut() {
                        effects::process_job_effects(world, eid, &job_type, obj, true);
                    }
                }
                "cancelled" => {
                    events::emit_job_event(world, "job_cancelled", &new_job, None);
                    if let Some(obj) = new_job.as_object_mut() {
                        effects::process_job_effects(world, eid, &job_type, obj, true);
                    }
                }
                _ => {}
            }
        }
        world.set_component(eid, "Job", new_job.clone()).unwrap();

        if let Some(cond_children) = cond_children.and_then(|v| v.as_array().cloned()) {
            for entry in cond_children {
                let spawn_if = entry
                    .get("spawn_if")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                let child_job = entry.get("job").expect("conditional child must have job");
                if should_spawn_conditional_child(world, &new_job, &spawn_if) {
                    let new_child_id = world.spawn_entity();
                    let mut new_child = child_job.clone();
                    new_child["id"] = serde_json::json!(new_child_id);
                    new_child["parent"] = serde_json::json!(eid);
                    world.set_component(new_child_id, "Job", new_child).unwrap();
                }
            }
        }
    }

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
        if let Some(mut job_obj) = world.get_component(eid, "Job").cloned() {
            if let Some(obj) = job_obj.as_object_mut() {
                match state.as_str() {
                    "failed" | "cancelled" => {
                        effects::process_job_effects(world, eid, &job_type, obj, true);
                    }
                    "complete" => {
                        effects::process_job_effects(world, eid, &job_type, obj, false);
                    }
                    _ => {}
                }
            }
        }
    }
}
