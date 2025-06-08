use crate::helpers::{json_to_lua_table, lua_error_from_any, lua_error_msg, lua_table_to_json};
use engine_core::ecs::world::World;
use mlua::{Function, Lua, RegistryKey, Result as LuaResult, Table};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub fn register_system_functions(
    lua: Rc<Lua>,
    globals: &Table,
    world: Rc<RefCell<World>>,
    lua_systems: Rc<RefCell<HashMap<String, RegistryKey>>>,
) -> LuaResult<()> {
    // register_system
    let lua_systems_outer = Rc::clone(&lua_systems);
    let world_rc = world.clone();
    let lua_outer = lua.clone();
    let register_system = lua.create_function_mut(
        move |_, (name, func, opts): (String, Function, Option<Table>)| {
            let key = lua_outer.create_registry_value(func)?;
            lua_systems_outer.borrow_mut().insert(name.clone(), key);

            let mut dependencies = Vec::new();
            if let Some(opts) = opts {
                if let Ok(dep_table) = opts.get::<Table>("dependencies") {
                    for dep in dep_table.sequence_values::<String>() {
                        dependencies.push(dep?);
                    }
                }
            }

            let system_name_for_closure = name.clone();
            let system_name_for_fn = system_name_for_closure.clone();
            let lua_systems_inner = Rc::clone(&lua_systems_outer);
            let lua_inner = lua_outer.clone();

            world_rc.borrow_mut().register_dynamic_system_with_deps(
                &system_name_for_closure,
                dependencies,
                move |_world, dt| {
                    let binding = lua_systems_inner.borrow();
                    let key = binding
                        .get(&system_name_for_fn)
                        .expect("Lua system not found");
                    let func: Function = lua_inner
                        .registry_value(key)
                        .expect("Invalid Lua registry key");
                    let _ = func.call::<()>(dt);
                },
            );

            Ok(())
        },
    )?;
    globals.set("register_system", register_system)?;

    // run_system
    let lua_systems_ref = Rc::clone(&lua_systems);
    let run_system = lua.create_function_mut(move |lua, name: String| {
        let systems = lua_systems_ref.borrow();
        if let Some(key) = systems.get(&name) {
            let func: Function = lua.registry_value(key)?;
            func.call::<()>(())?;
            Ok(())
        } else {
            Err(lua_error_msg(lua, "system not found"))
        }
    })?;
    globals.set("run_system", run_system)?;

    // run_native_system
    let world_native_run = world.clone();
    let lua_for_native = lua.clone();
    let run_native_system = lua.create_function_mut(move |_, name: String| {
        let mut world = world_native_run.borrow_mut();
        world
            .run_system(&name, None)
            .map_err(|e| lua_error_from_any(&lua_for_native, e))
    })?;
    globals.set("run_native_system", run_native_system)?;

    // assign_job(entity, job_type, fields)
    let world_assign_job = world.clone();
    let assign_job = {
        lua.create_function_mut(
            move |lua, (entity, job_type, fields): (u32, String, Option<Table>)| {
                let mut world = world_assign_job.borrow_mut();
                let mut job_val = serde_json::json!({
                    "job_type": job_type,
                    "status": "pending",
                    "progress": 0.0
                });
                if let Some(tbl) = fields {
                    let extra: JsonValue = lua_table_to_json(lua, &tbl, None)?;
                    if let Some(obj) = extra.as_object() {
                        for (k, v) in obj {
                            job_val[k] = v.clone();
                        }
                    }
                }
                world
                    .set_component(entity, "Job", job_val)
                    .map_err(|e| lua_error_from_any(lua, e))
            },
        )
    }?;
    globals.set("assign_job", assign_job)?;

    // poll_ecs_event
    let world_take_events = world.clone();
    let poll_ecs_event = lua.create_function_mut(move |lua, event_type: String| {
        let mut world = world_take_events.borrow_mut();
        let events = world.take_events(&event_type);
        let tbl = lua.create_table()?;
        for (i, val) in events.into_iter().enumerate() {
            tbl.set(i + 1, json_to_lua_table(lua, &val)?)?;
        }
        Ok(tbl)
    })?;
    globals.set("poll_ecs_event", poll_ecs_event)?;

    // get_job_types
    let world_for_job_types = world.clone();
    let get_job_types = lua.create_function(move |_, ()| {
        let world = world_for_job_types.borrow();
        let job_types = world.job_types.job_type_names();
        Ok(job_types)
    })?;
    globals.set("get_job_types", get_job_types)?;

    // register_job_type
    let world_for_jobs = world.clone();
    let lua_clone = lua.clone();
    let register_job_type =
        lua.create_function_mut(move |_, (name, func): (String, Function)| {
            let key = lua_clone.create_registry_value(func)?;
            let mut world = world_for_jobs.borrow_mut();
            world.job_types.register_lua(&name, key);
            Ok(())
        })?;
    globals.set("register_job_type", register_job_type)?;

    Ok(())
}
