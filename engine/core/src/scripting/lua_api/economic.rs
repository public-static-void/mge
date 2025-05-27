//! Economic system Lua helpers: stockpile, production job, etc.

use crate::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_economic_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // get_stockpile_resources(entity)
    let world_get = world.clone();
    let get_stockpile_resources = lua.create_function_mut(move |lua, entity: u32| {
        let world = world_get.borrow();
        if let Some(stockpile) = world.get_component(entity, "Stockpile") {
            if let Some(resources) = stockpile.get("resources") {
                crate::scripting::helpers::json_to_lua_table(lua, resources)
            } else {
                Ok(mlua::Value::Nil)
            }
        } else {
            Ok(mlua::Value::Nil)
        }
    })?;
    globals.set("get_stockpile_resources", get_stockpile_resources)?;

    // get_production_job(entity)
    let world_get = world.clone();
    let get_production_job = lua.create_function_mut(move |lua, entity: u32| {
        let world = world_get.borrow();
        if let Some(job) = world.get_component(entity, "ProductionJob") {
            crate::scripting::helpers::json_to_lua_table(lua, job)
        } else {
            Ok(mlua::Value::Nil)
        }
    })?;
    globals.set("get_production_job", get_production_job)?;

    Ok(())
}
