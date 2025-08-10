//! ECS world module for the Modular Game Engine.
//!
//! Defines the World struct, which holds all entities, components, systems, and loaded assets.

use crate::ecs::registry::ComponentRegistry;
use crate::ecs::system::SystemRegistry;
use crate::map::Map;
use crate::plugins::dynamic_systems::DynamicSystemRegistry;
use crate::systems::job::{JobBoard, JobTypeRegistry};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// Job handler modules
pub mod job_handlers;
/// Wasm exports
pub mod wasm;

mod component;
mod entity;
mod events;
mod map;
mod mode;
mod resources;
mod save_load;
mod systems;

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
    /// Map from recipe name to recipe definition (loaded from assets/recipes).
    #[serde(skip)]
    pub recipes: HashMap<String, JsonValue>,
    /// Map from job name to job definition (loaded from assets/jobs).
    #[serde(skip)]
    pub jobs: HashMap<String, JsonValue>,
    /// Job board
    #[serde(skip)]
    pub job_board: JobBoard,
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
            job_handler_registry: Arc::new(Mutex::new(
                crate::systems::job::job_handler_registry::JobHandlerRegistry::new(),
            )),
            effect_processor_registry: Some(std::sync::Arc::new(std::sync::Mutex::new(
                crate::systems::job::effect_processor_registry::EffectProcessorRegistry::new(),
            ))),
            map: None,
            event_queues: HashMap::new(),
            map_postprocessors: Vec::new(),
            map_validators: Vec::new(),
            ai_event_intents: VecDeque::new(),
            // --- Asset/data fields ---
            resource_definitions: HashMap::new(),
            recipes: HashMap::new(),
            jobs: HashMap::new(),
            job_board: JobBoard::default(),
        }
    }
}

impl Default for World {
    fn default() -> Self {
        panic!("World::default() is not supported. Use World::new(registry) instead.");
    }
}
