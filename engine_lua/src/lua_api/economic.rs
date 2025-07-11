//! Economic system Lua helpers: stockpile, production job, resource modification.

use crate::helpers::lua_error_from_any;
use engine_core::ecs::world::World;
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
                crate::helpers::json_to_lua_table(lua, resources)
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
            crate::helpers::json_to_lua_table(lua, job)
        } else {
            Ok(mlua::Value::Nil)
        }
    })?;
    globals.set("get_production_job", get_production_job)?;

    // get_production_job_progress(entity)
    let world_get_progress = world.clone();
    let get_production_job_progress = lua.create_function_mut(move |_, entity: u32| {
        let world = world_get_progress.borrow();
        if let Some(job) = world.get_component(entity, "ProductionJob") {
            Ok(job.get("progress").and_then(|v| v.as_i64()).unwrap_or(0))
        } else {
            Ok(0)
        }
    })?;
    globals.set("get_production_job_progress", get_production_job_progress)?;

    // set_production_job_progress(entity, value)
    let world_set_progress = world.clone();
    let set_production_job_progress =
        lua.create_function_mut(move |_, (entity, value): (u32, i64)| {
            let mut world = world_set_progress.borrow_mut();
            if let Some(mut job) = world.get_component(entity, "ProductionJob").cloned() {
                job["progress"] = serde_json::json!(value);
                world
                    .set_component(entity, "ProductionJob", job)
                    .map_err(mlua::Error::external)?;
            }
            Ok(())
        })?;
    globals.set("set_production_job_progress", set_production_job_progress)?;

    // get_production_job_state(entity)
    let world_get_state = world.clone();
    let get_production_job_state = lua.create_function_mut(move |_, entity: u32| {
        let world = world_get_state.borrow();
        if let Some(job) = world.get_component(entity, "ProductionJob") {
            Ok(job
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("pending")
                .to_string())
        } else {
            Ok("pending".to_string())
        }
    })?;
    globals.set("get_production_job_state", get_production_job_state)?;

    // set_production_job_state(entity, value)
    let world_set_state = world.clone();
    let set_production_job_state =
        lua.create_function_mut(move |_, (entity, value): (u32, String)| {
            let mut world = world_set_state.borrow_mut();
            if let Some(mut job) = world.get_component(entity, "ProductionJob").cloned() {
                job["state"] = serde_json::json!(value);
                world
                    .set_component(entity, "ProductionJob", job)
                    .map_err(mlua::Error::external)?;
            }
            Ok(())
        })?;
    globals.set("set_production_job_state", set_production_job_state)?;

    // modify_stockpile_resource(entity, kind, delta)
    let world_modify_stockpile = world.clone();
    let modify_stockpile_resource =
        lua.create_function_mut(move |lua, (entity, kind, delta): (u32, String, f64)| {
            let mut world = world_modify_stockpile.borrow_mut();
            world
                .modify_stockpile_resource(entity, &kind, delta)
                .map_err(|e| lua_error_from_any(lua, e))
        })?;
    globals.set("modify_stockpile_resource", modify_stockpile_resource)?;

    Ok(())
}
