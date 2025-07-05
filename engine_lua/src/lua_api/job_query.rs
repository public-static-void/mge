//! Job Query API: list, get, and filter jobs from Lua.

use crate::helpers::json_to_lua_table;
use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers job query functions in Lua:
///   - list_jobs([opts]): returns only active jobs by default;
///     pass {include_terminal=true} to include completed/cancelled/failed jobs.
///   - get_job(job_id): returns job by id or nil.
///   - find_jobs({filters}): advanced query.
pub fn register_job_query_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // list_jobs([opts])
    let world_list = world.clone();
    let list_jobs = lua.create_function(move |lua, opts: Option<Table>| {
        let include_terminal = opts
            .as_ref()
            .and_then(|t| t.get::<Option<bool>>("include_terminal").ok())
            .flatten()
            .unwrap_or(false);

        let world = world_list.borrow();
        let mut jobs = Vec::new();
        if let Some(job_map) = world.components.get("Job") {
            for (eid, comp) in job_map.iter() {
                let mut job = comp.clone();
                job["id"] = serde_json::json!(eid);
                let state = job.get("state").and_then(|v| v.as_str());
                let is_terminal =
                    matches!(state, Some("complete") | Some("failed") | Some("cancelled"));
                if !include_terminal && is_terminal {
                    continue;
                }
                jobs.push(json_to_lua_table(lua, &job)?);
            }
        }
        lua.create_sequence_from(jobs)
    })?;
    globals.set("list_jobs", list_jobs)?;

    // get_job(job_id)
    let world_get = world.clone();
    let get_job = lua.create_function(move |lua, job_id: u32| {
        let world = world_get.borrow();
        if let Some(job) = world.get_component(job_id, "Job") {
            let mut job = job.clone();
            job["id"] = serde_json::json!(job_id);
            json_to_lua_table(lua, &job)
        } else {
            Ok(Value::Nil)
        }
    })?;
    globals.set("get_job", get_job)?;

    // find_jobs({state=..., job_type=..., assigned_to=..., category=...})
    let world_find = world.clone();
    let find_jobs = lua.create_function(move |lua, filter: Option<Table>| {
        let world = world_find.borrow();
        let (state, job_type, assigned_to, category) = if let Some(filter) = filter {
            (
                filter.get::<Option<String>>("state")?,
                filter.get::<Option<String>>("job_type")?,
                filter.get::<Option<u32>>("assigned_to")?,
                filter.get::<Option<String>>("category")?,
            )
        } else {
            (None, None, None, None)
        };
        let mut jobs = Vec::new();
        if let Some(job_map) = world.components.get("Job") {
            for (eid, comp) in job_map.iter() {
                let mut job = comp.clone();
                if let Some(ref s) = state {
                    if job.get("state").and_then(|v| v.as_str()) != Some(s) {
                        continue;
                    }
                }
                if let Some(ref jt) = job_type {
                    if job.get("job_type").and_then(|v| v.as_str()) != Some(jt) {
                        continue;
                    }
                }
                if let Some(aid) = assigned_to {
                    if job.get("assigned_to").and_then(|v| v.as_u64()) != Some(aid as u64) {
                        continue;
                    }
                }
                if let Some(ref cat) = category {
                    if job.get("category").and_then(|v| v.as_str()) != Some(cat) {
                        continue;
                    }
                }
                job["id"] = serde_json::json!(eid);
                jobs.push(json_to_lua_table(lua, &job)?);
            }
        }
        lua.create_sequence_from(jobs)
    })?;
    globals.set("find_jobs", find_jobs)?;

    // advance_job_state(job_id)
    let world_advance = world.clone();
    let advance_job_state = lua.create_function(move |_, job_id: u32| {
        let mut world = world_advance.borrow_mut();
        let job = world
            .get_component(job_id, "Job")
            .ok_or_else(|| mlua::Error::external(format!("No job with id {job_id}")))?
            .clone();
        let new_job =
            engine_core::systems::job::system::process::process_job(&mut world, None, job_id, job);
        world
            .set_component(job_id, "Job", new_job)
            .map_err(|e| mlua::Error::external(format!("Failed to set job: {e}")))?;
        Ok(())
    })?;
    globals.set("advance_job_state", advance_job_state)?;

    // get_job_children(job_id)
    let world_get = world.clone();
    let get_job_children = lua.create_function(move |lua, job_id: u32| {
        let world = world_get.borrow();
        let job = world
            .get_component(job_id, "Job")
            .ok_or_else(|| mlua::Error::external(format!("No job with id {job_id}")))?;
        let children = job
            .get("children")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        crate::helpers::json_to_lua_table(lua, &children)
    })?;
    globals.set("get_job_children", get_job_children)?;

    // set_job_children(job_id, children)
    let world_set = world.clone();
    let set_job_children =
        lua.create_function_mut(move |lua, (job_id, children): (u32, mlua::Value)| {
            let children_json = crate::helpers::lua_value_to_json(lua, children, None)?;
            let mut world = world_set.borrow_mut();
            let mut job = world
                .get_component(job_id, "Job")
                .cloned()
                .ok_or_else(|| mlua::Error::external(format!("No job with id {job_id}")))?;
            job["children"] = children_json;
            world
                .set_component(job_id, "Job", job)
                .map_err(|e| mlua::Error::external(format!("Failed to set job: {e}")))?;
            Ok(())
        })?;
    globals.set("set_job_children", set_job_children)?;

    // get_job_dependencies(job_id)
    let world_get_deps = world.clone();
    let get_job_dependencies = lua.create_function(move |lua, job_id: u32| {
        let world = world_get_deps.borrow();
        let job = world
            .get_component(job_id, "Job")
            .ok_or_else(|| mlua::Error::external(format!("No job with id {job_id}")))?;
        let deps = job
            .get("dependencies")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        crate::helpers::json_to_lua_table(lua, &deps)
    })?;
    globals.set("get_job_dependencies", get_job_dependencies)?;

    // set_job_dependencies(job_id, deps)
    let world_set_deps = world.clone();
    let set_job_dependencies =
        lua.create_function_mut(move |lua, (job_id, deps): (u32, mlua::Value)| {
            let deps_json = crate::helpers::lua_value_to_json(lua, deps, None)?;
            let mut world = world_set_deps.borrow_mut();
            let mut job = world
                .get_component(job_id, "Job")
                .cloned()
                .ok_or_else(|| mlua::Error::external(format!("No job with id {job_id}")))?;
            job["dependencies"] = deps_json;
            world
                .set_component(job_id, "Job", job)
                .map_err(|e| mlua::Error::external(format!("Failed to set job: {e}")))?;
            Ok(())
        })?;
    globals.set("set_job_dependencies", set_job_dependencies)?;

    Ok(())
}
