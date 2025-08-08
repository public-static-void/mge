use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_job_cancel_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    let world_cancel = world.clone();
    let cancel_job = lua.create_function_mut(move |_lua, job_id: u32| {
        let mut world = world_cancel.borrow_mut();
        if let Some(mut job) = world.get_component(job_id, "Job").cloned() {
            job["state"] = serde_json::json!("cancelled");
            world
                .set_component(job_id, "Job", job)
                .map_err(mlua::Error::external)
        } else {
            Err(mlua::Error::external("Job not found"))
        }
    })?;
    globals.set("cancel_job", cancel_job)?;
    Ok(())
}
