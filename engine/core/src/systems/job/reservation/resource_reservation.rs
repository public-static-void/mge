//! Resource reservation system for jobs in pending state.

use crate::ecs::system::System;
use crate::ecs::world::World;
use serde_json::Value as JsonValue;

/// Status of resource reservation for a job.
#[derive(Debug, PartialEq, Eq)]
pub enum ResourceReservationStatus {
    /// Resources reserved.
    Reserved,
    /// Waiting for resources to be reserved.
    WaitingForResources,
    /// No resources required.
    NotRequired,
    /// No resources found.
    NotFound,
}

/// Tracks which resources are reserved for which jobs.
/// This is a stateless system: reservation state is stored in the world (on jobs/stockpiles).
#[derive(Default)]
pub struct ResourceReservationSystem;

impl ResourceReservationSystem {
    /// Creates a new instance of the resource reservation system.
    pub fn new() -> Self {
        Self
    }

    /// Checks all pending jobs, attempts to reserve stockpile resources.
    /// Exclusivity is enforced so each resource instance is only reserved once.
    pub fn run_reservation(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        use std::collections::HashMap;

        // Collect all stockpiles and their available resources (working copy for simulation only).
        let stockpile_eids = world.get_entities_with_component("Stockpile");
        let mut stockpile_working: HashMap<u32, serde_json::Map<String, JsonValue>> =
            HashMap::new();
        for &stockpile_eid in &stockpile_eids {
            if let Some(stockpile) = world.get_component(stockpile_eid, "Stockpile")
                && let Some(res) = stockpile.get("resources").and_then(|v| v.as_object())
            {
                stockpile_working.insert(stockpile_eid, res.clone());
            }
        }

        // Get all pending jobs, in ascending eid order for deterministic processing.
        let mut job_eids = world.get_entities_with_component("Job");
        job_eids.sort();

        // Clear previous reservations for all pending jobs.
        for &job_eid in &job_eids {
            let job = match world.get_component(job_eid, "Job") {
                Some(j) => j.clone(),
                None => continue,
            };
            if job.get("state").and_then(|v| v.as_str()) != Some("pending") {
                continue;
            }
            let mut job = job.clone();
            let obj = job.as_object_mut().unwrap();
            obj.remove("reserved_resources");
            obj.remove("reserved_stockpile");
            world.set_component(job_eid, "Job", job).unwrap();
        }

        // Process jobs and allocate resources strictly in this order, so only what is left can be reserved.
        for &job_eid in &job_eids {
            let job = match world.get_component(job_eid, "Job") {
                Some(j) => j.clone(),
                None => continue,
            };

            if job.get("state").and_then(|v| v.as_str()) != Some("pending") {
                continue;
            }

            let requirements = match job.get("resource_requirements").and_then(|v| v.as_array()) {
                Some(reqs) if !reqs.is_empty() => reqs,
                _ => continue, // No requirements, nothing to reserve
            };

            for (&stockpile_eid, resources) in stockpile_working.iter_mut() {
                if Self::can_reserve(resources, requirements) {
                    // Subtract reserved resources from the working set so following jobs can't claim them (virtual only)
                    for req in requirements {
                        let kind = req.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                        let amount = req.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
                        let available = resources.get(kind).and_then(|v| v.as_i64()).unwrap_or(0);
                        resources.insert(kind.to_string(), JsonValue::from(available - amount));
                    }
                    let mut job = job.clone();
                    job["reserved_resources"] = JsonValue::from(requirements.clone());
                    job["reserved_stockpile"] = JsonValue::from(stockpile_eid);
                    // Initialize job state to fetching_resources to trigger resource fetch
                    job["state"] = JsonValue::from("fetching_resources");
                    world.set_component(job_eid, "Job", job).unwrap();
                    break;
                }
            }
            // If not reserved, do nothing: no marker fields, no status field
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
        if let Some(reserved) = job.get("reserved_resources").and_then(|v| v.as_array())
            && !reserved.is_empty()
            && job.get("reserved_stockpile").is_some()
        {
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

        // Just clear reservation marker fields.
        let mut job = job.clone();
        let obj = job.as_object_mut().unwrap();
        obj.remove("reserved_resources");
        obj.remove("reserved_stockpile");
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
}

impl System for ResourceReservationSystem {
    fn name(&self) -> &'static str {
        "ResourceReservationSystem"
    }

    fn run(&mut self, world: &mut World, lua: Option<&mlua::Lua>) {
        self.run_reservation(world, lua);
    }
}
