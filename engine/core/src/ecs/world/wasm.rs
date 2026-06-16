use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};

/// Time of day
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TimeOfDay {
    /// Current hour.
    pub hour: u8,
    /// Current minute.
    pub minute: u8,
}

/// Wasm implementation of a world
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WasmWorld {
    /// Entities
    pub entities: Vec<u32>,
    /// Components
    pub components: HashMap<String, HashMap<u32, JsonValue>>,
    next_id: u32,
    /// Current game mode
    pub current_mode: String,
    /// Current turn
    pub turn: u32,
    /// Time of day
    pub time_of_day: TimeOfDay,
}

impl WasmWorld {
    /// Create a new world
    pub fn new() -> Self {
        WasmWorld {
            entities: Vec::new(),
            components: HashMap::new(),
            next_id: 1,
            current_mode: "colony".to_string(),
            turn: 0,
            time_of_day: TimeOfDay { hour: 6, minute: 0 },
        }
    }

    /// Spawn a new entity
    pub fn spawn_entity(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push(id);
        id
    }

    /// Despawn an entity
    pub fn despawn_entity(&mut self, entity: u32) {
        for comps in self.components.values_mut() {
            comps.remove(&entity);
        }
        self.entities.retain(|&id| id != entity);
    }

    /// Get all entities
    pub fn get_entities(&self) -> &[u32] {
        &self.entities
    }

    /// Get all entities with a specific component
    pub fn get_entities_with_component(&self, name: &str) -> Vec<u32> {
        self.components
            .get(name)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all entities with specific components
    pub fn get_entities_with_components(&self, names: &[&str]) -> Vec<u32> {
        if names.is_empty() {
            return self.entities.clone();
        }
        let mut sets: Vec<HashSet<u32>> = names
            .iter()
            .filter_map(|name| self.components.get(*name))
            .map(|comps| comps.keys().cloned().collect())
            .collect();
        if sets.is_empty() {
            return vec![];
        }
        let first = sets.pop().unwrap();
        sets.into_iter()
            .fold(first, |acc, set| acc.intersection(&set).cloned().collect())
            .into_iter()
            .collect()
    }

    /// Count all entities of a specific type
    pub fn count_entities_with_type(&self, type_str: &str) -> usize {
        self.get_entities_with_component(type_str).len()
    }

    /// Check if an entity is alive
    pub fn is_entity_alive(&self, entity_id: u32) -> bool {
        self.entities.contains(&entity_id)
    }

    /// Move an entity
    pub fn move_entity(&mut self, entity_id: u32, dx: f32, dy: f32) {
        // This implementation assumes a "Position" component with "x" and "y" fields.
        let comps = self.components.entry("Position".to_string()).or_default();
        let pos = comps
            .entry(entity_id)
            .or_insert_with(|| serde_json::json!({"x": 0.0, "y": 0.0}));

        let x = pos.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) + dx as f64;
        let y = pos.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) + dy as f64;

        *pos = serde_json::json!({"x": x, "y": y});
    }

    /// Damage an entity
    pub fn damage_entity(&mut self, entity_id: u32, amount: f32) {
        // This implementation assumes a "Health" component with an "hp" field.
        let comps = self.components.entry("Health".to_string()).or_default();
        let health = comps
            .entry(entity_id)
            .or_insert_with(|| serde_json::json!({"hp": 100.0}));

        let hp = health.get("hp").and_then(|v| v.as_f64()).unwrap_or(100.0) - amount as f64;
        *health = serde_json::json!({"hp": hp.max(0.0)});
    }

    /// Set a component on an entity from a JSON string.
    pub fn set_component(
        &mut self,
        entity_id: u32,
        component_name: &str,
        json_data: &str,
    ) -> Result<(), String> {
        let value: JsonValue = serde_json::from_str(json_data)
            .map_err(|e| format!("Failed to parse component JSON: {e}"))?;
        self.components
            .entry(component_name.to_string())
            .or_default()
            .insert(entity_id, value);
        Ok(())
    }

    /// Get a component from an entity as a JSON string.
    pub fn get_component(&self, entity_id: u32, component_name: &str) -> Option<String> {
        self.components
            .get(component_name)?
            .get(&entity_id)
            .map(|v| serde_json::to_string(v).unwrap_or_default())
    }

    /// Remove a component from an entity.
    pub fn remove_component(&mut self, entity_id: u32, component_name: &str) -> Result<(), String> {
        let comps = self
            .components
            .get_mut(component_name)
            .ok_or_else(|| format!("Component '{component_name}' not found"))?;
        comps
            .remove(&entity_id)
            .ok_or_else(|| format!("Entity {entity_id} has no component '{component_name}'"))?;
        // Clean up empty component type maps
        if comps.is_empty() {
            self.components.remove(component_name);
        }
        Ok(())
    }

    /// Advance the simulation by one tick.
    pub fn tick(&mut self) {
        self.turn += 1;
        self.advance_time_of_day();
    }

    /// Returns the current turn number.
    pub fn get_turn(&self) -> i32 {
        self.turn as i32
    }

    /// Sets the current game mode.
    pub fn set_mode(&mut self, mode: &str) {
        self.current_mode = mode.to_string();
    }

    /// Returns the current game mode.
    pub fn get_mode(&self) -> &str {
        &self.current_mode
    }

    /// Returns the list of available game modes.
    pub fn get_available_modes(&self) -> Vec<String> {
        let mut modes = vec!["colony".to_string()];
        for comps in self.components.values() {
            for value in comps.values() {
                if let Some(obj) = value.as_object()
                    && let Some(mode) = obj.get("_mode").and_then(|v| v.as_str())
                    && !modes.contains(&mode.to_string())
                {
                    modes.push(mode.to_string());
                }
            }
        }
        modes
    }

    /// Process deaths: entities with Health.current <= 0 become Corpses with Decay.
    pub fn process_deaths(&mut self) {
        let entity_ids: Vec<u32> = self.entities.clone();
        let mut to_convert = Vec::new();

        if let Some(healths) = self.components.get("Health") {
            for entity in &entity_ids {
                if let Some(value) = healths.get(entity) {
                    let current = value.get("current").and_then(|v| v.as_f64()).unwrap_or(1.0);
                    if current <= 0.0 {
                        to_convert.push(*entity);
                    }
                }
            }
        }

        for entity in to_convert {
            if let Some(healths) = self.components.get_mut("Health") {
                healths.remove(&entity);
            }
            self.components
                .entry("Corpse".to_string())
                .or_default()
                .insert(entity, serde_json::json!({}));
            self.components
                .entry("Decay".to_string())
                .or_default()
                .insert(entity, serde_json::json!({"time_remaining": 5}));
        }
    }

    /// Process decay: decrement Decay.time_remaining, despawn entities when it reaches 0.
    pub fn process_decay(&mut self) {
        let mut to_despawn = Vec::new();

        if let Some(decays) = self.components.get("Decay") {
            for (&entity, value) in decays.iter() {
                let remaining = value
                    .get("time_remaining")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                if remaining <= 1 {
                    to_despawn.push(entity);
                }
            }
        }

        if let Some(decays) = self.components.get_mut("Decay") {
            for (&entity, value) in decays.iter_mut() {
                if !to_despawn.contains(&entity)
                    && let Some(obj) = value.as_object_mut()
                    && let Some(tr) = obj.get_mut("time_remaining")
                    && let Some(t) = tr.as_u64()
                {
                    *tr = serde_json::json!(t - 1);
                }
            }
        }

        for entity in to_despawn {
            self.despawn_entity(entity);
        }
    }

    /// Returns the current time of day.
    pub fn get_time_of_day(&self) -> TimeOfDay {
        self.time_of_day
    }

    /// Reads a line of user input from stdin.
    pub fn get_user_input(&mut self) -> Option<String> {
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(0) => None,
            Ok(_) => {
                let trimmed = input.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            }
            Err(_) => None,
        }
    }

    /// Gets the Inventory component of an entity as a JSON string.
    pub fn get_inventory(&self, entity_id: u32) -> Option<String> {
        self.get_component(entity_id, "Inventory")
    }

    /// Sets the Inventory component on an entity from a JSON string.
    pub fn set_inventory(&mut self, entity_id: u32, json_data: &str) -> Result<(), String> {
        self.set_component(entity_id, "Inventory", json_data)
    }

    /// Adds an item (JSON value) to an entity's Inventory slots.
    pub fn add_item_to_inventory(&mut self, entity_id: u32, item_json: &str) -> Result<(), String> {
        let item_value: JsonValue = serde_json::from_str(item_json)
            .map_err(|e| format!("Failed to parse item JSON: {e}"))?;

        let inv = self
            .components
            .entry("Inventory".to_string())
            .or_default()
            .entry(entity_id)
            .or_insert_with(|| serde_json::json!({"slots": []}));

        let slots = inv
            .get_mut("slots")
            .and_then(|v| v.as_array_mut())
            .ok_or_else(|| "Inventory has no slots array".to_string())?;

        slots.push(item_value);
        Ok(())
    }

    /// Removes an item from an entity's Inventory at the given slot index.
    pub fn remove_item_from_inventory(
        &mut self,
        entity_id: u32,
        slot_id: i32,
    ) -> Result<(), String> {
        let comps = self
            .components
            .get_mut("Inventory")
            .ok_or_else(|| "No Inventory component found".to_string())?;
        let inv = comps
            .get_mut(&entity_id)
            .ok_or_else(|| format!("Entity {entity_id} has no Inventory component"))?;
        let slots = inv
            .get_mut("slots")
            .and_then(|v| v.as_array_mut())
            .ok_or_else(|| "No slots array in Inventory".to_string())?;
        let idx = slot_id as usize;
        if idx >= slots.len() {
            return Err("Slot index out of bounds".to_string());
        }
        slots.remove(idx);
        Ok(())
    }

    fn advance_time_of_day(&mut self) {
        self.time_of_day.minute += 1;
        if self.time_of_day.minute >= 60 {
            self.time_of_day.minute = 0;
            self.time_of_day.hour += 1;
            if self.time_of_day.hour >= 24 {
                self.time_of_day.hour = 0;
            }
        }
    }
}
