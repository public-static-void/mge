//! AI Behaviors API: set_patrol_route, get_patrol_route, set_ai_state, get_ai_state.

use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the AI behaviors API functions into the Lua globals table.
pub fn register_ai_behaviors_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // set_patrol_route(entity_id, waypoints_table) -> bool
    let w = world.clone();
    let set_patrol_route_fn = lua.create_function_mut(
        move |_, (entity_id, waypoints): (u32, Table)| {
            let mut world = w.borrow_mut();

            // Convert Lua table to JSON array of Position objects
            let waypoints_json: Vec<serde_json::Value> = (1..=waypoints.len()?)
                .filter_map(|i| {
                    let wp: Table = waypoints.get(i).ok()?;
                    let x: i32 = wp.get("x").ok()?;
                    let y: i32 = wp.get("y").ok()?;
                    let z: i32 = wp.get("z").ok()?;
                    Some(serde_json::json!({
                        "Square": {"x": x, "y": y, "z": z}
                    }))
                })
                .collect();

            let data = serde_json::json!({
                "waypoints": waypoints_json,
                "current_index": 0,
                "loop": true,
                "wait_ticks": 0
            });

            world
                .set_component(entity_id, "PatrolRoute", data)
                .map_err(mlua::Error::external)?;
            Ok(true)
        },
    )?;
    globals.set("set_patrol_route", set_patrol_route_fn)?;

    // get_patrol_route(entity_id) -> table | nil
    let w = world.clone();
    let get_patrol_route_fn = lua.create_function_mut(move |lua, entity_id: u32| {
        let world = w.borrow();
        let comp = world.get_component(entity_id, "PatrolRoute");
        match comp {
            Some(data) => {
                let result = lua.create_table()?;

                // Convert waypoints back to Lua table
                if let Some(waypoints) = data.get("waypoints").and_then(|v| v.as_array()) {
                    let lua_waypoints = lua.create_table()?;
                    for (i, wp) in waypoints.iter().enumerate() {
                        if let Some(square) = wp.get("Square") {
                            let x = square.get("x").and_then(|v| v.as_i64()).unwrap_or(0);
                            let y = square.get("y").and_then(|v| v.as_i64()).unwrap_or(0);
                            let z = square.get("z").and_then(|v| v.as_i64()).unwrap_or(0);
                            let wp_table = lua.create_table()?;
                            wp_table.set("x", x)?;
                            wp_table.set("y", y)?;
                            wp_table.set("z", z)?;
                            lua_waypoints.set(i + 1, wp_table)?;
                        }
                    }
                    result.set("waypoints", lua_waypoints)?;
                }

                if let Some(current_index) = data.get("current_index").and_then(|v| v.as_i64()) {
                    result.set("current_index", current_index)?;
                }
                if let Some(loop_val) = data.get("loop").and_then(|v| v.as_bool()) {
                    result.set("loop", loop_val)?;
                }
                if let Some(wait_ticks) = data.get("wait_ticks").and_then(|v| v.as_i64()) {
                    result.set("wait_ticks", wait_ticks)?;
                }

                Ok(Some(result))
            }
            None => Ok(None),
        }
    })?;
    globals.set("get_patrol_route", get_patrol_route_fn)?;

    // set_ai_state(entity_id, state_string) -> bool
    let w = world.clone();
    let set_ai_state_fn = lua.create_function_mut(
        move |_, (entity_id, state): (u32, String)| {
            // Validate state
            let valid_states = ["idle", "patrol", "chase", "attack", "flee"];
            if !valid_states.contains(&state.as_str()) {
                return Err(mlua::Error::external(format!(
                    "Invalid AI state: {}. Must be one of: idle, patrol, chase, attack, flee",
                    state
                )));
            }

            let mut world = w.borrow_mut();

            // Get existing EnemyAI or create default
            let mut data = world
                .get_component(entity_id, "EnemyAI")
                .cloned()
                .unwrap_or_else(|| {
                    serde_json::json!({
                        "state": "idle",
                        "alert_level": 0,
                        "detection_range": 8,
                        "attack_range": 1,
                        "flee_threshold": 0.25,
                        "target_faction": null,
                        "target_entity": null
                    })
                });

            data["state"] = serde_json::json!(state);

            world
                .set_component(entity_id, "EnemyAI", data)
                .map_err(mlua::Error::external)?;
            Ok(true)
        },
    )?;
    globals.set("set_ai_state", set_ai_state_fn)?;

    // get_ai_state(entity_id) -> table | nil
    let w = world;
    let get_ai_state_fn = lua.create_function_mut(move |lua, entity_id: u32| {
        let world = w.borrow();
        let comp = world.get_component(entity_id, "EnemyAI");
        match comp {
            Some(data) => {
                let result = lua.create_table()?;

                if let Some(state) = data.get("state").and_then(|v| v.as_str()) {
                    result.set("state", state)?;
                }
                if let Some(alert_level) = data.get("alert_level").and_then(|v| v.as_f64()) {
                    result.set("alert_level", alert_level)?;
                }
                if let Some(detection_range) = data.get("detection_range").and_then(|v| v.as_i64()) {
                    result.set("detection_range", detection_range)?;
                }
                if let Some(attack_range) = data.get("attack_range").and_then(|v| v.as_i64()) {
                    result.set("attack_range", attack_range)?;
                }
                if let Some(flee_threshold) = data.get("flee_threshold").and_then(|v| v.as_f64()) {
                    result.set("flee_threshold", flee_threshold)?;
                }
                if let Some(target_faction) = data.get("target_faction").and_then(|v| v.as_str()) {
                    result.set("target_faction", target_faction)?;
                }
                if let Some(target_entity) = data.get("target_entity").and_then(|v| v.as_i64()) {
                    result.set("target_entity", target_entity)?;
                }

                Ok(Some(result))
            }
            None => Ok(None),
        }
    })?;
    globals.set("get_ai_state", get_ai_state_fn)?;

    Ok(())
}