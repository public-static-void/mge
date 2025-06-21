//! Resource property lookup for economic/resource system.
//!
//! Provides helpers to look up unit weight and volume for a resource kind
//! from the global resource definitions in the World.

/// Look up unit weight and volume for a resource kind from the global resource definitions.
/// Falls back to 1.0 if not found.
///
/// # Arguments
/// * `world` - The ECS world.
/// * `kind` - The resource kind (e.g., "wood").
///
/// # Returns
/// (unit_weight, unit_volume) as (f64, f64)
pub fn get_resource_unit_properties(world: &crate::ecs::world::World, kind: &str) -> (f64, f64) {
    if let Some(res_def) = world.resource_definitions.get(kind) {
        let weight = res_def
            .get("unit_weight")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let volume = res_def
            .get("unit_volume")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        return (weight, volume);
    }
    (1.0, 1.0)
}
