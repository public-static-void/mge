use engine_core::ecs::world::World;
use serde_json::Value;

pub fn create_inventory(
    world: &mut World,
    slots: Vec<Value>,
    max_slots: usize,
    weight: f64,
    max_weight: f64,
    volume: f64,
    max_volume: f64,
) -> u32 {
    let eid = world.spawn_entity();
    let inventory = serde_json::json!({
        "slots": slots,
        "max_slots": max_slots,
        "weight": weight,
        "max_weight": max_weight,
        "volume": volume,
        "max_volume": max_volume
    });
    world.set_component(eid, "Inventory", inventory).unwrap();
    eid
}
