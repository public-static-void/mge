use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers camera scripting API into Lua.
pub fn register_camera_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // set_camera(x, y, z?)
    let world_set = world.clone();
    let set_camera = lua.create_function_mut(move |_, (x, y, z): (i64, i64, Option<i64>)| {
        let z = z.unwrap_or(0);
        let mut world = world_set.borrow_mut();
        // Find or create the camera entity
        let camera_id = world
            .get_entities_with_component("Camera")
            .first()
            .cloned()
            .unwrap_or_else(|| {
                let id = world.spawn_entity();
                if let Err(e) =
                    world.set_component(id, "Camera", serde_json::json!({ "x": x, "y": y, "z": z }))
                {
                    eprintln!("Failed to set Camera component on new entity: {e}");
                }
                if let Err(e) = world.set_component(
                    id,
                    "Position",
                    serde_json::json!({ "pos": { "Square": { "x": x, "y": y, "z": z } } }),
                ) {
                    eprintln!("Failed to set Position component on new entity: {e}");
                }
                id
            });
        // Always update Camera component with x, y, and z
        world
            .set_component(
                camera_id,
                "Camera",
                serde_json::json!({ "x": x, "y": y, "z": z }),
            )
            .map_err(|e| mlua::Error::external(format!("Failed to set Camera: {e}")))?;
        world
            .set_component(
                camera_id,
                "Position",
                serde_json::json!({ "pos": { "Square": { "x": x, "y": y, "z": z } } }),
            )
            .map_err(|e| mlua::Error::external(format!("Failed to set Position: {e}")))?;
        Ok(())
    })?;
    globals.set("set_camera", set_camera)?;

    // get_camera()
    let world_get = world.clone();
    let get_camera = lua.create_function_mut(move |lua, ()| {
        let world = world_get.borrow();
        if let Some(camera_id) = world.get_entities_with_component("Camera").first() {
            // Prefer Position component's pos.Square if present
            if let Some(pos) = world.get_component(*camera_id, "Position")
                && let Some(square) = pos.get("pos").and_then(|p| p.get("Square"))
            {
                let x = square.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
                let y = square.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
                let z = square.get("z").and_then(|v| v.as_i64()).unwrap_or(0);
                let tbl = lua.create_table()?;
                tbl.set("x", x)?;
                tbl.set("y", y)?;
                tbl.set("z", z)?;
                return Ok(LuaValue::Table(tbl));
            }
            // Fallback: try Camera component's x/y/z (for legacy support)
            if let Some(cam) = world.get_component(*camera_id, "Camera") {
                let x = cam.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
                let y = cam.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
                let z = cam.get("z").and_then(|v| v.as_i64()).unwrap_or(0);
                let tbl = lua.create_table()?;
                tbl.set("x", x)?;
                tbl.set("y", y)?;
                tbl.set("z", z)?;
                return Ok(LuaValue::Table(tbl));
            }
        }
        Ok(LuaValue::Nil)
    })?;
    globals.set("get_camera", get_camera)?;

    Ok(())
}
