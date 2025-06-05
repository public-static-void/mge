//! Miscellaneous API: tick, turn, user input, etc.

use crate::ecs::world::World;
use crate::scripting::helpers::lua_error_msg;
use crate::scripting::input::InputProvider;
use crate::systems::death_decay::{ProcessDeaths, ProcessDecay};
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub fn register_misc_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
    input_provider: Arc<Mutex<Box<dyn InputProvider + Send + Sync>>>,
) -> LuaResult<()> {
    // get_user_input(prompt)
    let input_provider_clone = input_provider.clone();
    let get_user_input = lua.create_function(move |lua, prompt: String| {
        let mut provider = input_provider_clone
            .lock()
            .map_err(|_| lua_error_msg(lua, "Input provider lock poisoned"))?;
        provider
            .read_line(&prompt)
            .map_err(|e| lua_error_msg(lua, &format!("Input error: {e}")))
    })?;
    globals.set("get_user_input", get_user_input)?;

    // count_entities_with_type(type_str)
    let world_count_type = world.clone();
    let count_entities_with_type = lua.create_function_mut(move |_, type_str: String| {
        let world = world_count_type.borrow();
        Ok(world.count_entities_with_type(&type_str))
    })?;
    globals.set("count_entities_with_type", count_entities_with_type)?;

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
