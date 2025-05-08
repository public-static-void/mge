//! # Lua Scripting Bridge
//!
//! ## Exposed Functions
//! - `spawn_entity()` -> entity id
//! - `set_position(entity, x, y)`
//! - `get_position(entity)` -> {x, y} or nil
//! - `set_health(entity, current, max)`
//! - `get_health(entity)` -> {current, max} or nil
//!
//! ## Adding More Components
//! 1. Extend `World` with your component storage.
//! 2. Add set/get methods.
//! 3. Register new Lua functions in `register_world`.
//! 4. Add Lua and Rust tests.

use mlua::LuaSerdeExt;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct ScriptEngine {
    lua: Lua,
}

impl ScriptEngine {
    pub fn new() -> Self {
        ScriptEngine { lua: Lua::new() }
    }

    pub fn run_script(&self, code: &str) -> LuaResult<()> {
        self.lua.load(code).exec()
    }

    fn lua_table_to_json(lua: &Lua, table: &Table) -> LuaResult<JsonValue> {
        lua.from_value(LuaValue::Table(table.clone()))
    }

    fn json_to_lua_table<'lua>(lua: &'lua Lua, value: &JsonValue) -> LuaResult<Table<'lua>> {
        let lua_value = lua.to_value(value)?;
        if let LuaValue::Table(tbl) = lua_value {
            Ok(tbl)
        } else {
            lua.create_table()
        }
    }

    pub fn register_world(&self, world: Rc<RefCell<World>>) -> mlua::Result<()> {
        let globals = self.lua.globals();

        // Spawn entity
        let world_spawn = world.clone();
        let spawn = self.lua.create_function_mut(move |_, ()| {
            let mut world = world_spawn.borrow_mut();
            Ok(world.spawn())
        })?;
        globals.set("spawn_entity", spawn)?;

        // set_component(entity, name, table)
        let world_set = world.clone();
        let set_component = self.lua.create_function_mut(
            move |lua, (entity, name, table): (u32, String, Table)| {
                let mut world = world_set.borrow_mut();
                let json_value: JsonValue = Self::lua_table_to_json(lua, &table)?;
                match world.set_component(entity, &name, json_value) {
                    Ok(_) => Ok(true),
                    Err(e) => Err(mlua::Error::external(e)),
                }
            },
        )?;
        globals.set("set_component", set_component)?;

        // get_component(entity, name)
        let world_get = world.clone();
        let get_component =
            self.lua
                .create_function_mut(move |lua, (entity, name): (u32, String)| {
                    let world = world_get.borrow();
                    if let Some(val) = world.get_component(entity, &name) {
                        let tbl = Self::json_to_lua_table(lua, val)?;
                        Ok(LuaValue::Table(tbl))
                    } else {
                        Ok(LuaValue::Nil)
                    }
                })?;
        globals.set("get_component", get_component)?;

        // set_mode(mode: String)
        let world_set_mode = world.clone();
        let set_mode = self.lua.create_function_mut(move |_, mode: String| {
            let mut world = world_set_mode.borrow_mut();
            world.current_mode = mode;
            Ok(())
        })?;
        globals.set("set_mode", set_mode)?;

        // move_all(dx, dy)
        let world_move = world.clone();
        let move_all = self
            .lua
            .create_function_mut(move |_, (dx, dy): (f32, f32)| {
                let mut world = world_move.borrow_mut();
                world.move_all(dx, dy);
                Ok(())
            })?;
        globals.set("move_all", move_all)?;

        let world_print = world.clone();
        let print_positions = self.lua.create_function_mut(move |_, ()| {
            let world = world_print.borrow();
            world.print_positions();
            Ok(())
        })?;
        globals.set("print_positions", print_positions)?;

        // damage_all(amount)
        let world_damage = world.clone();
        let damage_all = self.lua.create_function_mut(move |_, amount: f32| {
            let mut world = world_damage.borrow_mut();
            world.damage_all(amount);
            Ok(())
        })?;
        globals.set("damage_all", damage_all)?;

        // print_healths()
        let world_print_health = world.clone();
        let print_healths = self.lua.create_function_mut(move |_, ()| {
            let world = world_print_health.borrow();
            world.print_healths();
            Ok(())
        })?;
        globals.set("print_healths", print_healths)?;

        // tick()
        let world_tick = world.clone();
        let tick = self.lua.create_function_mut(move |_, ()| {
            let mut world = world_tick.borrow_mut();
            world.tick();
            Ok(())
        })?;
        globals.set("tick", tick)?;

        // get_turn()
        let world_get_turn = world.clone();
        let get_turn = self.lua.create_function_mut(move |_, ()| {
            let world = world_get_turn.borrow();
            Ok(world.turn)
        })?;
        globals.set("get_turn", get_turn)?;

        // process_deaths()
        let world_deaths = world.clone();
        let process_deaths = self.lua.create_function_mut(move |_, ()| {
            let mut world = world_deaths.borrow_mut();
            world.process_deaths();
            Ok(())
        })?;
        globals.set("process_deaths", process_deaths)?;

        // process_decay()
        let world_decay = world.clone();
        let process_decay = self.lua.create_function_mut(move |_, ()| {
            let mut world = world_decay.borrow_mut();
            world.process_decay();
            Ok(())
        })?;
        globals.set("process_decay", process_decay)?;

        // remove_entity(id)
        let world_remove = world.clone();
        let remove_entity = self.lua.create_function_mut(move |_, entity_id: u32| {
            let mut world = world_remove.borrow_mut();
            world.remove_entity(entity_id);
            Ok(())
        })?;
        globals.set("remove_entity", remove_entity)?;

        let world_get_entities = world.clone();
        let get_entities_with_component =
            self.lua.create_function_mut(move |_, name: String| {
                let world = world_get_entities.borrow();
                let ids = world.get_entities_with_component(&name);
                Ok(ids)
            })?;
        globals.set("get_entities_with_component", get_entities_with_component)?;

        let world_move_entity = world.clone();
        let move_entity =
            self.lua
                .create_function_mut(move |_, (entity, dx, dy): (u32, f32, f32)| {
                    let mut world = world_move_entity.borrow_mut();
                    world.move_entity(entity, dx, dy);
                    Ok(())
                })?;
        globals.set("move_entity", move_entity)?;

        let world_is_alive = world.clone();
        let is_entity_alive = self.lua.create_function_mut(move |_, entity: u32| {
            let world = world_is_alive.borrow();
            Ok(world.is_entity_alive(entity))
        })?;
        globals.set("is_entity_alive", is_entity_alive)?;

        let world_damage_entity = world.clone();
        let damage_entity =
            self.lua
                .create_function_mut(move |_, (entity, amount): (u32, f32)| {
                    let mut world = world_damage_entity.borrow_mut();
                    world.damage_entity(entity, amount);
                    Ok(())
                })?;
        globals.set("damage_entity", damage_entity)?;

        let world_count_type = world.clone();
        let count_entities_with_type =
            self.lua.create_function_mut(move |_, type_str: String| {
                let world = world_count_type.borrow();
                Ok(world.count_entities_with_type(&type_str))
            })?;
        globals.set("count_entities_with_type", count_entities_with_type)?;

        Ok(())
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

pub struct World {
    pub entities: Vec<u32>,
    pub components: HashMap<String, HashMap<u32, JsonValue>>,
    next_id: u32,
    current_mode: String,
    pub turn: u32,
}

impl World {
    pub fn new() -> Self {
        World {
            entities: Vec::new(),
            components: HashMap::new(),
            next_id: 1,
            current_mode: "colony".to_string(),
            turn: 0,
        }
    }

    pub fn spawn(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push(id);
        id
    }

    pub fn is_component_allowed_in_mode(component: &str, mode: &str) -> bool {
        match (component, mode) {
            ("Colony::Happiness", "colony") => true,
            ("Roguelike::Inventory", "roguelike") => true,
            ("Position", "colony") => true,
            ("Position", "roguelike") => true,
            ("Health", "colony") => true,
            ("Health", "roguelike") => true,
            ("Corpse", "colony") => true,
            ("Corpse", "roguelike") => true,
            ("Decay", "colony") => true,
            ("Decay", "roguelike") => true,
            ("Type", _) => true,
            // Add more as needed
            _ => false,
        }
    }

    // Generic set_component
    pub fn set_component(
        &mut self,
        entity: u32,
        name: &str,
        value: JsonValue,
    ) -> Result<(), String> {
        if !Self::is_component_allowed_in_mode(name, &self.current_mode) {
            return Err(format!(
                "Component {} not allowed in mode {}",
                name, self.current_mode
            ));
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
        // Optionally remove from entity list if you maintain one
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
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
