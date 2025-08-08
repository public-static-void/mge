use engine_core::ecs::world::World;
use engine_core::systems::job::ai::logic::assign_jobs;
use mlua::{Lua, Result as LuaResult, Table};
use serde_json::Value as JsonValue;
use std::cell::RefCell;
use std::rc::Rc;

use crate::helpers::{json_to_lua_table, lua_table_to_json};

/// Registers AI job assignment Lua API functions
pub fn register_job_ai_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // ai_assign_jobs(agent_id, args)
    let world_clone = world.clone();
    let ai_assign_jobs = lua.create_function_mut(move |_, (agent_id, _args): (u32, Table)| {
        let mut world = world_clone.borrow_mut();
        let job_board_ptr: *mut _ = &mut world.job_board;

        // Track current job before assignment
        let mut prev_current_job: Option<u64> = None;
        if let Some(agent_json) = world.get_component(agent_id, "Agent") {
            prev_current_job = agent_json.get("current_job").and_then(|v| v.as_u64());
        }

        unsafe {
            assign_jobs(&mut world, &mut *job_board_ptr, agent_id as u64, &[]);
        }

        // Set assigned_to on the job if assigned by AI
        if let Some(agent_json) = world.get_component(agent_id, "Agent")
            && let Some(current_job_id) = agent_json.get("current_job").and_then(|v| v.as_u64())
                && prev_current_job != Some(current_job_id)
                    && let Some(job_json) = world.get_component(current_job_id as u32, "Job") {
                        let mut job_json_obj = job_json.as_object().unwrap().clone();
                        job_json_obj.insert(
                            "assigned_to".to_string(),
                            JsonValue::Number(agent_id.into()),
                        );
                        world
                            .set_component(
                                current_job_id as u32,
                                "Job",
                                JsonValue::Object(job_json_obj),
                            )
                            .map_err(|e| {
                                mlua::Error::external(format!("Failed to set job component: {e}"))
                            })?;
                    }

        Ok(())
    })?;
    globals.set("ai_assign_jobs", ai_assign_jobs)?;

    // ai_query_jobs(agent_id) -> list of job tables assigned to agent_id
    let world_clone = world.clone();
    let ai_query_jobs = lua.create_function(move |lua_ctx, agent_id: u32| {
        let world = world_clone.borrow();

        let mut results: Vec<Table> = Vec::new();

        if let Some(job_map) = world.components.get("Job") {
            for (job_eid, job_json) in job_map {
                // Only count jobs with assigned_to as a Number matching agent_id
                if let Some(JsonValue::Number(n)) = job_json.get("assigned_to")
                    && n.as_u64() == Some(agent_id as u64) {
                        let tbl = match json_to_lua_table(lua_ctx, job_json)? {
                            mlua::Value::Table(t) => t,
                            _ => {
                                return Err(mlua::Error::external(
                                    "Expected JSON conversion to Lua table",
                                ));
                            }
                        };
                        tbl.set("id", *job_eid)?;
                        let state = job_json
                            .get("state")
                            .and_then(|v| v.as_str())
                            .unwrap_or("<no state>");
                        tbl.set("state", state)?;
                        tbl.set("assigned_to", n.as_u64().unwrap())?;
                        results.push(tbl);
                    }
            }
        }

        lua_ctx.create_sequence_from(results)
    })?;
    globals.set("ai_query_jobs", ai_query_jobs)?;

    // ai_modify_job_assignment(job_id, changes_table) -> bool success
    let world_clone = world.clone();
    let ai_modify_job_assignment =
        lua.create_function_mut(move |lua_ctx, (job_id, changes_table): (u32, Table)| {
            let mut world = world_clone.borrow_mut();

            let changes_json = lua_table_to_json(lua_ctx, &changes_table, None)?;

            let old_job_json_dbg = world.get_component(job_id, "Job").cloned();
            let mut job = old_job_json_dbg
                .clone()
                .ok_or_else(|| mlua::Error::external(format!("No job with id {job_id}")))?;

            if let (JsonValue::Object(job_map), JsonValue::Object(changes_map)) =
                (&mut job, &changes_json)
            {
                // If assigned_to is absent from changes_map and assigned_to isn't present in Lua table,
                // treat as explicit request to null (user set assigned_to = nil in Lua)
                let key_present = changes_table.contains_key("assigned_to").unwrap_or(false);
                if !changes_map.contains_key("assigned_to") && !key_present {
                    job_map.insert("assigned_to".to_string(), JsonValue::Null);
                }
                for (key, value) in changes_map {
                    if key == "assigned_to" {
                        if value.is_null() {
                            job_map.insert(key.clone(), JsonValue::Null);
                        } else {
                            job_map.insert(key.clone(), value.clone());
                        }
                    } else {
                        job_map.insert(key.clone(), value.clone());
                    }
                }
            } else {
                return Err(mlua::Error::external("Job or changes are not JSON objects"));
            }

            let _set_result = world.set_component(job_id, "Job", job.clone());
            if let Some(new_job_json) = world.get_component(job_id, "Job") {
                let _assigned_to = new_job_json.get("assigned_to");
            }
            Ok(true)
        })?;
    globals.set("ai_modify_job_assignment", ai_modify_job_assignment)?;

    Ok(())
}
