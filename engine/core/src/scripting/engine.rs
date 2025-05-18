use super::helpers::{json_to_lua_table, lua_table_to_json};
use super::input::{InputProvider, StdinInput};
use super::world::World;
use crate::systems::standard::{DamageAll, MoveAll, ProcessDeaths, ProcessDecay};
use crate::worldgen::{WorldgenError, WorldgenPlugin, WorldgenRegistry};
use mlua::RegistryKey;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct ScriptEngine {
    lua: Rc<Lua>,
    input_provider: Arc<Mutex<Box<dyn InputProvider + Send + Sync>>>,
    worldgen_registry: Rc<RefCell<WorldgenRegistry>>,
    lua_systems: Rc<RefCell<HashMap<String, RegistryKey>>>,
}

impl ScriptEngine {
    pub fn new() -> Self {
        Self::new_with_input(Box::new(StdinInput))
    }

    pub fn new_with_input(input_provider: Box<dyn InputProvider + Send + Sync>) -> Self {
        Self {
            lua: Rc::new(Lua::new()),
            input_provider: Arc::new(Mutex::new(input_provider)),
            worldgen_registry: Rc::new(RefCell::new(WorldgenRegistry::new())),
            lua_systems: Rc::new(RefCell::new(HashMap::new())),
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
                        json_to_lua_table(lua, val)
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
                world.register_system(MoveAll { dx, dy });
                world.run_system("MoveAll", None).unwrap();
                Ok(())
            })?;
        globals.set("move_all", move_all)?;

        // damage_all(amount)
        let world_damage = world.clone();
        let damage_all = self.lua.create_function_mut(move |_, amount: f32| {
            let mut world = world_damage.borrow_mut();
            world.register_system(DamageAll { amount });
            world.run_system("DamageAll", None).unwrap();
            Ok(())
        })?;
        globals.set("damage_all", damage_all)?;

        // tick()
        let world_tick = world.clone();
        let tick = self.lua.create_function_mut(move |_, ()| {
            let mut world = world_tick.borrow_mut();
            world.run_system("MoveAll", None).unwrap();
            world.run_system("DamageAll", None).unwrap();
            world.run_system("ProcessDeaths", None).unwrap();
            world.run_system("ProcessDecay", None).unwrap();
            world.turn += 1;
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
            world.register_system(ProcessDeaths);
            world.run_system("ProcessDeaths", None).unwrap();
            Ok(())
        })?;
        globals.set("process_deaths", process_deaths)?;

        // process_decay()
        let world_decay = world.clone();
        let process_decay = self.lua.create_function_mut(move |_, ()| {
            let mut world = world_decay.borrow_mut();
            world.register_system(ProcessDecay);
            world.run_system("ProcessDecay", None).unwrap();
            Ok(())
        })?;
        globals.set("process_decay", process_decay)?;

        // despawn_entity(id)
        let world_remove = world.clone();
        let despawn_entity = self.lua.create_function_mut(move |_, entity_id: u32| {
            let mut world = world_remove.borrow_mut();
            world.despawn_entity(entity_id);
            Ok(())
        })?;
        globals.set("despawn_entity", despawn_entity)?;

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
            Ok(world.registry.lock().unwrap().all_component_names())
        })?;
        globals.set("list_components", list_components)?;

        // get_component_schema(name)
        let world_get_schema = world.clone();
        let get_component_schema = self.lua.create_function_mut(move |lua, name: String| {
            let world = world_get_schema.borrow();
            if let Some(schema) = world.registry.lock().unwrap().get_schema_by_name(&name) {
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
            let modes = world.registry.lock().unwrap().all_modes();
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
        let save_to_file = self.lua.create_function_mut(move |_, filename: String| {
            let world = world_save.borrow();
            world
                .save_to_file(std::path::Path::new(&filename))
                .map_err(mlua::Error::external)
        })?;
        globals.set("save_to_file", save_to_file)?;

        // load_world(filename)
        let world_load = world.clone();
        let registry = world.borrow().registry.clone();
        let load_from_file = self.lua.create_function_mut(move |_, filename: String| {
            let mut world = world_load.borrow_mut();
            let loaded = World::load_from_file(std::path::Path::new(&filename), registry.clone())
                .map_err(mlua::Error::external)?;
            *world = loaded;
            Ok(())
        })?;
        globals.set("load_from_file", load_from_file)?;

        let worldgen_registry_register = self.worldgen_registry.clone();
        let worldgen_registry_list = self.worldgen_registry.clone();
        let worldgen_registry_invoke = self.worldgen_registry.clone();

        let register_worldgen = self.lua.create_function({
            let worldgen_registry_register = worldgen_registry_register.clone();
            move |lua, (name, func): (String, mlua::Function)| {
                // Store the Lua function in the registry and keep the key
                let func_registry_key = lua.create_registry_value(func)?;
                worldgen_registry_register
                    .borrow_mut()
                    .register(WorldgenPlugin::Lua {
                        name,
                        registry_key: func_registry_key,
                    });
                Ok(())
            }
        })?;
        globals.set("register_worldgen", register_worldgen)?;

        let list_worldgen = self
            .lua
            .create_function(move |_, ()| Ok(worldgen_registry_list.borrow().list_names()))?;
        globals.set("list_worldgen", list_worldgen)?;

        let invoke_worldgen =
            self.lua
                .create_function(move |lua, (name, params): (String, mlua::Table)| {
                    // Convert Lua params to serde_json::Value
                    let params_json = lua_table_to_json(lua, &params)?;
                    // Call the Lua worldgen plugin via the registry
                    let registry = worldgen_registry_invoke.borrow();
                    match registry.invoke_lua(lua, &name, &params_json) {
                        Ok(result_json) => {
                            // Convert result back to Lua value/table
                            json_to_lua_table(lua, &result_json)
                        }
                        Err(WorldgenError::NotFound) => Err(mlua::Error::external(format!(
                            "Worldgen plugin '{}' not found",
                            name
                        ))),
                        Err(WorldgenError::LuaError(e)) => Err(e),
                    }
                })?;
        globals.set("invoke_worldgen", invoke_worldgen)?;

        let lua_systems_outer = Rc::clone(&self.lua_systems);
        let world_rc = world.clone();
        let lua = Rc::clone(&self.lua);

        let register_system = self.lua.create_function_mut(
            move |_, (name, func, opts): (String, mlua::Function, Option<mlua::Table>)| {
                let key = lua.create_registry_value(func)?;
                lua_systems_outer.borrow_mut().insert(name.clone(), key);

                let mut dependencies = Vec::new();
                if let Some(opts) = opts {
                    if let Ok(dep_table) = opts.get::<_, mlua::Table>("dependencies") {
                        for dep in dep_table.sequence_values::<String>() {
                            dependencies.push(dep?);
                        }
                    }
                }

                let system_name_for_closure = name.clone();
                let system_name_for_fn = system_name_for_closure.clone();
                let lua_systems_inner = Rc::clone(&lua_systems_outer); // <-- CLONE HERE
                let lua = lua.clone();

                world_rc.borrow_mut().register_dynamic_system_with_deps(
                    &system_name_for_closure,
                    dependencies,
                    move |_world, dt| {
                        let binding = lua_systems_inner.borrow();
                        let key = binding
                            .get(&system_name_for_fn)
                            .expect("Lua system not found");
                        let func: mlua::Function =
                            lua.registry_value(key).expect("Invalid Lua registry key");
                        let _ = func.call::<_, ()>((dt,));
                    },
                );

                Ok(())
            },
        )?;
        globals.set("register_system", register_system)?;

        let lua_systems = Rc::clone(&self.lua_systems);
        let run_system = self.lua.create_function_mut(move |lua, name: String| {
            let systems = lua_systems.borrow();
            if let Some(key) = systems.get(&name) {
                let func: mlua::Function = lua.registry_value(key)?;
                func.call::<_, ()>(())?;
                Ok(())
            } else {
                Err(mlua::Error::external("system not found"))
            }
        })?;
        globals.set("run_system", run_system)?;

        let world_native_run = world.clone();
        let run_native_system = self.lua.create_function_mut(move |_, name: String| {
            let mut world = world_native_run.borrow_mut();
            world.run_system(&name, None).map_err(mlua::Error::external)
        })?;
        globals.set("run_native_system", run_native_system)?;

        let world_for_print = world.clone();
        let print_positions_fn = self.lua.create_function_mut(move |_, ()| {
            let world = world_for_print.borrow();
            print_positions(&world);
            Ok(())
        })?;
        globals.set("print_positions", print_positions_fn)?;

        let world_for_print = world.clone();
        let print_healths_fn = self.lua.create_function_mut(move |_, ()| {
            let world = world_for_print.borrow();
            print_healths(&world);
            Ok(())
        })?;
        globals.set("print_healths", print_healths_fn)?;

        // assign_job(entity, job_type, fields)
        let world_assign_job = world.clone();
        let assign_job = self.lua.create_function_mut(
            move |lua, (entity, job_type, fields): (u32, String, Option<Table>)| {
                let mut world = world_assign_job.borrow_mut();
                let mut job_val = serde_json::json!({
                    "job_type": job_type,
                    "status": "pending",
                    "progress": 0.0
                });
                if let Some(tbl) = fields {
                    let extra: JsonValue = lua_table_to_json(lua, &tbl)?;
                    if let Some(obj) = extra.as_object() {
                        for (k, v) in obj {
                            job_val[k] = v.clone();
                        }
                    }
                }
                world
                    .set_component(entity, "Job", job_val)
                    .map_err(mlua::Error::external)
            },
        )?;
        globals.set("assign_job", assign_job)?;

        let world_take_events = world.clone();
        let poll_ecs_event = self
            .lua
            .create_function_mut(move |lua, event_type: String| {
                let mut world = world_take_events.borrow_mut();
                let events = world.take_events(&event_type);
                let tbl = lua.create_table()?;
                for (i, val) in events.into_iter().enumerate() {
                    tbl.set(i + 1, json_to_lua_table(lua, &val)?)?;
                }
                Ok(tbl)
            })?;
        globals.set("poll_ecs_event", poll_ecs_event)?;

        let world_for_job_types = world.clone();
        let get_job_types = self.lua.create_function(move |_, ()| {
            let world = world_for_job_types.borrow();
            let job_types = world.job_types.job_type_names();
            Ok(job_types)
        })?;
        globals.set("get_job_types", get_job_types)?;

        let world_for_jobs = world.clone();
        let lua = Rc::clone(&self.lua);
        let register_job_type =
            self.lua
                .create_function_mut(move |_, (name, func): (String, mlua::Function)| {
                    let key = lua.create_registry_value(func)?;
                    let mut world = world_for_jobs.borrow_mut();
                    world.job_types.register_lua(&name, key);
                    Ok(())
                })?;
        globals.set("register_job_type", register_job_type)?;

        use super::event_bus::register_event_bus_and_globals;
        register_event_bus_and_globals(&self.lua, &globals, world.clone())?;

        Ok(())
    }
}

pub fn print_positions(world: &World) {
    if let Some(positions) = world.components.get("Position") {
        for (entity, value) in positions {
            println!("Entity {}: {:?}", entity, value);
        }
    } else {
        println!("No Position components found.");
    }
}

pub fn print_healths(world: &World) {
    if let Some(healths) = world.components.get("Health") {
        for (entity, value) in healths {
            println!("Entity {}: {:?}", entity, value);
        }
    } else {
        println!("No Health components found.");
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}
