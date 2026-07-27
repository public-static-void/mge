//! Designer loadout APIs: apply equipment sets and validate equipment.
//!
//! `apply_loadout` creates Item entities from registry definitions and equips them
//! atomically — rolling back on partial failure.
//! `validate_equipment` checks entity equipment against slot rules and registry state.

use super::World;
use crate::ecs::item::ItemRegistry;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// An equipment validation issue returned by `validate_equipment`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentIssue {
    pub slot: String,
    pub item_id: String,
    /// One of: "unknown_slot", "item_not_registered", "slot_mismatch", "unmet_requirements"
    pub reason: String,
}

/// Warning from template-level equipment validation.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields read by downstream consumers (Inspector, Scribe, Lua API)
pub struct TemplateWarning {
    pub component: String,
    pub slot: String,
    pub item_id: String,
    pub message: String,
}

/// Validate that all item IDs referenced in a template's Equipment component
/// exist in the item registry. Returns warnings for missing items — never errors.
pub fn validate_template_equipment(
    template: &crate::ecs::template::UnitTemplate,
    registry: &ItemRegistry,
) -> Vec<TemplateWarning> {
    let mut warnings = Vec::new();

    let equipment = match template.components.get("Equipment") {
        Some(e) => e,
        None => return warnings,
    };

    let slots = match equipment.get("slots").and_then(|v| v.as_object()) {
        Some(s) => s,
        None => return warnings,
    };

    for (slot_name, item_id_value) in slots {
        let item_id = match item_id_value.as_str() {
            Some(id) => id,
            None => continue,
        };

        if registry.get_item(item_id).is_none() {
            warnings.push(TemplateWarning {
                component: "Equipment".to_string(),
                slot: slot_name.clone(),
                item_id: item_id.to_string(),
                message: format!("Item '{item_id}' not found in registry"),
            });
        }
    }

    warnings
}

/// Load valid slot names from equipment_slots.json.
fn load_valid_slots() -> std::collections::HashSet<String> {
    let mut candidates = vec![
        "engine/assets/schemas/equipment_slots.json".to_string(),
        "../engine/assets/schemas/equipment_slots.json".to_string(),
    ];
    // Also try relative to CARGO_MANIFEST_DIR (for unit tests)
    if let Ok(dir) = std::env::var("CARGO_MANIFEST_DIR") {
        candidates.push(
            std::path::PathBuf::from(dir)
                .join("../../engine/assets/schemas/equipment_slots.json")
                .to_string_lossy()
                .into_owned(),
        );
    }

    for path_str in &candidates {
        let path = std::path::Path::new(path_str);
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(path)
            && let Ok(json) = serde_json::from_str::<JsonValue>(&content)
            && let Some(slot_list) = json.get("slots").and_then(|v| v.as_array())
        {
            return slot_list
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }
    }
    std::collections::HashSet::new()
}

impl World {
    /// Apply a named equipment set to an entity.
    ///
    /// For each slot→item_id in the set:
    /// 1. Looks up the item in the ItemRegistry
    /// 2. Spawns an Item entity with the item definition as its Item component
    /// 3. Adds the item_id to the entity's Inventory
    /// 4. Sets the Equipment slot
    ///
    /// Returns the entity ID on success.
    /// On partial failure, rolls back: despawns created items, removes them from
    /// inventory, and unequips any already-set slots. NFR002: best-effort atomicity —
    /// items 1..N-1 are rolled back when item N fails; items already equipped before
    /// `apply_loadout` are left as-is.
    pub fn apply_loadout(&mut self, entity: u32, set_name: &str) -> Result<u32, String> {
        // 1. Look up set
        let set = self
            .equipment_set_registry
            .get_set(set_name)
            .cloned()
            .ok_or_else(|| format!("Equipment set '{}' not found", set_name))?;

        // 2. Check entity has Inventory
        if self.get_component(entity, "Inventory").is_none() {
            return Err("Entity has no Inventory component".to_string());
        }

        // 3. Process each slot
        let mut equipped_slots: Vec<String> = Vec::new();
        let mut spawned_items: Vec<u32> = Vec::new();

        for (slot, item_id) in &set.items {
            // 3a. Get item definition
            let definition = self
                .item_registry
                .get_item(item_id)
                .cloned()
                .ok_or_else(|| {
                    // Rollback: despawn items, unequip slots
                    self.rollback_loadout(&spawned_items, &equipped_slots, entity);
                    format!("Item '{}' not found in registry", item_id)
                })?;

            // 3b. Spawn Item entity
            let item_eid = self.spawn_entity();
            if let Err(e) = self.set_component(item_eid, "Item", definition) {
                self.despawn_entity(item_eid);
                self.rollback_loadout(&spawned_items, &equipped_slots, entity);
                return Err(e);
            }
            spawned_items.push(item_eid);

            // 3c. Add item_id to entity's Inventory slots array
            if let Some(inv) = self.get_component(entity, "Inventory").cloned() {
                let mut new_inv = inv;
                if let Some(slots) = new_inv.get_mut("slots").and_then(|v| v.as_array_mut()) {
                    slots.push(JsonValue::String(item_id.clone()));
                }
                if let Err(e) = self.set_component(entity, "Inventory", new_inv) {
                    self.rollback_loadout(&spawned_items, &equipped_slots, entity);
                    return Err(e);
                }
            }

            // 3d. Set Equipment slot
            let mut equipment = self
                .get_component(entity, "Equipment")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({"slots": {}}));

            if let Some(slots_obj) = equipment.get_mut("slots").and_then(|v| v.as_object_mut()) {
                slots_obj.insert(slot.clone(), JsonValue::String(item_id.clone()));
            }

            if let Err(e) = self.set_component(entity, "Equipment", equipment) {
                self.rollback_loadout(&spawned_items, &equipped_slots, entity);
                return Err(e);
            }
            equipped_slots.push(slot.clone());
        }

        Ok(entity)
    }

    /// Rollback a partial loadout: despawn created items and unequip slots.
    fn rollback_loadout(&mut self, spawned_items: &[u32], equipped_slots: &[String], entity: u32) {
        // Despawn created item entities
        for &item_eid in spawned_items {
            self.despawn_entity(item_eid);
        }

        // Remove items from inventory and unequip slots
        if let Some(mut inv) = self.get_component(entity, "Inventory").cloned() {
            if let Some(slots) = inv.get_mut("slots").and_then(|v| v.as_array_mut()) {
                // Remove the item_ids we added (last N entries)
                let remove_count = equipped_slots.len();
                let new_len = slots.len().saturating_sub(remove_count);
                slots.truncate(new_len);
            }
            let _ = self.set_component(entity, "Inventory", inv);
        }

        // Unset the equipped slots
        if let Some(mut equipment) = self.get_component(entity, "Equipment").cloned() {
            if let Some(slots_obj) = equipment.get_mut("slots").and_then(|v| v.as_object_mut()) {
                for slot in equipped_slots {
                    slots_obj.insert(slot.clone(), JsonValue::Null);
                }
            }
            let _ = self.set_component(entity, "Equipment", equipment);
        }
    }

    /// Validate an entity's equipment against slot rules and the item registry.
    ///
    /// Returns a list of issues. An empty list means valid equipment.
    pub fn validate_equipment(&self, entity: u32) -> Vec<EquipmentIssue> {
        let mut issues = Vec::new();

        let equipment = match self.get_component(entity, "Equipment") {
            Some(e) => e,
            None => return issues,
        };

        let slots = match equipment.get("slots").and_then(|v| v.as_object()) {
            Some(s) => s,
            None => return issues,
        };

        let valid_slots = load_valid_slots();
        let entity_stats = self
            .get_component(entity, "Stats")
            .cloned()
            .unwrap_or_default();

        for (slot_name, item_id_value) in slots {
            // Skip empty slots
            let item_id = match item_id_value.as_str() {
                Some(id) => id,
                None => continue,
            };

            // Check: slot is valid
            if !valid_slots.is_empty() && !valid_slots.contains(slot_name.as_str()) {
                issues.push(EquipmentIssue {
                    slot: slot_name.clone(),
                    item_id: item_id.to_string(),
                    reason: "unknown_slot".to_string(),
                });
                continue;
            }

            // Check: item has registered definition
            let item_def = match self.item_registry.get_item(item_id) {
                Some(def) => def.clone(),
                None => {
                    issues.push(EquipmentIssue {
                        slot: slot_name.clone(),
                        item_id: item_id.to_string(),
                        reason: "item_not_registered".to_string(),
                    });
                    continue;
                }
            };

            // Check: item's slot field matches the slot name
            if let Some(item_slot) = item_def.get("slot").and_then(|v| v.as_str())
                && item_slot != slot_name
            {
                issues.push(EquipmentIssue {
                    slot: slot_name.clone(),
                    item_id: item_id.to_string(),
                    reason: "slot_mismatch".to_string(),
                });
                continue;
            }

            // Check: entity meets stat requirements
            if let Some(requirements) = item_def.get("requirements").and_then(|v| v.as_object()) {
                let mut unmet = false;
                for (req_key, req_val) in requirements {
                    if let (Some(req_num), Some(stat_val)) = (
                        req_val.as_i64(),
                        entity_stats.get(req_key).and_then(|v| v.as_i64()),
                    ) && stat_val < req_num
                    {
                        unmet = true;
                        break;
                    }
                }
                if unmet {
                    issues.push(EquipmentIssue {
                        slot: slot_name.clone(),
                        item_id: item_id.to_string(),
                        reason: "unmet_requirements".to_string(),
                    });
                }
            }
        }

        issues
    }
}
