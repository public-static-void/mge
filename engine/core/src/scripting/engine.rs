use super::api::register_api_functions;
use super::event_bus::register_event_bus_and_globals;
use super::input::{InputProvider, StdinInput};
use super::system_bridge::register_system_functions;
use super::worldgen_bridge::register_worldgen_functions;
use crate::ecs::world::World;
use crate::worldgen::WorldgenRegistry;
use mlua::RegistryKey;
use mlua::{Lua, Result as LuaResult};
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

        register_worldgen_functions(&self.lua, &globals, self.worldgen_registry.clone())?;

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

        register_api_functions(
            &self.lua,
            &globals,
            world.clone(),
            Arc::clone(&self.input_provider),
        )?;

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
