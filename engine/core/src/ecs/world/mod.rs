//! ECS world module for the Modular Game Engine.
//!
//! Defines the World struct, which holds all entities, components, systems, and loaded assets.

use crate::ecs::equipment_set::EquipmentSetRegistry;
use crate::ecs::item::ItemRegistry;
use crate::ecs::registry::ComponentRegistry;
use crate::ecs::system::SystemRegistry;
use crate::ecs::template::UnitTemplateRegistry;
use crate::loot::LootTableRegistry;
use crate::map::Map;
use crate::map::cell_key::CellKey;
use crate::map::fov::{BfsFovAlgorithm, FovAlgorithm, RecursiveShadowcasting};
use crate::plugins::dynamic_systems::DynamicSystemRegistry;
use crate::systems::job::{JobBoard, JobTypeRegistry};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex};

/// Job handler modules
pub mod job_handlers;
/// Season enum for time-of-day season cycle
pub mod season;
/// Wasm exports
pub mod wasm;

pub use season::Season;

mod component;
mod entity;
mod events;
pub mod loadout;
mod map;
mod mode;
mod resources;
mod save_load;
mod systems;
mod template;

/// Map postprocessor function
pub type MapPostprocessor = Arc<dyn Fn(&mut World) -> Result<(), String> + Send + Sync>;

/// Map validator function
pub type MapValidator = Arc<dyn Fn(&serde_json::Value) -> Result<(), String> + Send + Sync>;

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

/// The ECS world, which holds all entities, components, systems, and loaded assets.
#[derive(Serialize, Deserialize)]
pub struct World {
    /// List of all entity IDs in the world.
    pub entities: Vec<u32>,
    /// Map from component name to a map of entity IDs to component data.
    pub components: HashMap<String, HashMap<u32, JsonValue>>,
    next_id: u32,
    /// Current game mode.
    pub current_mode: String,
    /// Current turn number.
    pub turn: u32,
    /// Current time of day.
    pub time_of_day: TimeOfDay,
    /// Component registry
    #[serde(skip)]
    pub registry: Arc<Mutex<ComponentRegistry>>,
    /// System registry
    #[serde(skip)]
    pub systems: SystemRegistry,
    /// Event bus registry
    #[serde(skip)]
    pub event_buses: crate::ecs::event_bus_registry::EventBusRegistry,
    /// Dynamic system registry
    #[serde(skip)]
    pub dynamic_systems: DynamicSystemRegistry,
    /// Job type registry
    #[serde(skip)]
    pub job_types: JobTypeRegistry,
    /// Loot table registry (runtime-defined, not serialized)
    #[serde(skip)]
    pub loot_tables: LootTableRegistry,
    /// Job handler registry
    #[serde(skip)]
    pub job_handler_registry:
        Arc<Mutex<crate::systems::job::job_handler_registry::JobHandlerRegistry>>,
    /// Effect processor registry
    #[serde(skip)]
    pub effect_processor_registry: Option<
        std::sync::Arc<
            std::sync::Mutex<
                crate::systems::job::effect_processor_registry::EffectProcessorRegistry,
            >,
        >,
    >,
    /// Map
    #[serde(skip)]
    pub map: Option<Map>,
    /// Named map registry (runtime-only, not serialized). `World.map` remains the active map.
    #[serde(skip)]
    pub maps: HashMap<String, Map>,
    /// Name of the map currently held in `World.map`.
    #[serde(skip)]
    pub active_map: String,
    /// Region id -> local map name (transition anchor, runtime-only).
    #[serde(skip)]
    pub region_map_links: HashMap<String, String>,
    /// Region id -> entry cell on the linked local map (runtime-only).
    #[serde(skip)]
    pub region_entry_cells: HashMap<String, CellKey>,
    /// Source map name -> link to target map (scale-generic transitions, runtime-only).
    #[serde(skip)]
    pub map_links: HashMap<String, MapLink>,
    /// Active-map history for `exit_map()` (runtime-only).
    #[serde(skip)]
    pub map_stack: Vec<String>,
    /// Visible cells per entity (transient FOV state, not serialized)
    #[serde(skip)]
    pub visible_cells: HashMap<u32, HashSet<CellKey>>,
    /// Explored cells per entity (persistent fog-of-war state, serialized for save/load).
    /// Old saves without this field deserialize as empty (backward compatible).
    #[serde(default)]
    pub explored_cells: HashMap<u32, HashSet<CellKey>>,
    event_queues: HashMap<String, (VecDeque<JsonValue>, VecDeque<JsonValue>)>, // (write, read)
    /// Map postprocessors
    #[serde(skip)]
    pub map_postprocessors: Vec<MapPostprocessor>,
    /// Map validators
    #[serde(skip)]
    pub map_validators: Vec<MapValidator>,
    /// AI event intents
    #[serde(skip)]
    pub ai_event_intents: VecDeque<JsonValue>,

    // --- Asset/data fields ---
    /// Map from resource kind to resource definition (loaded from assets/resources).
    #[serde(skip)]
    pub resource_definitions: HashMap<String, JsonValue>,
    /// Map from material name to material definition (loaded from assets/materials).
    #[serde(skip)]
    pub material_definitions: HashMap<String, JsonValue>,
    /// Map from recipe name to recipe definition (loaded from assets/recipes).
    #[serde(skip)]
    pub recipes: HashMap<String, JsonValue>,
    /// Map from job name to job definition (loaded from assets/jobs).
    #[serde(skip)]
    pub jobs: HashMap<String, JsonValue>,
    /// Job board
    #[serde(skip)]
    pub job_board: JobBoard,

    /// Active FOV algorithm used by the FOV update system.
    #[serde(skip, default = "default_fov_algorithm")]
    pub fov_algorithm: Box<dyn FovAlgorithm>,

    /// Registered FOV algorithm implementations (name → instance).
    #[serde(skip, default = "default_fov_algorithms")]
    pub fov_algorithms: HashMap<String, Box<dyn FovAlgorithm>>,

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

/// A scale-generic link from a source map to a target map.
///
/// The source cell and target cell are topology-generic [`CellKey`]s, so a link
/// may connect any two topologies (square/hex/province in any combination).
/// Mapping is defined by explicit link entries, not geometric projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapLink {
    /// Name of the target map.
    pub target_map: String,
    /// Cell on the source map that anchors the link.
    pub source_cell: CellKey,
    /// Cell on the target map that anchors the link.
    pub target_cell: CellKey,
}

/// Default FOV algorithm factory (used by serde `#[serde(skip, default)]`).
fn default_fov_algorithm() -> Box<dyn FovAlgorithm> {
    Box::new(RecursiveShadowcasting)
}

/// Default FOV algorithm registry (used by serde `#[serde(skip, default)]`).
fn default_fov_algorithms() -> HashMap<String, Box<dyn FovAlgorithm>> {
    let mut m: HashMap<String, Box<dyn FovAlgorithm>> = HashMap::new();
    m.insert(
        "recursive_shadowcasting".to_string(),
        Box::new(RecursiveShadowcasting),
    );
    m.insert("bfs_flood_fill".to_string(), Box::new(BfsFovAlgorithm));
    m
}

impl World {
    /// Creates a new World with the given component registry.
    pub fn new(registry: Arc<Mutex<ComponentRegistry>>) -> Self {
        World {
            entities: Vec::new(),
            components: HashMap::new(),
            next_id: 1,
            current_mode: "colony".to_string(),
            turn: 0,
            time_of_day: TimeOfDay::default(),
            registry,
            systems: SystemRegistry::new(),
            event_buses: crate::ecs::event_bus_registry::EventBusRegistry::new(),
            dynamic_systems: DynamicSystemRegistry::new(),
            job_types: JobTypeRegistry::default(),
            loot_tables: LootTableRegistry::new(),
            job_handler_registry: Arc::new(Mutex::new(
                crate::systems::job::job_handler_registry::JobHandlerRegistry::new(),
            )),
            effect_processor_registry: Some(std::sync::Arc::new(std::sync::Mutex::new(
                crate::systems::job::effect_processor_registry::EffectProcessorRegistry::new(),
            ))),
            map: None,
            maps: HashMap::new(),
            active_map: String::new(),
            region_map_links: HashMap::new(),
            region_entry_cells: HashMap::new(),
            map_links: HashMap::new(),
            map_stack: Vec::new(),
            visible_cells: HashMap::new(),
            explored_cells: HashMap::new(),
            event_queues: HashMap::new(),
            map_postprocessors: Vec::new(),
            map_validators: Vec::new(),
            ai_event_intents: VecDeque::new(),
            // --- Asset/data fields ---
            resource_definitions: HashMap::new(),
            material_definitions: HashMap::new(),
            recipes: HashMap::new(),
            jobs: HashMap::new(),
            job_board: JobBoard::default(),
            fov_algorithm: Box::new(RecursiveShadowcasting),
            fov_algorithms: {
                let mut m: HashMap<String, Box<dyn FovAlgorithm>> = HashMap::new();
                m.insert(
                    "recursive_shadowcasting".to_string(),
                    Box::new(RecursiveShadowcasting),
                );
                m.insert("bfs_flood_fill".to_string(), Box::new(BfsFovAlgorithm));
                m
            },
            template_registry: UnitTemplateRegistry::new(),
            item_registry: ItemRegistry::new(),
            equipment_set_registry: EquipmentSetRegistry::new(),
        }
    }
}

impl World {
    /// Get the visible cells for an entity, if computed.
    pub fn get_visible_cells(&self, entity: u32) -> Option<&HashSet<CellKey>> {
        self.visible_cells.get(&entity)
    }

    /// Set the visible cells for an entity.
    pub fn set_visible_cells(&mut self, entity: u32, cells: HashSet<CellKey>) {
        self.visible_cells.insert(entity, cells);
    }

    /// Get the explored cells for an entity, if any.
    pub fn get_explored_cells(&self, entity: u32) -> Option<&HashSet<CellKey>> {
        self.explored_cells.get(&entity)
    }

    /// Set the explored cells for an entity.
    pub fn set_explored_cells(&mut self, entity: u32, cells: HashSet<CellKey>) {
        self.explored_cells.insert(entity, cells);
    }

    /// Reset (clear) fog-of-war for a single entity.
    pub fn reset_fog(&mut self, entity: u32) {
        self.explored_cells.remove(&entity);
    }

    /// Reset (clear) fog-of-war for all entities.
    pub fn reset_all_fog(&mut self) {
        self.explored_cells.clear();
    }

    /// Determine the visibility state of a cell for an entity.
    /// Returns 0 = UNEXPLORED, 1 = EXPLORED (seen before but not currently visible), 2 = VISIBLE.
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

    /// Return a reference to the active FOV algorithm.
    pub fn fov_algorithm(&self) -> &dyn FovAlgorithm {
        self.fov_algorithm.as_ref()
    }

    /// Replace the active FOV algorithm.
    pub fn set_fov_algorithm(&mut self, algo: Box<dyn FovAlgorithm>) {
        self.fov_algorithm = algo;
    }

    /// Register an FOV algorithm under a named key.
    ///
    /// Panics if the name is already registered.
    pub fn register_fov_algorithm(&mut self, name: &str, algo: Box<dyn FovAlgorithm>) {
        let old = self.fov_algorithms.insert(name.to_string(), algo);
        assert!(
            old.is_none(),
            "FOV algorithm '{name}' is already registered"
        );
    }

    /// Set the active FOV algorithm by looking up its name in the registry.
    ///
    /// Only algorithms whose type can be reconstructed (known built-in names)
    /// are supported via this method. Custom algorithms must be set directly
    /// via [`set_fov_algorithm`](Self::set_fov_algorithm).
    pub fn set_fov_algorithm_by_name(&mut self, name: &str) -> Result<(), String> {
        if !self.fov_algorithms.contains_key(name) {
            return Err(format!("FOV algorithm '{name}' is not registered"));
        }
        match name {
            "recursive_shadowcasting" => {
                self.fov_algorithm = Box::new(RecursiveShadowcasting);
                Ok(())
            }
            "bfs_flood_fill" => {
                self.fov_algorithm = Box::new(BfsFovAlgorithm);
                Ok(())
            }
            _ => Err(format!(
                "FOV algorithm '{name}' is registered but cannot be dynamically \
                 constructed. Use set_fov_algorithm() directly."
            )),
        }
    }
}

impl World {
    /// Load item definitions from a directory into the item registry.
    pub fn load_item_definitions(&mut self, dir: &std::path::Path) -> Result<(), String> {
        self.item_registry.load_items_from_dir(dir)
    }

    /// Get an item definition by ID from the item registry.
    pub fn get_item_definition(&self, id: &str) -> Option<&JsonValue> {
        self.item_registry.get_item(id)
    }

    /// Load equipment sets from a directory into the equipment set registry.
    pub fn load_equipment_sets(&mut self, dir: &std::path::Path) -> Result<(), String> {
        self.equipment_set_registry.load_sets_from_dir(dir)
    }

    /// Get an equipment set by name from the equipment set registry.
    pub fn get_equipment_set(&self, name: &str) -> Option<&crate::ecs::EquipmentSet> {
        self.equipment_set_registry.get_set(name)
    }
}

impl Default for World {
    fn default() -> Self {
        panic!("World::default() is not supported. Use World::new(registry) instead.");
    }
}
