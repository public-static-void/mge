use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::{Value as JsonValue, json};

/// Status of resource reservation for a job.
#[derive(Debug, PartialEq, Eq)]
pub enum ResourceReservationStatus {
    Reserved,
    WaitingForResources,
    NotRequired,
    NotFound,
}

/// Tracks which resources are reserved for which jobs.
/// This is a stateless system: reservation state is stored in the world (on jobs/stockpiles).
#[derive(Default)]
pub struct ResourceReservationSystem;

impl ResourceReservationSystem {
    pub fn new() -> Self {
        Self
    }

    /// Checks if a job's resource requirements can be reserved from available stockpiles.
    /// If so, marks the resources as reserved for that job.
    pub fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        let job_eids = world.get_entities_with_component("Job");
        for &job_eid in &job_eids {
            let job = match world.get_component(job_eid, "Job") {
                Some(j) => j.clone(),
                None => continue,
            };

            if job.get("status").and_then(|v| v.as_str()) != Some("pending") {
                continue;
            }

            let requirements = match job.get("resource_requirements").and_then(|v| v.as_array()) {
                Some(reqs) if !reqs.is_empty() => reqs,
                _ => continue, // No requirements, nothing to reserve
            };

            // Find a stockpile with enough resources
            let stockpile_eids = world.get_entities_with_component("Stockpile");
            let mut found = false;
            for &stockpile_eid in &stockpile_eids {
                let mut stockpile = match world.get_component(stockpile_eid, "Stockpile") {
                    Some(s) => s.clone(),
                    None => continue,
                };
                let resources = match stockpile
                    .get_mut("resources")
                    .and_then(|v| v.as_object_mut())
                {
                    Some(r) => r,
                    None => continue,
                };

                if Self::can_reserve(resources, requirements) {
                    // Reserve: subtract from available, record in job
                    Self::reserve(resources, requirements);
                    world
                        .set_component(stockpile_eid, "Stockpile", stockpile)
                        .unwrap();

                    // Mark reservation on job
                    let mut job = job.clone();
                    job["reserved_resources"] = JsonValue::from(requirements.clone());
                    job["reserved_stockpile"] = JsonValue::from(stockpile_eid);
                    world.set_component(job_eid, "Job", job).unwrap();

                    found = true;
                    break;
                }
            }
            if !found {
                // Could not reserve resources; job remains pending or is marked as waiting
                let mut job = job.clone();
                job["reservation_status"] = JsonValue::from("waiting");
                world.set_component(job_eid, "Job", job).unwrap();
            }
        }
    }

    /// Checks reservation status for a job.
    pub fn check_reservation_status(
        &self,
        world: &World,
        job_eid: u32,
    ) -> ResourceReservationStatus {
        let job = match world.get_component(job_eid, "Job") {
            Some(j) => j.clone(),
            None => return ResourceReservationStatus::NotFound,
        };
        let requirements = job.get("resource_requirements").and_then(|v| v.as_array());
        if requirements.is_none() || requirements.unwrap().is_empty() {
            return ResourceReservationStatus::NotRequired;
        }
        if job.get("reserved_resources").is_some() && job.get("reserved_stockpile").is_some() {
            return ResourceReservationStatus::Reserved;
        }
        ResourceReservationStatus::WaitingForResources
    }

    /// Releases reserved resources for a job (on completion/cancellation).
    pub fn release_reservation(&self, world: &mut World, job_eid: u32) {
        let job = match world.get_component(job_eid, "Job") {
            Some(j) => j.clone(),
            None => return,
        };
        let reserved = match job.get("reserved_resources").and_then(|v| v.as_array()) {
            Some(r) => r,
            None => return,
        };
        let stockpile_eid = match job.get("reserved_stockpile").and_then(|v| v.as_u64()) {
            Some(eid) => eid as u32,
            None => return,
        };
        let mut stockpile = match world.get_component(stockpile_eid, "Stockpile") {
            Some(s) => s.clone(),
            None => return,
        };
        let resources = match stockpile
            .get_mut("resources")
            .and_then(|v| v.as_object_mut())
        {
            Some(r) => r,
            None => return,
        };
        // Release: add back the reserved amounts
        for req in reserved {
            let kind = req.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            let amount = req.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
            let entry = resources.entry(kind.to_string()).or_insert(json!(0));
            *entry = json!(entry.as_i64().unwrap_or(0) + amount);
        }
        world
            .set_component(stockpile_eid, "Stockpile", stockpile)
            .unwrap();

        // Remove reservation markers from job
        let mut job = job.clone();
        job.as_object_mut().unwrap().remove("reserved_resources");
        job.as_object_mut().unwrap().remove("reserved_stockpile");
        job.as_object_mut().unwrap().remove("reservation_status");
        world.set_component(job_eid, "Job", job).unwrap();
    }

    /// Internal: checks if all requirements can be reserved from the given resources.
    fn can_reserve(
        resources: &serde_json::Map<String, JsonValue>,
        requirements: &[JsonValue],
    ) -> bool {
        for req in requirements {
            let kind = req.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            let amount = req.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
            let available = resources.get(kind).and_then(|v| v.as_i64()).unwrap_or(0);
            if available < amount {
                return false;
            }
        }
        true
    }

    /// Internal: subtracts reserved amounts from resources.
    fn reserve(resources: &mut serde_json::Map<String, JsonValue>, requirements: &[JsonValue]) {
        for req in requirements {
            let kind = req.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            let amount = req.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
            let entry = resources.entry(kind.to_string()).or_insert(json!(0));
            *entry = json!(entry.as_i64().unwrap_or(0) - amount);
        }
    }
}

impl System for ResourceReservationSystem {
    fn name(&self) -> &'static str {
        "ResourceReservationSystem"
    }

    fn run(&mut self, world: &mut World, lua: Option<&mlua::Lua>) {
        self.run(world, lua);
    }
}
