use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers job board API functions in Lua:
/// - get_job_board()
/// - get_job_board_policy()
/// - set_job_board_policy(policy)
/// - get_job_priority(job_id)
/// - set_job_priority(job_id, value)
pub fn register_job_board_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // get_job_board()
    let world_board = world.clone();
    let get_job_board = lua.create_function(move |lua, ()| {
        let mut world = world_board.borrow_mut();
        let world_ptr: *mut World = &mut *world;
        unsafe {
            world.job_board.update(&*world_ptr);
            let entries = world.job_board.jobs_with_metadata(&*world_ptr);
            let tbl = lua.create_table()?;
            for (i, entry) in entries.iter().enumerate() {
                let row = lua.create_table()?;
                row.set("eid", entry.eid)?;
                row.set("priority", entry.priority)?;
                row.set("state", entry.state.clone())?;
                tbl.set(i + 1, row)?;
            }
            Ok(tbl)
        }
    })?;
    globals.set("get_job_board", get_job_board)?;

    // get_job_board_policy()
    let world_policy = world.clone();
    let get_job_board_policy = lua.create_function(move |_, ()| {
        let world = world_policy.borrow();
        Ok(world.job_board.get_policy_name().to_string())
    })?;
    globals.set("get_job_board_policy", get_job_board_policy)?;

    // set_job_board_policy(policy)
    let world_set_policy = world.clone();
    let set_job_board_policy = lua.create_function_mut(move |_, policy: String| {
        let mut world = world_set_policy.borrow_mut();
        world
            .job_board
            .set_policy(&policy)
            .map_err(mlua::Error::external)?;
        Ok(())
    })?;
    globals.set("set_job_board_policy", set_job_board_policy)?;

    // get_job_priority(job_id)
    let world_get_priority = world.clone();
    let get_job_priority = lua.create_function(move |_, job_id: u32| {
        let world = world_get_priority.borrow();
        Ok(world.job_board.get_priority(&world, job_id))
    })?;
    globals.set("get_job_priority", get_job_priority)?;

    // set_job_priority(job_id, value)
    let world_set_priority = world.clone();
    let set_job_priority = lua.create_function_mut(move |_, (job_id, value): (u32, i64)| {
        let mut world = world_set_priority.borrow_mut();
        let world_ptr: *mut World = &mut *world;
        unsafe {
            world
                .job_board
                .set_priority(&mut *world_ptr, job_id, value)
                .map_err(mlua::Error::external)?;
        }
        Ok(())
    })?;
    globals.set("set_job_priority", set_job_priority)?;

    Ok(())
}
