use crate::helpers::{json_to_lua_table, lua_table_to_json, lua_value_to_json};
use engine_core::ecs::world::World;
use engine_core::map::Map;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the multi-scale map navigation scripting API into Lua.
///
/// Exposes the 9-function surface identically to Python and WASM (issue 61
/// parity invariant): the 4 M1 registry functions (`register_map`,
/// `set_active_map`, `get_map_names`, `get_active_map_name`) and the 5 M2
/// transition/mapping functions (`link_maps`, `enter_map`, `exit_map`,
/// `map_cell`, `unmap_cell`). Cells are Position-shaped tables
/// (`{Square={x,y,z}}` / `{Hex={q,r,z}}` / `{Province={id}}`); `map_cell` /
/// `unmap_cell` return `nil` when unlinked; errors are deterministic (not
/// panics).
pub fn register_multiscale_map_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // register_map(name, map_json)
    let world_register = world.clone();
    let register_map =
        lua.create_function_mut(move |lua, (name, map_table): (String, Table)| {
            let map_json = lua_table_to_json(lua, &map_table, None)?;
            let map = Map::from_json(&map_json).map_err(mlua::Error::external)?;
            let mut world = world_register.borrow_mut();
            world
                .register_map(&name, map)
                .map_err(mlua::Error::external)
        })?;
    globals.set("register_map", register_map)?;

    // set_active_map(name)
    let world_set_active = world.clone();
    let set_active_map = lua.create_function_mut(move |_, name: String| {
        let mut world = world_set_active.borrow_mut();
        world.set_active_map(&name).map_err(mlua::Error::external)
    })?;
    globals.set("set_active_map", set_active_map)?;

    // get_map_names()
    let world_names = world.clone();
    let get_map_names = lua.create_function_mut(move |lua, ()| {
        let world = world_names.borrow();
        let names = world.get_map_names();
        let arr = lua.create_table()?;
        for (i, name) in names.iter().enumerate() {
            arr.set(i + 1, name.as_str())?;
        }
        Ok(LuaValue::Table(arr))
    })?;
    globals.set("get_map_names", get_map_names)?;

    // get_active_map_name()
    let world_active_name = world.clone();
    let get_active_map_name = lua.create_function_mut(move |_, ()| {
        let world = world_active_name.borrow();
        Ok(world.get_active_map_name())
    })?;
    globals.set("get_active_map_name", get_active_map_name)?;

    // link_maps(source_map, source_cell, target_map, target_cell)
    let world_link = world.clone();
    let link_maps = lua.create_function_mut(
        move |lua,
              (source_map, source_cell, target_map, target_cell): (
            String,
            LuaValue,
            String,
            LuaValue,
        )| {
            let source_json = lua_value_to_json(lua, source_cell, None)?;
            let source_key = super::map::parse_cell_key(source_json)?;
            let target_json = lua_value_to_json(lua, target_cell, None)?;
            let target_key = super::map::parse_cell_key(target_json)?;
            let mut world = world_link.borrow_mut();
            world
                .link_maps(&source_map, source_key, &target_map, target_key)
                .map_err(mlua::Error::external)
        },
    )?;
    globals.set("link_maps", link_maps)?;

    // enter_map(name, entry_cell)
    let world_enter = world.clone();
    let enter_map =
        lua.create_function_mut(move |lua, (name, entry_cell): (String, LuaValue)| {
            let entry_json = lua_value_to_json(lua, entry_cell, None)?;
            let entry_key = super::map::parse_cell_key(entry_json)?;
            let mut world = world_enter.borrow_mut();
            world
                .enter_map(&name, entry_key)
                .map_err(mlua::Error::external)
        })?;
    globals.set("enter_map", enter_map)?;

    // exit_map()
    let world_exit = world.clone();
    let exit_map = lua.create_function_mut(move |_, ()| {
        let mut world = world_exit.borrow_mut();
        world.exit_map().map_err(mlua::Error::external)
    })?;
    globals.set("exit_map", exit_map)?;

    // map_cell(source_map, source_cell)
    let world_map_cell = world.clone();
    let map_cell =
        lua.create_function_mut(move |lua, (source_map, source_cell): (String, LuaValue)| {
            let source_json = lua_value_to_json(lua, source_cell, None)?;
            let source_key = super::map::parse_cell_key(source_json)?;
            let world = world_map_cell.borrow();
            match world.map_cell(&source_map, &source_key) {
                Some(cell) => Ok(json_to_lua_table(
                    lua,
                    &serde_json::to_value(cell).unwrap(),
                )?),
                None => Ok(LuaValue::Nil),
            }
        })?;
    globals.set("map_cell", map_cell)?;

    // unmap_cell(target_map, target_cell)
    let world_unmap_cell = world.clone();
    let unmap_cell =
        lua.create_function_mut(move |lua, (target_map, target_cell): (String, LuaValue)| {
            let target_json = lua_value_to_json(lua, target_cell, None)?;
            let target_key = super::map::parse_cell_key(target_json)?;
            let world = world_unmap_cell.borrow();
            match world.unmap_cell(&target_map, &target_key) {
                Some(cell) => Ok(json_to_lua_table(
                    lua,
                    &serde_json::to_value(cell).unwrap(),
                )?),
                None => Ok(LuaValue::Nil),
            }
        })?;
    globals.set("unmap_cell", unmap_cell)?;

    Ok(())
}
