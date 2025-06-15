use engine_core::ecs::world::World;
use serde_json::json;

/// Creates a basic equipment setup for testing.
/// Returns (world, power_ring_id, eid) for further manipulation.
pub fn setup_basic_equipment(world: &mut World) -> (u32, u32) {
    // Create an item with effects: { "strength": 3 }
    let power_ring_id = world.spawn_entity();
    let power_ring = json!({
        "id": "power_ring",
        "name": "Power Ring",
        "slot": "finger",
        "effects": {
            "strength": 3
        }
    });
    world
        .set_component(power_ring_id, "Item", power_ring)
        .unwrap();

    // Create equipment with empty finger slot
    let eid = world.spawn_entity();
    let equipment = json!({
        "slots": {
            "finger": null
        }
    });
    world.set_component(eid, "Equipment", equipment).unwrap();

    (power_ring_id, eid)
}

/// Creates a base stats component with the given values.
pub fn set_base_stats(world: &mut World, eid: u32, strength: f64, dexterity: Option<f64>) {
    let mut base_stats = json!({
        "strength": strength
    });
    if let Some(dex) = dexterity {
        base_stats["dexterity"] = json!(dex);
    }
    world.set_component(eid, "BaseStats", base_stats).unwrap();
}
