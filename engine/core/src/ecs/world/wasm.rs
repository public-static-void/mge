use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WasmWorld {
    pub entities: Vec<u32>,
    pub components: HashMap<String, HashMap<u32, JsonValue>>,
    next_id: u32,
    pub current_mode: String,
    pub turn: u32,
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
}
