//! Supply API: create_supply_link, remove_supply_link, list_supply_links,
//! set_supply_link_active, get_supply_link.
//!
//! Thin delegation over [`engine_core::supply`]: validation, atomicity, epsilon
//! discipline, and gate order live in core, so every bridge observes identical
//! behavior. Error messages prefix the core variant name so script-side
//! assertions can match on it.

use engine_core::ecs::world::World;
use engine_core::supply::{self, SupplyError};
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Variant token for a supply failure, surfaced in the script error text.
fn supply_variant(err: &SupplyError) -> &'static str {
    match err {
        SupplyError::SameEndpoint => "SameEndpoint",
        SupplyError::UnknownKind(_) => "UnknownKind",
        SupplyError::NonPositiveAmount => "NonPositiveAmount",
        SupplyError::NoStockpile(_) => "NoStockpile",
        SupplyError::UnknownLink(_) => "UnknownLink",
        SupplyError::Store(_) => "Store",
    }
}

/// Registers the supply API.
pub fn register_supply_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // create_supply_link(source, target, kind, amount_per_tick, capacity_per_tick)
    // -> link entity id. New links start active.
    let w = world.clone();
    let create_fn = lua.create_function_mut(
        move |_,
              (source, target, kind, amount, capacity): (u32, u32, String, f64, f64)|
              -> LuaResult<u32> {
            let mut world = w.borrow_mut();
            supply::create_supply_link(&mut world, source, target, &kind, amount, capacity)
                .map_err(|e| mlua::Error::external(format!("{}: {e}", supply_variant(&e))))
        },
    )?;
    globals.set("create_supply_link", create_fn)?;

    // remove_supply_link(link) — errors on unknown links
    let w = world.clone();
    let remove_fn = lua.create_function_mut(move |_, link: u32| -> LuaResult<()> {
        let mut world = w.borrow_mut();
        supply::remove_supply_link(&mut world, link)
            .map_err(|e| mlua::Error::external(format!("{}: {e}", supply_variant(&e))))?;
        Ok(())
    })?;
    globals.set("remove_supply_link", remove_fn)?;

    // list_supply_links() -> ascending array of link entity ids
    let w = world.clone();
    let list_fn = lua.create_function_mut(move |lua, ()| -> LuaResult<Table> {
        let world = w.borrow();
        let out = lua.create_table()?;
        for (index, id) in supply::list_supply_links(&world).iter().enumerate() {
            out.set(index + 1, *id)?;
        }
        Ok(out)
    })?;
    globals.set("list_supply_links", list_fn)?;

    // set_supply_link_active(link, active) — errors on unknown links
    let w = world.clone();
    let set_active_fn =
        lua.create_function_mut(move |_, (link, active): (u32, bool)| -> LuaResult<()> {
            let mut world = w.borrow_mut();
            supply::set_supply_link_active(&mut world, link, active)
                .map_err(|e| mlua::Error::external(format!("{}: {e}", supply_variant(&e))))?;
            Ok(())
        })?;
    globals.set("set_supply_link_active", set_active_fn)?;

    // get_supply_link(link) -> record table; errors on unknown links
    let w = world;
    let get_fn = lua.create_function_mut(move |lua, link: u32| -> LuaResult<Table> {
        let world = w.borrow();
        let record = supply::get_supply_link(&world, link).ok_or_else(|| {
            mlua::Error::external(format!("UnknownLink: {}", SupplyError::UnknownLink(link)))
        })?;
        let out = lua.create_table()?;
        out.set("source", record.source)?;
        out.set("target", record.target)?;
        out.set("kind", record.kind)?;
        out.set("amount_per_tick", record.amount_per_tick)?;
        out.set("capacity_per_tick", record.capacity_per_tick)?;
        out.set("active", record.active)?;
        Ok(out)
    })?;
    globals.set("get_supply_link", get_fn)?;

    Ok(())
}
