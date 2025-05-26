use crate::ecs::registry::ComponentRegistry;
use crate::ecs::system::SystemRegistry;
use crate::map::Map;
use crate::plugins::dynamic_systems::DynamicSystemRegistry;
use crate::scripting::ScriptEngine;
use crate::systems::job::JobTypeRegistry;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
mod component;
mod entity;
mod events;
mod misc;
mod resources;
mod save_load;
mod systems;

#[derive(Serialize, Deserialize)]
pub struct World {
    pub entities: Vec<u32>,
    pub components: HashMap<String, HashMap<u32, JsonValue>>,
    next_id: u32,
    pub current_mode: String,
    pub turn: u32,
    #[serde(skip)]
    pub registry: Arc<Mutex<ComponentRegistry>>,
    #[serde(skip)]
    pub systems: SystemRegistry,
    #[serde(skip)]
    pub event_buses: crate::ecs::event_bus_registry::EventBusRegistry,
    #[serde(skip)]
    pub dynamic_systems: DynamicSystemRegistry,
    #[serde(skip)]
    pub lua_engine: Option<ScriptEngine>,
    #[serde(skip)]
    pub job_types: JobTypeRegistry,
    #[serde(skip)]
    pub map: Option<Map>,
    event_queues: HashMap<String, (VecDeque<JsonValue>, VecDeque<JsonValue>)>, // (write, read)
}

impl World {
    pub fn new(registry: Arc<Mutex<ComponentRegistry>>) -> Self {
        World {
            entities: Vec::new(),
            components: HashMap::new(),
            next_id: 1,
            current_mode: "colony".to_string(),
            turn: 0,
            registry,
            systems: SystemRegistry::new(),
            event_buses: crate::ecs::event_bus_registry::EventBusRegistry::new(),
            dynamic_systems: DynamicSystemRegistry::new(),
            lua_engine: None,
            job_types: JobTypeRegistry::default(),
            map: None,
            event_queues: HashMap::new(),
        }
    }

    // --- Cell metadata API ---
    pub fn set_cell_metadata(&mut self, cell: &crate::map::CellKey, data: serde_json::Value) {
        if let Some(map) = &mut self.map {
            map.set_cell_metadata(cell, data);
        }
    }

    pub fn get_cell_metadata(&self, cell: &crate::map::CellKey) -> Option<&serde_json::Value> {
        self.map.as_ref().and_then(|m| m.get_cell_metadata(cell))
    }
}

impl Default for World {
    fn default() -> Self {
        panic!("World::default() is not supported. Use World::new(registry) instead.");
    }
}
