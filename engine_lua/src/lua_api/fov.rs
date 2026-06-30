//! FOV API: get_visible_cells, is_visible, set_sight, get_sight.

use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the FOV API functions into the Lua globals table.
pub fn register_fov_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // get_visible_cells(entity_id) -> table of {x, y, z} cell tables
    let w = world.clone();
    let get_visible_cells_fn = lua.create_function_mut(move |lua, entity_id: u32| {
        let world = w.borrow();
        let cells = world.get_visible_cells(entity_id);
        let results = lua.create_table()?;
        if let Some(cell_set) = cells {
            for (i, cell) in cell_set.iter().enumerate() {
                let entry = lua.create_table()?;
                match cell {
                    CellKey::Square { x, y, z } => {
                        entry.set("x", *x)?;
                        entry.set("y", *y)?;
                        entry.set("z", *z)?;
                    }
                    CellKey::Hex { q, r, z } => {
                        entry.set("q", *q)?;
                        entry.set("r", *r)?;
                        entry.set("z", *z)?;
                    }
                    CellKey::Province { id } => {
                        entry.set("id", id.clone())?;
                    }
                }
                results.set(i + 1, entry)?;
            }
        }
        Ok(results)
    })?;
    globals.set("get_visible_cells", get_visible_cells_fn)?;

    // is_visible(entity_id, x, y, z) -> bool
    let w = world.clone();
    let is_visible_fn =
        lua.create_function_mut(move |_, (entity_id, x, y, z): (u32, i32, i32, i32)| {
            let world = w.borrow();
            let cell = CellKey::Square { x, y, z };
            let result = world
                .get_visible_cells(entity_id)
                .map(|cells| cells.contains(&cell))
                .unwrap_or(false);
            Ok(result)
        })?;
    globals.set("is_visible", is_visible_fn)?;

    // set_sight(entity_id, range) — sets/updates Sight component
    let w = world.clone();
    let set_sight_fn = lua.create_function_mut(move |_, (entity_id, range): (u32, u32)| {
        let mut world = w.borrow_mut();
        let data = serde_json::json!({
            "range": range,
        });
        world
            .set_component(entity_id, "Sight", data)
            .map_err(mlua::Error::external)?;
        Ok(())
    })?;
    globals.set("set_sight", set_sight_fn)?;

    // set_fov_algorithm(name) — switch the active FOV algorithm
    let w = world.clone();
    let set_fov_algo_fn = lua.create_function_mut(move |_, name: String| {
        let mut world = w.borrow_mut();
        world
            .set_fov_algorithm_by_name(&name)
            .map_err(mlua::Error::external)?;
        Ok(())
    })?;
    globals.set("set_fov_algorithm", set_fov_algo_fn)?;

    // get_sight(entity_id) -> table | nil
    let w = world;
    let get_sight_fn = lua.create_function_mut(move |lua, entity_id: u32| {
        let world = w.borrow();
        let comp = world.get_component(entity_id, "Sight");
        match comp {
            Some(data) => {
                let result = lua.create_table()?;
                if let Some(range) = data.get("range").and_then(|v| v.as_u64()) {
                    result.set("range", range)?;
                }
                if let Some(radius) = data.get("radius").and_then(|v| v.as_u64()) {
                    result.set("radius", radius)?;
                }
                Ok(Some(result))
            }
            None => Ok(None),
        }
    })?;
    globals.set("get_sight", get_sight_fn)?;

    Ok(())
}
