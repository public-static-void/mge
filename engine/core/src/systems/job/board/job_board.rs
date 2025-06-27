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
    fn requirements_are_empty_or_zero(requirements: &[serde_json::Value]) -> bool {
        requirements.is_empty()
            || requirements
                .iter()
                .all(|req| req.get("amount").and_then(|a| a.as_i64()).unwrap_or(0) == 0)
    }

    pub fn update(&mut self, world: &World) {
        self.jobs.clear();
        let mut candidates: Vec<(u32, i64, u64, u64)> = Vec::new();
        for eid in world.get_entities_with_component("Job") {
            if let Some(job) = world.get_component(eid, "Job") {
                let assigned = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0);
                let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                let priority = job
                    .get("effective_priority")
                    .and_then(|v| v.as_i64())
                    .or_else(|| job.get("priority").and_then(|v| v.as_i64()))
                    .unwrap_or(0);
                let assignment_count = job
                    .get("assignment_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let last_assigned_tick = job
                    .get("last_assigned_tick")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let resource_requirements = job.get("resource_requirements");
                let has_reservation = job.get("reserved_resources").is_some()
                    && job.get("reserved_stockpile").is_some();

                let requirements_satisfied = resource_requirements.is_none()
                    || resource_requirements
                        .and_then(|v| v.as_array())
                        .map(|arr| Self::requirements_are_empty_or_zero(arr))
                        .unwrap_or(true)
                    || has_reservation;

                if assigned == 0
                    && (state == "pending" || state == "interrupted")
                    && !job
                        .get("blocked")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    && requirements_satisfied
                {
                    candidates.push((eid, priority, assignment_count, last_assigned_tick));
                }
            }
        }
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
                let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                let resource_requirements = job.get("resource_requirements");
                let has_reservation = job.get("reserved_resources").is_some()
                    && job.get("reserved_stockpile").is_some();
                let requirements_satisfied = resource_requirements.is_none()
                    || resource_requirements
                        .and_then(|v| v.as_array())
                        .map(|arr| Self::requirements_are_empty_or_zero(arr))
                        .unwrap_or(true)
                    || has_reservation;
                assigned == 0
                    && (state == "pending" || state == "interrupted")
                    && requirements_satisfied
            } else {
                false
            }
        }) {
            if let Some(job) = world.get_component(job_eid, "Job") {
                let mut job = job.clone();
                job["assigned_to"] = JsonValue::from(actor_eid);
                // If the job was interrupted, reset to pending so it goes through the assignment pipeline
                if job.get("state").and_then(|v| v.as_str()) == Some("interrupted") {
                    job["state"] = JsonValue::from("pending");
                }
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
