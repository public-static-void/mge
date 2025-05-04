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

use mlua::{Lua, Result, Value};
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

    pub fn run_script(&self, code: &str) -> Result<()> {
        self.lua.load(code).exec()
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

        // Set position
        let world_set = world.clone();
        let set_position =
            self.lua
                .create_function_mut(move |_, (entity, x, y): (u32, f32, f32)| {
                    let mut world = world_set.borrow_mut();
                    world.set_position(entity, Position { x, y });
                    Ok(())
                })?;
        globals.set("set_position", set_position)?;

        // Get position
        let world_get = world.clone();
        let get_position = self.lua.create_function_mut(move |lua, entity: u32| {
            let world = world_get.borrow();
            if let Some(pos) = world.get_position(entity) {
                let tbl = lua.create_table()?;
                tbl.set("x", pos.x)?;
                tbl.set("y", pos.y)?;
                Ok(Value::Table(tbl))
            } else {
                Ok(Value::Nil)
            }
        })?;
        globals.set("get_position", get_position)?;

        // Set health
        let world_set_health = world.clone();
        let set_health =
            self.lua
                .create_function_mut(move |_, (entity, current, max): (u32, f32, f32)| {
                    let mut world = world_set_health.borrow_mut();
                    world.set_health(entity, Health { current, max });
                    Ok(())
                })?;
        globals.set("set_health", set_health)?;

        // Get health
        let world_get_health = world.clone();
        let get_health = self.lua.create_function_mut(move |lua, entity: u32| {
            let world = world_get_health.borrow();
            if let Some(health) = world.get_health(entity) {
                let tbl = lua.create_table()?;
                tbl.set("current", health.current)?;
                tbl.set("max", health.max)?;
                Ok(Value::Table(tbl))
            } else {
                Ok(Value::Nil)
            }
        })?;
        globals.set("get_health", get_health)?;

        Ok(())
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Debug)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

pub struct World {
    pub entities: Vec<u32>,
    pub positions: HashMap<u32, Position>,
    pub healths: HashMap<u32, Health>,
    next_id: u32,
}

impl World {
    pub fn new() -> Self {
        World {
            entities: Vec::new(),
            positions: HashMap::new(),
            healths: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn spawn(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push(id);
        id
    }

    pub fn set_position(&mut self, entity: u32, pos: Position) {
        self.positions.insert(entity, pos);
    }

    pub fn get_position(&self, entity: u32) -> Option<&Position> {
        self.positions.get(&entity)
    }

    pub fn set_health(&mut self, entity: u32, health: Health) {
        self.healths.insert(entity, health);
    }

    pub fn get_health(&self, entity: u32) -> Option<&Health> {
        self.healths.get(&entity)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
