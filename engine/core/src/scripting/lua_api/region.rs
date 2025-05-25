//! Region and zone queries for scripting API.

use crate::ecs::world::World;
use crate::scripting::helpers::json_to_lua_table;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_region_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // get_entities_in_region(region_id)
    let world_entities_in_region = world.clone();
    let get_entities_in_region = lua.create_function_mut(move |_, region_id: String| {
        let world = world_entities_in_region.borrow();
        Ok(world.entities_in_region(&region_id))
    })?;
    globals.set("get_entities_in_region", get_entities_in_region)?;

    // get_entities_in_region_kind(kind)
    let world_entities_in_region_kind = world.clone();
    let get_entities_in_region_kind = lua.create_function_mut(move |_, kind: String| {
        let world = world_entities_in_region_kind.borrow();
        Ok(world.entities_in_region_kind(&kind))
    })?;
    globals.set("get_entities_in_region_kind", get_entities_in_region_kind)?;

    // get_cells_in_region(region_id)
    let world_cells_in_region = world.clone();
    let get_cells_in_region = lua.create_function_mut(move |lua, region_id: String| {
        let world = world_cells_in_region.borrow();
        let cells = world.cells_in_region(&region_id);
        json_to_lua_table(lua, &serde_json::Value::Array(cells))
    })?;
    globals.set("get_cells_in_region", get_cells_in_region)?;

    // get_cells_in_region_kind(kind)
    let world_cells_in_region_kind = world.clone();
    let get_cells_in_region_kind = lua.create_function_mut(move |lua, kind: String| {
        let world = world_cells_in_region_kind.borrow();
        let cells = world.cells_in_region_kind(&kind);
        json_to_lua_table(lua, &serde_json::Value::Array(cells))
    })?;
    globals.set("get_cells_in_region_kind", get_cells_in_region_kind)?;

    Ok(())
}
