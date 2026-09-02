use crate::ecs::equipment_set::{EquipmentSet, EquipmentSetRegistry};
use crate::ecs::item::ItemRegistry;
use crate::ecs::template::UnitTemplateRegistry;
use crate::ecs::world::component::enforce_schema_defaults;
use crate::ecs::world::loadout::EquipmentIssue;
use crate::loot::LootTableRegistry;
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
    /// Current day (0-indexed, incremented on hour wrap 23→0).
    #[serde(default)]
    pub day: u64,
}

/// Camera state for the WASM world.
///
/// Flat `{x, y, z}` viewport, identical in shape to the Lua/Python camera
/// surfaces. `width`/`height` were removed in the parity revert (R008); old
/// saved state carrying those fields still deserializes because serde ignores
/// unknown fields (no `deny_unknown_fields`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    /// X position of the camera viewport.
    pub x: i32,
    /// Y position of the camera viewport.
    pub y: i32,
    /// Z-level of the camera viewport.
    #[serde(default)]
    pub z: i32,
}

/// A simplified job event record stored in WasmWorld.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmJobEvent {
    /// Timestamp of the event (milliseconds since UNIX epoch).
    pub timestamp: u128,
    /// Type of the event (e.g., "job_progressed", "job_completed").
    pub event_type: String,
    /// Payload of the event as arbitrary JSON.
    pub payload: serde_json::Value,
}

/// A UI event record stored in the poll-based event queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmUiEvent {
    /// Widget ID that generated the event
    pub widget_id: u32,
    /// Event type string (e.g. "click", "key_press")
    pub event_type: String,
    /// Event data as arbitrary JSON
    pub event_data: serde_json::Value,
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

/// A scale-generic link from a source map to a target map (WASM bridge mirror
/// of core `MapLink`). Cells are topology-generic [`CellKey`]s, so a link may
/// connect any two topologies (square/hex/province in any combination).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmMapLink {
    /// Name of the target map.
    pub target_map: String,
    /// Cell on the source map that anchors the link.
    pub source_cell: CellKey,
    /// Cell on the target map that anchors the link.
    pub target_cell: CellKey,
}

/// Injectable input source for `WasmWorld::get_user_input()`.
///
/// Defaults to `Stdin` which blocks on `std::io::stdin().read_line()`.
/// Tests inject `Mock` to avoid hanging on stdin reads.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum InputSource {
    /// Block on real stdin (production default).
    #[default]
    Stdin,
    /// Pre-loaded responses consumed FIFO; returns `None` when empty.
    Mock(Vec<String>),
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
    /// Named map registry (runtime-only, not serialized). `WasmWorld.map` remains the active map.
    #[serde(skip)]
    pub maps: HashMap<String, WasmMap>,
    /// Name of the map currently held in `WasmWorld.map`.
    #[serde(skip)]
    pub active_map: String,
    /// Source map name -> link to target map (scale-generic transitions, runtime-only).
    #[serde(skip)]
    pub map_links: HashMap<String, WasmMapLink>,
    /// Active-map history for `exit_map()` (runtime-only).
    #[serde(skip)]
    pub map_stack: Vec<String>,
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

    /// Registered job type metadata: job_type_name → JSON metadata.
    #[serde(default)]
    pub job_type_data: HashMap<String, JsonValue>,

    /// Registered job type names (for get_job_types, insertion order preserved).
    #[serde(default)]
    pub job_type_names: Vec<String>,

    /// Current job board policy name (default: "priority").
    #[serde(default)]
    pub job_board_policy: String,

    /// Entity IDs of jobs currently on the board.
    #[serde(default)]
    pub job_board_jobs: Vec<u32>,

    /// Instance-local job event log.
    #[serde(default)]
    pub job_event_log: Vec<WasmJobEvent>,

    /// Widget registry: widget_id → serialized widget properties as JSON
    #[serde(default)]
    pub widget_registry: HashMap<u32, JsonValue>,

    /// Widget types: widget_id → type name string
    #[serde(default)]
    pub widget_types: HashMap<u32, String>,

    /// Parent lookup: widget_id → parent_id (reverse of widget_tree)
    #[serde(default)]
    pub widget_parents: HashMap<u32, u32>,

    /// Registered custom widget type names (for register_widget dedup)
    #[serde(default)]
    pub widget_types_set: HashSet<String>,

    /// Auto-incrementing widget ID counter.
    #[serde(default)]
    pub next_widget_id: u32,

    /// Parent widget ID -> list of child widget IDs (forward tree lookup).
    #[serde(default)]
    pub widget_tree: HashMap<u32, Vec<u32>>,

    /// Top-level widget IDs (no parent).
    #[serde(default)]
    pub widget_roots: Vec<u32>,

    /// UI event queue: pushed by trigger_event, drained by poll_ui_events
    #[serde(default)]
    pub ui_event_queue: Vec<WasmUiEvent>,

    /// Currently focused widget ID (0 = none)
    #[serde(default)]
    pub focused_widget: u32,

    /// Per-entity visible cell cache (computed by FovUpdateSystem).
    #[serde(skip)]
    pub visible_cells: HashMap<u32, HashSet<CellKey>>,

    /// Explored cells per entity (persistent fog-of-war state, serialized for save/load).
    /// Old saves without this field deserialize as empty (backward compatible).
    #[serde(default)]
    pub explored_cells: HashMap<u32, HashSet<CellKey>>,

    /// Loot table registry (runtime-defined, not serialized).
    #[serde(skip)]
    pub loot_tables: LootTableRegistry,

    /// Material definitions loaded at initialization: name → JSON properties.
    #[serde(default)]
    pub material_definitions: HashMap<String, JsonValue>,

    /// Active FOV algorithm name (for display/debugging).
    #[serde(skip, default = "default_fov_algo_name")]
    pub fov_algorithm_name: String,

    /// Injectable input source for `get_user_input()`.
    #[serde(skip, default)]
    pub input_source: InputSource,

    /// Unit template registry for spawning entities from data-driven templates.
    #[serde(skip)]
    pub template_registry: UnitTemplateRegistry,

    /// Item definition registry for O(1) item lookups.
    #[serde(skip)]
    pub item_registry: ItemRegistry,

    /// Equipment set registry for named loadout blueprints.
    #[serde(skip)]
    pub equipment_set_registry: EquipmentSetRegistry,
}

fn default_fov_algo_name() -> String {
    "recursive_shadowcasting".to_string()
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
            time_of_day: TimeOfDay {
                hour: 6,
                minute: 0,
                day: 0,
            },
            camera: None,
            event_buses: HashMap::new(),
            event_reader_positions: HashMap::new(),
            systems: HashMap::new(),
            component_schemas: HashMap::new(),
            map: None,
            maps: HashMap::new(),
            active_map: String::new(),
            map_links: HashMap::new(),
            map_stack: Vec::new(),
            discovered_export_names: Vec::new(),
            map_validator_names: Vec::new(),
            map_postprocessor_names: Vec::new(),
            wasm_worldgen_plugins: Vec::new(),
            wasm_worldgen_validators: Vec::new(),
            wasm_worldgen_postprocessors: Vec::new(),
            job_type_data: HashMap::new(),
            job_type_names: Vec::new(),
            job_board_policy: "priority".to_string(),
            job_board_jobs: Vec::new(),
            job_event_log: Vec::new(),
            visible_cells: HashMap::new(),
            explored_cells: HashMap::new(),
            widget_registry: HashMap::new(),
            widget_types: HashMap::new(),
            widget_parents: HashMap::new(),
            widget_types_set: HashSet::new(),
            next_widget_id: 1,
            widget_tree: HashMap::new(),
            widget_roots: Vec::new(),
            ui_event_queue: Vec::new(),
            focused_widget: 0,
            loot_tables: LootTableRegistry::new(),
            material_definitions: HashMap::new(),
            fov_algorithm_name: "recursive_shadowcasting".to_string(),
            input_source: InputSource::default(),
            template_registry: UnitTemplateRegistry::new(),
            item_registry: ItemRegistry::new(),
            equipment_set_registry: EquipmentSetRegistry::new(),
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
    ///
    /// Mutates `x`/`y` (Square) or `q`/`r` (Hex) in place, preserving `z` and
    /// every other field. Province (`id`-only) positions are left unchanged.
    /// When the entity has no Position, a flat default at the requested delta
    /// with `z: 0.0` is created.
    pub fn move_entity(&mut self, entity_id: u32, dx: f32, dy: f32) {
        let comps = self.components.entry("Position".to_string()).or_default();
        if let std::collections::hash_map::Entry::Vacant(e) = comps.entry(entity_id) {
            e.insert(serde_json::json!({"x": dx as f64, "y": dy as f64, "z": 0.0}));
            return;
        }
        let pos = comps.get_mut(&entity_id).unwrap();
        let obj = match pos.as_object_mut() {
            Some(obj) => obj,
            None => return,
        };
        if obj.contains_key("x") {
            let x = obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) + dx as f64;
            let y = obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) + dy as f64;
            obj.insert("x".to_string(), serde_json::json!(x));
            obj.insert("y".to_string(), serde_json::json!(y));
        } else if obj.contains_key("q") {
            let q = obj.get("q").and_then(|v| v.as_f64()).unwrap_or(0.0) + dx as f64;
            let r = obj.get("r").and_then(|v| v.as_f64()).unwrap_or(0.0) + dy as f64;
            obj.insert("q".to_string(), serde_json::json!(q));
            obj.insert("r".to_string(), serde_json::json!(r));
        }
        // Province ("id"-only) and other position shapes are left unchanged.
    }

    /// Move an entity in three dimensions (flat Position format).
    ///
    /// Square positions shift `x`/`y`/`z` by `dx`/`dy`/`dz`; Hex positions shift
    /// `q`/`r`/`z` by `dx`/`dy`/`dz`. Province (`id`-only) positions are left
    /// unchanged. When the entity has no Position, a flat default at the
    /// requested `x`/`y` delta with `z: 0.0` is created, mirroring
    /// [`move_entity`](Self::move_entity).
    pub fn move_entity_3d(&mut self, entity_id: u32, dx: f32, dy: f32, dz: f32) {
        let comps = self.components.entry("Position".to_string()).or_default();
        if let std::collections::hash_map::Entry::Vacant(e) = comps.entry(entity_id) {
            e.insert(serde_json::json!({"x": dx as f64, "y": dy as f64, "z": 0.0}));
            return;
        }
        let pos = comps.get_mut(&entity_id).unwrap();
        let obj = match pos.as_object_mut() {
            Some(obj) => obj,
            None => return,
        };
        if obj.contains_key("x") {
            let x = obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) + dx as f64;
            let y = obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) + dy as f64;
            let z = obj.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) + dz as f64;
            obj.insert("x".to_string(), serde_json::json!(x));
            obj.insert("y".to_string(), serde_json::json!(y));
            obj.insert("z".to_string(), serde_json::json!(z));
        } else if obj.contains_key("q") {
            let q = obj.get("q").and_then(|v| v.as_f64()).unwrap_or(0.0) + dx as f64;
            let r = obj.get("r").and_then(|v| v.as_f64()).unwrap_or(0.0) + dy as f64;
            let z = obj.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) + dz as f64;
            obj.insert("q".to_string(), serde_json::json!(q));
            obj.insert("r".to_string(), serde_json::json!(r));
            obj.insert("z".to_string(), serde_json::json!(z));
        }
        // Province ("id"-only) and other position shapes are left unchanged.
    }

    /// Damage an entity.
    ///
    /// Routes through PendingDamage when the entity has a Body component,
    /// otherwise falls back to direct Health subtraction.
    pub fn damage_entity(&mut self, entity_id: u32, amount: f32) {
        let has_body = self
            .components
            .get("Body")
            .is_some_and(|m| m.contains_key(&entity_id));

        if has_body {
            self.append_pending_damage(entity_id, amount as f64, None);
        } else {
            let comps = self.components.entry("Health".to_string()).or_default();
            let health = comps
                .entry(entity_id)
                .or_insert_with(|| serde_json::json!({"hp": 100.0}));

            let hp = health.get("hp").and_then(|v| v.as_f64()).unwrap_or(100.0) - amount as f64;
            *health = serde_json::json!({"hp": hp.max(0.0)});
        }
    }

    /// Apply targeted damage to a specific body part.
    pub fn damage_entity_part(&mut self, entity_id: u32, part_name: &str, amount: f32) {
        let has_body = self
            .components
            .get("Body")
            .is_some_and(|m| m.contains_key(&entity_id));
        if has_body {
            self.append_pending_damage(entity_id, amount as f64, Some(part_name));
        }
    }

    /// Appends a damage entry to the entity's PendingDamage component.
    fn append_pending_damage(&mut self, entity_id: u32, amount: f64, target_part: Option<&str>) {
        let target_part_val = match target_part {
            Some(name) => serde_json::json!(name),
            None => serde_json::json!(null),
        };

        let damage_entry = serde_json::json!({
            "amount": amount,
            "target_part": target_part_val
        });

        if let Some(pending) = self.components.get_mut("PendingDamage")
            && let Some(value) = pending.get_mut(&entity_id)
            && let Some(obj) = value.as_object_mut()
            && let Some(damages) = obj.get_mut("damages")
            && let Some(arr) = damages.as_array_mut()
        {
            arr.push(damage_entry);
        } else {
            let pending = serde_json::json!({
                "damages": [damage_entry]
            });
            self.components
                .entry("PendingDamage".to_string())
                .or_default()
                .insert(entity_id, pending);
        }
    }

    /// Process PendingDamage for all entities with Body components.
    /// Mirrors the BodyPartDamageSystem logic for WASM host API.
    pub fn process_body_part_damage(&mut self) {
        // Collect entities that have both Body and PendingDamage
        let entities: Vec<u32> = {
            let body_ids: std::collections::HashSet<u32> = self
                .components
                .get("Body")
                .map(|m| m.keys().copied().collect())
                .unwrap_or_default();
            let pending_ids: std::collections::HashSet<u32> = self
                .components
                .get("PendingDamage")
                .map(|m| m.keys().copied().collect())
                .unwrap_or_default();
            body_ids.intersection(&pending_ids).copied().collect()
        };

        for entity in entities {
            let pending = match self
                .components
                .get("PendingDamage")
                .and_then(|m| m.get(&entity))
                .cloned()
            {
                Some(p) => p,
                None => continue,
            };

            let damages = match pending.get("damages").and_then(|v| v.as_array()) {
                Some(d) if !d.is_empty() => d.clone(),
                _ => {
                    if let Some(pd) = self.components.get_mut("PendingDamage") {
                        pd.remove(&entity);
                    }
                    continue;
                }
            };

            let mut body = match self
                .components
                .get("Body")
                .and_then(|m| m.get(&entity))
                .cloned()
            {
                Some(b) => b,
                None => {
                    if let Some(pd) = self.components.get_mut("PendingDamage") {
                        pd.remove(&entity);
                    }
                    continue;
                }
            };

            if let Some(parts) = body.get_mut("parts").and_then(|v| v.as_array_mut()) {
                // Phase 1: Collect damage amounts per part name
                let mut damage_per_part: std::collections::HashMap<String, f64> =
                    std::collections::HashMap::new();

                for damage_entry in &damages {
                    let amount = damage_entry
                        .get("amount")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    if amount <= 0.0 {
                        continue;
                    }

                    let target_part = damage_entry.get("target_part").and_then(|v| v.as_str());

                    if let Some(target) = target_part {
                        *damage_per_part.entry(target.to_string()).or_insert(0.0) += amount;
                    } else {
                        // Untargeted: distribute proportionally across non-missing parts
                        let mut part_info: Vec<(String, f64, String)> = Vec::new();
                        Self::collect_part_info_static(parts, &mut part_info);

                        let total_max_hp: f64 = part_info
                            .iter()
                            .filter(|(_, _, status)| status != "missing")
                            .map(|(_, max_hp, _)| max_hp)
                            .sum();

                        if total_max_hp > 0.0 {
                            for (name, max_hp, status) in &part_info {
                                if status == "missing" {
                                    continue;
                                }
                                let weight = max_hp / total_max_hp;
                                let share = amount * weight;
                                *damage_per_part.entry(name.clone()).or_insert(0.0) += share;
                            }
                        }
                    }
                }

                // Phase 2: Apply accumulated damage to each part
                for (name, amount) in &damage_per_part {
                    Self::apply_damage_to_named_part_static(parts, name, *amount);
                }
            }

            // Recompute Health.current as sum of all parts' hp
            let total_hp = {
                let mut hp_vals: Vec<f64> = Vec::new();
                if let Some(parts) = body.get("parts").and_then(|v| v.as_array()) {
                    Self::collect_part_hp_static(parts, &mut hp_vals);
                }
                hp_vals.iter().sum::<f64>().max(0.0)
            };

            self.components
                .entry("Body".to_string())
                .or_default()
                .insert(entity, body);

            // Update Health.current
            if let Some(healths) = self.components.get_mut("Health")
                && let Some(value) = healths.get_mut(&entity)
                && let Some(obj) = value.as_object_mut()
                && let Some(current) = obj.get_mut("current")
            {
                *current = serde_json::json!(total_hp);
            }

            if let Some(pd) = self.components.get_mut("PendingDamage") {
                pd.remove(&entity);
            }
        }
    }

    fn collect_part_info_static(
        parts: &[serde_json::Value],
        result: &mut Vec<(String, f64, String)>,
    ) {
        for part in parts {
            let name = part
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();
            let max_hp = part.get("max_hp").and_then(|v| v.as_f64()).unwrap_or(25.0);
            let status = part
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("healthy")
                .to_string();
            result.push((name, max_hp, status));
            if let Some(children) = part.get("children").and_then(|v| v.as_array()) {
                Self::collect_part_info_static(children, result);
            }
        }
    }

    fn apply_damage_to_named_part_static(
        parts: &mut [serde_json::Value],
        target: &str,
        amount: f64,
    ) {
        for part in parts.iter_mut() {
            if part.get("name").and_then(|n| n.as_str()) == Some(target) {
                // Convert to owned String before mutating part to avoid borrow conflict
                let cur_status = part
                    .get("status")
                    .and_then(|s| s.as_str())
                    .unwrap_or("healthy")
                    .to_string();
                if cur_status == "missing" {
                    return;
                }
                let hp = part.get("hp").and_then(|v| v.as_f64()).unwrap_or(25.0);
                let actual = amount.min(hp);
                let new_hp = (hp - actual).max(0.0);
                part["hp"] = serde_json::json!(new_hp);

                // Update status based on HP thresholds
                let max_hp = part.get("max_hp").and_then(|v| v.as_f64()).unwrap_or(25.0);
                let new_status = if cur_status == "broken" && new_hp <= 0.0 {
                    "missing"
                } else if new_hp <= 0.0 {
                    "broken"
                } else if new_hp <= 0.5 * max_hp {
                    "wounded"
                } else {
                    "healthy"
                };
                part["status"] = serde_json::json!(new_status);
                return;
            }
            if let Some(children) = part.get_mut("children").and_then(|v| v.as_array_mut()) {
                Self::apply_damage_to_named_part_static(children, target, amount);
            }
        }
    }

    fn collect_part_hp_static(parts: &[serde_json::Value], result: &mut Vec<f64>) {
        for part in parts {
            result.push(part.get("hp").and_then(|v| v.as_f64()).unwrap_or(25.0));
            if let Some(children) = part.get("children").and_then(|v| v.as_array()) {
                Self::collect_part_hp_static(children, result);
            }
        }
    }

    /// Set a component on an entity from a JSON string.
    ///
    /// Validates against schema (if registered) and enforces defaults,
    /// matching `World::set_component` behavior.
    pub fn set_component(
        &mut self,
        entity_id: u32,
        component_name: &str,
        json_data: &str,
    ) -> Result<(), String> {
        let mut value: JsonValue = serde_json::from_str(json_data)
            .map_err(|e| format!("Failed to parse component JSON: {e}"))?;

        // Validate against schema if one is registered for this component
        if let Some(schema) = self.component_schemas.get(component_name) {
            enforce_schema_defaults(&mut value, schema);

            let validator = jsonschema::validator_for(schema)
                .map_err(|e| format!("Schema compile error: {e}"))?;
            let mut errors = validator.iter_errors(&value);
            if let Some(first_error) = errors.next() {
                let mut msgs = vec![first_error.to_string()];
                msgs.extend(errors.map(|e| e.to_string()));
                let msg = msgs.join(", ");
                return Err(format!("Schema validation failed: {msg}"));
            }
        }

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
        self.simulate_fluid();
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

    /// Reads a line of user input from the configured input source.
    ///
    /// When `input_source` is `Stdin`, blocks on `std::io::stdin().read_line()`.
    /// When `Mock`, pops from the front of the pre-loaded queue; returns `None` if empty.
    pub fn get_user_input(&mut self) -> Option<String> {
        match &mut self.input_source {
            InputSource::Stdin => {
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
            InputSource::Mock(queue) => {
                if queue.is_empty() {
                    None
                } else {
                    Some(queue.remove(0))
                }
            }
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

    /// Set the active FOV algorithm by name.
    pub fn set_fov_algorithm_by_name(&mut self, name: &str) -> Result<(), String> {
        match name {
            "recursive_shadowcasting" | "bfs_flood_fill" => {
                self.fov_algorithm_name = name.to_string();
                Ok(())
            }
            _ => Err(format!("FOV algorithm '{name}' is not registered")),
        }
    }

    /// Sets the camera viewport position and z-level.
    ///
    /// 3-arg parity form (`x`, `y`, `z`) matching Lua `set_camera(x, y, z?)`
    /// and Python `set_camera(x, y, z=0)` — the identical camera surface
    /// across all three scripting bridges (R008).
    pub fn set_camera(&mut self, x: i32, y: i32, z: i32) {
        self.camera = Some(Camera { x, y, z });
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
        // Check inventory
        let inv = self
            .components
            .get("Inventory")
            .and_then(|m| m.get(&entity_id))
            .ok_or_else(|| "Entity has no Inventory".to_string())?;
        let inv_slots = inv
            .get("slots")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "No slots array in Inventory".to_string())?;
        if !inv_slots
            .iter()
            .any(|v| v == &serde_json::Value::String(item_id.to_string()))
        {
            return Err("Item not in Inventory".to_string());
        }

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

    /// Enqueue a production job on an entity.
    /// Returns true if enqueued, false if entity already has a ProductionJob.
    pub fn enqueue_production_job(
        &mut self,
        entity_id: u32,
        recipe_name: &str,
        priority: i32,
        batch_size: i32,
    ) -> bool {
        // Check if entity already has a ProductionJob
        if self
            .components
            .get("ProductionJob")
            .is_some_and(|m| m.contains_key(&entity_id))
        {
            return false;
        }
        let priority_val = priority as i64;
        let batch_size_val = if batch_size < 1 { 1 } else { batch_size as i64 };
        let job = serde_json::json!({
            "recipe": recipe_name,
            "progress": 0,
            "state": "pending",
            "priority": priority_val,
            "batch_size": batch_size_val,
        });
        self.components
            .entry("ProductionJob".to_string())
            .or_default()
            .insert(entity_id, job);
        true
    }

    /// Returns the ProductionJob component as a JSON string, or None.
    pub fn get_production_queue(&self, entity_id: u32) -> Option<String> {
        self.get_component(entity_id, "ProductionJob")
    }

    /// Returns completed production jobs for an entity as a JSON array string.
    /// Clears consumed events. Returns "[]" if no completions.
    pub fn get_completed_production_jobs(&mut self, entity_id: u32) -> String {
        let events = self
            .event_buses
            .remove("production_completed")
            .unwrap_or_default();
        let filtered: Vec<serde_json::Value> = events
            .into_iter()
            .filter(|ev| ev.get("entity").and_then(|v| v.as_u64()) == Some(entity_id as u64))
            .collect();
        self.event_reader_positions.remove("production_completed");
        serde_json::to_string(&filtered).unwrap_or_else(|_| "[]".to_string())
    }

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

    /// Adds a cell at (x, y, z). Sets topology type to "square" if unset.
    ///
    /// Topology dispatch (R003): `"hex"` → `CellKey::Hex { q: x, r: y, z }`;
    /// `"square"`/`"none"`/empty → `CellKey::Square` (topology defaults to
    /// `"square"` when unset, byte-identical to the pre-parity behavior);
    /// `"province"` → no cell appended (province is intentionally z-less —
    /// `CellKey::Province` carries only an id, R007).
    pub fn add_cell(&mut self, x: i32, y: i32, z: i32) {
        let map = self.map.get_or_insert_with(WasmMap::default);
        if map.topology_type.is_empty() || map.topology_type == "none" {
            map.topology_type = "square".to_string();
        }
        if map.topology_type == "province" {
            return;
        }
        let cell = if map.topology_type == "hex" {
            CellKey::Hex { q: x, r: y, z }
        } else {
            CellKey::Square { x, y, z }
        };
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

    /// Returns entity IDs whose flat Position is on the given z-level.
    ///
    /// Matches both Square (`x`/`y`/`z`) and Hex (`q`/`r`/`z`) flat positions;
    /// id-only (Province) positions never match.
    pub fn entities_in_zlevel(&self, z: i32) -> Vec<u32> {
        self.entities
            .iter()
            .copied()
            .filter(|&eid| {
                self.components
                    .get("Position")
                    .and_then(|m| m.get(&eid))
                    .is_some_and(|pos| {
                        let has_coord = pos.get("x").is_some() || pos.get("q").is_some();
                        has_coord
                            && pos.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as i32 == z
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

    /// Returns the fluid state for a cell, if any.
    ///
    /// Extracts the `"fluid"` value from the cell's metadata. Returns `None`
    /// when the cell has no metadata or no `"fluid"` key.
    pub fn get_fluid(&self, cell_json: &str) -> Option<String> {
        let cell_key: CellKey = serde_json::from_str(cell_json).ok()?;
        let key = serde_json::to_string(&cell_key).ok()?;
        let meta = self.map.as_ref()?.cell_metadata.get(&key)?;
        let fluid = meta.get("fluid")?;
        Some(serde_json::to_string(fluid).unwrap_or_default())
    }

    /// Run the fluid simulation on the WASM map for one tick.
    ///
    /// Replicates the core `FluidSimulationSystem` algorithm (horizontal
    /// flooding, z-flow, magma-water interaction, blocking keys) against the
    /// WASM world's string-keyed `cell_metadata`. Deterministic cell ordering
    /// mirrors the core system (z desc, x asc, y asc, Province last by id).
    pub fn simulate_fluid(&mut self) {
        use crate::systems::fluid::{FLUID_BLOCK_LEVEL, FLUID_MAX_LEVEL};
        use std::cmp::Reverse;
        use std::collections::HashMap;

        /// Per-type fluid levels.
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
        struct FluidState {
            water: i64,
            magma: i64,
        }
        impl FluidState {
            fn total(self) -> i64 {
                self.water + self.magma
            }
            fn is_empty(self) -> bool {
                self.water == 0 && self.magma == 0
            }
        }

        fn read_fluid(meta: &JsonValue) -> FluidState {
            let Some(fluid) = meta.get("fluid") else {
                return FluidState::default();
            };
            if let Some(level) = fluid.get("level").and_then(JsonValue::as_i64) {
                match fluid.get("type").and_then(JsonValue::as_str) {
                    Some("water") => FluidState {
                        water: level.max(0),
                        magma: 0,
                    },
                    Some("magma") => FluidState {
                        water: 0,
                        magma: level.max(0),
                    },
                    _ => FluidState::default(),
                }
            } else {
                FluidState {
                    water: fluid
                        .get("water")
                        .and_then(JsonValue::as_i64)
                        .unwrap_or(0)
                        .max(0),
                    magma: fluid
                        .get("magma")
                        .and_then(JsonValue::as_i64)
                        .unwrap_or(0)
                        .max(0),
                }
            }
        }

        fn fluid_json(state: FluidState) -> JsonValue {
            if state.water > 0 && state.magma > 0 {
                serde_json::json!({ "water": state.water, "magma": state.magma })
            } else if state.water > 0 {
                serde_json::json!({ "type": "water", "level": state.water })
            } else if state.magma > 0 {
                serde_json::json!({ "type": "magma", "level": state.magma })
            } else {
                serde_json::json!({ "type": "water", "level": 0 })
            }
        }

        fn cell_below_key(cell: &CellKey) -> Option<CellKey> {
            match cell {
                CellKey::Square { x, y, z } => Some(CellKey::Square {
                    x: *x,
                    y: *y,
                    z: z - 1,
                }),
                CellKey::Hex { q, r, z } => Some(CellKey::Hex {
                    q: *q,
                    r: *r,
                    z: z - 1,
                }),
                CellKey::Province { .. } => None,
            }
        }

        fn sort_key(cell: &CellKey) -> (u8, Reverse<i32>, i32, i32, String) {
            match cell {
                CellKey::Square { x, y, z } => (0, Reverse(*z), *x, *y, String::new()),
                CellKey::Hex { q, r, z } => (0, Reverse(*z), *q, *r, String::new()),
                CellKey::Province { id } => (1, Reverse(0), 0, 0, id.clone()),
            }
        }

        let map = match self.map.as_ref() {
            Some(m) => m,
            None => return,
        };

        // Snapshot: clone cells, neighbors, and metadata so we can release the
        // immutable borrow before calling self.send_event() and applying writes.
        let mut cells = map.cells.clone();
        cells.sort_by_key(sort_key);

        let has_fluid = cells.iter().any(|cell| {
            let key = serde_json::to_string(cell).unwrap_or_default();
            map.cell_metadata
                .get(&key)
                .map(|meta| read_fluid(meta).total() > 0)
                .unwrap_or(false)
        });
        if !has_fluid {
            return;
        }

        let cell_keys_json: Vec<String> = cells
            .iter()
            .filter_map(|c| serde_json::to_string(c).ok())
            .collect();
        let neighbor_keys: HashMap<String, Vec<String>> = map
            .neighbors
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let initial_meta: HashMap<String, JsonValue> = cell_keys_json
            .iter()
            .filter_map(|k| map.cell_metadata.get(k).map(|v| (k.clone(), v.clone())))
            .collect();
        // Release map borrow — all reads are done from the snapshots above.
        // ... (immutable borrow ends here)

        let mut pending: HashMap<String, JsonValue> = HashMap::new();
        let mut pre_fluid_blocking: HashMap<String, (Option<bool>, Option<bool>)> = HashMap::new();
        let mut steam_events: Vec<String> = Vec::new();

        for (i, cell) in cells.iter().enumerate() {
            let key = &cell_keys_json[i];
            let meta = pending
                .get(key)
                .or(initial_meta.get(key))
                .cloned()
                .unwrap_or_default();
            let mut state = read_fluid(&meta);
            if state.is_empty() {
                continue;
            }

            // Magma-water interaction.
            if state.water > 0 && state.magma > 0 {
                state.water -= 1;
                state.magma -= 1;
                steam_events.push(
                    serde_json::json!({
                        "cell": cell,
                        "water": state.water,
                        "magma": state.magma
                    })
                    .to_string(),
                );
            }

            // Downward z-flow.
            if let Some(below) = cell_below_key(cell) {
                let below_key = serde_json::to_string(&below).unwrap_or_default();
                if cells.contains(&below) {
                    let below_meta = pending
                        .get(&below_key)
                        .or(initial_meta.get(&below_key))
                        .cloned()
                        .unwrap_or_default();
                    let mut below_state = read_fluid(&below_meta);
                    let before = below_state;
                    if state.water > 0 && below_state.water < FLUID_MAX_LEVEL {
                        let t = state.water.min(1).min(FLUID_MAX_LEVEL - below_state.water);
                        state.water -= t;
                        below_state.water += t;
                    }
                    if state.magma > 0 && below_state.magma < FLUID_MAX_LEVEL {
                        let t = state.magma.min(1).min(FLUID_MAX_LEVEL - below_state.magma);
                        state.magma -= t;
                        below_state.magma += t;
                    }
                    if below_state != before {
                        pending.insert(below_key, fluid_json(below_state));
                    }
                }
            }

            // Horizontal flooding.
            let neighbors = neighbor_keys.get(key).cloned().unwrap_or_default();
            let mut sorted_neighbors: Vec<String> = neighbors
                .iter()
                .filter_map(|n| {
                    let _ck: CellKey = serde_json::from_str(n).ok()?;
                    Some(n.clone())
                })
                .collect();
            sorted_neighbors.sort_by(|a, b| {
                let ka: CellKey = serde_json::from_str(a).unwrap();
                let kb: CellKey = serde_json::from_str(b).unwrap();
                sort_key(&ka).cmp(&sort_key(&kb))
            });
            for neighbor_key in sorted_neighbors {
                let neighbor_exists = cells
                    .iter()
                    .any(|c| serde_json::to_string(c).ok().as_deref() == Some(&neighbor_key));
                if !neighbor_exists {
                    continue;
                }
                let neighbor_meta = pending
                    .get(&neighbor_key)
                    .or(initial_meta.get(&neighbor_key))
                    .cloned()
                    .unwrap_or_default();
                let mut neighbor_state = read_fluid(&neighbor_meta);
                let before = neighbor_state;
                if neighbor_state.water < state.water {
                    let t = ((state.water - neighbor_state.water) / 2)
                        .min(1)
                        .min(FLUID_MAX_LEVEL - neighbor_state.water);
                    if t > 0 {
                        state.water -= t;
                        neighbor_state.water += t;
                    }
                }
                if neighbor_state.magma < state.magma {
                    let t = ((state.magma - neighbor_state.magma) / 2)
                        .min(1)
                        .min(FLUID_MAX_LEVEL - neighbor_state.magma);
                    if t > 0 {
                        state.magma -= t;
                        neighbor_state.magma += t;
                    }
                }
                if neighbor_state != before {
                    pending.insert(neighbor_key, fluid_json(neighbor_state));
                }
            }

            // Write back source cell.
            pending.insert(key.clone(), fluid_json(state));

            // Blocking keys.
            if state.total() >= FLUID_BLOCK_LEVEL {
                if !pre_fluid_blocking.contains_key(key) {
                    let walkable = meta.get("walkable").and_then(JsonValue::as_bool);
                    let transparent = meta.get("transparent").and_then(JsonValue::as_bool);
                    pre_fluid_blocking.insert(key.clone(), (walkable, transparent));
                }
                pending.insert(
                    key.clone(),
                    serde_json::json!({
                        "fluid": fluid_json(state),
                        "walkable": false,
                        "transparent": false
                    }),
                );
            } else if let Some((walkable, transparent)) = pre_fluid_blocking.remove(key) {
                pending.insert(
                    key.clone(),
                    serde_json::json!({
                        "fluid": fluid_json(state),
                        "walkable": walkable.unwrap_or(true),
                        "transparent": transparent.unwrap_or(true)
                    }),
                );
            }
        }

        // Send steam events (self borrow released).
        for payload in &steam_events {
            let _ = self.send_event("steam", payload);
        }

        // Apply all pending metadata writes.
        if let Some(map) = self.map.as_mut() {
            for (k, v) in pending {
                map.cell_metadata.insert(k, v);
            }
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

    // ---- Multi-scale Map Navigation API ----

    /// Register a named map from a `map.json`-shaped JSON string. Errors on duplicate name.
    ///
    /// Mirrors the Lua/Python `register_map(name, map_json)` surface (issue 61
    /// parity): `map_json` carries `topology` + `cells` (with optional per-cell
    /// `neighbors`/`metadata`), converted to a [`WasmMap`] for the bridge state.
    pub fn register_map(&mut self, name: &str, map_json: &str) -> Result<(), String> {
        if self.maps.contains_key(name) {
            return Err(format!("Map '{name}' is already registered"));
        }
        let map = wasm_map_from_map_json(map_json)?;
        self.maps.insert(name.to_string(), map);
        Ok(())
    }

    /// Set the active map to a registered map. Errors on unknown name.
    pub fn set_active_map(&mut self, name: &str) -> Result<(), String> {
        let map = self
            .maps
            .get(name)
            .ok_or_else(|| format!("Map '{name}' is not registered"))?;
        self.map = Some(map.clone());
        self.active_map = name.to_string();
        Ok(())
    }

    /// List all registered map names (unspecified order).
    pub fn get_map_names(&self) -> Vec<String> {
        self.maps.keys().cloned().collect()
    }

    /// Name of the current active map.
    pub fn get_active_map_name(&self) -> String {
        self.active_map.clone()
    }

    /// Link a source map cell to a target map cell. Errors on unknown map name.
    ///
    /// The link is stored per source map (a map's exit target). Both cells are
    /// topology-generic [`CellKey`]s, so any two topologies may be linked.
    pub fn link_maps(
        &mut self,
        source_map: &str,
        source_cell: CellKey,
        target_map: &str,
        target_cell: CellKey,
    ) -> Result<(), String> {
        if !self.maps.contains_key(source_map) {
            return Err(format!("Map '{source_map}' is not registered"));
        }
        if !self.maps.contains_key(target_map) {
            return Err(format!("Map '{target_map}' is not registered"));
        }
        self.map_links.insert(
            source_map.to_string(),
            WasmMapLink {
                target_map: target_map.to_string(),
                source_cell,
                target_cell,
            },
        );
        Ok(())
    }

    /// Transition: set the active map to `name` and position the camera at `entry_cell`.
    ///
    /// Pushes the current active map onto the map stack (so `exit_map()` can
    /// return to it). Errors on unknown map name. Province cells carry no
    /// x/y/z, so the flat camera falls back to zeros (mirroring core
    /// `set_camera_position`).
    pub fn enter_map(&mut self, name: &str, entry_cell: CellKey) -> Result<(), String> {
        let map = self
            .maps
            .get(name)
            .ok_or_else(|| format!("Map '{name}' is not registered"))?;
        if !self.active_map.is_empty() {
            self.map_stack.push(self.active_map.clone());
        }
        self.map = Some(map.clone());
        self.active_map = name.to_string();
        let (x, y, z) = match entry_cell {
            CellKey::Square { x, y, z } => (x, y, z),
            CellKey::Hex { q, r, z } => (q, r, z),
            CellKey::Province { .. } => (0, 0, 0),
        };
        self.camera = Some(Camera { x, y, z });
        Ok(())
    }

    /// Transition: return to the previously active map (pop the map stack).
    ///
    /// Errors if the stack is empty (no prior map).
    pub fn exit_map(&mut self) -> Result<(), String> {
        let prev = self
            .map_stack
            .pop()
            .ok_or_else(|| "Cannot exit map: no previous map on the stack".to_string())?;
        let map = self
            .maps
            .get(&prev)
            .ok_or_else(|| format!("Map '{prev}' is not registered"))?;
        self.map = Some(map.clone());
        self.active_map = prev;
        Ok(())
    }

    /// Map a cell on `source_map` to the linked cell on the target map.
    ///
    /// Returns `None` if no link exists from that source cell.
    pub fn map_cell(&self, source_map: &str, source_cell: &CellKey) -> Option<CellKey> {
        let link = self.map_links.get(source_map)?;
        if &link.source_cell == source_cell {
            Some(link.target_cell.clone())
        } else {
            None
        }
    }

    /// Reverse mapping: map a cell on `target_map` back to the linked source cell.
    ///
    /// Returns `None` if no link exists to that target cell.
    pub fn unmap_cell(&self, target_map: &str, target_cell: &CellKey) -> Option<CellKey> {
        self.map_links
            .values()
            .find(|link| link.target_map == target_map && &link.target_cell == target_cell)
            .map(|link| link.source_cell.clone())
    }

    // ---- FOV API ----

    /// Returns visible cells for an entity, or None if not computed.
    pub fn get_visible_cells(&self, entity: u32) -> Option<&HashSet<CellKey>> {
        self.visible_cells.get(&entity)
    }

    /// Sets visible cells for an entity.
    pub fn set_visible_cells(&mut self, entity: u32, cells: HashSet<CellKey>) {
        self.visible_cells.insert(entity, cells);
    }

    /// Returns explored cells for an entity, or None if not computed.
    pub fn get_explored_cells(&self, entity: u32) -> Option<&HashSet<CellKey>> {
        self.explored_cells.get(&entity)
    }

    /// Sets explored cells for an entity.
    pub fn set_explored_cells(&mut self, entity: u32, cells: HashSet<CellKey>) {
        self.explored_cells.insert(entity, cells);
    }

    /// Reset (clear) fog-of-war for a single entity.
    pub fn reset_fog(&mut self, entity: u32) {
        self.explored_cells.remove(&entity);
    }

    /// Determine the visibility state of a cell for an entity.
    /// Returns 0 = UNEXPLORED, 1 = EXPLORED, 2 = VISIBLE.
    pub fn get_visibility_state(&self, entity: u32, cell: &CellKey) -> u8 {
        let visible = self
            .visible_cells
            .get(&entity)
            .map(|cells| cells.contains(cell))
            .unwrap_or(false);
        if visible {
            return 2;
        }
        let explored = self
            .explored_cells
            .get(&entity)
            .map(|cells| cells.contains(cell))
            .unwrap_or(false);
        if explored { 1 } else { 0 }
    }

    // ---- UI Widget API ----

    /// Get widget type name. Returns None if not found.
    pub fn ui_get_widget_type(&self, id: u32) -> Option<String> {
        self.widget_types.get(&id).cloned()
    }

    /// Get parent widget ID. Returns None if root/not found.
    pub fn ui_get_parent(&self, id: u32) -> Option<u32> {
        self.widget_parents.get(&id).copied()
    }

    /// Set widget z-order. Returns false if widget not found.
    pub fn ui_set_z_order(&mut self, id: u32, z: i32) -> bool {
        if let Some(props) = self.widget_registry.get_mut(&id) {
            if let serde_json::Value::Object(obj) = props {
                obj.insert(
                    "z_order".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(z)),
                );
            }
            true
        } else {
            false
        }
    }

    /// Get widget z-order. Returns 0 if widget not found or no z-order set.
    pub fn ui_get_z_order(&self, id: u32) -> i32 {
        self.widget_registry
            .get(&id)
            .and_then(|props| props.get("z_order"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32
    }

    /// Register a custom widget type. Returns false if already registered.
    pub fn ui_register_widget_type(&mut self, type_name: &str) -> bool {
        if self.widget_types_set.contains(type_name) {
            return false;
        }
        self.widget_types_set.insert(type_name.to_string());
        true
    }

    /// Creates a new widget with the given type and props JSON string.
    /// Returns the auto-assigned widget ID, or 0 on failure (unknown widget type or bad props).
    pub fn ui_create_widget(&mut self, widget_type: &str, props: &str) -> u32 {
        const VALID_TYPES: &[&str] = &[
            "Button",
            "Label",
            "Panel",
            "Checkbox",
            "Dropdown",
            "TextInput",
            "ContextMenu",
        ];
        let is_valid =
            VALID_TYPES.contains(&widget_type) || self.widget_types_set.contains(widget_type);
        if !is_valid {
            return 0;
        }
        let mut props_value: JsonValue = match serde_json::from_str(props) {
            Ok(v) => v,
            Err(_) => return 0,
        };
        if let Some(obj) = props_value.as_object_mut() {
            obj.insert("_type".to_string(), serde_json::json!(widget_type));
        } else {
            props_value = serde_json::json!({"_type": widget_type});
        }
        let id = self.next_widget_id;
        self.next_widget_id += 1;
        self.widget_registry.insert(id, props_value);
        self.widget_types.insert(id, widget_type.to_string());
        self.widget_tree.insert(id, Vec::new());
        self.widget_roots.push(id);
        id
    }

    /// Removes a widget and all its descendants recursively.
    /// Returns true if the widget was found and removed.
    pub fn ui_remove_widget(&mut self, id: u32) -> bool {
        if !self.widget_registry.contains_key(&id) {
            return false;
        }
        if let Some(children) = self.widget_tree.remove(&id) {
            for &child_id in &children {
                self.ui_remove_widget(child_id);
            }
        }
        self.widget_registry.remove(&id);
        self.widget_types.remove(&id);
        if let Some(parent_id) = self.widget_parents.remove(&id)
            && let Some(children) = self.widget_tree.get_mut(&parent_id)
        {
            children.retain(|&c| c != id);
        }
        self.widget_roots.retain(|&r| r != id);
        true
    }

    /// Merges props JSON into an existing widget's properties.
    /// Returns true if the widget was found and updated.
    pub fn ui_set_widget_props(&mut self, id: u32, props: &str) -> bool {
        if !self.widget_registry.contains_key(&id) {
            return false;
        }
        let props_value: JsonValue = match serde_json::from_str(props) {
            Ok(v) => v,
            Err(_) => return false,
        };
        if let Some(existing) = self.widget_registry.get_mut(&id) {
            if let (Some(existing_map), Some(props_map)) =
                (existing.as_object_mut(), props_value.as_object())
            {
                for (key, value) in props_map {
                    existing_map.insert(key.clone(), value.clone());
                }
            } else {
                self.widget_registry.insert(id, props_value);
            }
        }
        true
    }

    /// Returns the props JSON string for a widget, or None if not found.
    pub fn ui_get_widget_props(&self, id: u32) -> Option<String> {
        let props = self.widget_registry.get(&id)?;
        serde_json::to_string(props).ok()
    }

    /// Adds an existing widget as a child of another widget.
    /// Returns true on success, false if either widget not found.
    pub fn ui_add_child(&mut self, parent_id: u32, child_id: u32) -> bool {
        if !self.widget_registry.contains_key(&parent_id) {
            return false;
        }
        if !self.widget_registry.contains_key(&child_id) {
            return false;
        }
        if let Some(&old_parent) = self.widget_parents.get(&child_id)
            && let Some(children) = self.widget_tree.get_mut(&old_parent)
        {
            children.retain(|&c| c != child_id);
        }
        self.widget_roots.retain(|&r| r != child_id);
        self.widget_tree
            .entry(parent_id)
            .or_default()
            .push(child_id);
        self.widget_parents.insert(child_id, parent_id);
        true
    }

    /// Returns the list of child widget IDs for a parent as a JSON array string.
    /// Returns None if the widget is not found.
    pub fn ui_get_children(&self, parent_id: u32) -> Option<String> {
        if !self.widget_registry.contains_key(&parent_id) {
            return None;
        }
        let children = self
            .widget_tree
            .get(&parent_id)
            .cloned()
            .unwrap_or_default();
        Some(serde_json::to_string(&children).unwrap_or_else(|_| "[]".to_string()))
    }

    /// Removes a child from a parent, making the child a root widget.
    /// Returns true on success, false if not a valid parent-child pair.
    pub fn ui_remove_child(&mut self, parent_id: u32, child_id: u32) -> bool {
        if !self.widget_registry.contains_key(&parent_id) {
            return false;
        }
        if !self.widget_registry.contains_key(&child_id) {
            return false;
        }
        let actual_parent = match self.widget_parents.get(&child_id) {
            Some(&p) => p,
            None => return false,
        };
        if actual_parent != parent_id {
            return false;
        }
        if let Some(children) = self.widget_tree.get_mut(&parent_id) {
            children.retain(|&c| c != child_id);
        }
        self.widget_parents.remove(&child_id);
        self.widget_roots.push(child_id);
        true
    }

    /// Loads a widget tree from a JSON string.
    /// Expected format: `{"widgets": [{"id": 1, "type": "Button", "props": {...}, "children": [2, 3]}, ...]}`
    /// Returns a JSON array of created widget IDs, or None on failure.
    pub fn ui_load_json(&mut self, json_str: &str) -> Option<String> {
        let value: JsonValue = serde_json::from_str(json_str).ok()?;
        let widgets = value.get("widgets")?.as_array()?;

        if widgets.is_empty() {
            return None;
        }

        let mut max_id = 0u32;
        let mut widget_entries: Vec<(u32, String, JsonValue, Vec<u32>)> = Vec::new();

        for w in widgets {
            let wid = w.get("id")?.as_u64()? as u32;
            let w_type = w.get("type")?.as_str()?.to_string();
            let props = w
                .get("props")
                .cloned()
                .unwrap_or(JsonValue::Object(serde_json::Map::new()));
            let children: Vec<u32> = w
                .get("children")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_u64().map(|v| v as u32))
                        .collect()
                })
                .unwrap_or_default();

            if self.widget_registry.contains_key(&wid) {
                return None;
            }

            widget_entries.push((wid, w_type, props, children));
            max_id = max_id.max(wid);
        }

        if max_id >= self.next_widget_id {
            self.next_widget_id = max_id + 1;
        }

        let created_ids: Vec<u32> = widget_entries.iter().map(|(id, _, _, _)| *id).collect();

        for (wid, w_type, props, _) in &widget_entries {
            self.widget_registry.insert(*wid, props.clone());
            self.widget_types.insert(*wid, w_type.clone());
            self.widget_tree.insert(*wid, Vec::new());
            self.widget_roots.push(*wid);
        }

        for (wid, _w_type, _props, children) in &widget_entries {
            for &child_id in children {
                if !self.widget_registry.contains_key(&child_id) {
                    return None;
                }
                self.widget_roots.retain(|&r| r != child_id);
                if let Some(&old_parent) = self.widget_parents.get(&child_id)
                    && let Some(old_children) = self.widget_tree.get_mut(&old_parent)
                {
                    old_children.retain(|&c| c != child_id);
                }
                self.widget_tree.entry(*wid).or_default().push(child_id);
                self.widget_parents.insert(child_id, *wid);
            }
        }

        serde_json::to_string(&created_ids).ok()
    }

    // ---- UI Events (Module 3) ----

    /// Sets keyboard focus to a widget. Returns false if not found.
    pub fn ui_focus_widget(&mut self, widget_id: u32) -> bool {
        if !self.widget_registry.contains_key(&widget_id) {
            return false;
        }
        self.focused_widget = widget_id;
        true
    }

    /// Dispatches an event to a widget, pushing to event queue.
    /// Returns false if widget not found.
    pub fn ui_trigger_event(&mut self, widget_id: u32, event_type: &str, event_data: &str) -> bool {
        if !self.widget_registry.contains_key(&widget_id) {
            return false;
        }
        let data: serde_json::Value =
            serde_json::from_str(event_data).unwrap_or(serde_json::Value::Null);
        self.ui_event_queue.push(WasmUiEvent {
            widget_id,
            event_type: event_type.to_string(),
            event_data: data,
        });
        true
    }

    /// Drains the UI event queue and returns events as JSON array string.
    pub fn ui_poll_events(&mut self) -> String {
        let events = std::mem::take(&mut self.ui_event_queue);
        serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string())
    }

    /// Renders the widget tree to terminal output.
    pub fn ui_present(&self) {
        for &root_id in &self.widget_roots {
            self.render_widget(root_id, 0);
        }
    }

    /// Recursive helper for ui_present: renders a widget and its children.
    fn render_widget(&self, id: u32, depth: usize) {
        let indent = "  ".repeat(depth);
        let type_name = self
            .widget_types
            .get(&id)
            .map(|s| s.as_str())
            .unwrap_or("?");
        let props = self.widget_registry.get(&id);
        let summary = props
            .and_then(|p| p.get("text").and_then(|v| v.as_str()))
            .or_else(|| props.and_then(|p| p.get("label").and_then(|v| v.as_str())))
            .unwrap_or("");
        println!("{}{} #{} \"{}\"", indent, type_name, id, summary);
        if let Some(children) = self.widget_tree.get(&id) {
            for &child_id in children {
                self.render_widget(child_id, depth + 1);
            }
        }
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

    // ---- Job System Core API (Module 1) ----

    /// Returns the list of registered job type names.
    pub fn get_job_type_names(&self) -> Vec<String> {
        self.job_type_names.clone()
    }

    /// Registers a job type with a name and metadata JSON string.
    /// Metadata is parsed as JSON and stored in `job_type_data`.
    pub fn register_job_type(&mut self, name: &str, metadata: &str) -> Result<(), String> {
        let meta_val: JsonValue =
            serde_json::from_str(metadata).map_err(|e| format!("Invalid metadata JSON: {e}"))?;
        if !self.job_type_names.contains(&name.to_string()) {
            self.job_type_names.push(name.to_string());
        }
        self.job_type_data.insert(name.to_string(), meta_val);
        Ok(())
    }

    /// Returns the metadata JSON for a job type, or None if not found.
    pub fn get_job_type_metadata(&self, name: &str) -> Option<String> {
        self.job_type_data
            .get(name)
            .map(|v| serde_json::to_string(v).unwrap_or_default())
    }

    // ---- Job Board API (Module 2) ----

    /// Returns the full job board as a JSON array of `{eid, priority, state}` entries.
    pub fn get_job_board(&self) -> String {
        let entries: Vec<JsonValue> = self
            .job_board_jobs
            .iter()
            .filter_map(|&eid| {
                self.get_component(eid, "Job").and_then(|job_str| {
                    let job: JsonValue = serde_json::from_str(&job_str).ok()?;
                    Some(serde_json::json!({
                        "eid": eid,
                        "priority": job.get("priority").and_then(|v| v.as_i64()).unwrap_or(0),
                        "state": job.get("state").and_then(|v| v.as_str()).unwrap_or(""),
                    }))
                })
            })
            .collect();
        serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
    }

    /// Sets the job board scheduling policy. Valid policies: "priority", "fifo", "lifo".
    pub fn set_job_board_policy(&mut self, policy: &str) -> Result<(), String> {
        match policy {
            "priority" | "fifo" | "lifo" => {
                self.job_board_policy = policy.to_string();
                Ok(())
            }
            _ => Err(format!("Unknown policy: {policy}")),
        }
    }

    /// Adds a job entity ID to the job board.
    pub fn add_job_to_job_board(&mut self, job_id: u32) {
        self.job_board_jobs.push(job_id);
    }

    // ---- Job Query API (Module 3) ----

    /// Lists all jobs, optionally including terminal states (complete, failed, cancelled).
    /// Returns a JSON array of job objects with an injected `id` field.
    pub fn list_jobs(&self, include_terminal: bool) -> String {
        let mut jobs: Vec<JsonValue> = Vec::new();
        if let Some(job_map) = self.components.get("Job") {
            for (&eid, job) in job_map {
                let mut j = job.clone();
                j["id"] = serde_json::json!(eid);
                let is_terminal = matches!(
                    j.get("state").and_then(|v| v.as_str()),
                    Some("complete") | Some("failed") | Some("cancelled")
                );
                if !include_terminal && is_terminal {
                    continue;
                }
                jobs.push(j);
            }
        }
        serde_json::to_string(&jobs).unwrap_or_else(|_| "[]".to_string())
    }

    /// Finds jobs matching the given filter criteria. filter is a JSON string with optional
    /// fields: `state`, `job_type`, `assigned_to`, `category`.
    pub fn find_jobs(&self, filter_str: &str) -> String {
        let filter: JsonValue = serde_json::from_str(filter_str).unwrap_or(JsonValue::Null);
        let state = filter.get("state").and_then(|v| v.as_str());
        let job_type = filter.get("job_type").and_then(|v| v.as_str());
        let assigned_to = filter.get("assigned_to").and_then(|v| v.as_u64());
        let category = filter.get("category").and_then(|v| v.as_str());

        let mut jobs: Vec<JsonValue> = Vec::new();
        if let Some(job_map) = self.components.get("Job") {
            for (&eid, job) in job_map {
                if let Some(s) = state
                    && job.get("state").and_then(|v| v.as_str()) != Some(s)
                {
                    continue;
                }
                if let Some(jt) = job_type
                    && job.get("job_type").and_then(|v| v.as_str()) != Some(jt)
                {
                    continue;
                }
                if let Some(at) = assigned_to
                    && job.get("assigned_to").and_then(|v| v.as_u64()) != Some(at)
                {
                    continue;
                }
                if let Some(cat) = category
                    && job.get("category").and_then(|v| v.as_str()) != Some(cat)
                {
                    continue;
                }
                let mut j = job.clone();
                j["id"] = serde_json::json!(eid);
                jobs.push(j);
            }
        }
        serde_json::to_string(&jobs).unwrap_or_else(|_| "[]".to_string())
    }

    /// Advances a job's state through the simplified state machine:
    /// `pending → going_to_site → at_site → in_progress → complete`.
    /// Emits a `job_progressed` event on each advancement.
    pub fn advance_job_state(&mut self, job_id: u32) -> Result<(), String> {
        let job_str = self
            .get_component(job_id, "Job")
            .ok_or_else(|| format!("No job with id {job_id}"))?;
        let mut job: JsonValue =
            serde_json::from_str(&job_str).map_err(|e| format!("Invalid job JSON: {e}"))?;
        let state = job
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let new_state = match state.as_str() {
            "pending" => "going_to_site",
            "going_to_site" => "at_site",
            "at_site" => "in_progress",
            "in_progress" => {
                let progress = job.get("progress").and_then(|v| v.as_f64()).unwrap_or(0.0);
                job["progress"] = serde_json::json!(progress + 0.1);
                if progress + 0.1 >= 1.0 {
                    "complete"
                } else {
                    "in_progress"
                }
            }
            other => return Err(format!("Cannot advance job from state '{other}'")),
        };
        job["state"] = serde_json::json!(new_state);
        let json_str = serde_json::to_string(&job).map_err(|e| e.to_string())?;
        // Emit job_progressed event
        self.job_event_log.push(WasmJobEvent {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            event_type: "job_progressed".to_string(),
            payload: job.clone(),
        });
        self.set_component(job_id, "Job", &json_str)?;
        Ok(())
    }

    // ---- Job AI API (Module 6) ----

    /// Simplified AI job assignment: scans all unassigned pending jobs and assigns the
    /// highest-priority job to the given agent. Does NOT use engine_core's assign_jobs logic.
    pub fn ai_assign_jobs(&mut self, agent_id: u32) -> Result<(), String> {
        let agent_str = self
            .get_component(agent_id, "Agent")
            .ok_or_else(|| format!("No Agent component on entity {agent_id}"))?;
        let mut agent: JsonValue =
            serde_json::from_str(&agent_str).map_err(|e| format!("Invalid Agent JSON: {e}"))?;

        // Skip if agent already has a current job
        if agent
            .get("current_job")
            .and_then(|v| v.as_u64())
            .is_some_and(|id| id > 0)
        {
            return Ok(());
        }

        // Scan all unassigned pending jobs, pick highest priority
        let mut best_job_id = None;
        let mut best_utility = f64::NEG_INFINITY;

        if let Some(job_map) = self.components.get("Job") {
            for (&eid, job) in job_map {
                let assigned = job.get("assigned_to").and_then(|v| v.as_u64()).unwrap_or(0);
                let state = job.get("state").and_then(|v| v.as_str()).unwrap_or("");
                if assigned == 0 && state == "pending" {
                    let utility = job.get("priority").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    if utility > best_utility {
                        best_utility = utility;
                        best_job_id = Some(eid);
                    }
                }
            }
        }

        if let Some(job_id) = best_job_id {
            // Update Job component: set assigned_to, assignment_count, last_assigned_tick
            let mut job = self
                .get_component(job_id, "Job")
                .and_then(|s| serde_json::from_str::<JsonValue>(&s).ok())
                .unwrap_or_default();
            job["assigned_to"] = serde_json::json!(agent_id);
            job["assignment_count"] = serde_json::json!(
                job.get("assignment_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    + 1
            );
            job["last_assigned_tick"] = serde_json::json!(self.turn);
            let json_str = serde_json::to_string(&job).map_err(|e| e.to_string())?;
            self.set_component(job_id, "Job", &json_str)?;

            // Update Agent component: set current_job
            agent["current_job"] = serde_json::json!(job_id);
            let agent_str = serde_json::to_string(&agent).map_err(|e| e.to_string())?;
            self.set_component(agent_id, "Agent", &agent_str)?;
        }

        Ok(())
    }

    /// Returns a JSON array of jobs assigned to the given agent.
    pub fn ai_query_jobs(&self, agent_id: u32) -> String {
        let mut jobs: Vec<JsonValue> = Vec::new();
        if let Some(job_map) = self.components.get("Job") {
            for (&eid, job) in job_map {
                if job.get("assigned_to").and_then(|v| v.as_u64()) == Some(agent_id as u64) {
                    let mut j = job.clone();
                    j["id"] = serde_json::json!(eid);
                    j["state"] = j.get("state").cloned().unwrap_or(JsonValue::Null);
                    jobs.push(j);
                }
            }
        }
        serde_json::to_string(&jobs).unwrap_or_else(|_| "[]".to_string())
    }

    // ---- Job Events API (Module 7) ----

    /// No-op: deliver callbacks is a no-op in WASM since callbacks don't exist.
    /// Returns true for API parity with Lua/Python.
    pub fn deliver_callbacks(&self) -> bool {
        true
    }

    /// Returns the full job event log as a JSON array.
    pub fn get_job_event_log(&self) -> String {
        serde_json::to_string(&self.job_event_log).unwrap_or_else(|_| "[]".to_string())
    }

    /// Returns events filtered by event type as a JSON array.
    pub fn get_job_events_by_type(&self, event_type: &str) -> String {
        let filtered: Vec<&WasmJobEvent> = self
            .job_event_log
            .iter()
            .filter(|e| e.event_type == event_type)
            .collect();
        serde_json::to_string(&filtered).unwrap_or_else(|_| "[]".to_string())
    }

    /// Returns events with timestamp >= the given tick as a JSON array.
    pub fn get_job_events_since(&self, tick: u32) -> String {
        let filtered: Vec<&WasmJobEvent> = self
            .job_event_log
            .iter()
            .filter(|e| e.timestamp >= tick as u128)
            .collect();
        serde_json::to_string(&filtered).unwrap_or_else(|_| "[]".to_string())
    }

    /// Clears the job event log.
    pub fn clear_job_event_log(&mut self) {
        self.job_event_log.clear();
    }

    fn advance_time_of_day(&mut self) {
        self.time_of_day.minute += 1;
        if self.time_of_day.minute >= 60 {
            self.time_of_day.minute = 0;
            self.time_of_day.hour += 1;
            if self.time_of_day.hour >= 24 {
                self.time_of_day.hour = 0;
                self.time_of_day.day += 1;
            }
        }
    }

    // ---- Unit Template API ----

    /// Load all .json template files from a directory.
    pub fn load_templates_from_dir(&mut self, dir: &str) -> Result<(), String> {
        let path = std::path::Path::new(dir);
        self.template_registry.load_templates_from_dir(path)
    }

    /// Register a single template from a JSON string.
    pub fn register_template_from_json(&mut self, template_json: &str) -> Result<(), String> {
        let template: crate::ecs::template::UnitTemplate = serde_json::from_str(template_json)
            .map_err(|e| format!("Invalid template JSON: {e}"))?;
        self.template_registry.register_template(template);
        Ok(())
    }

    /// Spawn an entity from a named template, with optional overrides JSON.
    pub fn spawn_from_template(
        &mut self,
        template_name: &str,
        overrides_json: Option<&str>,
    ) -> Result<u32, String> {
        let template = self
            .template_registry
            .get_template(template_name)
            .ok_or_else(|| format!("Template '{template_name}' not found"))?
            .clone();

        let overrides: Option<serde_json::Map<String, JsonValue>> = match overrides_json {
            Some(json) => {
                let val: JsonValue = serde_json::from_str(json)
                    .map_err(|e| format!("Failed to parse overrides JSON: {e}"))?;
                match val {
                    JsonValue::Object(map) => Some(map),
                    _ => return Err("Overrides must be a JSON object".to_string()),
                }
            }
            None => None,
        };

        let entity = self.spawn_entity();

        for (comp_name, comp_data) in &template.components {
            let mut final_data = comp_data.clone();

            if let Some(ref ov) = overrides
                && let Some(override_data) = ov.get(comp_name)
            {
                crate::ecs::world::World::deep_merge(&mut final_data, override_data);
            }

            let json_str = serde_json::to_string(&final_data)
                .map_err(|e| format!("Failed to serialize component data: {e}"))?;
            self.set_component(entity, comp_name, &json_str)?;
        }

        Ok(entity)
    }

    /// Get a template definition by name as JSON string.
    pub fn get_template_json(&self, name: &str) -> Option<String> {
        self.template_registry
            .get_template(name)
            .and_then(|t| serde_json::to_string(t).ok())
    }

    /// List all registered template names.
    pub fn list_template_names(&self) -> Vec<String> {
        self.template_registry.list_templates()
    }

    // ---- Designer API ----

    /// Load item definitions from a directory.
    pub fn load_item_definitions_from_dir(&mut self, dir: &str) -> Result<(), String> {
        let path = std::path::Path::new(dir);
        self.item_registry.load_items_from_dir(path)
    }

    /// Register a single item from JSON.
    pub fn register_item_from_json(&mut self, json: &str) -> Result<(), String> {
        let definition: JsonValue =
            serde_json::from_str(json).map_err(|e| format!("Invalid item JSON: {e}"))?;
        self.item_registry.register_item(definition)
    }

    /// Get an item definition by ID as JSON string.
    pub fn get_item_definition_json(&self, id: &str) -> Option<String> {
        self.item_registry
            .get_item(id)
            .and_then(|v| serde_json::to_string(v).ok())
    }

    /// List all registered item IDs.
    pub fn list_item_names(&self) -> Vec<String> {
        self.item_registry.list_items()
    }

    /// Load equipment sets from a directory.
    pub fn load_equipment_sets_from_dir(&mut self, dir: &str) -> Result<(), String> {
        let path = std::path::Path::new(dir);
        self.equipment_set_registry.load_sets_from_dir(path)
    }

    /// Register an equipment set from JSON.
    pub fn register_equipment_set_from_json(&mut self, json: &str) -> Result<(), String> {
        let set: EquipmentSet =
            serde_json::from_str(json).map_err(|e| format!("Invalid equipment set JSON: {e}"))?;
        self.equipment_set_registry.register_set(set);
        Ok(())
    }

    /// Apply an equipment set to an entity.
    pub fn apply_loadout(&mut self, entity: u32, set_name: &str) -> Result<u32, String> {
        let set = self
            .equipment_set_registry
            .get_set(set_name)
            .ok_or_else(|| format!("Equipment set '{set_name}' not found"))?
            .clone();

        // Check entity has Inventory
        if self.get_component(entity, "Inventory").is_none() {
            return Err("Entity has no Inventory component".to_string());
        }

        let mut equipped_slots: Vec<String> = Vec::new();
        let mut spawned_items: Vec<u32> = Vec::new();

        for (slot, item_id) in &set.items {
            // Get item definition from registry
            let definition = match self.item_registry.get_item(item_id) {
                Some(def) => def.clone(),
                None => {
                    self.rollback_loadout(&spawned_items, &equipped_slots, entity);
                    return Err(format!(
                        "Item '{item_id}' not found in registry (required by set '{set_name}')"
                    ));
                }
            };

            // Spawn item entity with Item component
            let item_eid = self.spawn_entity();
            let item_json = serde_json::to_string(&definition)
                .map_err(|e| format!("Failed to serialize item definition: {e}"))?;
            if let Err(e) = self.set_component(item_eid, "Item", &item_json) {
                self.despawn_entity(item_eid);
                self.rollback_loadout(&spawned_items, &equipped_slots, entity);
                return Err(e);
            }
            spawned_items.push(item_eid);

            // Add item_id string to entity's Inventory slots array
            if let Some(inv_str) = self.get_component(entity, "Inventory") {
                let mut new_inv: JsonValue = serde_json::from_str(&inv_str)
                    .map_err(|e| format!("Failed to parse Inventory: {e}"))?;
                if let Some(slots) = new_inv.get_mut("slots").and_then(|v| v.as_array_mut()) {
                    slots.push(JsonValue::String(item_id.clone()));
                }
                let new_inv_json = serde_json::to_string(&new_inv)
                    .map_err(|e| format!("Failed to serialize Inventory: {e}"))?;
                if let Err(e) = self.set_component(entity, "Inventory", &new_inv_json) {
                    self.rollback_loadout(&spawned_items, &equipped_slots, entity);
                    return Err(e);
                }
            }

            // Set Equipment slot
            let mut equipment: JsonValue = self
                .get_component(entity, "Equipment")
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_else(|| serde_json::json!({"slots": {}}));

            if let Some(slots_obj) = equipment.get_mut("slots").and_then(|v| v.as_object_mut()) {
                slots_obj.insert(slot.clone(), JsonValue::String(item_id.clone()));
            }

            let eq_json = serde_json::to_string(&equipment)
                .map_err(|e| format!("Failed to serialize Equipment: {e}"))?;
            if let Err(e) = self.set_component(entity, "Equipment", &eq_json) {
                self.rollback_loadout(&spawned_items, &equipped_slots, entity);
                return Err(e);
            }
            equipped_slots.push(slot.clone());
        }

        Ok(entity)
    }

    /// Rollback a partial loadout: despawn created items and unequip slots.
    fn rollback_loadout(&mut self, spawned_items: &[u32], equipped_slots: &[String], entity: u32) {
        for &item_eid in spawned_items {
            self.despawn_entity(item_eid);
        }

        if let Some(inv_str) = self.get_component(entity, "Inventory")
            && let Ok(mut inv) = serde_json::from_str::<JsonValue>(&inv_str)
        {
            if let Some(slots) = inv.get_mut("slots").and_then(|v| v.as_array_mut()) {
                let remove_count = equipped_slots.len();
                let new_len = slots.len().saturating_sub(remove_count);
                slots.truncate(new_len);
            }
            if let Ok(json) = serde_json::to_string(&inv) {
                let _ = self.set_component(entity, "Inventory", &json);
            }
        }

        if let Some(eq_str) = self.get_component(entity, "Equipment")
            && let Ok(mut equipment) = serde_json::from_str::<JsonValue>(&eq_str)
        {
            if let Some(slots_obj) = equipment.get_mut("slots").and_then(|v| v.as_object_mut()) {
                for slot in equipped_slots {
                    slots_obj.insert(slot.clone(), JsonValue::Null);
                }
            }
            if let Ok(json) = serde_json::to_string(&equipment) {
                let _ = self.set_component(entity, "Equipment", &json);
            }
        }
    }

    /// Get the matching equipment set for an entity as JSON, or None.
    pub fn get_loadout_json(&self, entity: u32) -> Option<String> {
        let equipment = self.get_component(entity, "Equipment")?;
        let equip_val: JsonValue = serde_json::from_str(&equipment).ok()?;
        let entity_slots = equip_val.get("slots")?.as_object()?;

        let entity_items: HashMap<&str, &str> = entity_slots
            .iter()
            .filter_map(|(slot, item_id)| item_id.as_str().map(|id| (slot.as_str(), id)))
            .collect();

        let set_names = self.equipment_set_registry.list_sets();
        for set_name in &set_names {
            if let Some(set) = self.equipment_set_registry.get_set(set_name)
                && set.items.len() == entity_items.len()
                && set.items.iter().all(|(slot, item_id)| {
                    entity_items.get(slot.as_str()) == Some(&item_id.as_str())
                })
            {
                let result = serde_json::json!({
                    "name": set.name,
                    "items": set.items,
                });
                return serde_json::to_string(&result).ok();
            }
        }

        None
    }

    /// Validate an entity's equipment. Returns JSON string of issues.
    pub fn validate_equipment_json(&self, entity: u32) -> String {
        let issues = self.validate_equipment(entity);
        serde_json::to_string(&serde_json::json!({
            "issues": issues.iter().map(|i| {
                serde_json::json!({
                    "slot": i.slot,
                    "item_id": i.item_id,
                    "reason": i.reason,
                })
            }).collect::<Vec<_>>(),
        }))
        .unwrap_or_else(|_| r#"{"issues":[]}"#.to_string())
    }

    /// Validate equipment for an entity (core logic).
    pub fn validate_equipment(&self, entity: u32) -> Vec<EquipmentIssue> {
        let mut issues = Vec::new();

        let equipment = match self.get_component(entity, "Equipment") {
            Some(e) => e,
            None => return issues,
        };

        let equip_val: JsonValue = match serde_json::from_str(&equipment) {
            Ok(v) => v,
            Err(_) => return issues,
        };

        let slots = match equip_val.get("slots").and_then(|v| v.as_object()) {
            Some(s) => s,
            None => return issues,
        };

        for (slot, item_id_val) in slots {
            let item_id = match item_id_val.as_str() {
                Some(id) => id,
                None => continue,
            };

            // Check if item is registered
            if self.item_registry.get_item(item_id).is_none() {
                issues.push(EquipmentIssue {
                    slot: slot.clone(),
                    item_id: item_id.to_string(),
                    reason: "item_not_registered".to_string(),
                });
            }
        }

        issues
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

/// Convert a `map.json`-shaped JSON string to a [`WasmMap`].
///
/// Mirrors `Map::from_json` semantics for the WASM bridge: `topology` selects
/// the cell coordinate keys (`x`/`y`/`z` for square, `q`/`r`/`z` for hex,
/// `id` for province). Explicit per-cell `neighbors` are carried into the
/// adjacency map; when absent, square/hex adjacency is inferred from the cell
/// set (4-way / 6-way) exactly as the core deserializer does, so a registered
/// map behaves identically across all three bridges. Province neighbors must
/// be explicit.
fn wasm_map_from_map_json(map_json: &str) -> Result<WasmMap, String> {
    let value: JsonValue =
        serde_json::from_str(map_json).map_err(|e| format!("Failed to parse map JSON: {e}"))?;
    let topology = value
        .get("topology")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Map JSON missing 'topology'".to_string())?;
    let cells = value
        .get("cells")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Map JSON missing 'cells' array".to_string())?;

    let mut map = WasmMap {
        topology_type: topology.to_string(),
        ..WasmMap::default()
    };

    let cell_from_json = |cell: &JsonValue| -> Result<CellKey, String> {
        match topology {
            "square" => Ok(CellKey::Square {
                x: cell.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                y: cell.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                z: cell.get("z").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            }),
            "hex" => Ok(CellKey::Hex {
                q: cell.get("q").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                r: cell.get("r").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                z: cell.get("z").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
            }),
            "province" => Ok(CellKey::Province {
                id: cell
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Province cell missing 'id'".to_string())?
                    .to_string(),
            }),
            other => Err(format!("Unknown topology '{other}'")),
        }
    };

    // First pass: add all cells so neighbor inference can reference the set.
    for cell in cells {
        let key = cell_from_json(cell)?;
        if !map.cells.contains(&key) {
            map.cells.push(key);
        }
    }

    // Second pass: explicit neighbors, inferred adjacency, and metadata.
    for cell in cells {
        let key = cell_from_json(cell)?;
        let key_str = serde_json::to_string(&key).unwrap();

        if let Some(neighs) = cell.get("neighbors").and_then(|v| v.as_array()) {
            let mut neighbor_keys = Vec::new();
            for n in neighs {
                let nkey = cell_from_json(n)?;
                let nkey_str = serde_json::to_string(&nkey).unwrap();
                if !neighbor_keys.contains(&nkey_str) {
                    neighbor_keys.push(nkey_str);
                }
            }
            map.neighbors.insert(key_str.clone(), neighbor_keys);
        } else {
            let candidates: Vec<CellKey> = match topology {
                "square" => match key {
                    CellKey::Square { x, y, z } => [
                        CellKey::Square { x: x + 1, y, z },
                        CellKey::Square { x: x - 1, y, z },
                        CellKey::Square { x, y: y + 1, z },
                        CellKey::Square { x, y: y - 1, z },
                    ]
                    .to_vec(),
                    _ => Vec::new(),
                },
                "hex" => match key {
                    CellKey::Hex { q, r, z } => [
                        CellKey::Hex { q: q + 1, r, z },
                        CellKey::Hex { q: q - 1, r, z },
                        CellKey::Hex { q, r: r + 1, z },
                        CellKey::Hex { q, r: r - 1, z },
                        CellKey::Hex {
                            q: q + 1,
                            r: r - 1,
                            z,
                        },
                        CellKey::Hex {
                            q: q - 1,
                            r: r + 1,
                            z,
                        },
                    ]
                    .to_vec(),
                    _ => Vec::new(),
                },
                // Province adjacency must be explicit.
                _ => Vec::new(),
            };
            let mut neighbor_keys = Vec::new();
            for candidate in candidates {
                if map.cells.contains(&candidate) {
                    let nkey_str = serde_json::to_string(&candidate).unwrap();
                    if !neighbor_keys.contains(&nkey_str) {
                        neighbor_keys.push(nkey_str);
                    }
                }
            }
            if !neighbor_keys.is_empty() {
                map.neighbors.insert(key_str.clone(), neighbor_keys);
            }
        }

        if let Some(meta) = cell.get("metadata") {
            map.cell_metadata.insert(key_str, meta.clone());
        }
    }

    Ok(map)
}
