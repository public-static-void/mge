use crate::ecs::world::World;
use serde_json::Value as JsonValue;

#[derive(Default)]
pub struct JobBoard {
    pub jobs: Vec<u32>, // entity IDs of unassigned jobs
}

#[derive(Debug, PartialEq, Eq)]
pub enum JobAssignmentResult {
    Assigned(u32),
    NoJobsAvailable,
}

impl JobBoard {
    pub fn update(&mut self, world: &World) {
        self.jobs.clear();
        for eid in world.get_entities_with_component("Job") {
            if let Some(job) = world.get_component(eid, "Job") {
                let assigned = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0);
                let status = job.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if assigned == 0 && status == "pending" {
                    self.jobs.push(eid);
                }
            }
        }
    }

    pub fn claim_job(&mut self, actor_eid: u32, world: &mut World) -> JobAssignmentResult {
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
                // Optionally: job["status"] = JsonValue::from("in_progress");
                world.set_component(job_eid, "Job", job).unwrap();
            }
            self.jobs.retain(|&eid| eid != job_eid);
            JobAssignmentResult::Assigned(job_eid)
        } else {
            JobAssignmentResult::NoJobsAvailable
        }
    }
}
