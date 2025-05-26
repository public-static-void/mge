use crate::ecs::world::World;
use crate::map::CellKey;
use crate::scripting::helpers::{json_to_lua_table, lua_value_to_json};
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

    // add_neighbor(from, to)
    let world_add_neighbor = world.clone();
    let add_neighbor = lua.create_function_mut(move |lua, (from, to): (LuaValue, LuaValue)| {
        let mut world = world_add_neighbor.borrow_mut();
        // Helper to convert Lua table {0,0,0} or {x=0,y=0,z=0} to (i32, i32, i32)
        fn table_to_xyz(_lua: &Lua, val: LuaValue) -> mlua::Result<(i32, i32, i32)> {
            let t = match val {
                LuaValue::Table(t) => t,
                _ => {
                    return Err(mlua::Error::FromLuaConversionError {
                        from: val.type_name(),
                        to: "table".to_string(),
                        message: Some("Expected table".to_string()),
                    });
                }
            };
            // Accept {x=..,y=..,z=..} or {..,..,..}
            let x = t.get("x").or_else(|_| t.get(1))?;
            let y = t.get("y").or_else(|_| t.get(2))?;
            let z = t.get("z").or_else(|_| t.get(3))?;
            Ok((x, y, z))
        }
        let from_xyz = table_to_xyz(lua, from)?;
        let to_xyz = table_to_xyz(lua, to)?;
        if let Some(map) = &mut world.map {
            if let Some(square) = map
                .topology
                .as_any_mut()
                .downcast_mut::<crate::map::SquareGridMap>()
            {
                square.add_neighbor(from_xyz, to_xyz);
            }
        }
        Ok(())
    })?;
    globals.set("add_neighbor", add_neighbor)?;

    // entities_in_cell(cell)
    let world_entities_in_cell = world.clone();
    let entities_in_cell = lua.create_function_mut(move |lua, cell: LuaValue| {
        let world = world_entities_in_cell.borrow();
        let cell_json = crate::scripting::helpers::lua_value_to_json(lua, cell, None)?;
        let cell_key: crate::map::CellKey =
            serde_json::from_value(cell_json).map_err(mlua::Error::external)?;
        let entities = world.entities_in_cell(&cell_key);
        let arr = lua.create_table()?;
        for (i, eid) in entities.iter().enumerate() {
            arr.set(i + 1, *eid)?;
        }
        Ok(LuaValue::Table(arr))
    })?;
    globals.set("entities_in_cell", entities_in_cell)?;

    // --- get_cell_metadata(cell) ---
    let world_get_cell_meta = world.clone();
    let get_cell_metadata = lua.create_function_mut(move |lua, cell: LuaValue| {
        let world = world_get_cell_meta.borrow();
        let cell_json = lua_value_to_json(lua, cell, None)?;
        let cell_key: CellKey = serde_json::from_value(cell_json).map_err(mlua::Error::external)?;
        if let Some(meta) = world.get_cell_metadata(&cell_key) {
            Ok(json_to_lua_table(lua, meta)?)
        } else {
            Ok(LuaValue::Nil)
        }
    })?;
    globals.set("get_cell_metadata", get_cell_metadata)?;

    // --- set_cell_metadata(cell, metadata) ---
    let world_set_cell_meta = world.clone();
    let set_cell_metadata =
        lua.create_function_mut(move |lua, (cell, meta): (LuaValue, LuaValue)| {
            let mut world = world_set_cell_meta.borrow_mut();
            let cell_json = lua_value_to_json(lua, cell, None)?;
            let cell_key: CellKey =
                serde_json::from_value(cell_json).map_err(mlua::Error::external)?;
            let meta_json = lua_value_to_json(lua, meta, None)?;
            world.set_cell_metadata(&cell_key, meta_json);
            Ok(())
        })?;
    globals.set("set_cell_metadata", set_cell_metadata)?;

    Ok(())
}
