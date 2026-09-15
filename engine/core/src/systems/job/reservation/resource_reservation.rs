//! Resource reservation system for jobs in pending state.

use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::systems::job::reservation::resource_reservation_ops::ResourceReservationOps;
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
    pub fn run_reservation<W: ResourceReservationOps>(&mut self, world: &mut W) {
        use std::collections::HashMap;

        // Collect all stockpiles and their available resources (working copy for simulation only).
        let stockpile_eids = world.get_entities_with_component("Stockpile");
        let mut stockpile_working: HashMap<u32, serde_json::Map<String, JsonValue>> =
            HashMap::new();
        for &stockpile_eid in &stockpile_eids {
            if let Some(stockpile) = world.get_component_value(stockpile_eid, "Stockpile")
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
            let job = match world.get_component_value(job_eid, "Job") {
                Some(j) => j,
                None => continue,
            };
            if job.get("state").and_then(|v| v.as_str()) != Some("pending") {
                continue;
            }
            let mut job = job;
            if let Some(obj) = job.as_object_mut() {
                obj.remove("reserved_resources");
                obj.remove("reserved_stockpile");
            }
            let _ = world.set_component_value(job_eid, "Job", job);
        }

        // Process jobs and allocate resources strictly in this order, so only what is left can be reserved.
        // Stockpiles are visited in ascending entity-id order for determinism.
        let mut stockpile_order: Vec<u32> = stockpile_working.keys().copied().collect();
        stockpile_order.sort_unstable();
        for &job_eid in &job_eids {
            let job = match world.get_component_value(job_eid, "Job") {
                Some(j) => j,
                None => continue,
            };

            if job.get("state").and_then(|v| v.as_str()) != Some("pending") {
                continue;
            }

            let requirements = match job.get("resource_requirements").and_then(|v| v.as_array()) {
                Some(reqs) if !reqs.is_empty() => reqs.clone(),
                _ => continue,
            };

            for &stockpile_eid in &stockpile_order {
                let resources = match stockpile_working.get_mut(&stockpile_eid) {
                    Some(r) => r,
                    None => continue,
                };
                if Self::can_reserve(resources, &requirements) {
                    for req in &requirements {
                        let kind = req.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                        let amount = req.get("amount").map(Self::amount_as_i64).unwrap_or(0);
                        let available = resources.get(kind).map(Self::amount_as_i64).unwrap_or(0);
                        resources.insert(kind.to_string(), JsonValue::from(available - amount));
                    }
                    let mut job = job;
                    job["reserved_resources"] = JsonValue::from(requirements);
                    job["reserved_stockpile"] = JsonValue::from(stockpile_eid);
                    job["state"] = JsonValue::from("fetching_resources");
                    let _ = world.set_component_value(job_eid, "Job", job);
                    break;
                }
            }
        }
    }

    /// Checks reservation status for a job.
    pub fn check_reservation_status<W: ResourceReservationOps>(
        &self,
        world: &W,
        job_eid: u32,
    ) -> ResourceReservationStatus {
        let job = match world.get_component_value(job_eid, "Job") {
            Some(j) => j,
            None => return ResourceReservationStatus::NotFound,
        };
        let requirements = job.get("resource_requirements").and_then(|v| v.as_array());
        if requirements.is_none_or(|r| r.is_empty()) {
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
    pub fn release_reservation<W: ResourceReservationOps>(&self, world: &mut W, job_eid: u32) {
        let job = match world.get_component_value(job_eid, "Job") {
            Some(j) => j,
            None => return,
        };

        // Just clear reservation marker fields.
        let mut job = job;
        if let Some(obj) = job.as_object_mut() {
            obj.remove("reserved_resources");
            obj.remove("reserved_stockpile");
        }
        let _ = world.set_component_value(job_eid, "Job", job);
    }

    /// Internal: checks if all requirements can be reserved from the given resources.
    /// Integer-only: amounts are non-negative integers end-to-end, so reserved
    /// and consumed totals cannot drift through float truncation.
    fn can_reserve(
        resources: &serde_json::Map<String, JsonValue>,
        requirements: &[JsonValue],
    ) -> bool {
        for req in requirements {
            let kind = req.get("kind").and_then(|v| v.as_str()).unwrap_or("");
            let amount = req.get("amount").map(Self::amount_as_i64).unwrap_or(0);
            let available = resources.get(kind).map(Self::amount_as_i64).unwrap_or(0);
            if available < amount {
                return false;
            }
        }
        true
    }

    /// Reads a JSON amount as a non-negative integer with no float conversion.
    /// Non-integer values (floats, strings, missing) read as zero, and
    /// negatives clamp to zero, so reservation math stays integer-only.
    fn amount_as_i64(value: &JsonValue) -> i64 {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|u| i64::try_from(u).ok()))
            .unwrap_or(0)
            .max(0)
    }
}

impl System for ResourceReservationSystem {
    fn name(&self) -> &'static str {
        "ResourceReservationSystem"
    }

    fn run(&mut self, world: &mut World) {
        self.run_reservation(world);
    }
}
