use crate::ecs::world::World;
use crate::map::CellKey;
use crate::scripting::helpers::json_to_lua_table;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers map/topology scripting API into Lua.
pub fn register_map_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // get_map_topology_type()
    let world_topo = world.clone();
    let get_map_topology_type = lua.create_function_mut(move |_, ()| {
        let world = world_topo.borrow();
        let topo = world
            .map
            .as_ref()
            .map(|m| m.topology_type())
            .unwrap_or("none");
        Ok(topo)
    })?;
    globals.set("get_map_topology_type", get_map_topology_type)?;

    // get_all_cells()
    let world_cells = world.clone();
    let get_all_cells = lua.create_function_mut(move |lua, ()| {
        let world = world_cells.borrow();
        let cells = world
            .map
            .as_ref()
            .map(|m| m.all_cells())
            .unwrap_or_default();
        let arr = lua.create_table()?;
        for (i, cell) in cells.iter().enumerate() {
            arr.set(
                i + 1,
                json_to_lua_table(lua, &serde_json::to_value(cell).unwrap())?,
            )?;
        }
        Ok(LuaValue::Table(arr))
    })?;
    globals.set("get_all_cells", get_all_cells)?;

    // add_cell(x, y, z)
    let world_add_cell = world.clone();
    let add_cell = lua.create_function_mut(move |_, (x, y, z): (i32, i32, i32)| {
        let mut world = world_add_cell.borrow_mut();
        if let Some(map) = &mut world.map {
            if let Some(square) = map
                .topology
                .as_any_mut()
                .downcast_mut::<crate::map::SquareGridMap>()
            {
                square.add_cell(x, y, z);
            }
        }
        Ok(())
    })?;
    globals.set("add_cell", add_cell)?;

    // get_neighbors(cell)
    let world_neighbors = world.clone();
    let get_neighbors = lua.create_function_mut(move |lua, cell: LuaValue| {
        let world = world_neighbors.borrow();
        let cell_json = crate::scripting::helpers::lua_value_to_json(lua, cell, None)?;
        let cell_key: CellKey = serde_json::from_value(cell_json).map_err(mlua::Error::external)?;
        let neighbors = world
            .map
            .as_ref()
            .map(|m| m.neighbors(&cell_key))
            .unwrap_or_default();
        let arr = lua.create_table()?;
        for (i, n) in neighbors.iter().enumerate() {
            arr.set(
                i + 1,
                json_to_lua_table(lua, &serde_json::to_value(n).unwrap())?,
            )?;
        }
        Ok(LuaValue::Table(arr))
    })?;
    globals.set("get_neighbors", get_neighbors)?;

    Ok(())
}
