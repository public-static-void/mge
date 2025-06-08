use crate::ecs::registry::ComponentRegistry;
use crate::ecs::system::SystemRegistry;
use crate::map::Map;
use crate::plugins::dynamic_systems::DynamicSystemRegistry;
use crate::systems::job::JobTypeRegistry;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

mod component;
mod entity;
mod events;
mod map;
mod mode;
mod resources;
mod save_load;
mod systems;
pub mod wasm;

pub type MapPostprocessor = Arc<dyn Fn(&mut World) -> Result<(), String> + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TimeOfDay {
    pub hour: u8,
    pub minute: u8,
}

#[derive(Serialize, Deserialize)]
pub struct World {
    pub entities: Vec<u32>,
    pub components: HashMap<String, HashMap<u32, JsonValue>>,
    next_id: u32,
    pub current_mode: String,
    pub turn: u32,
    pub time_of_day: TimeOfDay,
    #[serde(skip)]
    pub registry: Arc<Mutex<ComponentRegistry>>,
    #[serde(skip)]
    pub systems: SystemRegistry,
    #[serde(skip)]
    pub event_buses: crate::ecs::event_bus_registry::EventBusRegistry,
    #[serde(skip)]
    pub dynamic_systems: DynamicSystemRegistry,
    #[serde(skip)]
    pub job_types: JobTypeRegistry,
    #[serde(skip)]
    pub job_handler_registry: crate::systems::job::job_handler_registry::JobHandlerRegistry,
    #[serde(skip)]
    pub effect_processor_registry:
        Option<crate::systems::job::effect_processor_registry::EffectProcessorRegistry>,
    #[serde(skip)]
    pub map: Option<Map>,
    event_queues: HashMap<String, (VecDeque<JsonValue>, VecDeque<JsonValue>)>, // (write, read)
    #[serde(skip)]
    pub map_postprocessors: Vec<MapPostprocessor>,
    #[serde(skip)]
    pub ai_event_intents: VecDeque<JsonValue>,
}

impl World {
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
            job_handler_registry:
                crate::systems::job::job_handler_registry::JobHandlerRegistry::new(),
            effect_processor_registry: Some(
                crate::systems::job::effect_processor_registry::EffectProcessorRegistry::new(),
            ),
            map: None,
            event_queues: HashMap::new(),
            map_postprocessors: Vec::new(),
            ai_event_intents: VecDeque::new(),
        }
    }
}

impl Default for World {
    fn default() -> Self {
        panic!("World::default() is not supported. Use World::new(registry) instead.");
    }
}
