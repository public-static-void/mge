use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers camera scripting API into Lua.
pub fn register_camera_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // set_camera(x, y)
    let world_set = world.clone();
    let set_camera = lua.create_function_mut(move |_, (x, y): (i64, i64)| {
        let mut world = world_set.borrow_mut();
        // Find or create the camera entity
        let camera_id = world
            .get_entities_with_component("Camera")
            .first()
            .cloned()
            .unwrap_or_else(|| {
                let id = world.spawn_entity();
                world
                    .set_component(id, "Camera", serde_json::json!({ "x": x, "y": y }))
                    .unwrap();
                world
                    .set_component(
                        id,
                        "Position",
                        serde_json::json!({ "pos": { "Square": { "x": x, "y": y, "z": 0 } } }),
                    )
                    .unwrap();
                id
            });
        // Always update Camera component with x and y
        world
            .set_component(camera_id, "Camera", serde_json::json!({ "x": x, "y": y }))
            .unwrap();
        world
            .set_component(
                camera_id,
                "Position",
                serde_json::json!({ "pos": { "Square": { "x": x, "y": y, "z": 0 } } }),
            )
            .unwrap();
        Ok(())
    })?;
    globals.set("set_camera", set_camera)?;

    // get_camera()
    let world_get = world.clone();
    let get_camera = lua.create_function_mut(move |lua, ()| {
        let world = world_get.borrow();
        if let Some(camera_id) = world.get_entities_with_component("Camera").first() {
            // Prefer Position component's pos.Square if present
            if let Some(pos) = world.get_component(*camera_id, "Position") {
                if let Some(square) = pos.get("pos").and_then(|p| p.get("Square")) {
                    let x = square.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
                    let y = square.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
                    let tbl = lua.create_table()?;
                    tbl.set("x", x)?;
                    tbl.set("y", y)?;
                    return Ok(LuaValue::Table(tbl));
                }
            }
            // Fallback: try Camera component's x/y (for legacy support)
            if let Some(cam) = world.get_component(*camera_id, "Camera") {
                let x = cam.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
                let y = cam.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
                let tbl = lua.create_table()?;
                tbl.set("x", x)?;
                tbl.set("y", y)?;
                return Ok(LuaValue::Table(tbl));
            }
        }
        Ok(LuaValue::Nil)
    })?;
    globals.set("get_camera", get_camera)?;

    Ok(())
}
