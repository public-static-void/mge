use super::input::{InputProvider, StdinInput};
use super::world::World;
use mlua::LuaSerdeExt;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct ScriptEngine {
    lua: Lua,
    input_provider: Arc<Mutex<Box<dyn InputProvider + Send + Sync>>>,
}

impl ScriptEngine {
    pub fn new() -> Self {
        Self::new_with_input(Box::new(StdinInput))
    }

    pub fn new_with_input(input_provider: Box<dyn InputProvider + Send + Sync>) -> Self {
        Self {
            lua: Lua::new(),
            input_provider: Arc::new(Mutex::new(input_provider)),
        }
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

    pub fn register_world(&mut self, world: Rc<RefCell<World>>) -> mlua::Result<()> {
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

        let input_provider = Arc::clone(&self.input_provider);
        let get_user_input = self.lua.create_function(move |_, prompt: String| {
            let mut provider = input_provider
                .lock()
                .map_err(|_| mlua::Error::external("Input provider lock poisoned"))?;
            provider.read_line(&prompt).map_err(mlua::Error::external)
        })?;
        globals.set("get_user_input", get_user_input)?;

        Ok(())
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}
