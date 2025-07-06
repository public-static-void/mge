use crate::ecs::world::World;
use serde_json::Value as JsonValue;

/// Trait for pluggable job scheduling policies.
pub trait SchedulingPolicy: Send + Sync {
    /// Given a slice of (eid, priority, assignment_count, last_assigned_tick, created_at),
    /// sort the candidates in-place according to the policy.
    fn sort_candidates(&self, candidates: &mut Vec<(u32, i64, u64, u64, u64)>);

    /// Returns the name of the policy.
    fn name(&self) -> &'static str;
}

/// Priority-based scheduling (descending priority, then assignment count, then tick, then eid).
pub struct PriorityPolicy;
impl SchedulingPolicy for PriorityPolicy {
    fn sort_candidates(&self, candidates: &mut Vec<(u32, i64, u64, u64, u64)>) {
        candidates.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then(a.2.cmp(&b.2))
                .then(a.3.cmp(&b.3))
                .then(a.0.cmp(&b.0))
        });
    }
    fn name(&self) -> &'static str {
        "priority"
    }
}

/// FIFO: First-In, First-Out (oldest job first by created_at).
pub struct FifoPolicy;
impl SchedulingPolicy for FifoPolicy {
    fn sort_candidates(&self, candidates: &mut Vec<(u32, i64, u64, u64, u64)>) {
        candidates.sort_by(|a, b| a.4.cmp(&b.4).then(a.0.cmp(&b.0)));
    }
    fn name(&self) -> &'static str {
        "fifo"
    }
}

/// LIFO: Last-In, First-Out (newest job first by created_at).
pub struct LifoPolicy;
impl SchedulingPolicy for LifoPolicy {
    fn sort_candidates(&self, candidates: &mut Vec<(u32, i64, u64, u64, u64)>) {
        candidates.sort_by(|a, b| b.4.cmp(&a.4).then(b.0.cmp(&a.0)));
    }
    fn name(&self) -> &'static str {
        "lifo"
    }
}

pub struct JobBoard {
    pub jobs: Vec<u32>, // entity IDs of unassigned jobs, sorted by policy
    policy: Box<dyn SchedulingPolicy>,
}

/// Struct for job board metadata, suitable for scripting bridges.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct JobBoardEntry {
    pub eid: u32,
    pub priority: i64,
    pub state: String,
    // Add more fields as needed (assignment_count, created_at, etc.)
}

#[derive(Debug, PartialEq, Eq)]
pub enum JobAssignmentResult {
    Assigned(u32),
    NoJobsAvailable,
}

impl Default for JobBoard {
    fn default() -> Self {
        Self::with_policy(Box::new(PriorityPolicy))
    }
}

impl JobBoard {
    fn requirements_are_empty_or_zero(requirements: &[serde_json::Value]) -> bool {
        requirements.is_empty()
            || requirements
                .iter()
                .all(|req| req.get("amount").and_then(|a| a.as_i64()).unwrap_or(0) == 0)
    }

    /// Create a JobBoard with the given scheduling policy.
    pub fn with_policy(policy: Box<dyn SchedulingPolicy>) -> Self {
        JobBoard {
            jobs: Vec::new(),
            policy,
        }
    }

    pub fn update(&mut self, world: &World) {
        self.jobs.clear();
        let mut candidates: Vec<(u32, i64, u64, u64, u64)> = Vec::new();
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
                let created_at = job
                    .get("created_at")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(eid as u64);

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
                    candidates.push((
                        eid,
                        priority,
                        assignment_count,
                        last_assigned_tick,
                        created_at,
                    ));
                }
            }
        }
        self.policy.sort_candidates(&mut candidates);
        self.jobs = candidates
            .into_iter()
            .map(|(eid, _, _, _, _)| eid)
            .collect();
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

    /// Returns a vector of JobBoardEntry for all jobs currently on the board.
    pub fn jobs_with_metadata(&self, world: &World) -> Vec<JobBoardEntry> {
        self.jobs
            .iter()
            .filter_map(|&eid| {
                world.get_component(eid, "Job").map(|job| JobBoardEntry {
                    eid,
                    priority: job.get("priority").and_then(|v| v.as_i64()).unwrap_or(0),
                    state: job
                        .get("state")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
            })
            .collect()
    }

    /// Returns the current policy as a string.
    pub fn get_policy_name(&self) -> &str {
        self.policy.name()
    }

    /// Sets the scheduling policy by string name.
    pub fn set_policy(&mut self, policy: &str) -> Result<(), String> {
        self.policy = match policy {
            "priority" => Box::new(PriorityPolicy),
            "fifo" => Box::new(FifoPolicy),
            "lifo" => Box::new(LifoPolicy),
            _ => return Err(format!("Unknown policy: {policy}")),
        };
        Ok(())
    }

    /// Gets the priority for a job.
    pub fn get_priority(&self, world: &World, eid: u32) -> Option<i64> {
        world
            .get_component(eid, "Job")
            .and_then(|job| job.get("priority").and_then(|v| v.as_i64()))
    }

    /// Sets the priority for a job.
    pub fn set_priority(&self, world: &mut World, eid: u32, value: i64) -> Result<(), String> {
        if let Some(mut job) = world.get_component(eid, "Job").cloned() {
            job["priority"] = serde_json::json!(value);
            world
                .set_component(eid, "Job", job)
                .map_err(|e| e.to_string())
        } else {
            Err(format!("No job with eid {eid}"))
        }
    }
}
