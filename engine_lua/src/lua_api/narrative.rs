//! Narrative API: register_scenario, list_scenarios, get_scenario,
//! poll_pending_decisions, get_pending_decision, resolve_decision,
//! get_narrative_history.
//!
//! Thin delegation over [`engine_core::narrative`]: trigger evaluation and
//! the pending-decision lifecycle live in core, so every bridge observes
//! identical behavior. Payload shapes serialize from the same core types,
//! keeping the Lua/Python/WASM surfaces byte-compatible.

use crate::helpers::json_to_lua_table;
use engine_core::ecs::world::World;
use engine_core::narrative;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the narrative API.
pub fn register_narrative_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // register_scenario(def_json) — errors on malformed definitions
    let w = world.clone();
    let register_scenario_fn =
        lua.create_function_mut(move |_, def_json: String| -> LuaResult<()> {
            let mut world = w.borrow_mut();
            narrative::register_scenario(&mut world, &def_json).map_err(mlua::Error::external)?;
            Ok(())
        })?;
    globals.set("register_scenario", register_scenario_fn)?;

    // list_scenarios() -> array of scenario definitions in id order
    let w = world.clone();
    let list_scenarios_fn = lua.create_function_mut(move |lua, ()| -> LuaResult<LuaValue> {
        let world = w.borrow();
        let defs = narrative::list_scenarios(&world);
        let json_value = serde_json::to_value(defs).unwrap_or_default();
        json_to_lua_table(lua, &json_value)
    })?;
    globals.set("list_scenarios", list_scenarios_fn)?;

    // get_scenario(id) -> definition table or nil
    let w = world.clone();
    let get_scenario_fn =
        lua.create_function_mut(move |lua, id: String| -> LuaResult<LuaValue> {
            let world = w.borrow();
            match narrative::get_scenario(&world, &id) {
                Some(def) => {
                    let json_value = serde_json::to_value(def).unwrap_or_default();
                    json_to_lua_table(lua, &json_value)
                }
                None => Ok(LuaValue::Nil),
            }
        })?;
    globals.set("get_scenario", get_scenario_fn)?;

    // poll_pending_decisions() -> array of live pending decisions in id order
    let w = world.clone();
    let poll_pending_decisions_fn =
        lua.create_function_mut(move |lua, ()| -> LuaResult<LuaValue> {
            let world = w.borrow();
            let pending = narrative::list_pending_decisions(&world);
            let json_value = serde_json::to_value(pending).unwrap_or_default();
            json_to_lua_table(lua, &json_value)
        })?;
    globals.set("poll_pending_decisions", poll_pending_decisions_fn)?;

    // get_pending_decision(id) -> decision table or nil
    let w = world.clone();
    let get_pending_decision_fn =
        lua.create_function_mut(move |lua, id: i64| -> LuaResult<LuaValue> {
            let world = w.borrow();
            let decision_id = u64::try_from(id).unwrap_or(u64::MAX);
            match narrative::get_pending_decision(&world, decision_id) {
                Some(decision) => {
                    let json_value = serde_json::to_value(decision).unwrap_or_default();
                    json_to_lua_table(lua, &json_value)
                }
                None => Ok(LuaValue::Nil),
            }
        })?;
    globals.set("get_pending_decision", get_pending_decision_fn)?;

    // resolve_decision(id, choice_id) — errors on unknown/settled decisions
    let w = world.clone();
    let resolve_decision_fn =
        lua.create_function_mut(move |_, (id, choice_id): (i64, String)| -> LuaResult<()> {
            let decision_id = u64::try_from(id).unwrap_or(u64::MAX);
            let mut world = w.borrow_mut();
            narrative::resolve_decision(&mut world, decision_id, &choice_id)
                .map_err(mlua::Error::external)?;
            Ok(())
        })?;
    globals.set("resolve_decision", resolve_decision_fn)?;

    // get_narrative_history() -> turn-indexed records in append order
    let w = world;
    let get_narrative_history_fn =
        lua.create_function_mut(move |lua, ()| -> LuaResult<LuaValue> {
            let world = w.borrow();
            let history = narrative::get_narrative_history(&world);
            let json_value = serde_json::to_value(history).unwrap_or_default();
            json_to_lua_table(lua, &json_value)
        })?;
    globals.set("get_narrative_history", get_narrative_history_fn)?;

    Ok(())
}
