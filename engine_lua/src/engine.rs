use super::event_bus::register_event_bus_and_globals;
use super::input::{InputProvider, StdinInput};
use super::lua_api::register_all_api_functions;
use super::system_bridge::register_system_functions;
use crate::lua_api::world::register_world_api;
use crate::lua_api::worldgen::register_worldgen_api;
use engine_core::ecs::world::World;
use engine_core::mods::loader::ModScriptEngine;
use engine_core::worldgen::{GLOBAL_WORLDGEN_REGISTRY, WorldgenRegistry};
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
    /// Creates a new ScriptEngine
    pub fn new() -> Self {
        Self::new_with_input(Box::new(StdinInput))
    }

    pub fn new_with_input(input_provider: Box<dyn InputProvider + Send + Sync>) -> Self {
        use mlua::{Lua, LuaOptions, StdLib};

        // Create Lua-VM with all standard libs (incl. debug)
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

            let require_json = lua
                .create_function(|lua, path: String| {
                    let json_str = std::fs::read_to_string(&path).map_err(|e| {
                        mlua::Error::external(format!("Failed to read file: {}", e))
                    })?;
                    let json_val: serde_json::Value =
                        serde_json::from_str(&json_str).map_err(|e| {
                            mlua::Error::external(format!("Failed to parse JSON: {}", e))
                        })?;
                    crate::helpers::json_to_lua_table(lua, &json_val)
                })
                .expect("Failed to create require_json function");
            globals
                .set("require_json", require_json)
                .expect("Failed to set require_json function");

            // Expose array metatable for marking tables as arrays
            let array_mt = lua
                .create_table()
                .expect("Failed to create array metatable");
            array_mt
                .set("__is_array", true)
                .expect("Failed to set __is_array");
            globals
                .set("array_mt", array_mt)
                .expect("Failed to set array_mt global");
        }

        // --- Import plugins from the global registry into the local Lua registry ---
        let worldgen_registry = {
            let mut local = WorldgenRegistry::new();
            {
                let global = GLOBAL_WORLDGEN_REGISTRY.lock().unwrap();
                local.import_threadsafe_plugins(&global);
            }
            std::rc::Rc::new(std::cell::RefCell::new(local))
        };

        Self {
            lua: std::rc::Rc::new(lua),
            input_provider: std::sync::Arc::new(std::sync::Mutex::new(input_provider)),
            worldgen_registry,
            lua_systems: std::rc::Rc::new(
                std::cell::RefCell::new(std::collections::HashMap::new()),
            ),
        }
    }

    /// Runs the given Lua script
    pub fn run_script(&self, code: &str) -> LuaResult<()> {
        self.lua.load(code).call(())
    }

    /// Registers the world to the Lua VM and exposes it as a global `world` userdata
    pub fn register_world(&mut self, world: Rc<RefCell<World>>) -> mlua::Result<()> {
        let globals = self.lua.globals();

        // Expose the ECS world as a Lua userdata with methods
        register_world_api(&self.lua, &globals, world.clone())?;

        register_worldgen_api(&self.lua, &globals, self.worldgen_registry.clone())?;

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

    /// Sets the command line arguments
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

    /// Returns a mutable reference to the worldgen registry.
    pub fn worldgen_registry_mut(&self) -> std::cell::RefMut<'_, WorldgenRegistry> {
        self.worldgen_registry.borrow_mut()
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ModScriptEngine for ScriptEngine {
    fn run_script(&mut self, script: &str) -> Result<(), String> {
        // Call the inherent method, not the trait method!
        ScriptEngine::run_script(self, script).map_err(|e| e.to_string())
    }
}
