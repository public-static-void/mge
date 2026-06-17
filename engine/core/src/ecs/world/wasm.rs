use crate::map::CellKey;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet};
use std::path::Path;

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

/// Simplified serializable map data for the WASM world.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WasmMap {
    /// Topology type: "square", "hex", "province", or "none"
    pub topology_type: String,
    /// All cells in the map
    pub cells: Vec<CellKey>,
    /// Adjacency list: canonical CellKey JSON string → Vec of neighbor canonical CellKey JSON strings
    pub neighbors: HashMap<String, Vec<String>>,
    /// Per-cell metadata: canonical CellKey JSON string → metadata JSON
    pub cell_metadata: HashMap<String, JsonValue>,
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
    /// Component schemas loaded at initialization: name → JSON Schema
    #[serde(default)]
    pub component_schemas: HashMap<String, JsonValue>,
    /// Map data for spatial operations
    #[serde(default)]
    pub map: Option<WasmMap>,
    /// Export names discovered during WASM module instantiation
    #[serde(default)]
    pub discovered_export_names: Vec<String>,
    /// Export names registered as map validators
    #[serde(default)]
    pub map_validator_names: Vec<String>,
    /// Export names registered as map postprocessors
    #[serde(default)]
    pub map_postprocessor_names: Vec<String>,
    /// WASM worldgen plugin names registered by the guest module
    #[serde(default)]
    pub wasm_worldgen_plugins: Vec<String>,
    /// WASM worldgen validator export names
    #[serde(default)]
    pub wasm_worldgen_validators: Vec<String>,
    /// WASM worldgen postprocessor export names
    #[serde(default)]
    pub wasm_worldgen_postprocessors: Vec<String>,
}

/// Load all JSON schema files from a directory.
/// Returns a map from schema name (using the "name" field if present, otherwise the filename stem) to parsed JSON schema.
/// Missing or invalid JSON files are silently skipped.
pub fn load_schemas_from_dir(schema_dir: &Path) -> HashMap<String, JsonValue> {
    let mut schemas = HashMap::new();
    if let Ok(entries) = std::fs::read_dir(schema_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json")
                && let Ok(content) = std::fs::read_to_string(&path)
                && let Ok(value) = serde_json::from_str::<JsonValue>(&content)
            {
                let name = value
                    .get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| path.file_stem().unwrap().to_string_lossy().to_string());
                schemas.insert(name, value);
            }
        }
    }
    schemas
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
            component_schemas: HashMap::new(),
            map: None,
            discovered_export_names: Vec::new(),
            map_validator_names: Vec::new(),
            map_postprocessor_names: Vec::new(),
            wasm_worldgen_plugins: Vec::new(),
            wasm_worldgen_validators: Vec::new(),
            wasm_worldgen_postprocessors: Vec::new(),
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

    /// Returns all events for the given type as a JSON array string, removing the bus entry.
    /// Returns "[]" if no events exist for that type.
    pub fn take_events(&mut self, event_type: &str) -> String {
        let events = self.event_buses.remove(event_type).unwrap_or_default();
        self.event_reader_positions.remove(event_type);
        serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string())
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

        if let Some(existing) = slots.get(slot)
            && !existing.is_null()
        {
            return Err(format!("Slot '{slot}' is already occupied"));
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

    // ---- Component Introspection ----

    /// Returns all known component names, combining schema names and stored component names.
    pub fn list_components(&self) -> Vec<String> {
        let mut names: Vec<String> = self.component_schemas.keys().cloned().collect();
        for name in self.components.keys() {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        names.sort();
        names
    }

    /// Returns the JSON schema for a named component, or None if not found.
    pub fn get_component_schema(&self, name: &str) -> Option<String> {
        self.component_schemas
            .get(name)
            .map(|schema| serde_json::to_string(schema).unwrap_or_default())
    }

    // ---- Region API ----

    /// Returns all entities whose Region component has the given id.
    pub fn entities_in_region(&self, region_id: &str) -> Vec<u32> {
        self.get_entities_with_component("Region")
            .into_iter()
            .filter(|&eid| {
                self.components
                    .get("Region")
                    .and_then(|m| m.get(&eid))
                    .and_then(|val| val.get("id"))
                    .map(|id_val| match id_val {
                        JsonValue::String(s) => s == region_id,
                        JsonValue::Array(arr) => arr.iter().any(|v| v.as_str() == Some(region_id)),
                        _ => false,
                    })
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Returns all entities whose Region component has the given kind.
    pub fn entities_in_region_kind(&self, kind: &str) -> Vec<u32> {
        self.get_entities_with_component("Region")
            .into_iter()
            .filter(|&eid| {
                self.components
                    .get("Region")
                    .and_then(|m| m.get(&eid))
                    .and_then(|val| val.get("kind"))
                    .and_then(|k| k.as_str())
                    .map(|k| k == kind)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Returns all cells (from RegionAssignment component) assigned to the given region_id.
    pub fn cells_in_region(&self, region_id: &str) -> Vec<JsonValue> {
        self.get_entities_with_component("RegionAssignment")
            .into_iter()
            .filter_map(|eid| {
                self.components
                    .get("RegionAssignment")
                    .and_then(|m| m.get(&eid))
                    .and_then(|val| {
                        let cell = val.get("cell").cloned()?;
                        let rid = val.get("region_id");
                        match rid {
                            Some(JsonValue::String(s)) if s == region_id => Some(cell),
                            Some(JsonValue::Array(arr))
                                if arr.iter().any(|v| v.as_str() == Some(region_id)) =>
                            {
                                Some(cell)
                            }
                            _ => None,
                        }
                    })
            })
            .collect()
    }

    /// Returns all cells (from RegionAssignment component) assigned to regions of the given kind.
    pub fn cells_in_region_kind(&self, kind: &str) -> Vec<JsonValue> {
        self.get_entities_with_component("RegionAssignment")
            .into_iter()
            .filter_map(|eid| {
                self.components
                    .get("RegionAssignment")
                    .and_then(|m| m.get(&eid))
                    .and_then(|val| {
                        let k = val.get("kind").and_then(|v| v.as_str());
                        let cell = val.get("cell").cloned()?;
                        if k == Some(kind) { Some(cell) } else { None }
                    })
            })
            .collect()
    }

    // ---- Economic API ----

    /// Returns the stockpile resources JSON for an entity, or None if missing.
    pub fn get_stockpile_resources(&self, entity_id: u32) -> Option<String> {
        self.components
            .get("Stockpile")
            .and_then(|m| m.get(&entity_id))
            .and_then(|v| v.get("resources"))
            .map(|r| serde_json::to_string(r).unwrap_or_default())
    }

    /// Returns the full ProductionJob component for an entity, or None.
    pub fn get_production_job(&self, entity_id: u32) -> Option<String> {
        self.get_component(entity_id, "ProductionJob")
    }

    /// Returns the progress field from a ProductionJob, defaulting to 0.0.
    pub fn get_production_job_progress(&self, entity_id: u32) -> f64 {
        self.components
            .get("ProductionJob")
            .and_then(|m| m.get(&entity_id))
            .and_then(|v| v.get("progress"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    }

    /// Sets the progress field on a ProductionJob.
    pub fn set_production_job_progress(&mut self, entity_id: u32, value: f64) {
        if let Some(job) = self
            .components
            .get_mut("ProductionJob")
            .and_then(|m| m.get_mut(&entity_id))
        {
            job["progress"] = serde_json::json!(value);
        }
    }

    /// Returns the state field from a ProductionJob, defaulting to "pending".
    pub fn get_production_job_state(&self, entity_id: u32) -> String {
        self.components
            .get("ProductionJob")
            .and_then(|m| m.get(&entity_id))
            .and_then(|v| v.get("state"))
            .and_then(|v| v.as_str())
            .unwrap_or("pending")
            .to_string()
    }

    /// Sets the state field on a ProductionJob.
    pub fn set_production_job_state(&mut self, entity_id: u32, value: &str) {
        if let Some(job) = self
            .components
            .get_mut("ProductionJob")
            .and_then(|m| m.get_mut(&entity_id))
        {
            job["state"] = serde_json::json!(value);
        }
    }

    /// Modifies a stockpile resource by delta. Returns error if balance would go negative.
    pub fn modify_stockpile_resource(
        &mut self,
        entity_id: u32,
        kind: &str,
        delta: f64,
    ) -> Result<(), String> {
        let stockpile = self
            .components
            .get_mut("Stockpile")
            .and_then(|m| m.get_mut(&entity_id));
        if let Some(obj) = stockpile.and_then(|v| v.as_object_mut())
            && let Some(resources) = obj.get_mut("resources").and_then(|v| v.as_object_mut())
        {
            let current = resources.get(kind).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let new_amount = current + delta;
            if new_amount < 0.0 {
                return Err("Not enough resource".to_string());
            }
            resources.insert(kind.to_string(), serde_json::json!(new_amount));
            Ok(())
        } else {
            Err("Stockpile component not found".to_string())
        }
    }

    /// Returns the reserved_resources from a Job component, or None if absent/empty.
    pub fn get_job_resource_reservations(&self, entity_id: u32) -> Option<String> {
        self.components
            .get("Job")
            .and_then(|m| m.get(&entity_id))
            .and_then(|v| v.get("reserved_resources"))
            .and_then(|arr| {
                if arr.as_array().is_none_or(|a| a.is_empty()) {
                    None
                } else {
                    Some(serde_json::to_string(arr).unwrap_or_default())
                }
            })
    }

    // ---- Body API ----

    /// Returns the Body component for an entity, or None.
    pub fn get_body(&self, entity_id: u32) -> Option<String> {
        self.get_component(entity_id, "Body")
    }

    /// Sets the Body component on an entity from a JSON string.
    pub fn set_body(&mut self, entity_id: u32, json_data: &str) -> Result<(), String> {
        self.set_component(entity_id, "Body", json_data)
    }

    /// Adds a body part (JSON) to an entity's Body. Creates the Body component if absent.
    pub fn add_body_part(&mut self, entity_id: u32, part_json: &str) -> Result<(), String> {
        let part_value: JsonValue = serde_json::from_str(part_json)
            .map_err(|e| format!("Failed to parse part JSON: {e}"))?;
        let body = self
            .components
            .entry("Body".to_string())
            .or_default()
            .entry(entity_id)
            .or_insert_with(|| serde_json::json!({"parts": []}));
        let parts = body
            .get_mut("parts")
            .and_then(|v| v.as_array_mut())
            .ok_or_else(|| "Body has no parts array".to_string())?;
        parts.push(part_value);
        Ok(())
    }

    /// Recursively finds and removes a body part by name. Returns error if not found.
    pub fn remove_body_part(&mut self, entity_id: u32, name: &str) -> Result<(), String> {
        let body = self
            .components
            .get_mut("Body")
            .and_then(|m| m.get_mut(&entity_id))
            .ok_or_else(|| "No Body component found".to_string())?;

        fn remove_recursive(parts: &mut Vec<JsonValue>, name: &str) -> bool {
            let mut i = 0;
            while i < parts.len() {
                if parts[i].get("name").and_then(|n| n.as_str()) == Some(name) {
                    parts.remove(i);
                    return true;
                }
                if let Some(children) = parts[i].get_mut("children").and_then(|v| v.as_array_mut())
                    && remove_recursive(children, name)
                {
                    return true;
                }
                i += 1;
            }
            false
        }

        let parts = body
            .get_mut("parts")
            .and_then(|v| v.as_array_mut())
            .ok_or_else(|| "No parts array in Body".to_string())?;

        if remove_recursive(parts, name) {
            Ok(())
        } else {
            Err("Body part not found".to_string())
        }
    }

    /// Recursively finds a body part by name and returns its JSON. Returns None if not found.
    pub fn get_body_part(&self, entity_id: u32, name: &str) -> Option<String> {
        let body = self.components.get("Body")?.get(&entity_id)?;

        fn find_recursive(parts: &[JsonValue], name: &str) -> Option<JsonValue> {
            for part in parts {
                if part.get("name").and_then(|n| n.as_str()) == Some(name) {
                    return Some(part.clone());
                }
                if let Some(children) = part.get("children").and_then(|v| v.as_array())
                    && let Some(found) = find_recursive(children, name)
                {
                    return Some(found);
                }
            }
            None
        }

        let parts = body.get("parts")?.as_array()?;
        find_recursive(parts, name).map(|v| serde_json::to_string(&v).unwrap_or_default())
    }

    // ---- Map API ----

    /// Returns the map topology type, or "none" if no map is initialized.
    pub fn get_map_topology_type(&self) -> String {
        self.map
            .as_ref()
            .map(|m| m.topology_type.clone())
            .unwrap_or_else(|| "none".to_string())
    }

    /// Returns all cells in the map.
    pub fn get_all_cells(&self) -> Vec<CellKey> {
        self.map
            .as_ref()
            .map(|m| m.cells.clone())
            .unwrap_or_default()
    }

    /// Adds a square cell at (x, y, z). Sets topology type to "square" if unset.
    pub fn add_cell(&mut self, x: i32, y: i32, z: i32) {
        let map = self.map.get_or_insert_with(WasmMap::default);
        if map.topology_type.is_empty() || map.topology_type == "none" {
            map.topology_type = "square".to_string();
        }
        let cell = CellKey::Square { x, y, z };
        if !map.cells.contains(&cell) {
            map.cells.push(cell);
        }
    }

    /// Returns neighbors of a cell as a JSON array string.
    pub fn get_neighbors(&self, cell_json: &str) -> String {
        let map = match self.map.as_ref() {
            Some(m) => m,
            None => return "[]".to_string(),
        };
        let cell_key: CellKey = match serde_json::from_str(cell_json) {
            Ok(k) => k,
            Err(_) => return "[]".to_string(),
        };
        let key = serde_json::to_string(&cell_key).unwrap_or_default();
        map.neighbors
            .get(&key)
            .map(|neighbors| {
                let cells: Vec<CellKey> = neighbors
                    .iter()
                    .filter_map(|n| serde_json::from_str(n).ok())
                    .collect();
                serde_json::to_string(&cells).unwrap_or_else(|_| "[]".to_string())
            })
            .unwrap_or_else(|| "[]".to_string())
    }

    /// Adds a bidirectional neighbor edge between two cells.
    pub fn add_neighbor(&mut self, from_json: &str, to_json: &str) -> Result<(), String> {
        let from_key: CellKey =
            serde_json::from_str(from_json).map_err(|e| format!("Invalid from cell: {e}"))?;
        let to_key: CellKey =
            serde_json::from_str(to_json).map_err(|e| format!("Invalid to cell: {e}"))?;

        let map = self.map.get_or_insert_with(WasmMap::default);
        let from_str = serde_json::to_string(&from_key).unwrap();
        let to_str = serde_json::to_string(&to_key).unwrap();

        if !map.cells.contains(&from_key) {
            map.cells.push(from_key);
        }
        if !map.cells.contains(&to_key) {
            map.cells.push(to_key);
        }

        map.neighbors
            .entry(from_str.clone())
            .or_default()
            .push(to_str.clone());
        map.neighbors.entry(to_str).or_default().push(from_str);
        Ok(())
    }

    /// Returns entity IDs whose Position component matches the given cell.
    pub fn entities_in_cell(&self, cell_json: &str) -> Vec<u32> {
        let cell_key: CellKey = match serde_json::from_str(cell_json) {
            Ok(k) => k,
            Err(_) => return vec![],
        };
        self.entities
            .iter()
            .copied()
            .filter(|&eid| {
                self.components
                    .get("Position")
                    .and_then(|m| m.get(&eid))
                    .is_some_and(|pos| match &cell_key {
                        CellKey::Square {
                            x: cx,
                            y: cy,
                            z: cz,
                        } => {
                            let px = pos.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
                            let py = pos.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
                            let pz = pos.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
                            *cx == px && *cy == py && *cz == pz
                        }
                        CellKey::Hex {
                            q: cq,
                            r: cr,
                            z: cz,
                        } => {
                            let pq = pos.get("q").and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
                            let pr = pos.get("r").and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
                            let pz = pos.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as i32;
                            *cq == pq && *cr == pr && *cz == pz
                        }
                        CellKey::Province { id: cid } => pos
                            .get("id")
                            .and_then(|v| v.as_str())
                            .is_some_and(|pid| pid == cid),
                    })
            })
            .collect()
    }

    /// Returns cell metadata for a cell, or None if absent.
    pub fn get_cell_metadata(&self, cell_json: &str) -> Option<String> {
        let cell_key: CellKey = serde_json::from_str(cell_json).ok()?;
        let key = serde_json::to_string(&cell_key).ok()?;
        self.map
            .as_ref()?
            .cell_metadata
            .get(&key)
            .map(|v| serde_json::to_string(v).unwrap_or_default())
    }

    /// Sets metadata for a cell.
    pub fn set_cell_metadata(&mut self, cell_json: &str, meta_json: &str) {
        if let Ok(cell_key) = serde_json::from_str::<CellKey>(cell_json)
            && let Ok(meta) = serde_json::from_str::<JsonValue>(meta_json)
        {
            let key = serde_json::to_string(&cell_key).unwrap();
            let map = self.map.get_or_insert_with(WasmMap::default);
            map.cell_metadata.insert(key, meta);
        }
    }

    /// BFS shortest path between two cells. Returns None if no path exists.
    pub fn find_path(&self, start_json: &str, goal_json: &str) -> Option<String> {
        use std::collections::VecDeque;

        let start: CellKey = serde_json::from_str(start_json).ok()?;
        let goal: CellKey = serde_json::from_str(goal_json).ok()?;
        let map = self.map.as_ref()?;

        let start_key = serde_json::to_string(&start).ok()?;
        let goal_key = serde_json::to_string(&goal).ok()?;

        if start_key == goal_key {
            let result = serde_json::json!({
                "path": [start],
                "total_cost": 0
            });
            return serde_json::to_string(&result).ok();
        }

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<String, String> = HashMap::new();

        queue.push_back(start_key.clone());
        visited.insert(start_key.clone());
        let mut found = false;

        while let Some(current) = queue.pop_front() {
            if current == goal_key {
                found = true;
                break;
            }
            if let Some(neighbors) = map.neighbors.get(&current) {
                for neighbor_key in neighbors {
                    if !visited.contains(neighbor_key) {
                        visited.insert(neighbor_key.clone());
                        parent.insert(neighbor_key.clone(), current.clone());
                        queue.push_back(neighbor_key.clone());
                    }
                }
            }
        }

        if !found {
            return None;
        }

        let mut path_keys = Vec::new();
        let mut cur = goal_key;
        loop {
            let cell: CellKey = serde_json::from_str(&cur).ok()?;
            path_keys.push(cell);
            if cur == start_key {
                break;
            }
            cur = parent.get(&cur)?.clone();
        }
        path_keys.reverse();

        let result = serde_json::json!({
            "path": path_keys,
            "total_cost": (path_keys.len() - 1) as f64
        });
        serde_json::to_string(&result).ok()
    }

    /// Replaces the entire WasmMap with parsed JSON.
    pub fn apply_generated_map(&mut self, map_json: &str) -> Result<(), String> {
        let parsed: WasmMap =
            serde_json::from_str(map_json).map_err(|e| format!("Failed to parse map JSON: {e}"))?;
        self.map = Some(parsed);
        Ok(())
    }

    /// Register a discovered export as a map validator.
    pub fn register_map_validator(&mut self, name: &str) -> Result<(), String> {
        if self.map_validator_names.contains(&name.to_string()) {
            return Err(format!("Map validator '{}' already registered", name));
        }
        self.map_validator_names.push(name.to_string());
        Ok(())
    }

    /// Clear all registered map validators.
    pub fn clear_map_validators(&mut self) {
        self.map_validator_names.clear();
    }

    /// Register a discovered export as a map postprocessor.
    pub fn register_map_postprocessor(&mut self, name: &str) -> Result<(), String> {
        if self.map_postprocessor_names.contains(&name.to_string()) {
            return Err(format!("Map postprocessor '{}' already registered", name));
        }
        self.map_postprocessor_names.push(name.to_string());
        Ok(())
    }

    /// Clear all registered map postprocessors.
    pub fn clear_map_postprocessors(&mut self) {
        self.map_postprocessor_names.clear();
    }

    /// Apply a chunk of map data (cells + neighbors + metadata) to the world.
    /// The `chunk_json` format:
    /// ```json
    /// {
    ///   "cells": [{"x": 0, "y": 0, "z": 0}, ...],
    ///   "neighbors": [{"from": {"x": 0, "y": 0, "z": 0}, "to": {"x": 1, "y": 0, "z": 0}}, ...],
    ///   "metadata": {"(0,0,0)": {"terrain": "grass"}}
    /// }
    /// ```
    pub fn apply_chunk(&mut self, chunk_json: &str) -> Result<(), String> {
        let chunk: JsonValue = serde_json::from_str(chunk_json)
            .map_err(|e| format!("Failed to parse chunk JSON: {e}"))?;

        // Parse and add cells
        if let Some(cells) = chunk.get("cells").and_then(|v| v.as_array()) {
            for cell in cells {
                let x = cell.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = cell.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let z = cell.get("z").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                self.add_cell(x, y, z);
            }
        }

        // Parse and add neighbors
        if let Some(neighbors) = chunk.get("neighbors").and_then(|v| v.as_array()) {
            for n in neighbors {
                let from = n.get("from");
                let to = n.get("to");
                if let (Some(f), Some(t)) = (from, to) {
                    let fx = f.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let fy = f.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let fz = f.get("z").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let tx = t.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let ty = t.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let tz = t.get("z").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let from_key = CellKey::Square {
                        x: fx,
                        y: fy,
                        z: fz,
                    };
                    let to_key = CellKey::Square {
                        x: tx,
                        y: ty,
                        z: tz,
                    };
                    self.add_neighbor(
                        &serde_json::to_string(&from_key).map_err(|e| e.to_string())?,
                        &serde_json::to_string(&to_key).map_err(|e| e.to_string())?,
                    )?;
                }
            }
        }

        // Parse and add metadata
        if let Some(metadata) = chunk.get("metadata").and_then(|v| v.as_object()) {
            let map = self.map.get_or_insert_with(WasmMap::default);
            for (cell_key_str, meta) in metadata {
                map.cell_metadata.insert(cell_key_str.clone(), meta.clone());
            }
        }

        Ok(())
    }

    /// Returns the number of cells in the map, or 0 if no map.
    pub fn get_map_cell_count(&self) -> i32 {
        self.map.as_ref().map(|m| m.cells.len() as i32).unwrap_or(0)
    }

    // ---- Economic Reservation API ----

    /// Runs resource reservation for all pending jobs.
    /// Uses ResourceReservationSystem internally; WasmWorld implements ResourceReservationOps.
    pub fn reserve_job_resources(&mut self) {
        let mut system = crate::systems::job::reservation::ResourceReservationSystem::new();
        system.run_reservation(self);
    }

    /// Releases reserved resources for a specific job.
    pub fn release_job_resource_reservations(&mut self, entity_id: u32) {
        let system = crate::systems::job::reservation::ResourceReservationSystem::new();
        system.release_reservation(self, entity_id);
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

use crate::systems::job::reservation::resource_reservation_ops::ResourceReservationOps;

impl ResourceReservationOps for WasmWorld {
    fn get_entities_with_component(&self, name: &str) -> Vec<u32> {
        WasmWorld::get_entities_with_component(self, name)
    }

    fn get_component_value(&self, entity: u32, name: &str) -> Option<JsonValue> {
        self.get_component(entity, name)
            .and_then(|s| serde_json::from_str(&s).ok())
    }

    fn set_component_value(
        &mut self,
        entity: u32,
        name: &str,
        value: JsonValue,
    ) -> Result<(), String> {
        let json_str = serde_json::to_string(&value).map_err(|e| e.to_string())?;
        self.set_component(entity, name, &json_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_world_reservation_flow() {
        let mut world = WasmWorld::new();

        // Create a stockpile entity
        let stockpile_eid = world.spawn_entity();
        let stockpile_data = serde_json::json!({"resources": {"iron_ore": 100.0}});
        world
            .set_component(
                stockpile_eid,
                "Stockpile",
                &serde_json::to_string(&stockpile_data).unwrap(),
            )
            .unwrap();

        // Verify stockpile was stored
        let stockpile_str = world.get_component(stockpile_eid, "Stockpile").unwrap();
        let stockpile_val: JsonValue = serde_json::from_str(&stockpile_str).unwrap();
        assert_eq!(
            stockpile_val["resources"]["iron_ore"], 100.0,
            "Stockpile should have iron_ore 100.0"
        );

        // Create a job entity
        let job_eid = world.spawn_entity();
        let job_data = serde_json::json!({
            "state": "pending",
            "resource_requirements": [{"kind": "iron_ore", "amount": 10}]
        });
        world
            .set_component(job_eid, "Job", &serde_json::to_string(&job_data).unwrap())
            .unwrap();

        // Verify job was stored
        let job_str = world.get_component(job_eid, "Job").unwrap();
        let job_val: JsonValue = serde_json::from_str(&job_str).unwrap();
        assert_eq!(job_val["state"], "pending", "Job state should be pending");
        assert!(
            job_val.get("resource_requirements").is_some(),
            "Job should have resource_requirements"
        );

        // Verify entity lists
        let stockpile_entities = world.get_entities_with_component("Stockpile");
        assert_eq!(
            stockpile_entities,
            vec![stockpile_eid],
            "Should find stockpile entity"
        );
        let job_entities = world.get_entities_with_component("Job");
        assert_eq!(job_entities, vec![job_eid], "Should find job entity");

        // Check: no reservations yet
        let reservations = world.get_job_resource_reservations(job_eid);
        assert!(
            reservations.is_none(),
            "Should have no reservations before reserve"
        );

        // Run reservation
        world.reserve_job_resources();

        // Check: should have reservations
        let reservations = world.get_job_resource_reservations(job_eid);
        assert!(
            reservations.is_some(),
            "Should have reservations after reserve"
        );
        let reservations_str = reservations.unwrap();
        let res_value: JsonValue = serde_json::from_str(&reservations_str).unwrap();
        assert!(
            res_value.is_array(),
            "Reserved resources should be an array"
        );
        assert!(
            !res_value.as_array().unwrap().is_empty(),
            "Reserved resources should not be empty"
        );

        // Release reservation
        world.release_job_resource_reservations(job_eid);

        // Check: reservations cleared
        let reservations = world.get_job_resource_reservations(job_eid);
        assert!(
            reservations.is_none(),
            "Should have no reservations after release"
        );
    }

    #[test]
    fn test_wasm_world_reservation_insufficient_resources() {
        let mut world = WasmWorld::new();

        // Stockpile with insufficient resources
        let stockpile_eid = world.spawn_entity();
        let stockpile_data = serde_json::json!({"resources": {"iron_ore": 5.0}});
        world
            .set_component(
                stockpile_eid,
                "Stockpile",
                &serde_json::to_string(&stockpile_data).unwrap(),
            )
            .unwrap();

        // Job needs 10 iron_ore
        let job_eid = world.spawn_entity();
        let job_data = serde_json::json!({
            "state": "pending",
            "resource_requirements": [{"kind": "iron_ore", "amount": 10}]
        });
        world
            .set_component(job_eid, "Job", &serde_json::to_string(&job_data).unwrap())
            .unwrap();

        world.reserve_job_resources();

        // Should still have no reservations (insufficient resources)
        let reservations = world.get_job_resource_reservations(job_eid);
        assert!(
            reservations.is_none(),
            "Should have no reservations with insufficient resources"
        );
    }
}
