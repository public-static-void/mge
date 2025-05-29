use super::event_bus::register_event_bus_and_globals;
use super::input::{InputProvider, StdinInput};
use super::lua_api::register_all_api_functions;
use super::system_bridge::register_system_functions;
use crate::ecs::world::World;
use crate::scripting::helpers::json_to_lua_table;
use crate::scripting::lua_api::worldgen::register_worldgen_api;
use crate::worldgen::WorldgenRegistry;
use mlua::RegistryKey;
use mlua::{Lua, Result as LuaResult};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct ScriptEngine {
    pub lua: Rc<Lua>,
    input_provider: Arc<Mutex<Box<dyn InputProvider + Send + Sync>>>,
    worldgen_registry: Rc<RefCell<WorldgenRegistry>>,
    lua_systems: Rc<RefCell<HashMap<String, RegistryKey>>>,
}

impl ScriptEngine {
    pub fn new() -> Self {
        Self::new_with_input(Box::new(StdinInput))
    }

    pub fn new_with_input(input_provider: Box<dyn InputProvider + Send + Sync>) -> Self {
        use mlua::{Lua, LuaOptions, StdLib};

        // Lua-VM mit allen Standardbibliotheken (inkl. debug) erzeugen
        let lua = unsafe { Lua::unsafe_new_with(StdLib::ALL, LuaOptions::default()) };

        {
            let globals = lua.globals();
            let print = lua
                .create_function(|_, args: mlua::Variadic<mlua::Value>| {
                    let mut out = String::new();
                    for (i, v) in args.iter().enumerate() {
                        if i > 0 {
                            out.push('\t');
                        }
                        out.push_str(&v.to_string()?);
                    }
                    println!("{}", out);
                    Ok(())
                })
                .expect("Failed to create print function");
            globals
                .set("print", print)
                .expect("Failed to set print function");

            // --- BEGIN: Register require_json ---
            let require_json = lua
                .create_function(|lua, path: String| {
                    let json_str = std::fs::read_to_string(&path).map_err(|e| {
                        mlua::Error::external(format!("Failed to read file: {}", e))
                    })?;
                    let json_val: serde_json::Value =
                        serde_json::from_str(&json_str).map_err(|e| {
                            mlua::Error::external(format!("Failed to parse JSON: {}", e))
                        })?;
                    json_to_lua_table(lua, &json_val)
                })
                .expect("Failed to create require_json function");
            globals
                .set("require_json", require_json)
                .expect("Failed to set require_json function");
            // --- END: Register require_json ---
        }

        Self {
            lua: Rc::new(lua),
            input_provider: Arc::new(Mutex::new(input_provider)),
            worldgen_registry: Rc::new(RefCell::new(WorldgenRegistry::new())),
            lua_systems: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn run_script(&self, code: &str) -> LuaResult<()> {
        self.lua.load(code).call(())
    }

    pub fn register_world(&mut self, world: Rc<RefCell<World>>) -> mlua::Result<()> {
        let globals = self.lua.globals();

        register_worldgen_api(&self.lua, &globals, self.worldgen_registry.clone())?;

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

        register_event_bus_and_globals(&self.lua, &globals, world.clone())?;

        register_system_functions(
            Rc::clone(&self.lua),
            &globals,
            world.clone(),
            self.lua_systems.clone(),
        )?;

        register_all_api_functions(
            &self.lua,
            &globals,
            world.clone(),
            Arc::clone(&self.input_provider),
            Rc::clone(&self.worldgen_registry),
        )?;

        Ok(())
    }

    pub fn set_lua_args(&self, args: Vec<String>) {
        let globals = self.lua.globals();
        let lua_args = self
            .lua
            .create_table()
            .expect("Failed to create Lua table for args");
        for (i, val) in args.iter().enumerate() {
            lua_args
                .set(i + 1, val.as_str())
                .expect("Failed to set arg in Lua table");
        }
        globals
            .set("arg", lua_args)
            .expect("Failed to set global arg in Lua");
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
