//! Trade API: transfer_stockpile_resource, has_active_trade_treaty,
//! execute_treaty_trade.
//!
//! Thin delegation over [`engine_core::trade`]: validation, atomicity, epsilon
//! discipline, and gate order live in core, so every bridge observes identical
//! behavior. Error messages prefix the core variant name so script-side
//! assertions can match on it.

use engine_core::ecs::world::World;
use engine_core::trade::{self, TradeError, TransferError};
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Variant token for a transfer failure, surfaced in the script error text.
fn transfer_variant(err: &TransferError) -> &'static str {
    match err {
        TransferError::NoStockpile(_) => "NoStockpile",
        TransferError::UnknownKind(_) => "UnknownKind",
        TransferError::NonPositiveAmount => "NonPositiveAmount",
        TransferError::InsufficientFunds { .. } => "InsufficientFunds",
    }
}

/// Variant token for a treaty-trade failure, surfaced in the script error text.
fn trade_variant(err: &TradeError) -> &'static str {
    match err {
        TradeError::Transfer(inner) => transfer_variant(inner),
        TradeError::NoLiveTreaty => "NoLiveTreaty",
        TradeError::RelationIsWar => "RelationIsWar",
    }
}

/// Registers the trade API.
pub fn register_trade_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // transfer_stockpile_resource(from, to, kind, amount) — ungated scripter-trust move
    let w = world.clone();
    let transfer_fn = lua.create_function_mut(
        move |_, (from, to, kind, amount): (u32, u32, String, f64)| -> LuaResult<()> {
            let mut world = w.borrow_mut();
            trade::transfer_stockpile_resource(&mut world, from, to, &kind, amount)
                .map_err(|e| mlua::Error::external(format!("{}: {e}", transfer_variant(&e))))?;
            Ok(())
        },
    )?;
    globals.set("transfer_stockpile_resource", transfer_fn)?;

    // has_active_trade_treaty(faction_a, faction_b) -> boolean
    let w = world.clone();
    let has_treaty_fn =
        lua.create_function_mut(move |_, (fa, fb): (String, String)| -> LuaResult<bool> {
            let world = w.borrow();
            Ok(trade::has_active_trade_treaty(&world, &fa, &fb))
        })?;
    globals.set("has_active_trade_treaty", has_treaty_fn)?;

    // execute_treaty_trade(from, to, kind, amount) — treaty + peace gated move
    let w = world;
    let execute_fn = lua.create_function_mut(
        move |_, (from, to, kind, amount): (u32, u32, String, f64)| -> LuaResult<()> {
            let mut world = w.borrow_mut();
            trade::execute_treaty_trade(&mut world, from, to, &kind, amount)
                .map_err(|e| mlua::Error::external(format!("{}: {e}", trade_variant(&e))))?;
            Ok(())
        },
    )?;
    globals.set("execute_treaty_trade", execute_fn)?;

    Ok(())
}
