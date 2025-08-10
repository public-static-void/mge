//! Time of Day API: get the current time of day.

use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Register the time of day API.
pub fn register_time_of_day_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // get_time_of_day()
    let world_time = world.clone();
    let get_time_of_day = lua.create_function_mut(move |lua, ()| {
        let world = world_time.borrow();
        let time = world.get_time_of_day();
        let tbl = lua.create_table()?;
        tbl.set("hour", time.hour)?;
        tbl.set("minute", time.minute)?;
        Ok(tbl)
    })?;
    globals.set("get_time_of_day", get_time_of_day)?;

    Ok(())
}
