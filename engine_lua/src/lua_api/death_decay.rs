//! Deaths/Decay API: process deaths and decay systems.

use engine_core::ecs::world::World;
use engine_core::systems::death_decay::{ProcessDeaths, ProcessDecay};
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the death/decay API.
pub fn register_death_decay_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // process_deaths()
    let world_deaths = world.clone();
    let process_deaths = lua.create_function_mut(move |_, ()| {
        let mut world = world_deaths.borrow_mut();
        world.register_system(ProcessDeaths);
        world.run_system("ProcessDeaths", None).unwrap();
        Ok(())
    })?;
    globals.set("process_deaths", process_deaths)?;

    // process_decay()
    let world_decay = world.clone();
    let process_decay = lua.create_function_mut(move |_, ()| {
        let mut world = world_decay.borrow_mut();
        world.register_system(ProcessDecay);
        world.run_system("ProcessDecay", None).unwrap();
        Ok(())
    })?;
    globals.set("process_decay", process_decay)?;

    Ok(())
}
