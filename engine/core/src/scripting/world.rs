/// The main ECS world. Use this to register and run systems.
///
/// # Example
/// ```ignore
/// # use engine_core::scripting::world::World;
/// # use engine_core::ecs::system::System;
/// # use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
/// # struct MySystem;
/// # impl System for MySystem {
/// #     fn name(&self) -> &'static str { "MySystem" }
/// #     fn run(&mut self, _world: &mut World) {}
/// # }
/// let registry = Arc::new(ComponentRegistry::new());
/// let mut world = World::new(registry);
/// world.register_system(MySystem);
/// world.run_system("MySystem").unwrap();
/// ```
use crate::ecs::event::EventBus;
use crate::ecs::registry::ComponentRegistry;
use crate::ecs::system::SystemRegistry;
use jsonschema::{Draft, JSONSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize)]
pub struct World {
    pub entities: Vec<u32>,
    pub components: HashMap<String, HashMap<u32, JsonValue>>,
    next_id: u32,
    pub current_mode: String,
    pub turn: u32,
    #[serde(skip)]
    pub registry: Arc<ComponentRegistry>,
    #[serde(skip)]
    pub systems: SystemRegistry,
    #[serde(skip)]
    pub event_buses: HashMap<String, Arc<Mutex<EventBus<JsonValue>>>>,
}

impl World {
    pub fn new(registry: Arc<ComponentRegistry>) -> Self {
        World {
            entities: Vec::new(),
            components: HashMap::new(),
            next_id: 1,
            current_mode: "colony".to_string(),
            turn: 0,
            registry,
            systems: SystemRegistry::new(),
            event_buses: HashMap::new(),
        }
    }

    pub fn spawn_entity(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push(id);
        id
    }

    pub fn is_component_allowed_in_mode(&self, component: &str, mode: &str) -> bool {
        if let Some(schema) = self.registry.get_schema_by_name(component) {
            schema.modes.contains(&mode.to_string())
        } else {
            false
        }
    }

    // Generic set_component
    pub fn set_component(
        &mut self,
        entity: u32,
        name: &str,
        value: JsonValue,
    ) -> Result<(), String> {
        if !self.is_component_allowed_in_mode(name, &self.current_mode) {
            return Err(format!(
                "Component {} not allowed in mode {}",
                name, self.current_mode
            ));
        }

        if let Some(schema) = self.registry.get_schema_by_name(name) {
            let compiled = JSONSchema::options()
                .with_draft(Draft::Draft7)
                .compile(&serde_json::to_value(&schema.schema).unwrap())
                .map_err(|e| format!("Schema compile error: {e}"))?;
            let result = compiled.validate(&value);
            if let Err(errors) = result {
                let msg = errors.map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
                return Err(format!("Schema validation failed: {msg}"));
            }
        }

        self.components
            .entry(name.to_string())
            .or_default()
            .insert(entity, value);
        Ok(())
    }

    // Generic get_component
    pub fn get_component(&self, entity: u32, name: &str) -> Option<&JsonValue> {
        self.components.get(name)?.get(&entity)
    }

    pub fn get_entities(&self) -> Vec<u32> {
        self.entities.clone()
    }

    /// Return all entity IDs that have *all* of the given components.
    pub fn get_entities_with_components(&self, names: &[&str]) -> Vec<u32> {
        // If no names given, return all entities
        if names.is_empty() {
            return self.entities.clone();
        }
        // For each component, get the set of entity IDs that have it
        let mut sets: Vec<std::collections::HashSet<u32>> = names
            .iter()
            .filter_map(|name| self.components.get(*name))
            .map(|comps| comps.keys().cloned().collect())
            .collect();
        if sets.is_empty() {
            return vec![];
        }
        // Intersect all sets to get entities that have all components
        let first = sets.pop().unwrap();
        sets.into_iter()
            .fold(first, |acc, set| acc.intersection(&set).cloned().collect())
            .into_iter()
            .collect()
    }

    pub fn move_all(&mut self, dx: f32, dy: f32) {
        if let Some(positions) = self.components.get_mut("Position") {
            for (_entity, value) in positions.iter_mut() {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(x) = obj.get_mut("x") {
                        if let Some(x_val) = x.as_f64() {
                            *x = serde_json::json!(x_val + dx as f64);
                        }
                    }
                    if let Some(y) = obj.get_mut("y") {
                        if let Some(y_val) = y.as_f64() {
                            *y = serde_json::json!(y_val + dy as f64);
                        }
                    }
                }
            }
        }
    }

    pub fn print_positions(&self) {
        if let Some(positions) = self.components.get("Position") {
            for (entity, value) in positions {
                println!("Entity {}: {:?}", entity, value);
            }
        } else {
            println!("No Position components found.");
        }
    }

    pub fn damage_all(&mut self, amount: f32) {
        if let Some(healths) = self.components.get_mut("Health") {
            for (_entity, value) in healths.iter_mut() {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(current) = obj.get_mut("current") {
                        if let Some(cur_val) = current.as_f64() {
                            let new_val = (cur_val - amount as f64).max(0.0);
                            *current = serde_json::json!(new_val);
                        }
                    }
                }
            }
        }
    }

    pub fn print_healths(&self) {
        if let Some(healths) = self.components.get("Health") {
            for (entity, value) in healths {
                println!("Entity {}: {:?}", entity, value);
            }
        } else {
            println!("No Health components found.");
        }
    }

    pub fn tick(&mut self) {
        // Example: move all entities by (1, 0) and damage all by 1
        self.move_all(1.0, 0.0);
        self.damage_all(1.0);
        self.process_deaths();
        self.process_decay();
        self.turn += 1;
    }

    pub fn process_deaths(&mut self) {
        let mut to_remove = Vec::new();

        if let Some(healths) = self.components.get_mut("Health") {
            for (&entity, value) in healths.iter() {
                if let Some(obj) = value.as_object() {
                    if let Some(current) = obj.get("current") {
                        if current.as_f64().unwrap_or(1.0) <= 0.0 {
                            to_remove.push(entity);
                        }
                    }
                }
            }
        }

        for entity in to_remove {
            // Remove Health component
            if let Some(healths) = self.components.get_mut("Health") {
                healths.remove(&entity);
            }

            // Add Corpse component
            self.set_component(entity, "Corpse", serde_json::json!({}))
                .ok();

            // Add Decay component with default time_remaining (e.g., 5 ticks)
            self.set_component(entity, "Decay", serde_json::json!({ "time_remaining": 5 }))
                .ok();
        }
    }

    pub fn process_decay(&mut self) {
        let mut to_remove_entities = Vec::new();

        if let Some(decays) = self.components.get_mut("Decay") {
            for (&entity, value) in decays.iter_mut() {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(time_remaining) = obj.get_mut("time_remaining") {
                        if let Some(t) = time_remaining.as_u64() {
                            if t <= 1 {
                                to_remove_entities.push(entity);
                            } else {
                                *time_remaining = serde_json::json!(t - 1);
                            }
                        }
                    }
                }
            }
        }

        for entity in to_remove_entities {
            self.remove_entity(entity);
        }
    }

    pub fn remove_entity(&mut self, entity: u32) {
        // Remove all components associated with the entity
        for comps in self.components.values_mut() {
            comps.remove(&entity);
        }
        // Remove from entity list
        self.entities.retain(|&id| id != entity);
    }

    pub fn get_entities_with_component(&self, name: &str) -> Vec<u32> {
        self.components
            .get(name)
            .map(|map| map.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub fn move_entity(&mut self, entity: u32, dx: f32, dy: f32) {
        if let Some(positions) = self.components.get_mut("Position") {
            if let Some(value) = positions.get_mut(&entity) {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(x) = obj.get_mut("x") {
                        if let Some(x_val) = x.as_f64() {
                            *x = serde_json::json!(x_val + dx as f64);
                        }
                    }
                    if let Some(y) = obj.get_mut("y") {
                        if let Some(y_val) = y.as_f64() {
                            *y = serde_json::json!(y_val + dy as f64);
                        }
                    }
                }
            }
        }
    }

    pub fn is_entity_alive(&self, entity: u32) -> bool {
        if let Some(health) = self.get_component(entity, "Health") {
            health
                .get("current")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                > 0.0
        } else {
            false
        }
    }

    pub fn damage_entity(&mut self, entity: u32, amount: f32) {
        if let Some(healths) = self.components.get_mut("Health") {
            if let Some(value) = healths.get_mut(&entity) {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(current) = obj.get_mut("current") {
                        if let Some(cur_val) = current.as_f64() {
                            *current = serde_json::json!((cur_val - amount as f64).max(0.0));
                        }
                    }
                }
            }
        }
    }

    pub fn count_entities_with_type(&self, type_str: &str) -> usize {
        self.get_entities_with_component("Type")
            .into_iter()
            .filter(|&id| {
                self.get_component(id, "Type")
                    .and_then(|v| v.get("kind"))
                    .and_then(|k| k.as_str())
                    .map(|k| k == type_str)
                    .unwrap_or(false)
            })
            .count()
    }

    pub fn register_system<S: crate::ecs::system::System + 'static>(&mut self, system: S) {
        self.systems.register_system(system);
    }

    pub fn run_system(&mut self, name: &str) -> Result<(), String> {
        // Take the system out to avoid double mutable borrow
        if let Some(mut system) = self.systems.take_system(name) {
            system.run(self);
            self.systems.register_system_boxed(name.to_string(), system);
            Ok(())
        } else {
            Err(format!("System '{}' not found", name))
        }
    }

    pub fn list_systems(&self) -> Vec<String> {
        self.systems.list_systems()
    }

    /// Modify the amount of a resource for a given entity.
    /// Negative `delta` removes, positive adds. Returns Err if insufficient resource.
    pub fn modify_resource_amount(
        &mut self,
        entity_id: u32,
        kind: &str,
        delta: f64,
    ) -> Result<(), String> {
        // Find the Resource component for this entity
        let comp = self
            .components
            .get_mut("Resource")
            .and_then(|map| map.get_mut(&entity_id));
        if let Some(resource) = comp {
            if let Some(obj) = resource.as_object_mut() {
                // Check kind matches
                if obj.get("kind").and_then(|v| v.as_str()) != Some(kind) {
                    return Err("Resource kind mismatch".to_string());
                }
                // Get current amount
                let amount = obj.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let new_amount = amount + delta;
                if new_amount < 0.0 {
                    return Err("Not enough resource".to_string());
                }
                obj.insert("amount".to_string(), serde_json::json!(new_amount));
                return Ok(());
            }
        }
        Err("Resource component not found".to_string())
    }

    pub fn modify_stockpile_resource(
        &mut self,
        entity_id: u32,
        kind: &str,
        delta: f64,
    ) -> Result<(), String> {
        let comp = self
            .components
            .get_mut("Stockpile")
            .and_then(|map| map.get_mut(&entity_id));
        if let Some(stockpile) = comp {
            if let Some(obj) = stockpile.as_object_mut() {
                if let Some(resources) = obj.get_mut("resources").and_then(|v| v.as_object_mut()) {
                    let current = resources.get(kind).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let new_amount = current + delta;
                    if new_amount < 0.0 {
                        return Err("Not enough resource".to_string());
                    }
                    resources.insert(kind.to_string(), serde_json::json!(new_amount));
                    return Ok(());
                }
            }
        }
        Err("Stockpile component not found".to_string())
    }

    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)
    }

    pub fn load_from_file(
        path: &std::path::Path,
        registry: Arc<ComponentRegistry>,
    ) -> Result<Self, std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        let mut world: Self = serde_json::from_str(&json)?;
        world.registry = registry; // Re-inject registry if needed
        Ok(world)
    }

    pub fn send_event(&mut self, event_type: &str, payload: JsonValue) -> Result<(), String> {
        println!(
            "Rust: send_event called for type '{}' with payload {:?}",
            event_type, payload
        );
        let bus = self
            .event_buses
            .entry(event_type.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(EventBus::<JsonValue>::default())));
        bus.lock().unwrap().send(payload);
        Ok(())
    }

    pub fn get_event_bus(&self, event_type: &str) -> Option<Arc<Mutex<EventBus<JsonValue>>>> {
        self.event_buses.get(event_type).cloned()
    }

    pub fn get_or_create_event_bus(&mut self, event_type: &str) -> Arc<Mutex<EventBus<JsonValue>>> {
        self.event_buses
            .entry(event_type.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(EventBus::<JsonValue>::default())))
            .clone()
    }

    pub fn update_event_buses(&self) {
        for bus in self.event_buses.values() {
            bus.lock().unwrap().update();
        }
    }
}

impl Default for World {
    fn default() -> Self {
        panic!("World::default() is not supported. Use World::new(registry) instead.");
    }
}
