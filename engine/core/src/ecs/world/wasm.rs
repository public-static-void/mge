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

/// Camera state for the WASM world.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    /// X position of the camera viewport.
    pub x: i32,
    /// Y position of the camera viewport.
    pub y: i32,
    /// Width of the camera viewport.
    pub width: i32,
    /// Height of the camera viewport.
    pub height: i32,
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
    /// Camera state
    #[serde(default)]
    pub camera: Option<Camera>,
    /// Event buses — maps event type → list of event payloads
    #[serde(default)]
    pub event_buses: HashMap<String, Vec<JsonValue>>,
    /// Reader positions per event bus for poll_event tracking
    #[serde(default)]
    event_reader_positions: HashMap<String, usize>,
    /// Registered systems — maps name → system type
    #[serde(default)]
    pub systems: HashMap<String, String>,
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
            camera: None,
            event_buses: HashMap::new(),
            event_reader_positions: HashMap::new(),
            systems: HashMap::new(),
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

    /// Serializes world state to a JSON file.
    pub fn save_to_file(&self, path: &str) -> Result<(), String> {
        let json = serde_json::to_string(self).map_err(|e| e.to_string())?;
        std::fs::write(path, &json).map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Deserializes world state from a JSON file, replacing current state.
    pub fn load_from_file(&mut self, path: &str) -> Result<(), String> {
        let json = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let loaded: WasmWorld = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        *self = loaded;
        Ok(())
    }

    /// Sets the camera viewport position and dimensions.
    pub fn set_camera(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.camera = Some(Camera {
            x,
            y,
            width,
            height,
        });
    }

    /// Returns the current camera state as a JSON string, or None if unset.
    pub fn get_camera(&self) -> Option<String> {
        self.camera
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap_or_default())
    }

    /// Sends an event to the given event bus.
    pub fn send_event(&mut self, event_type: &str, event_data: &str) -> Result<(), String> {
        let value: JsonValue = serde_json::from_str(event_data)
            .map_err(|e| format!("Failed to parse event JSON: {e}"))?;
        self.event_buses
            .entry(event_type.to_string())
            .or_default()
            .push(value);
        Ok(())
    }

    /// Returns all unconsumed events for the given type as a JSON array string.
    pub fn poll_event(&self, event_type: &str) -> String {
        let pos = self
            .event_reader_positions
            .get(event_type)
            .copied()
            .unwrap_or(0);
        let events = self
            .event_buses
            .get(event_type)
            .map(|v| &v[pos..])
            .unwrap_or(&[]);
        serde_json::to_string(events).unwrap_or_else(|_| "[]".to_string())
    }

    /// Advances all event reader positions to the end, consuming all events.
    pub fn update_event_buses(&mut self) {
        for (name, bus) in &self.event_buses {
            self.event_reader_positions
                .entry(name.clone())
                .or_insert(bus.len());
            *self.event_reader_positions.get_mut(name).unwrap() = bus.len();
        }
    }

    /// Registers a system with the given name and type.
    pub fn register_system(&mut self, name: &str, system_type: &str) {
        self.systems
            .insert(name.to_string(), system_type.to_string());
    }

    /// Runs a registered system by name.
    /// Currently checks for system existence; execution stub for WASM callback integration.
    pub fn run_system(&self, name: &str) -> Result<(), String> {
        if self.systems.contains_key(name) {
            Ok(())
        } else {
            Err(format!("System '{name}' not found"))
        }
    }

    /// Assigns a move path (JSON array) to an agent's Agent component.
    pub fn assign_move_path(&mut self, agent_id: u32, path_json: &str) -> Result<(), String> {
        let path_value: JsonValue = serde_json::from_str(path_json)
            .map_err(|e| format!("Failed to parse move path JSON: {e}"))?;

        let agent = self
            .components
            .entry("Agent".to_string())
            .or_default()
            .entry(agent_id)
            .or_insert_with(|| serde_json::json!({"entity_id": agent_id}));

        agent["move_path"] = path_value;
        Ok(())
    }

    /// Checks if an agent's Position matches the given cell coordinates.
    /// `cell_json` must be a JSON object with one key identifying the cell kind
    /// (e.g. `{"Square": {"x": 0, "y": 0, "z": 0}}`).
    pub fn is_agent_at_cell(&self, agent_id: u32, cell_json: &str) -> bool {
        let pos = match self.components.get("Position") {
            Some(comps) => match comps.get(&agent_id) {
                Some(p) => p,
                None => return false,
            },
            None => return false,
        };

        let cell: JsonValue = match serde_json::from_str(cell_json) {
            Ok(c) => c,
            Err(_) => return false,
        };

        let cell_obj = match cell.as_object() {
            Some(o) => o,
            None => return false,
        };

        let (kind, coords) = match cell_obj.iter().next() {
            Some((k, v)) => (k.as_str(), v),
            None => return false,
        };

        match kind {
            "Square" => {
                let cx = coords.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                let cy = coords.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                let cz = coords.get("z").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                let px = pos.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let py = pos.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let pz = pos.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0);
                (px - cx).abs() < f64::EPSILON
                    && (py - cy).abs() < f64::EPSILON
                    && (pz - cz).abs() < f64::EPSILON
            }
            "Hex" => {
                let cq = coords.get("q").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                let cr = coords.get("r").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                let cz = coords.get("z").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                let pq = pos.get("q").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let pr = pos.get("r").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let pz = pos.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0);
                (pq - cq).abs() < f64::EPSILON
                    && (pr - cr).abs() < f64::EPSILON
                    && (pz - cz).abs() < f64::EPSILON
            }
            "Province" => {
                let cid = coords.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let pid = pos.get("id").and_then(|v| v.as_str()).unwrap_or("");
                cid == pid
            }
            _ => false,
        }
    }

    /// Returns true if the agent has no `move_path` or the path array is empty.
    pub fn is_move_path_empty(&self, agent_id: u32) -> bool {
        match self.components.get("Agent") {
            Some(agents) => match agents.get(&agent_id) {
                Some(agent) => match agent.get("move_path") {
                    Some(path) => !path.as_array().is_some_and(|a| !a.is_empty()),
                    None => true,
                },
                None => true,
            },
            None => true,
        }
    }

    /// Gets the Equipment component of an entity as a JSON string.
    pub fn get_equipment(&self, entity_id: u32) -> Option<String> {
        self.get_component(entity_id, "Equipment")
    }

    /// Sets the Equipment component on an entity from a JSON string.
    pub fn set_equipment(&mut self, entity_id: u32, json_data: &str) -> Result<(), String> {
        self.set_component(entity_id, "Equipment", json_data)
    }

    /// Equips an item into a slot on the entity's Equipment component.
    pub fn equip_item(&mut self, entity_id: u32, item_id: &str, slot: &str) -> Result<(), String> {
        let equipment = self
            .components
            .entry("Equipment".to_string())
            .or_default()
            .entry(entity_id)
            .or_insert_with(|| serde_json::json!({"slots": {}}));

        let slots = equipment
            .get_mut("slots")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| "Equipment slots field is missing or not an object".to_string())?;

        if let Some(existing) = slots.get(slot) {
            if !existing.is_null() {
                return Err(format!("Slot '{slot}' is already occupied"));
            }
        }

        slots.insert(
            slot.to_string(),
            serde_json::Value::String(item_id.to_string()),
        );
        Ok(())
    }

    /// Unequips an item from a slot on the entity's Equipment component.
    pub fn unequip_item(&mut self, entity_id: u32, slot: &str) -> Result<(), String> {
        let equipment = self
            .components
            .get_mut("Equipment")
            .ok_or_else(|| "No Equipment component found".to_string())?;
        let slots = equipment
            .get_mut(&entity_id)
            .ok_or_else(|| format!("Entity {entity_id} has no Equipment component"))?
            .get_mut("slots")
            .and_then(|v| v.as_object_mut())
            .ok_or_else(|| "Equipment slots field is missing or not an object".to_string())?;

        slots.insert(slot.to_string(), serde_json::Value::Null);
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
