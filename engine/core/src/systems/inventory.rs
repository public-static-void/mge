use crate::ecs::world::World;
use serde_json::{Value as JsonValue, json};

fn aggregate_inventory(inv: &JsonValue) -> (f64, f64, usize) {
    let mut total_weight = inv.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let mut total_volume = inv.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let mut total_items = 0;

    if let Some(slots) = inv.get("slots").and_then(|v| v.as_array()) {
        for slot in slots {
            if slot.is_object() {
                // Nested inventory (container)
                let (w, v, n) = aggregate_inventory(slot);
                total_weight += w;
                total_volume += v;
                total_items += n;
            } else {
                // Assume slot is an item ID string
                total_items += 1;
            }
        }
    }
    (total_weight, total_volume, total_items)
}

/// System that checks for constrains concerning the inventor e.g. if an inventory is encumbered
pub struct InventoryConstraintSystem;

impl crate::ecs::system::System for InventoryConstraintSystem {
    fn name(&self) -> &'static str {
        "InventoryConstraintSystem"
    }

    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        for eid in world.get_entities_with_component("Inventory") {
            if let Some(inv) = world.get_component(eid, "Inventory").cloned() {
                let (weight, volume, slots) = aggregate_inventory(&inv);
                let max_slots = inv
                    .get("max_slots")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(u64::MAX) as usize;
                let max_weight = inv
                    .get("max_weight")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(f64::INFINITY);
                let max_volume = inv
                    .get("max_volume")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(f64::INFINITY);

                let encumbered = slots > max_slots || weight > max_weight || volume > max_volume;

                let mut new_inv = inv.clone();
                new_inv["encumbered"] = json!(encumbered);
                new_inv["weight"] = json!(weight);
                new_inv["volume"] = json!(volume);
                let _ = world.set_component(eid, "Inventory", new_inv);

                if encumbered {
                    let _ = world.send_event("inventory_encumbered", json!({ "entity": eid }));
                }
            }
        }
    }
}
