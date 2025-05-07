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
}

impl World {
    pub fn new() -> Self {
        World {
            entities: Vec::new(),
            components: HashMap::new(),
            next_id: 1,
            current_mode: "colony".to_string(),
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
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
