use crate::helpers::{json_to_lua_table, lua_error_from_any, lua_table_to_json};
use engine_core::ecs::world::World;
use mlua::{Function, Lua, LuaSerdeExt, RegistryKey, Result as LuaResult, Table};
use once_cell::sync::Lazy;
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

thread_local! {
    static LUA_JOB_HANDLERS: RefCell<HashMap<String, RegistryKey>> = RefCell::new(HashMap::new());
}

#[derive(Clone)]
pub struct LuaJobCall {
    pub job_type: String,
    pub entity: u32,
    pub assigned_to: u32,
    pub job_id: u32,
    pub job: serde_json::Value,
}

pub static LUA_JOB_CALL_QUEUE: Lazy<Arc<Mutex<Vec<LuaJobCall>>>> =
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

pub fn register_job_system_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // assign_job(entity, job_type, fields)
    let world_assign_job = world.clone();
    let assign_job = lua.create_function_mut(
        move |lua, (entity, job_type, fields): (u32, String, Option<Table>)| {
            let mut world = world_assign_job.borrow_mut();
            let mut job_val = serde_json::json!({
                "id": entity,
                "job_type": job_type,
                "state": "pending",
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
    )?;
    globals.set("assign_job", assign_job)?;

    // get_job_types()
    let world_for_job_types = world.clone();
    let get_job_types = lua.create_function(move |_, ()| {
        let world = world_for_job_types.borrow();
        let job_types = world.job_types.job_type_names();
        let job_types_owned: Vec<String> = job_types.into_iter().map(|s| s.to_string()).collect();
        Ok(job_types_owned)
    })?;
    globals.set("get_job_types", get_job_types)?;

    // register_job_type(name, func)
    let world_for_jobs = world.clone();
    let lua_clone = lua.clone();
    let register_job_type =
        lua.create_function_mut(move |_, (name, func): (String, Function)| {
            let key = lua_clone.create_registry_value(func)?;
            LUA_JOB_HANDLERS.with(|handlers| {
                handlers.borrow_mut().insert(name.clone(), key);
            });
            let mut world = world_for_jobs.borrow_mut();
            world.job_types.register_lua(&name, name.clone());

            let job_type_name = name.clone();
            world.register_job_handler(&name, move |_world, assigned_to, job_id, job| {
                let entity = job.get("id").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let call = LuaJobCall {
                    job_type: job_type_name.clone(),
                    entity,
                    assigned_to,
                    job_id,
                    job: job.clone(),
                };
                LUA_JOB_CALL_QUEUE.lock().unwrap().push(call);
                job.clone()
            });

            Ok(())
        })?;
    globals.set("register_job_type", register_job_type)?;

    // get_job_type_metadata(name)
    let world_for_job_type_metadata = world.clone();
    let get_job_type_metadata = lua.create_function(move |lua, name: String| {
        let world = world_for_job_type_metadata.borrow();
        if let Some(data) = world.job_types.get_data(&name) {
            let json = serde_json::to_value(data).map_err(|e| {
                mlua::Error::external(format!("Failed to serialize JobTypeData: {e}"))
            })?;
            crate::helpers::json_to_lua_table(lua, &json)
        } else {
            Ok(mlua::Value::Nil)
        }
    })?;
    globals.set("get_job_type_metadata", get_job_type_metadata)?;

    Ok(())
}

/// Processes all queued Lua job handler calls.
/// This must be called on the main thread, typically once per tick after ECS/job system runs.
pub fn process_lua_job_calls(lua: &Lua, world: &Rc<RefCell<World>>) {
    let mut queue = LUA_JOB_CALL_QUEUE.lock().unwrap();
    while let Some(call) = queue.pop() {
        LUA_JOB_HANDLERS.with(|handlers| {
            if let Some(key) = handlers.borrow().get(&call.job_type) {
                let func: Function = lua.registry_value(key).expect("Invalid Lua registry key");
                let job_table = json_to_lua_table(lua, &call.job).unwrap();
                let progress_json = call.job.get("progress").cloned().unwrap_or_default();
                let progress_lua = lua.to_value(&progress_json).unwrap();
                let result = func.call::<mlua::Value>((job_table, progress_lua)).unwrap();
                let updated_job = match result {
                    mlua::Value::Table(table) => {
                        crate::helpers::lua_table_to_json(lua, &table, None).unwrap()
                    }
                    _ => panic!("Lua job handler must return a table"),
                };
                let mut world_ref = world.borrow_mut();
                world_ref
                    .set_component(call.entity, "Job", updated_job)
                    .unwrap();
            }
        });
    }
}

/// Calls a Lua job handler by job type name.
/// Returns an error if the handler is not registered.
pub fn call_lua_job_handler<A>(
    lua: &mlua::Lua,
    job_type: &str,
    args: A,
) -> mlua::Result<mlua::MultiValue>
where
    A: mlua::IntoLuaMulti,
{
    LUA_JOB_HANDLERS.with(|handlers| {
        if let Some(key) = handlers.borrow().get(job_type) {
            let func: mlua::Function = lua.registry_value(key)?;
            func.call(args)
        } else {
            Err(mlua::Error::RuntimeError(format!(
                "Lua job handler not found for job type: {job_type}"
            )))
        }
    })
}
