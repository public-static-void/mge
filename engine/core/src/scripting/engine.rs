use super::helpers::{json_to_lua_table, lua_table_to_json};
use super::input::{InputProvider, StdinInput};
use super::world::World;
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

    pub fn register_world(&mut self, world: Rc<RefCell<World>>) -> mlua::Result<()> {
        let globals = self.lua.globals();

        // Spawn entity
        let world_spawn = world.clone();
        let spawn_entity = self.lua.create_function_mut(move |_, ()| {
            let mut world = world_spawn.borrow_mut();
            Ok(world.spawn_entity())
        })?;
        globals.set("spawn_entity", spawn_entity)?;

        // set_component(entity, name, table)
        let world_set = world.clone();
        let set_component = self.lua.create_function_mut(
            move |lua, (entity, name, table): (u32, String, Table)| {
                let mut world = world_set.borrow_mut();
                let json_value: JsonValue = lua_table_to_json(lua, &table)?;
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
                        let tbl = json_to_lua_table(lua, val)?;
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

        // remove_component(entity, name)
        let world_remove_component = world.clone();
        let remove_component =
            self.lua
                .create_function_mut(move |_, (entity, name): (u32, String)| {
                    let mut world = world_remove_component.borrow_mut();
                    if let Some(comps) = world.components.get_mut(&name) {
                        comps.remove(&entity);
                    }
                    Ok(())
                })?;
        globals.set("remove_component", remove_component)?;

        // get_entities()
        let world_get_entities = world.clone();
        let get_entities = self.lua.create_function_mut(move |_, ()| {
            let world = world_get_entities.borrow();
            Ok(world.entities.clone())
        })?;
        globals.set("get_entities", get_entities)?;

        // list_components()
        let world_list_components = world.clone();
        let list_components = self.lua.create_function_mut(move |_, ()| {
            let world = world_list_components.borrow();
            Ok(world.registry.all_component_names())
        })?;
        globals.set("list_components", list_components)?;

        // get_component_schema(name)
        let world_get_schema = world.clone();
        let get_component_schema = self.lua.create_function_mut(move |lua, name: String| {
            let world = world_get_schema.borrow();
            if let Some(schema) = world.registry.get_schema_by_name(&name) {
                let json = serde_json::to_value(&schema.schema).map_err(mlua::Error::external)?;
                super::helpers::json_to_lua_table(lua, &json)
            } else {
                Err(mlua::Error::external("Component schema not found"))
            }
        })?;
        globals.set("get_component_schema", get_component_schema)?;

        // get_mode()
        let world_get_mode = world.clone();
        let get_mode = self.lua.create_function_mut(move |_, ()| {
            let world = world_get_mode.borrow();
            Ok(world.current_mode.clone())
        })?;
        globals.set("get_mode", get_mode)?;

        // get_available_modes()
        let world_get_modes = world.clone();
        let get_available_modes = self.lua.create_function_mut(move |_, ()| {
            let world = world_get_modes.borrow();
            let modes = world.registry.all_modes();
            Ok(modes.into_iter().collect::<Vec<String>>())
        })?;
        globals.set("get_available_modes", get_available_modes)?;

        let world_query = world.clone();
        let get_entities_with_components =
            self.lua
                .create_function_mut(move |_lua, names: mlua::Table| {
                    let world = world_query.borrow();
                    // Convert Lua table to Vec<String>
                    let mut rust_names = Vec::new();
                    for pair in names.sequence_values::<String>() {
                        rust_names.push(pair?);
                    }
                    let name_refs: Vec<&str> = rust_names.iter().map(|s| s.as_str()).collect();
                    Ok(world.get_entities_with_components(&name_refs))
                })?;
        globals.set("get_entities_with_components", get_entities_with_components)?;

        let world_modify_stockpile = world.clone();
        let modify_stockpile_resource =
            self.lua
                .create_function_mut(move |_, (entity, kind, delta): (u32, String, f64)| {
                    let mut world = world_modify_stockpile.borrow_mut();
                    world
                        .modify_stockpile_resource(entity, &kind, delta)
                        .map_err(mlua::Error::external)
                })?;
        globals.set("modify_stockpile_resource", modify_stockpile_resource)?;

        // save_world(filename)
        let world_save = world.clone();
        let save_world = self.lua.create_function_mut(move |_, filename: String| {
            let world = world_save.borrow();
            world
                .save_to_file(std::path::Path::new(&filename))
                .map_err(mlua::Error::external)
        })?;
        globals.set("save_world", save_world)?;

        // load_world(filename)
        let world_load = world.clone();
        let registry = world.borrow().registry.clone();
        let load_world = self.lua.create_function_mut(move |_, filename: String| {
            let mut world = world_load.borrow_mut();
            let loaded = World::load_from_file(std::path::Path::new(&filename), registry.clone())
                .map_err(mlua::Error::external)?;
            *world = loaded;
            Ok(())
        })?;
        globals.set("load_world", load_world)?;

        Ok(())
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}
