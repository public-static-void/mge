use std::cell::RefCell;
use std::rc::Rc;

use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table, Value};

/// Registers the job mutation API
pub fn register_job_mutation_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // set_job_field(job_id, field, value)
    let world_set = world.clone();
    let set_job_field =
        lua.create_function_mut(move |lua, (job_id, field, value): (u32, String, Value)| {
            let mut world = world_set.borrow_mut();
            if let Some(mut job) = world.get_component(job_id, "Job").cloned() {
                let val = crate::helpers::lua_value_to_json(lua, value, None)?;
                job[&field] = val;
                world
                    .set_component(job_id, "Job", job)
                    .map_err(|e| crate::helpers::lua_error_from_any(lua, e))
            } else {
                Err(crate::helpers::lua_error_msg(lua, "Job not found"))
            }
        })?;
    globals.set("set_job_field", set_job_field)?;

    // update_job(job_id, fields)
    let world_update = world.clone();
    let update_job = lua.create_function_mut(move |lua, (job_id, fields): (u32, Table)| {
        let mut world = world_update.borrow_mut();
        if let Some(mut job) = world.get_component(job_id, "Job").cloned() {
            let extra = crate::helpers::lua_table_to_json(lua, &fields, None)?;
            if let Some(obj) = extra.as_object() {
                for (k, v) in obj {
                    job[k] = v.clone();
                }
            }
            world
                .set_component(job_id, "Job", job)
                .map_err(|e| crate::helpers::lua_error_from_any(lua, e))
        } else {
            Err(crate::helpers::lua_error_msg(lua, "Job not found"))
        }
    })?;
    globals.set("update_job", update_job)?;

    Ok(())
}
