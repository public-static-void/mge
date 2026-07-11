//! Tech Tree and Research API: get_tech_tree, get_tech_node, get_tech_progress,
//! get_completed_techs, is_tech_completed, get_research_queue,
//! get_research_queue_progress, research_tech, cancel_research,
//! clear_research_queue, can_research_tech.

use crate::helpers::json_to_lua_table;
use engine_core::ecs::world::World;
use engine_core::tech_tree;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the tech tree and research API.
pub fn register_tech_tree_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // get_tech_tree() -> table array of tech nodes
    let get_tech_tree_fn = lua.create_function(|lua, ()| {
        let nodes = tech_tree::get_tech_tree();
        let json_value = serde_json::to_value(nodes).unwrap_or_default();
        json_to_lua_table(lua, &json_value)
    })?;
    globals.set("get_tech_tree", get_tech_tree_fn)?;

    // get_tech_node(tech_id) -> table or nil
    let get_tech_node_fn = lua.create_function(|lua, tech_id: String| -> LuaResult<LuaValue> {
        match tech_tree::get_tech_node(&tech_id) {
            Some(node) => {
                let json_value = serde_json::to_value(node).unwrap_or_default();
                json_to_lua_table(lua, &json_value)
            }
            None => Ok(LuaValue::Nil),
        }
    })?;
    globals.set("get_tech_node", get_tech_node_fn)?;

    // get_tech_progress(entity) -> table or nil
    let w = world.clone();
    let get_tech_progress_fn =
        lua.create_function_mut(move |lua, entity: u32| -> LuaResult<LuaValue> {
            let world = w.borrow();
            match tech_tree::get_tech_progress(&world, entity) {
                Some(val) => json_to_lua_table(lua, &val),
                None => Ok(LuaValue::Nil),
            }
        })?;
    globals.set("get_tech_progress", get_tech_progress_fn)?;

    // get_completed_techs(entity) -> array of strings
    let w = world.clone();
    let get_completed_techs_fn =
        lua.create_function_mut(move |_, entity: u32| -> LuaResult<Vec<String>> {
            let world = w.borrow();
            Ok(tech_tree::get_completed_techs(&world, entity))
        })?;
    globals.set("get_completed_techs", get_completed_techs_fn)?;

    // is_tech_completed(entity, tech_id) -> boolean
    let w = world.clone();
    let is_tech_completed_fn = lua.create_function_mut(
        move |_, (entity, tech_id): (u32, String)| -> LuaResult<bool> {
            let world = w.borrow();
            Ok(tech_tree::is_tech_completed(&world, entity, &tech_id))
        },
    )?;
    globals.set("is_tech_completed", is_tech_completed_fn)?;

    // get_research_queue(entity) -> array of strings
    let w = world.clone();
    let get_research_queue_fn =
        lua.create_function_mut(move |_, entity: u32| -> LuaResult<Vec<String>> {
            let world = w.borrow();
            Ok(tech_tree::get_research_queue(&world, entity))
        })?;
    globals.set("get_research_queue", get_research_queue_fn)?;

    // get_research_queue_progress(entity) -> table
    let w = world.clone();
    let get_research_queue_progress_fn =
        lua.create_function_mut(move |lua, entity: u32| -> LuaResult<LuaValue> {
            let world = w.borrow();
            let val = tech_tree::get_research_queue_progress(&world, entity);
            json_to_lua_table(lua, &val)
        })?;
    globals.set(
        "get_research_queue_progress",
        get_research_queue_progress_fn,
    )?;

    // research_tech(entity, tech_id) — error on failure
    let w = world.clone();
    let research_tech_fn = lua.create_function_mut(
        move |_, (entity, tech_id): (u32, String)| -> LuaResult<()> {
            let mut world = w.borrow_mut();
            tech_tree::research_tech(&mut world, entity, &tech_id)
                .map_err(mlua::Error::external)?;
            Ok(())
        },
    )?;
    globals.set("research_tech", research_tech_fn)?;

    // cancel_research(entity, tech_id) — no return
    let w = world.clone();
    let cancel_research_fn = lua.create_function_mut(
        move |_, (entity, tech_id): (u32, String)| -> LuaResult<()> {
            let mut world = w.borrow_mut();
            tech_tree::cancel_research(&mut world, entity, &tech_id)
                .map_err(mlua::Error::external)?;
            Ok(())
        },
    )?;
    globals.set("cancel_research", cancel_research_fn)?;

    // clear_research_queue(entity) — no return
    let w = world.clone();
    let clear_research_queue_fn =
        lua.create_function_mut(move |_, entity: u32| -> LuaResult<()> {
            let mut world = w.borrow_mut();
            tech_tree::clear_research_queue(&mut world, entity).map_err(mlua::Error::external)?;
            Ok(())
        })?;
    globals.set("clear_research_queue", clear_research_queue_fn)?;

    // can_research_tech(entity, tech_id) -> boolean, reason_string
    let w = world;
    let can_research_tech_fn = lua.create_function_mut(
        move |_, (entity, tech_id): (u32, String)| -> LuaResult<(bool, String)> {
            let world = w.borrow();
            match tech_tree::can_research_tech(&world, entity, &tech_id) {
                Ok(true) => Ok((true, String::new())),
                Ok(false) => Ok((false, "Unknown reason".to_string())),
                Err(reason) => Ok((false, reason)),
            }
        },
    )?;
    globals.set("can_research_tech", can_research_tech_fn)?;

    Ok(())
}
