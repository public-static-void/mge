use crate::helpers::{json_to_lua_table, lua_table_to_json, lua_value_to_json};
use engine_core::ecs::world::World;
use engine_core::map::CellKey;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Helper: parse a CellKey from a serde_json::Value, supporting both enum and ergonomic square form.
fn parse_cell_key(cell_json: serde_json::Value) -> Result<CellKey, mlua::Error> {
    // Try standard enum deserialization first
    if let Ok(cell_key) = serde_json::from_value::<CellKey>(cell_json.clone()) {
        return Ok(cell_key);
    }
    // Fallback: treat as Square if x/y/z fields are present
    if let serde_json::Value::Object(ref obj) = cell_json
        && obj.contains_key("x")
        && obj.contains_key("y")
        && obj.contains_key("z")
    {
        return Ok(CellKey::Square {
            x: obj["x"].as_i64().unwrap() as i32,
            y: obj["y"].as_i64().unwrap() as i32,
            z: obj["z"].as_i64().unwrap() as i32,
        });
    }
    Err(mlua::Error::external("Invalid cell key format"))
}

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
        if let Some(map) = &mut world.map
            && let Some(square) = map
                .topology
                .as_any_mut()
                .downcast_mut::<engine_core::map::SquareGridMap>()
        {
            square.add_cell(x, y, z);
        }
        Ok(())
    })?;
    globals.set("add_cell", add_cell)?;

    // get_neighbors(cell)
    let world_neighbors = world.clone();
    let get_neighbors = lua.create_function_mut(move |lua, cell: LuaValue| {
        let world = world_neighbors.borrow();
        let cell_json = lua_value_to_json(lua, cell, None)?;
        let cell_key = parse_cell_key(cell_json)?;
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
            let x = t.get("x").or_else(|_| t.get(1))?;
            let y = t.get("y").or_else(|_| t.get(2))?;
            let z = t.get("z").or_else(|_| t.get(3))?;
            Ok((x, y, z))
        }
        let from_xyz = table_to_xyz(lua, from)?;
        let to_xyz = table_to_xyz(lua, to)?;
        if let Some(map) = &mut world.map
            && let Some(square) = map
                .topology
                .as_any_mut()
                .downcast_mut::<engine_core::map::SquareGridMap>()
        {
            square.add_neighbor(from_xyz, to_xyz);
        }
        Ok(())
    })?;
    globals.set("add_neighbor", add_neighbor)?;

    // entities_in_cell(cell)
    let world_entities_in_cell = world.clone();
    let entities_in_cell = lua.create_function_mut(move |lua, cell: LuaValue| {
        let world = world_entities_in_cell.borrow();
        let cell_json = lua_value_to_json(lua, cell, None)?;
        let cell_key = parse_cell_key(cell_json)?;
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
        let cell_key = parse_cell_key(cell_json)?;
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
            let cell_key = parse_cell_key(cell_json)?;
            let meta_json = lua_value_to_json(lua, meta, None)?;
            world.set_cell_metadata(&cell_key, meta_json);
            Ok(())
        })?;
    globals.set("set_cell_metadata", set_cell_metadata)?;

    // find_path(start_cell, goal_cell)
    let world_find_path = world.clone();
    let find_path = lua.create_function_mut(move |lua, (start, goal): (LuaValue, LuaValue)| {
        let world = world_find_path.borrow();
        let start_json = lua_value_to_json(lua, start, None)?;
        let goal_json = lua_value_to_json(lua, goal, None)?;
        let start_key = parse_cell_key(start_json)?;
        let goal_key = parse_cell_key(goal_json)?;
        if let Some(result) = world.find_path(&start_key, &goal_key) {
            let arr = lua.create_table()?;
            for (i, cell) in result.path.iter().enumerate() {
                arr.set(
                    i + 1,
                    json_to_lua_table(lua, &serde_json::to_value(cell).unwrap())?,
                )?;
            }
            let out = lua.create_table()?;
            out.set("path", arr)?;
            out.set("total_cost", result.total_cost)?;
            Ok(LuaValue::Table(out))
        } else {
            Ok(LuaValue::Nil)
        }
    })?;
    globals.set("find_path", find_path)?;

    // apply_generated_map(map_table)
    let world_for_apply = world.clone();
    let apply_generated_map = lua.create_function_mut(move |lua, map_table: Table| {
        let map_json = lua_table_to_json(lua, &map_table, None)?;
        let mut world = world_for_apply.borrow_mut();
        world
            .apply_generated_map(&map_json)
            .map_err(mlua::Error::external)
    })?;
    globals.set("apply_generated_map", apply_generated_map)?;

    // get_map_topology_type()
    let world_for_topology = world.clone();
    let get_map_topology_type = lua.create_function(move |_, ()| {
        let world = world_for_topology.borrow();
        Ok(world
            .map
            .as_ref()
            .map(|m| m.topology_type().to_string())
            .unwrap_or_else(|| "none".to_string()))
    })?;
    globals.set("get_map_topology_type", get_map_topology_type)?;

    // get_map_cell_count()
    let world_for_cell_count = world.clone();
    let get_map_cell_count = lua.create_function(move |_, ()| {
        let world = world_for_cell_count.borrow();
        Ok(world.map.as_ref().map(|m| m.all_cells().len()).unwrap_or(0))
    })?;
    globals.set("get_map_cell_count", get_map_cell_count)?;

    Ok(())
}
