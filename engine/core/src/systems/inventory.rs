use crate::ecs::world::World;
use serde_json::json;

pub struct InventoryConstraintSystem;

impl crate::ecs::system::System for InventoryConstraintSystem {
    fn name(&self) -> &'static str {
        "InventoryConstraintSystem"
    }

    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        for eid in world.get_entities_with_component("Inventory") {
            if let Some(inv) = world.get_component(eid, "Inventory").cloned() {
                let slots = inv
                    .get("slots")
                    .and_then(|v| v.as_array())
                    .map(|a| a.len())
                    .unwrap_or(0);
                let max_slots = inv
                    .get("max_slots")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(u64::MAX) as usize;
                let weight = inv.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let max_weight = inv
                    .get("max_weight")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(f64::INFINITY);
                let volume = inv.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let max_volume = inv
                    .get("max_volume")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(f64::INFINITY);

                let encumbered = slots > max_slots || weight > max_weight || volume > max_volume;

                // Set the encumbered status in the inventory component
                let mut new_inv = inv.clone();
                new_inv["encumbered"] = json!(encumbered);
                let _ = world.set_component(eid, "Inventory", new_inv);

                // Optionally emit an event for other systems (e.g., movement penalty)
                if encumbered {
                    let _ = world.send_event("inventory_encumbered", json!({ "entity": eid }));
                }
            }
        }
    }
}
