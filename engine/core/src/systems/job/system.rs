use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::systems::job::{children, dependencies, phases};
use topo_sort::{SortResults, TopoSort};

#[derive(Default)]
pub struct JobSystem;

impl JobSystem {
    pub fn new() -> Self {
        JobSystem
    }

    fn cleanup_agent_on_job_status(&self, world: &mut World, job: &serde_json::Value) {
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

    fn process_job_effects(
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

        let applied_effects = job
            .entry("applied_effects")
            .or_insert_with(|| serde_json::Value::Array(vec![]))
            .as_array_mut()
            .unwrap();

        if on_cancel {
            for idx in applied_effects.iter().filter_map(|v| v.as_u64()) {
                if let Some(effect) = effects.get(idx as usize) {
                    effect_registry.lock().unwrap().rollback_effects(
                        world,
                        job_id,
                        &[effect.clone()],
                    );
                }
            }
            applied_effects.clear();
        } else {
            for (idx, effect) in effects.iter().enumerate() {
                if !applied_effects
                    .iter()
                    .any(|v| v.as_u64() == Some(idx as u64))
                {
                    effect_registry.lock().unwrap().process_effects(
                        world,
                        job_id,
                        &[effect.clone()],
                    );
                    applied_effects.push(serde_json::Value::from(idx as u64));
                }
            }
        }
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
        if let Some(status) = job.get("status") {
            payload.insert("status".to_string(), status.clone());
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

        // --- dependency failure ---
        if let Some(dep_fail_status) = dependencies::dependency_failure_status(world, &job) {
            job["status"] = serde_json::json!(dep_fail_status);

            if dep_fail_status == "failed" {
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
        if !dependencies::dependencies_complete(world, &job) {
            if job.get("status").and_then(|v| v.as_str()) != Some("pending") {
                job["status"] = serde_json::json!("pending");
            }
            return job;
        }

        // --- cancelled ---
        if is_cancelled {
            job["status"] = serde_json::json!("cancelled");
        }

        // --- children jobs ---
        if let Some(children_val) = job.get_mut("children") {
            let (new_children, all_children_complete) =
                children::process_job_children(self, world, _lua, eid, children_val, is_cancelled);
            *children_val = new_children;
            if all_children_complete {
                job["status"] = serde_json::json!("complete");
            }
        }

        // --- should_fail ---
        if job
            .get("should_fail")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            job["status"] = serde_json::json!("failed");
            return job;
        }

        // --- always set to in_progress if pending ---
        if job.get("status").and_then(|v| v.as_str()) == Some("pending") {
            job["status"] = serde_json::json!("in_progress");
        }

        // --- phase transitions ---
        let job = match job.get("phase").and_then(|v| v.as_str()) {
            Some("pending") | None => phases::handle_pending_phase(world, eid, job),
            Some("going_to_site") => phases::handle_going_to_site_phase(world, eid, job),
            Some("fetching_resources") => phases::handle_fetching_resources_phase(world, eid, job),
            Some("delivering_resources") => {
                phases::handle_delivering_resources_phase(world, eid, job)
            }
            _ => job,
        };
        // Always run handler/progress logic after phase handling
        self.process_job_progress(world, eid, job_type, job)
    }

    fn process_job_progress(
        &self,
        world: &mut World,
        _eid: u32,
        job_type: String,
        mut job: serde_json::Value,
    ) -> serde_json::Value {
        let current_status = job
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !matches!(current_status.as_str(), "failed" | "complete" | "cancelled") {
            let assigned_to = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let job_id = job.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

            {
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
        }

        if !matches!(current_status.as_str(), "failed" | "complete" | "cancelled") {
            if current_status == "in_progress" {
                let assigned_to =
                    job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
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
                    job["status"] = serde_json::json!("complete");
                }
            }
            job
        } else {
            if current_status == "complete"
                && job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0) < 3.0
            {
                job["progress"] = serde_json::json!(3.0);
            }
            job
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
            let deps: Vec<u32> = job
                .get("dependencies")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str()?.parse().ok())
                        .collect()
                })
                .unwrap_or_default();
            sorter.insert(eid, deps);
        }

        let sorted_eids = match sorter.into_vec_nodes() {
            SortResults::Full(order) => order,
            SortResults::Partial(cycle) => {
                panic!("Cycle detected in job dependencies: {:?}", cycle);
            }
        };

        for eid in sorted_eids {
            let old_job = world.get_component(eid, "Job").unwrap().clone();
            let mut new_job = self.process_job(world, lua, eid, old_job.clone());
            let old_status = old_job
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let new_status = new_job.get("status").and_then(|v| v.as_str()).unwrap_or("");
            let job_type = new_job
                .get("job_type")
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string();

            if old_status != new_status {
                if matches!(new_status, "complete" | "failed" | "cancelled") {
                    self.cleanup_agent_on_job_status(world, &new_job);
                }
                match new_status {
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
            world.set_component(eid, "Job", new_job).unwrap();
        }
    }
}
