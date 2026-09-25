//! World-level vehicle operations backing the Lua/Python/WASM bridges.
//!
//! Errors are `Result`-style strings with no panics: `"no_vehicle"`,
//! `"no_rider"`, `"already_mounted"`, `"full"`, `"not_mounted"`, `"no_path"`.

use super::World;
use crate::map::cell_key::CellKey;
use crate::systems::vehicle::{cell_blocked, cell_to_step_json, occupants_of, vehicle_params};
use serde_json::json;

impl World {
    /// Embark `rider` onto `vehicle`.
    ///
    /// Records mount membership in the vehicle `occupants` array, clears the
    /// rider's `Agent.move_path` so it never steps independently while
    /// mounted, and emits `vehicle_embarked`. A full vehicle is rejected with
    /// no state change plus `vehicle_embark_rejected{reason:"full"}`. Riders
    /// already mounted elsewhere must disembark first (no transfers), and
    /// vehicle entities cannot ride (`"no_rider"`).
    pub fn embark(&mut self, vehicle: u32, rider: u32) -> Result<(), String> {
        let vehicle_json = self
            .get_component(vehicle, "Vehicle")
            .cloned()
            .ok_or_else(|| "no_vehicle".to_string())?;
        if !self.entity_exists(rider) || self.has_component(rider, "Vehicle") {
            return Err("no_rider".to_string());
        }
        if self.is_mounted(rider) {
            return Err("already_mounted".to_string());
        }
        let (capacity, _, _) = vehicle_params(&vehicle_json);
        let mut occupants = occupants_of(&vehicle_json);
        if occupants.len() as u64 >= capacity {
            let _ = self.send_event(
                "vehicle_embark_rejected",
                json!({"vehicle": vehicle, "rider": rider, "reason": "full"}),
            );
            return Err("full".to_string());
        }
        occupants.push(rider);
        let mut updated = vehicle_json;
        updated["occupants"] = json!(occupants);
        self.set_component(vehicle, "Vehicle", updated)
            .map_err(|e| e.to_string())?;
        if let Some(mut agent) = self.get_component(rider, "Agent").cloned() {
            if let Some(obj) = agent.as_object_mut() {
                obj.remove("move_path");
            }
            let _ = self.set_component(rider, "Agent", agent);
        }
        let _ = self.send_event(
            "vehicle_embarked",
            json!({"vehicle": vehicle, "rider": rider}),
        );
        Ok(())
    }

    /// Disembark `rider` from its vehicle.
    ///
    /// Removes the rider from `occupants`, sets the rider `Position` to the
    /// vehicle's current cell, and emits `vehicle_disembarked`.
    pub fn disembark(&mut self, rider: u32) -> Result<(), String> {
        let vehicle = self
            .vehicle_of(rider)
            .ok_or_else(|| "not_mounted".to_string())?;
        let vehicle_json = self
            .get_component(vehicle, "Vehicle")
            .cloned()
            .ok_or_else(|| "not_mounted".to_string())?;
        let occupants: Vec<u32> = occupants_of(&vehicle_json)
            .into_iter()
            .filter(|id| *id != rider)
            .collect();
        let mut updated = vehicle_json;
        updated["occupants"] = json!(occupants);
        self.set_component(vehicle, "Vehicle", updated)
            .map_err(|e| e.to_string())?;
        if let Some(position) = self.get_component(vehicle, "Position").cloned() {
            let _ = self.set_component(rider, "Position", position);
        }
        let _ = self.send_event(
            "vehicle_disembarked",
            json!({"vehicle": vehicle, "rider": rider}),
        );
        Ok(())
    }

    /// Compute a path with the existing A* and store the terrain-valid prefix
    /// in `Vehicle.move_path`, returning the stored step count.
    ///
    /// Steps are validated against the R007 terrain guard; the stored path is
    /// truncated at the first violating step and `vehicle_move_blocked` is
    /// emitted. A fully-blocked path stores an empty path. With no map,
    /// no vehicle position, or an unreachable goal the stored path is left
    /// unchanged and `"no_path"` is returned. The A* itself is untouched.
    pub fn assign_vehicle_path(&mut self, vehicle: u32, goal: &CellKey) -> Result<usize, String> {
        let vehicle_json = self
            .get_component(vehicle, "Vehicle")
            .cloned()
            .ok_or_else(|| "no_vehicle".to_string())?;
        let start = self
            .get_component(vehicle, "Position")
            .and_then(CellKey::from_position)
            .ok_or_else(|| "no_path".to_string())?;
        let path = self
            .find_path(&start, goal)
            .map(|result| result.path)
            .ok_or_else(|| "no_path".to_string())?;
        if path.len() <= 1 {
            let mut updated = vehicle_json;
            if let Some(obj) = updated.as_object_mut() {
                obj.remove("move_path");
            }
            self.set_component(vehicle, "Vehicle", updated)
                .map_err(|e| e.to_string())?;
            return Ok(0);
        }
        let (_, _, blocked_terrains) = vehicle_params(&vehicle_json);
        let mut stored = Vec::new();
        let mut updated = vehicle_json;
        for cell in path.iter().skip(1) {
            if cell_blocked(self, &blocked_terrains, cell) {
                if let Some(obj) = updated.as_object_mut() {
                    obj.remove("move_path");
                }
                if !stored.is_empty() {
                    updated["move_path"] = json!(stored);
                }
                self.set_component(vehicle, "Vehicle", updated)
                    .map_err(|e| e.to_string())?;
                let _ = self.send_event(
                    "vehicle_move_blocked",
                    json!({"vehicle": vehicle, "cell": cell_to_step_json(cell)}),
                );
                return Ok(stored.len());
            }
            stored.push(cell_to_step_json(cell));
        }
        updated["move_path"] = json!(stored);
        self.set_component(vehicle, "Vehicle", updated)
            .map_err(|e| e.to_string())?;
        Ok(stored.len())
    }

    /// Occupant entity IDs of `vehicle`, or empty when it has no `Vehicle`
    /// component.
    pub fn vehicle_occupants(&self, vehicle: u32) -> Vec<u32> {
        self.get_component(vehicle, "Vehicle")
            .map(occupants_of)
            .unwrap_or_default()
    }

    /// True when `rider` is listed in any live vehicle's `occupants`.
    pub fn is_mounted(&self, rider: u32) -> bool {
        self.vehicle_of(rider).is_some()
    }

    /// The vehicle carrying `rider`, or `None` when the rider is not mounted.
    fn vehicle_of(&self, rider: u32) -> Option<u32> {
        let mut ids = self.get_entities_with_component("Vehicle");
        ids.sort_unstable();
        ids.into_iter().find(|vid| {
            self.get_component(*vid, "Vehicle")
                .map(occupants_of)
                .is_some_and(|occ| occ.contains(&rider))
        })
    }
}
