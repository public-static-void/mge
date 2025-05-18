use crate::ecs::event::EventBus;
use crate::ecs::registry::ComponentRegistry;
use crate::ecs::system::SystemRegistry;
use crate::map::Map;
use crate::plugins::dynamic_systems::DynamicSystemRegistry;
use crate::scripting::ScriptEngine;
use crate::systems::job::JobTypeRegistry;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
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
    pub event_buses: HashMap<String, Arc<Mutex<EventBus<JsonValue>>>>,
    #[serde(skip)]
    pub dynamic_systems: DynamicSystemRegistry,
    #[serde(skip)]
    pub lua_engine: Option<ScriptEngine>,
    #[serde(skip)]
    pub job_types: JobTypeRegistry,
    #[serde(skip)]
    pub map: Option<Map>,
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
            event_buses: HashMap::new(),
            dynamic_systems: DynamicSystemRegistry::new(),
            lua_engine: None,
            job_types: JobTypeRegistry::default(),
            map: None,
        }
    }
}

impl Default for World {
    fn default() -> Self {
        panic!("World::default() is not supported. Use World::new(registry) instead.");
    }
}
