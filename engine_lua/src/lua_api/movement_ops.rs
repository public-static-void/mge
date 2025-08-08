//! Lua bridge for movement operations in the job system.

use engine_core::ecs::world::World;
use engine_core::map::CellKey;
use engine_core::systems::job::ops::movement_ops;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;

/// Helper to convert a Lua table to a Rust CellKey enum.
/// Expects the Lua table to have exactly one key: "Square", "Hex", or "Region",
/// with the relevant coordinate data as a nested table.
fn from_lua_cell(table: &Table) -> LuaResult<CellKey> {
    // Collect keys from the table, expecting exactly one key
    let keys: Vec<String> = table
        .pairs::<LuaValue, LuaValue>()
        .filter_map(|res| res.ok())
        .filter_map(|(k, _v)| match k {
            LuaValue::String(s) => s.to_str().ok().map(|str| str.to_string()),
            _ => None,
        })
        .collect();

    if keys.len() != 1 {
        return Err(mlua::Error::external(
            "Cell table must have exactly one key (Square, Hex, or Region)",
        ));
    }

    let kind = &keys[0];
    match kind.as_str() {
        "Square" => {
            let pos_table: Table = table.get("Square")?;
            let x: i32 = pos_table.get("x")?;
            let y: i32 = pos_table.get("y")?;
            let z: i32 = pos_table.get("z")?;
            Ok(CellKey::Square { x, y, z })
        }
        "Hex" => {
            let pos_table: Table = table.get("Hex")?;
            let q: i32 = pos_table.get("q")?;
            let r: i32 = pos_table.get("r")?;
            let z: i32 = pos_table.get("z")?;
            Ok(CellKey::Hex { q, r, z })
        }
        "Region" => {
            let pos_table: Table = table.get("Region")?;
            let id: String = pos_table.get("id")?;
            Ok(CellKey::Region { id })
        }
        _ => Err(mlua::Error::external(format!("Unknown cell kind '{kind}'"))),
    }
}

/// Registers movement operations to the Lua global environment.
pub fn register_movement_ops_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // assign_move_path(agent_id: u32, from_cell: Table, to_cell: Table)
    let world_clone = world.clone();
    let assign_move_path = lua.create_function_mut(
        move |_, (agent_id, from_cell_table, to_cell_table): (u32, Table, Table)| {
            let from_cell = from_lua_cell(&from_cell_table)?;
            let to_cell = from_lua_cell(&to_cell_table)?;

            let mut world = world_clone.borrow_mut();

            let move_path_vec = {
                if let Some(map) = &world.map {
                    if let Some(pathfinding) = map.find_path(&from_cell, &to_cell) {
                        // skip the start cell and convert path cells to JSON
                        pathfinding
                            .path
                            .iter()
                            .skip(1)
                            .map(|cell| match cell {
                                CellKey::Square { x, y, z } => {
                                    json!({ "Square": { "x": x, "y": y, "z": z } })
                                }
                                CellKey::Hex { q, r, z } => {
                                    json!({ "Hex": { "q": q, "r": r, "z": z } })
                                }
                                CellKey::Region { id } => json!({ "Region": { "id": id } }),
                            })
                            .collect::<Vec<_>>()
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            };

            let mut agent = world
                .get_component(agent_id, "Agent")
                .cloned()
                .unwrap_or_else(|| {
                    json!({
                        "entity_id": agent_id,
                        // optionally add other default Agent fields here
                    })
                });

            agent["move_path"] = json!(move_path_vec);

            world
                .set_component(agent_id, "Agent", agent)
                .map_err(mlua::Error::external)?;

            Ok(())
        },
    )?;
    globals.set("assign_move_path", assign_move_path)?;

    // is_agent_at_cell(agent_id: u32, cell: Table) -> bool
    let world_clone = world.clone();
    let is_agent_at_cell =
        lua.create_function(move |_, (agent_id, cell_table): (u32, Table)| {
            let cell = from_lua_cell(&cell_table)?;
            let world = world_clone.borrow();
            Ok(movement_ops::is_agent_at_cell(&world, agent_id, &cell))
        })?;
    globals.set("is_agent_at_cell", is_agent_at_cell)?;

    // is_move_path_empty(agent_id: u32) -> bool
    let world_clone = world.clone();
    let is_move_path_empty = lua.create_function(move |_, agent_id: u32| {
        let world = world_clone.borrow();
        Ok(movement_ops::is_move_path_empty(&world, agent_id))
    })?;
    globals.set("is_move_path_empty", is_move_path_empty)?;

    Ok(())
}
