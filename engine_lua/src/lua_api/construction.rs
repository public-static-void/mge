//! Construction Lua helpers: blueprint placement, state query, cancel, demolish.
//!
//! Exposes the identical 4-op bridge surface as globals (the sandbox blocks
//! `require`, so no module table is used):
//! `place_blueprint(building_type, cell, required_materials, required_work)`,
//! `get_construction_state(site_id)`, `cancel_construction(site_id)`,
//! `demolish_building(building_id)`. All four return a mode-gated error
//! outside colony mode instead of diverging silently.

use crate::helpers::{json_to_lua_table, lua_value_to_json};
use crate::lua_api::map::parse_cell_key;
use engine_core::ecs::world::World;
use engine_core::map::CellKey;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Parses `required_materials` (array of `{kind, amount}`) from a Lua value.
fn parse_materials(lua: &Lua, value: LuaValue) -> LuaResult<Vec<(String, i64)>> {
    let json = lua_value_to_json(lua, value, Some("array"))?;
    let arr = json
        .as_array()
        .ok_or_else(|| mlua::Error::external("required_materials must be an array"))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let kind = item
            .get("kind")
            .and_then(|v| v.as_str())
            .ok_or_else(|| mlua::Error::external("material entry missing string 'kind'"))?
            .to_string();
        let amount = item
            .get("amount")
            .and_then(|v| {
                v.as_i64()
                    .or_else(|| v.as_u64().and_then(|u| i64::try_from(u).ok()))
            })
            .ok_or_else(|| mlua::Error::external("material entry missing integer 'amount'"))?;
        out.push((kind, amount));
    }
    Ok(out)
}

/// Parses a [`CellKey`] from a Lua cell value via the shared map parser
/// (enum form first, then pos-wrapped and bare x/y/z fallbacks).
fn parse_cell(lua: &Lua, value: LuaValue) -> LuaResult<CellKey> {
    let json = lua_value_to_json(lua, value, None)?;
    parse_cell_key(json)
}

/// Registers the construction API globals.
pub fn register_construction_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // place_blueprint(building_type, cell, required_materials, required_work) -> site id
    let world_place = world.clone();
    let place_blueprint = lua.create_function_mut(
        move |lua,
              (building_type, cell, materials, required_work): (
            String,
            LuaValue,
            LuaValue,
            i64,
        )| {
            let mut world = world_place.borrow_mut();
            if world.get_mode() != "colony" {
                return Err(mlua::Error::external(format!(
                    "place_blueprint: world is in '{}' mode; construction requires 'colony' mode",
                    world.get_mode()
                )));
            }
            let cell_key = parse_cell(lua, cell)?;
            let material_list = parse_materials(lua, materials)?;
            engine_core::systems::construction::place_blueprint(
                &mut world,
                &building_type,
                &cell_key,
                &material_list,
                required_work,
            )
            .map_err(mlua::Error::external)
        },
    )?;
    globals.set("place_blueprint", place_blueprint)?;

    // get_construction_state(site_id) -> { state, progress, required_work, building_type }
    let world_state = world.clone();
    let get_construction_state = lua.create_function_mut(move |lua, site_id: u32| {
        let world = world_state.borrow();
        if world.get_mode() != "colony" {
            return Err(mlua::Error::external(format!(
                "get_construction_state: world is in '{}' mode; construction requires 'colony' mode",
                world.get_mode()
            )));
        }
        let state = engine_core::systems::construction::get_construction_state(&world, site_id)
            .map_err(mlua::Error::external)?;
        json_to_lua_table(lua, &state)
    })?;
    globals.set("get_construction_state", get_construction_state)?;

    // cancel_construction(site_id) -> true
    let world_cancel = world.clone();
    let cancel_construction = lua.create_function_mut(move |_, site_id: u32| {
        let mut world = world_cancel.borrow_mut();
        if world.get_mode() != "colony" {
            return Err(mlua::Error::external(format!(
                "cancel_construction: world is in '{}' mode; construction requires 'colony' mode",
                world.get_mode()
            )));
        }
        engine_core::systems::construction::cancel_construction(&mut world, site_id)
            .map_err(mlua::Error::external)
    })?;
    globals.set("cancel_construction", cancel_construction)?;

    // demolish_building(building_id) -> true
    let world_demolish = world.clone();
    let demolish_building = lua.create_function_mut(move |_, building_id: u32| {
        let mut world = world_demolish.borrow_mut();
        if world.get_mode() != "colony" {
            return Err(mlua::Error::external(format!(
                "demolish_building: world is in '{}' mode; construction requires 'colony' mode",
                world.get_mode()
            )));
        }
        engine_core::systems::construction::demolish_building(&mut world, building_id)
            .map_err(mlua::Error::external)
    })?;
    globals.set("demolish_building", demolish_building)?;

    Ok(())
}
