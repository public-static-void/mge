use crate::ecs::world::World;
use serde_json::Value;
use std::collections::HashMap;

pub type EffectHandler = Box<dyn Fn(&mut World, u32, &Value) + Send + Sync>;

#[derive(Default)]
pub struct EffectProcessorRegistry {
    handlers: HashMap<String, EffectHandler>,
}

impl EffectProcessorRegistry {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    pub fn register_handler<F>(&mut self, action: &str, handler: F)
    where
        F: Fn(&mut World, u32, &Value) + Send + Sync + 'static,
    {
        self.handlers.insert(action.to_string(), Box::new(handler));
    }

    pub fn process_effects(&mut self, world: &mut World, eid: u32, effects: &[Value]) {
        for effect in effects {
            if let Some(action) = effect.get("action").and_then(|v| v.as_str()) {
                if let Some(handler) = self.handlers.get(action) {
                    handler(world, eid, effect);
                }
            }
        }
    }
}
