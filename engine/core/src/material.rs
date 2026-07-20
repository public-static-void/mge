//! Material property lookup and entity material management.
//!
//! Provides helpers to look up material definitions, attach Material
//! components to entities, and query registered material names.

use crate::ecs::world::World;
use serde_json::Value;

/// Default material properties returned for unknown material names.
pub fn default_material() -> Value {
    serde_json::json!({
        "name": "default",
        "density": 1.0,
        "hardness": 1.0,
        "flammability": 0.0,
        "thermal_conductivity": 0.0,
        "melting_point": 9999.0
    })
}

/// Look up material properties by name from the world's material definitions.
///
/// Returns the definition `Value`, or `default_material()` if not found.
pub fn get_material_properties(world: &World, name: &str) -> Value {
    world
        .material_definitions
        .get(name)
        .cloned()
        .unwrap_or_else(default_material)
}

/// Attach a Material component to an entity.
///
/// Returns an error if the material name is not in the registry.
pub fn set_entity_material(
    world: &mut World,
    entity_id: u32,
    material_name: &str,
) -> Result<(), String> {
    if !world.material_definitions.contains_key(material_name) {
        return Err(format!(
            "Unknown material '{material_name}'. Available: {:?}",
            world.material_definitions.keys().collect::<Vec<_>>()
        ));
    }
    let component = serde_json::json!({
        "material": material_name
    });
    world.set_component(entity_id, "Material", component)
}

/// Get the Material component value for an entity, or `None` if absent.
pub fn get_entity_material(world: &World, entity_id: u32) -> Option<Value> {
    world.get_component(entity_id, "Material").cloned()
}

/// Return all registered material definition names.
pub fn get_material_names(world: &World) -> Vec<String> {
    world.material_definitions.keys().cloned().collect()
}
