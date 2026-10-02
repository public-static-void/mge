//! Lore API: generate_founding_history, list_chronicle, get_chronicle_entry,
//! render_chronicle, chronicle_len, clear_lore_history.
//!
//! Thin delegation over [`engine_core::lore`]: backfill generation and the
//! chronicle read-model live in core, so every bridge observes identical
//! behavior. Payload shapes serialize from the same core types, keeping the
//! Lua/Python/WASM surfaces byte-compatible.

use crate::helpers::json_to_lua_table;
use engine_core::ecs::world::World;
use engine_core::lore::{self, ChronicleFilter, ChronicleKind};
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use std::cell::RefCell;
use std::rc::Rc;

/// Parses a lowercase kind token into its chronicle kind.
///
/// Rejects anything outside the fixed `fired|resolved|expired|founding` set
/// so invalid filters error instead of silently matching nothing.
fn parse_kind(raw: &str) -> LuaResult<ChronicleKind> {
    match raw {
        "fired" => Ok(ChronicleKind::Fired),
        "resolved" => Ok(ChronicleKind::Resolved),
        "expired" => Ok(ChronicleKind::Expired),
        "founding" => Ok(ChronicleKind::Founding),
        other => Err(mlua::Error::external(format!(
            "Invalid chronicle kind: {other}"
        ))),
    }
}

/// Parses an optional filter table into its core filter.
///
/// A nil filter means unbounded; each missing key stays unbounded. Mirrors the
/// Python `dict | None` and WASM JSON-filter shapes key for key.
fn parse_filter(filter: Option<Table>) -> LuaResult<ChronicleFilter> {
    let mut out = ChronicleFilter::default();
    if let Some(table) = filter {
        out.scenario_id = table.get::<Option<String>>("scenario_id")?;
        if let Some(kind_raw) = table.get::<Option<String>>("kind")? {
            out.kind = Some(parse_kind(&kind_raw)?);
        }
        out.turn_from = table.get::<Option<u64>>("turn_from")?;
        out.turn_to = table.get::<Option<u64>>("turn_to")?;
    }
    Ok(out)
}

/// Registers the lore API.
pub fn register_lore_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // generate_founding_history(seed, era_count) -> appended count; errors on
    // negative inputs or core backfill failures
    let w = world.clone();
    let generate_founding_history_fn =
        lua.create_function_mut(move |_, (seed, era_count): (i64, i64)| -> LuaResult<i64> {
            let seed_value = u64::try_from(seed)
                .map_err(|_| mlua::Error::external(format!("Invalid lore seed: {seed}")))?;
            let era_value = u32::try_from(era_count)
                .map_err(|_| mlua::Error::external(format!("Invalid era count: {era_count}")))?;
            let mut world = w.borrow_mut();
            let count = lore::generate_founding_history(&mut world, seed_value, era_value)
                .map_err(mlua::Error::external)?;
            Ok(count as i64)
        })?;
    globals.set("generate_founding_history", generate_founding_history_fn)?;

    // list_chronicle(filter_or_nil) -> array of entries in (turn, id) order
    let w = world.clone();
    let list_chronicle_fn =
        lua.create_function_mut(move |lua, filter: Option<Table>| -> LuaResult<LuaValue> {
            let world = w.borrow();
            let entries = lore::list_chronicle(&world, parse_filter(filter)?);
            let json_value = serde_json::to_value(entries).unwrap_or_default();
            json_to_lua_table(lua, &json_value)
        })?;
    globals.set("list_chronicle", list_chronicle_fn)?;

    // get_chronicle_entry(id) -> entry table or nil
    let w = world.clone();
    let get_chronicle_entry_fn =
        lua.create_function_mut(move |lua, id: i64| -> LuaResult<LuaValue> {
            let world = w.borrow();
            let entry_id = u64::try_from(id).unwrap_or(u64::MAX);
            match lore::get_chronicle_entry(&world, entry_id) {
                Some(entry) => {
                    let json_value = serde_json::to_value(entry).unwrap_or_default();
                    json_to_lua_table(lua, &json_value)
                }
                None => Ok(LuaValue::Nil),
            }
        })?;
    globals.set("get_chronicle_entry", get_chronicle_entry_fn)?;

    // render_chronicle(filter_or_nil) -> array of template-shaped strings
    let w = world.clone();
    let render_chronicle_fn =
        lua.create_function_mut(move |lua, filter: Option<Table>| -> LuaResult<LuaValue> {
            let world = w.borrow();
            let lines = lore::render_chronicle_text(&world, parse_filter(filter)?);
            let json_value = serde_json::to_value(lines).unwrap_or_default();
            json_to_lua_table(lua, &json_value)
        })?;
    globals.set("render_chronicle", render_chronicle_fn)?;

    // chronicle_len() -> number of chronicle entries
    let w = world.clone();
    let chronicle_len_fn = lua.create_function_mut(move |_, ()| -> LuaResult<i64> {
        let world = w.borrow();
        Ok(lore::chronicle_len(&world) as i64)
    })?;
    globals.set("chronicle_len", chronicle_len_fn)?;

    // clear_lore_history() -> nil; regen support before seeded backfill reruns
    let w = world;
    let clear_lore_history_fn = lua.create_function_mut(move |_, ()| -> LuaResult<()> {
        let mut world = w.borrow_mut();
        lore::clear_lore_history(&mut world);
        Ok(())
    })?;
    globals.set("clear_lore_history", clear_lore_history_fn)?;

    Ok(())
}
