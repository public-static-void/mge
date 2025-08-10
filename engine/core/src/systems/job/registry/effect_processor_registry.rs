use crate::ecs::world::World;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Effect handler
pub type EffectHandler = dyn Fn(&mut World, u32, &Value) + Send + Sync;

/// Effect processor registry
#[derive(Default)]
pub struct EffectProcessorRegistry {
    handlers: HashMap<String, Arc<EffectHandler>>,
    undo_handlers: HashMap<String, Arc<EffectHandler>>,
}

impl EffectProcessorRegistry {
    /// Create a new effect processor registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            undo_handlers: HashMap::new(),
        }
    }

    /// Register a handler
    pub fn register_handler<F>(&mut self, action: &str, handler: F)
    where
        F: Fn(&mut World, u32, &Value) + Send + Sync + 'static,
    {
        self.handlers.insert(action.to_string(), Arc::new(handler));
    }

    /// Register an undo handler
    pub fn register_undo_handler<F>(&mut self, action: &str, handler: F)
    where
        F: Fn(&mut World, u32, &Value) + Send + Sync + 'static,
    {
        self.undo_handlers
            .insert(action.to_string(), Arc::new(handler));
    }

    /// Process effects
    /// Deadlock-free, recursive effect processing for Arc<Mutex<EffectProcessorRegistry>>
    pub fn process_effects_arc(
        effect_proc: &Arc<Mutex<EffectProcessorRegistry>>,
        world: &mut World,
        eid: u32,
        effects: &[Value],
    ) {
        // Collect handlers to call after releasing the lock
        let to_call: Vec<_> = {
            let registry = effect_proc.lock().unwrap();
            effects
                .iter()
                .filter_map(|effect| {
                    effect
                        .get("action")
                        .and_then(|v| v.as_str())
                        .and_then(|action| {
                            registry
                                .handlers
                                .get(action)
                                .map(|handler| (Arc::clone(handler), effect.clone()))
                        })
                })
                .collect()
        };
        // Call handlers outside of the lock
        for (handler, effect) in to_call {
            handler(world, eid, &effect);
        }
    }

    /// Rollback effects
    /// Deadlock-free, recursive effect processing for Arc<Mutex<EffectProcessorRegistry>>
    pub fn rollback_effects_arc(
        effect_proc: &Arc<Mutex<EffectProcessorRegistry>>,
        world: &mut World,
        eid: u32,
        effects: &[Value],
    ) {
        let to_call: Vec<_> = {
            let registry = effect_proc.lock().unwrap();
            effects
                .iter()
                .filter_map(|effect| {
                    effect
                        .get("action")
                        .and_then(|v| v.as_str())
                        .and_then(|action| {
                            let undo_action = format!("Undo{action}");
                            registry
                                .undo_handlers
                                .get(&undo_action)
                                .map(|handler| (Arc::clone(handler), effect.clone()))
                        })
                })
                .collect()
        };
        for (handler, effect) in to_call {
            handler(world, eid, &effect);
        }
    }

    /// Process effects
    /// Non-Arc version for single-threaded or direct use
    pub fn process_effects(&mut self, world: &mut World, eid: u32, effects: &[Value]) {
        let to_call: Vec<_> = effects
            .iter()
            .filter_map(|effect| {
                effect
                    .get("action")
                    .and_then(|v| v.as_str())
                    .and_then(|action| {
                        self.handlers
                            .get(action)
                            .map(|handler| (Arc::clone(handler), effect.clone()))
                    })
            })
            .collect();
        for (handler, effect) in to_call {
            handler(world, eid, &effect);
        }
    }

    /// Rollback effects
    /// Non-Arc version for single-threaded or direct use
    pub fn rollback_effects(&mut self, world: &mut World, eid: u32, effects: &[Value]) {
        let to_call: Vec<_> = effects
            .iter()
            .filter_map(|effect| {
                effect
                    .get("action")
                    .and_then(|v| v.as_str())
                    .and_then(|action| {
                        let undo_action = format!("Undo{action}");
                        self.undo_handlers
                            .get(&undo_action)
                            .map(|handler| (Arc::clone(handler), effect.clone()))
                    })
            })
            .collect();
        for (handler, effect) in to_call {
            handler(world, eid, &effect);
        }
    }
}
