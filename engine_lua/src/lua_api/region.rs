//! Region and zone queries for scripting API.
//!
//! Read queries (`get_entities_in_region[_kind]`, `get_cells_in_region[_kind]`)
//! are ungated. The eight zone management globals mirror the Python/WASM
//! surface with identical names, argument order, and JSON value shapes:
//! `designate_zone(kind, label_or_nil, shape_table)`,
//! `remove_zone(zone_id)`, `rename_zone(zone_id, label)`,
//! `set_zone_kind(zone_id, kind)`, `assign_cells_to_zone(zone_id, cells)`,
//! `unassign_cells_from_zone(zone_id, cells)`, `list_zones()`,
//! `get_zone(zone_id)`. `shape_table` is `{rect = {x0, y0, z, x1, y1}}` for
//! Square-topology rectangles or `{cells = {...}}` for explicit cell lists.
//! Mutators propagate the core colony-gate error outside colony mode.

use crate::helpers::{json_to_lua_table, lua_value_to_json};
use engine_core::ecs::world::{World, ZoneShape};
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Parses a [`ZoneShape`] from the Lua shape table.
///
/// Accepts `{rect = {x0, y0, z, x1, y1}}` (all integers) or
/// `{cells = {...}}` (array of cell values). Anything else errors.
fn parse_zone_shape(lua: &Lua, value: LuaValue) -> LuaResult<ZoneShape> {
    let json = lua_value_to_json(lua, value, None)?;
    let obj = json
        .as_object()
        .ok_or_else(|| mlua::Error::external("shape must be a table with 'rect' or 'cells'"))?;
    if let Some(rect) = obj.get("rect") {
        let get = |key: &str| {
            rect.get(key)
                .and_then(|v| v.as_i64())
                .ok_or_else(|| mlua::Error::external(format!("shape.rect missing integer '{key}'")))
        };
        return Ok(ZoneShape::Rect {
            x0: get("x0")?,
            y0: get("y0")?,
            z: get("z")?,
            x1: get("x1")?,
            y1: get("y1")?,
        });
    }
    if let Some(cells) = obj.get("cells") {
        let arr = cells
            .as_array()
            .ok_or_else(|| mlua::Error::external("shape.cells must be an array"))?;
        return Ok(ZoneShape::Cells(arr.clone()));
    }
    Err(mlua::Error::external(
        "shape must contain 'rect' or 'cells'",
    ))
}

/// Parses an optional label: nil becomes None, strings pass through.
fn parse_optional_label(value: LuaValue) -> LuaResult<Option<String>> {
    match value {
        LuaValue::Nil => Ok(None),
        LuaValue::String(s) => Ok(Some(s.to_str()?.to_string())),
        _ => Err(mlua::Error::external("label must be a string or nil")),
    }
}

/// Parses a cell array argument into JSON values.
fn parse_cells(lua: &Lua, value: LuaValue) -> LuaResult<Vec<serde_json::Value>> {
    let json = lua_value_to_json(lua, value, Some("array"))?;
    json.as_array()
        .cloned()
        .ok_or_else(|| mlua::Error::external("cells must be an array"))
}

/// Register region API
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

    // designate_zone(kind, label_or_nil, shape_table) -> zone id
    let world_designate = world.clone();
    let designate_zone = lua.create_function_mut(
        move |lua, (kind, label, shape): (String, LuaValue, LuaValue)| {
            let mut world = world_designate.borrow_mut();
            let label = parse_optional_label(label)?;
            let zone_shape = parse_zone_shape(lua, shape)?;
            world
                .designate_zone(&kind, label.as_deref(), zone_shape)
                .map_err(mlua::Error::external)
        },
    )?;
    globals.set("designate_zone", designate_zone)?;

    // remove_zone(zone_id) -> boolean
    let world_remove = world.clone();
    let remove_zone = lua.create_function_mut(move |_, zone_id: String| {
        let mut world = world_remove.borrow_mut();
        world.remove_zone(&zone_id).map_err(mlua::Error::external)
    })?;
    globals.set("remove_zone", remove_zone)?;

    // rename_zone(zone_id, label) -> boolean
    let world_rename = world.clone();
    let rename_zone = lua.create_function_mut(move |_, (zone_id, label): (String, String)| {
        let mut world = world_rename.borrow_mut();
        world
            .rename_zone(&zone_id, &label)
            .map_err(mlua::Error::external)
    })?;
    globals.set("rename_zone", rename_zone)?;

    // set_zone_kind(zone_id, kind) -> boolean
    let world_rekind = world.clone();
    let set_zone_kind = lua.create_function_mut(move |_, (zone_id, kind): (String, String)| {
        let mut world = world_rekind.borrow_mut();
        world
            .set_zone_kind(&zone_id, &kind)
            .map_err(mlua::Error::external)
    })?;
    globals.set("set_zone_kind", set_zone_kind)?;

    // assign_cells_to_zone(zone_id, cells) -> boolean
    let world_assign = world.clone();
    let assign_cells_to_zone =
        lua.create_function_mut(move |lua, (zone_id, cells): (String, LuaValue)| {
            let mut world = world_assign.borrow_mut();
            let cells = parse_cells(lua, cells)?;
            world
                .assign_cells_to_zone(&zone_id, cells)
                .map_err(mlua::Error::external)
        })?;
    globals.set("assign_cells_to_zone", assign_cells_to_zone)?;

    // unassign_cells_from_zone(zone_id, cells) -> boolean
    let world_unassign = world.clone();
    let unassign_cells_from_zone =
        lua.create_function_mut(move |lua, (zone_id, cells): (String, LuaValue)| {
            let mut world = world_unassign.borrow_mut();
            let cells = parse_cells(lua, cells)?;
            world
                .unassign_cells_from_zone(&zone_id, cells)
                .map_err(mlua::Error::external)
        })?;
    globals.set("unassign_cells_from_zone", unassign_cells_from_zone)?;

    // list_zones() -> array of {id, label, kind, cell_count}
    let world_list = world.clone();
    let list_zones = lua.create_function_mut(move |lua, ()| {
        let world = world_list.borrow();
        json_to_lua_table(lua, &serde_json::Value::Array(world.list_zones()))
    })?;
    globals.set("list_zones", list_zones)?;

    // get_zone(zone_id) -> table or nil
    let world_get = world.clone();
    let get_zone = lua.create_function_mut(move |lua, zone_id: String| {
        let world = world_get.borrow();
        match world.get_zone(&zone_id) {
            Some(zone) => json_to_lua_table(lua, &zone),
            None => Ok(LuaValue::Nil),
        }
    })?;
    globals.set("get_zone", get_zone)?;

    Ok(())
}
