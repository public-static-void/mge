use engine_core::ecs::world::World;

/// Test-only helpers for resource manipulation.
pub trait ResourceTestHelpers {
    /// Set the total amount of a resource kind across all stockpiles.
    fn set_global_resource(&mut self, kind: &str, amount: f64);
}

impl ResourceTestHelpers for World {
    fn set_global_resource(&mut self, kind: &str, amount: f64) {
        // Zero out all stockpiles
        if let Some(stockpiles) = self.components.get_mut("Stockpile") {
            for stockpile in stockpiles.values_mut() {
                if let Some(resources) = stockpile
                    .get_mut("resources")
                    .and_then(|v| v.as_object_mut())
                {
                    resources.insert(kind.to_string(), serde_json::json!(0.0));
                }
            }
            // Set the amount in the first stockpile found
            if let Some((_eid, stockpile)) = stockpiles.iter_mut().next()
                && let Some(resources) = stockpile
                    .get_mut("resources")
                    .and_then(|v| v.as_object_mut())
            {
                resources.insert(kind.to_string(), serde_json::json!(amount));
            }
        }
    }
}
