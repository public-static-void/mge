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

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

pub struct World {
    pub entities: Vec<u32>,
    pub positions: HashMap<u32, Position>,
    next_id: u32,
}

impl World {
    pub fn new() -> Self {
        World {
            entities: Vec::new(),
            positions: HashMap::new(),
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
}
