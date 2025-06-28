use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::systems::job::{children, dependencies, states};
use topo_sort::{SortResults, TopoSort};

fn should_spawn_conditional_child(
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
        return crate::systems::job::dependencies::evaluate_world_state(world, ws);
    }
    if let Some(es) = spawn_if.get("entity_state") {
        return crate::systems::job::dependencies::evaluate_entity_state(world, es);
    }
    false
}

#[derive(Default)]
pub struct JobSystem;

impl JobSystem {
    pub fn new() -> Self {
        JobSystem
    }

    fn cleanup_agent_on_job_state(&self, world: &mut World, job: &serde_json::Value) {
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

    pub fn process_job_effects(
        world: &mut World,
        job_id: u32,
        job_type: &str,
        job: &mut serde_json::Map<String, serde_json::Value>,
        on_cancel: bool,
    ) {
        let empty = Vec::new();
        let effects: Vec<_> = world
            .job_types
            .get_data(job_type)
            .map(|jt| jt.effects.clone())
            .unwrap_or_else(|| empty.clone());

        let effect_registry = world.effect_processor_registry.as_ref().unwrap().clone();

        // Extract applied_effects BEFORE mutable borrow
        let mut applied_effect_indices = job
            .get("applied_effects")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_else(Vec::new);

        use crate::systems::job::dependencies::{evaluate_entity_state, evaluate_world_state};

        fn process_effect_and_chains(
            effect_registry: &std::sync::Arc<std::sync::Mutex<crate::systems::job::registry::effect_processor_registry::EffectProcessorRegistry>>,
            world: &mut World,
            job_id: u32,
            effect: &serde_json::Value,
        ) {
            effect_registry.lock().unwrap().process_effects(
                world,
                job_id,
                std::slice::from_ref(effect),
            );
            if let Some(chained) = effect.get("effects").and_then(|v| v.as_array()) {
                for chained_effect in chained {
                    process_effect_and_chains(effect_registry, world, job_id, chained_effect);
                }
            }
        }

        if on_cancel {
            for idx in &applied_effect_indices {
                if let Some(effect_idx) = idx.as_u64() {
                    if let Some(effect) = effects.get(effect_idx as usize) {
                        let effect_value = serde_json::to_value(effect.clone()).unwrap();
                        effect_registry.lock().unwrap().rollback_effects(
                            world,
                            job_id,
                            &[effect_value],
                        );
                    }
                }
            }
            applied_effect_indices.clear();
        } else {
            // --- Incremental: Apply only one effect per tick ---
            let mut next_effect_idx = None;
            for (idx, effect) in effects.iter().enumerate() {
                let already_applied = applied_effect_indices
                    .iter()
                    .any(|v| v.as_u64() == Some(idx as u64));
                if already_applied {
                    continue;
                }
                let should_apply = match effect.get("condition") {
                    None => true,
                    Some(cond) => {
                        if let Some(ws) = cond.get("world_state") {
                            evaluate_world_state(world, ws)
                        } else if let Some(es) = cond.get("entity_state") {
                            evaluate_entity_state(world, es)
                        } else {
                            false
                        }
                    }
                };
                if !should_apply {
                    continue;
                }
                next_effect_idx = Some(idx);
                break;
            }
            if let Some(idx) = next_effect_idx {
                let effect = &effects[idx];
                let effect_value = serde_json::to_value(effect.clone()).unwrap();
                process_effect_and_chains(&effect_registry, world, job_id, &effect_value);
                applied_effect_indices.push(serde_json::Value::from(idx as u64));
            }
        }

        job.insert(
            "applied_effects".to_string(),
            serde_json::Value::Array(applied_effect_indices),
        );
    }

    pub fn emit_job_event(
        world: &mut crate::ecs::world::World,
        event: &str,
        job: &serde_json::Value,
        extra: Option<&serde_json::Map<String, serde_json::Value>>,
    ) {
        let mut payload = serde_json::Map::new();
        if let Some(id) = job.get("id") {
            payload.insert("entity".to_string(), id.clone());
        }
        if let Some(job_type) = job.get("job_type") {
            payload.insert("job_type".to_string(), job_type.clone());
        }
        if let Some(state) = job.get("state") {
            payload.insert("state".to_string(), state.clone());
        }
        if let Some(progress) = job.get("progress") {
            payload.insert("progress".to_string(), progress.clone());
        }
        if let Some(assigned_to) = job.get("assigned_to") {
            payload.insert("assigned_to".to_string(), assigned_to.clone());
        }
        if let Some(extra) = extra {
            for (k, v) in extra {
                payload.insert(k.clone(), v.clone());
            }
        }
        world
            .send_event(event, serde_json::Value::Object(payload))
            .ok();
    }

    pub fn process_job(
        &self,
        world: &mut World,
        _lua: Option<&mlua::Lua>,
        eid: u32,
        mut job: serde_json::Value,
    ) -> serde_json::Value {
        let job_type = job
            .get("job_type")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let is_cancelled = job
            .get("cancelled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let cancelled_cleanup_done = job
            .get("cancelled_cleanup_done")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // If job is assigned to an agent, but agent does not exist, interrupt and rollback
        if let Some(agent_id) = job.get("assigned_to").and_then(|v| v.as_u64()) {
            let agent_id = agent_id as u32;
            if world.get_component(agent_id, "Agent").is_none() {
                // Mark as interrupted, rollback all applied effects
                job["state"] = serde_json::json!("interrupted");
                let job_id = job.get("id").and_then(|v| v.as_u64()).unwrap_or(eid as u64) as u32;
                if let Some(obj) = job.as_object_mut() {
                    Self::process_job_effects(world, job_id, &job_type, obj, true);
                }
                // Optionally: cleanup agent assignment fields
                job.as_object_mut().unwrap().remove("assigned_to");
                // Save and return immediately
                world.set_component(eid, "Job", job.clone()).unwrap();
                return job;
            }
        }

        if let Some(dep_fail_state) = dependencies::dependency_failure_state(world, &job) {
            job["state"] = serde_json::json!(dep_fail_state);

            if dep_fail_state == "failed" {
                if let Some(to_spawn) = job
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
            }
            return job;
        }

        if !dependencies::dependencies_satisfied(world, &job) {
            if job.get("state").and_then(|v| v.as_str()) != Some("pending") {
                job["state"] = serde_json::json!("pending");
            }
            return job;
        }

        if is_cancelled && !cancelled_cleanup_done {
            job["state"] = serde_json::json!("cancelled");
            crate::systems::job::states::handle_job_cancellation_cleanup(world, &job);
            job["cancelled_cleanup_done"] = serde_json::json!(true);
            world.set_component(eid, "Job", job.clone()).unwrap();
        }

        if let Some(children_val) = job.get_mut("children") {
            let (new_children, all_children_complete) =
                children::process_job_children(self, world, _lua, eid, children_val, is_cancelled);
            *children_val = new_children;
            if all_children_complete {
                job["state"] = serde_json::json!("complete");
            }
        }

        if job
            .get("should_fail")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            job["state"] = serde_json::json!("failed");
            return job;
        }

        let job = match job.get("state").and_then(|v| v.as_str()) {
            Some("pending") | None => states::handle_pending_state(world, eid, job),
            Some("going_to_site") => states::handle_going_to_site_state(world, eid, job),
            Some("fetching_resources") => states::handle_fetching_resources_state(world, eid, job),
            Some("delivering_resources") => {
                states::handle_delivering_resources_state(world, eid, job)
            }
            Some("at_site") => states::handle_at_site_state(world, eid, job),
            _ => job,
        };
        world.set_component(eid, "Job", job.clone()).unwrap();
        self.process_job_progress(world, eid, job_type, job)
    }

    fn process_job_progress(
        &self,
        world: &mut World,
        eid: u32,
        job_type: String,
        mut job: serde_json::Value,
    ) -> serde_json::Value {
        let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");

        // Custom handler always takes priority
        if !matches!(state, "failed" | "complete" | "cancelled") {
            let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let job_id = job.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

            let registry = world.job_handler_registry.lock().unwrap();
            if let Some(handler) = registry.get(&job_type) {
                let prev_progress = job.get("progress").and_then(|v| v.as_f64());
                let result = handler(world, assigned_to, job_id, &job);
                let new_progress = result.get("progress").and_then(|v| v.as_f64());
                drop(registry);
                if new_progress != prev_progress {
                    Self::emit_job_event(world, "job_progressed", &result, None);
                }
                return result;
            }
        }

        match state {
            "pending" => states::handle_pending_state(world, eid, job),
            "going_to_site" => states::handle_going_to_site_state(world, eid, job),
            "fetching_resources" => states::handle_fetching_resources_state(world, eid, job),
            "delivering_resources" => states::handle_delivering_resources_state(world, eid, job),
            "at_site" => states::handle_at_site_state(world, eid, job),
            "in_progress" => {
                // Extract job_type_str before mut borrow
                let job_type_str = job
                    .get("job_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default")
                    .to_string();
                let job_id = job.get("id").and_then(|v| v.as_u64()).unwrap_or(eid as u64) as u32;
                if let Some(obj) = job.as_object_mut() {
                    JobSystem::process_job_effects(world, job_id, &job_type_str, obj, false);
                }

                let assigned_to =
                    job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

                // Only progress if agent is at the site (if applicable)
                let mut at_site = true;
                if assigned_to != 0 {
                    if let Some(target_pos) = job.get("target_position") {
                        if let Some(agent_pos) = world.get_component(assigned_to, "Position") {
                            let agent_cell = crate::map::CellKey::from_position(agent_pos);
                            let target_cell = crate::map::CellKey::from_position(target_pos);
                            if let (Some(agent_cell), Some(target_cell)) = (agent_cell, target_cell)
                            {
                                if agent_cell != target_cell {
                                    at_site = false;
                                }
                            }
                        }
                    }
                }
                if !at_site {
                    return job;
                }

                // Progress the job
                let mut progress_increment = 1.0;
                if assigned_to != 0 {
                    if let Some(agent) = world.get_component(assigned_to, "Agent") {
                        let skills = agent.get("skills").and_then(|v| v.as_object());
                        let skill = skills
                            .and_then(|map| map.get(&job_type))
                            .and_then(|v| v.as_f64())
                            .unwrap_or(1.0);
                        let stamina = agent
                            .get("stamina")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(100.0);
                        progress_increment = 1.0 * skill * (stamina / 100.0);
                        if progress_increment < 0.1 {
                            progress_increment = 0.1;
                        }
                    }
                }
                let prev_progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let progress = prev_progress + progress_increment;
                job["progress"] = serde_json::json!(progress);
                if progress != prev_progress {
                    Self::emit_job_event(world, "job_progressed", &job, None);
                }
                if progress >= 3.0 {
                    job["progress"] = serde_json::json!(progress.max(3.0));
                    job["state"] = serde_json::json!("complete");
                }
                job
            }
            // Terminal and unknown states: just return the job as-is.
            _ => job,
        }
    }
}

impl System for JobSystem {
    fn name(&self) -> &'static str {
        "JobSystem"
    }

    fn run(&mut self, world: &mut World, lua: Option<&mlua::Lua>) {
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
            let mut new_job = self.process_job(world, lua, eid, old_job.clone());
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

            let cond_children = if old_state != new_state
                && matches!(new_state, "complete" | "failed" | "cancelled")
            {
                new_job.get("conditional_children").cloned()
            } else {
                None
            };

            if old_state != new_state {
                if matches!(new_state, "complete" | "failed" | "cancelled") {
                    self.cleanup_agent_on_job_state(world, &new_job);
                }
                match new_state {
                    "complete" => {
                        Self::emit_job_event(world, "job_completed", &new_job, None);
                        if let Some(obj) = new_job.as_object_mut() {
                            Self::process_job_effects(world, eid, &job_type, obj, false);
                        }
                    }
                    "failed" => {
                        Self::emit_job_event(world, "job_failed", &new_job, None);
                        if let Some(obj) = new_job.as_object_mut() {
                            Self::process_job_effects(world, eid, &job_type, obj, true);
                        }
                    }
                    "cancelled" => {
                        Self::emit_job_event(world, "job_cancelled", &new_job, None);
                        if let Some(obj) = new_job.as_object_mut() {
                            Self::process_job_effects(world, eid, &job_type, obj, true);
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
                            JobSystem::process_job_effects(world, eid, &job_type, obj, true);
                        }
                        "complete" => {
                            JobSystem::process_job_effects(world, eid, &job_type, obj, false);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
