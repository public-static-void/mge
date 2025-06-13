use crate::ecs::world::World;
use serde_json::Value as JsonValue;

#[derive(Default)]
pub struct JobBoard {
    pub jobs: Vec<u32>, // entity IDs of unassigned jobs, sorted by priority
}

#[derive(Debug, PartialEq, Eq)]
pub enum JobAssignmentResult {
    Assigned(u32),
    NoJobsAvailable,
}

impl JobBoard {
    pub fn update(&mut self, world: &World) {
        self.jobs.clear();
        let mut candidates: Vec<(u32, i64, u64, u64)> = Vec::new();
        for eid in world.get_entities_with_component("Job") {
            if let Some(job) = world.get_component(eid, "Job") {
                let assigned = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0);
                let status = job.get("status").and_then(|v| v.as_str()).unwrap_or("");
                let priority = job.get("priority").and_then(|v| v.as_i64()).unwrap_or(0);
                let assignment_count = job
                    .get("assignment_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let last_assigned_tick = job
                    .get("last_assigned_tick")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if assigned == 0
                    && status == "pending"
                    && !job
                        .get("blocked")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                {
                    candidates.push((eid, priority, assignment_count, last_assigned_tick));
                }
            }
        }
        // Sort by: priority desc, least assigned, earliest last assigned, entity ID for stability
        candidates.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then(a.2.cmp(&b.2))
                .then(a.3.cmp(&b.3))
                .then(a.0.cmp(&b.0))
        });
        self.jobs = candidates.into_iter().map(|(eid, _, _, _)| eid).collect();
    }

    pub fn claim_job(
        &mut self,
        actor_eid: u32,
        world: &mut World,
        current_tick: u64,
    ) -> JobAssignmentResult {
        if let Some(&job_eid) = self.jobs.iter().find(|&&eid| {
            if let Some(job) = world.get_component(eid, "Job") {
                let assigned = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0);
                let status = job.get("status").and_then(|v| v.as_str()).unwrap_or("");
                assigned == 0 && status == "pending"
            } else {
                false
            }
        }) {
            if let Some(job) = world.get_component(job_eid, "Job") {
                let mut job = job.clone();
                job["assigned_to"] = JsonValue::from(actor_eid);
                job["assignment_count"] = JsonValue::from(
                    job.get("assignment_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                        + 1,
                );
                job["last_assigned_tick"] = JsonValue::from(current_tick);
                world.set_component(job_eid, "Job", job).unwrap();
            }
            self.jobs.retain(|&eid| eid != job_eid);
            JobAssignmentResult::Assigned(job_eid)
        } else {
            JobAssignmentResult::NoJobsAvailable
        }
    }
}
