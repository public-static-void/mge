//! Vehicle Lua helpers: embark, disembark, path assignment, occupancy query.
//!
//! Exposes the identical 5-op bridge surface as globals (the sandbox blocks
//! `require`, so no module table is used):
//! `embark_vehicle(vehicle_id, rider_id) -> ok, err`,
//! `disembark_vehicle(rider_id) -> ok, err`,
//! `assign_vehicle_path(vehicle_id, goal_cell) -> steps`,
//! `get_vehicle_occupants(vehicle_id) -> [entity ids]`,
//! `is_mounted(rider_id) -> bool`.
//! The `goal_cell` table shape matches the cell tables `assign_move_path`
//! accepts (enum form first, then the shared bare x/y/z fallback).

use crate::helpers::lua_value_to_json;
use crate::lua_api::map::parse_cell_key;
use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the vehicle API globals.
pub fn register_vehicle_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // embark_vehicle(vehicle_id, rider_id) -> ok, err
    let world_embark = world.clone();
    let embark_vehicle = lua.create_function_mut(
        move |_, (vehicle_id, rider_id): (u32, u32)| -> LuaResult<(bool, Option<String>)> {
            let mut world = world_embark.borrow_mut();
            match world.embark(vehicle_id, rider_id) {
                Ok(()) => Ok((true, None)),
                Err(err) => Ok((false, Some(err))),
            }
        },
    )?;
    globals.set("embark_vehicle", embark_vehicle)?;

    // disembark_vehicle(rider_id) -> ok, err
    let world_disembark = world.clone();
    let disembark_vehicle = lua.create_function_mut(
        move |_, rider_id: u32| -> LuaResult<(bool, Option<String>)> {
            let mut world = world_disembark.borrow_mut();
            match world.disembark(rider_id) {
                Ok(()) => Ok((true, None)),
                Err(err) => Ok((false, Some(err))),
            }
        },
    )?;
    globals.set("disembark_vehicle", disembark_vehicle)?;

    // assign_vehicle_path(vehicle_id, goal_cell) -> steps
    let world_assign = world.clone();
    let assign_vehicle_path = lua.create_function_mut(
        move |lua, (vehicle_id, goal): (u32, LuaValue)| -> LuaResult<i64> {
            let goal_json = lua_value_to_json(lua, goal, None)?;
            let goal_cell = parse_cell_key(goal_json)?;
            let mut world = world_assign.borrow_mut();
            world
                .assign_vehicle_path(vehicle_id, &goal_cell)
                .map(|steps| steps as i64)
                .map_err(mlua::Error::external)
        },
    )?;
    globals.set("assign_vehicle_path", assign_vehicle_path)?;

    // get_vehicle_occupants(vehicle_id) -> [entity ids]
    let world_occupants = world.clone();
    let get_vehicle_occupants =
        lua.create_function_mut(move |_, vehicle_id: u32| -> LuaResult<Vec<u32>> {
            let world = world_occupants.borrow();
            Ok(world.vehicle_occupants(vehicle_id))
        })?;
    globals.set("get_vehicle_occupants", get_vehicle_occupants)?;

    // is_mounted(rider_id) -> bool
    let world_mounted = world;
    let is_mounted = lua.create_function_mut(move |_, rider_id: u32| -> LuaResult<bool> {
        let world = world_mounted.borrow();
        Ok(world.is_mounted(rider_id))
    })?;
    globals.set("is_mounted", is_mounted)?;

    Ok(())
}
