//! Crafting Lua helpers: recipe registration, gating, order control.
//!
//! Exposes the identical 6-op bridge surface as globals (the sandbox blocks
//! `require`, so no module table is used):
//! `register_craft_recipe(name, recipe_json) -> nil | error`,
//! `list_craft_recipes() -> [names]`,
//! `can_craft(crafter, recipe) -> ok, err`,
//! `start_craft(crafter, recipe) -> ok, err`,
//! `get_craft_state(crafter) -> table | nil`,
//! `cancel_craft(crafter) -> ok, err`.
//! Error strings are byte-identical to the Rust/Python/WASM surfaces:
//! `unknown_recipe`, `already_crafting`, `missing_tool:<item>`,
//! `missing_material:<material>`, `missing_input:<kind>`,
//! `insufficient_skill`, `no_craft_order`.
//!
//! Example:
//! ```lua
//! register_craft_recipe("iron_sword", '{"name":"iron_sword","duration":3,...}')
//! local ok, err = start_craft(crafter, "iron_sword")
//! ```

use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the crafting API globals.
pub fn register_craft_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // register_craft_recipe(name, recipe_json) -> nil | error
    let world_register = world.clone();
    let register_craft_recipe =
        lua.create_function_mut(move |_, (name, recipe_json): (String, String)| {
            let recipe: serde_json::Value = serde_json::from_str(&recipe_json)
                .map_err(|e| mlua::Error::runtime(format!("Invalid recipe JSON: {e}")))?;
            let mut world = world_register.borrow_mut();
            world
                .register_craft_recipe(name, recipe)
                .map_err(mlua::Error::runtime)
        })?;
    globals.set("register_craft_recipe", register_craft_recipe)?;

    // list_craft_recipes() -> [names], sorted ascending
    let world_list = world.clone();
    let list_craft_recipes = lua.create_function_mut(move |_, ()| {
        let world = world_list.borrow();
        Ok(world.list_craft_recipes())
    })?;
    globals.set("list_craft_recipes", list_craft_recipes)?;

    // can_craft(crafter, recipe) -> ok, err
    let world_can = world.clone();
    let can_craft = lua.create_function_mut(
        move |_, (crafter, recipe): (u32, String)| -> LuaResult<(bool, Option<String>)> {
            let world = world_can.borrow();
            match world.can_craft(crafter, &recipe) {
                Ok(()) => Ok((true, None)),
                Err(err) => Ok((false, Some(err))),
            }
        },
    )?;
    globals.set("can_craft", can_craft)?;

    // start_craft(crafter, recipe) -> ok, err
    let world_start = world.clone();
    let start_craft = lua.create_function_mut(
        move |_, (crafter, recipe): (u32, String)| -> LuaResult<(bool, Option<String>)> {
            let mut world = world_start.borrow_mut();
            match world.start_craft(crafter, &recipe) {
                Ok(()) => Ok((true, None)),
                Err(err) => Ok((false, Some(err))),
            }
        },
    )?;
    globals.set("start_craft", start_craft)?;

    // get_craft_state(crafter) -> table | nil
    let world_state = world.clone();
    let get_craft_state = lua.create_function_mut(move |lua, crafter: u32| {
        let world = world_state.borrow();
        match world.get_craft_state(crafter) {
            Some(order) => crate::helpers::json_to_lua_table(lua, &order).map(Some),
            None => Ok(None),
        }
    })?;
    globals.set("get_craft_state", get_craft_state)?;

    // cancel_craft(crafter) -> ok, err
    let world_cancel = world;
    let cancel_craft = lua.create_function_mut(
        move |_, crafter: u32| -> LuaResult<(bool, Option<String>)> {
            let mut world = world_cancel.borrow_mut();
            match world.cancel_craft(crafter) {
                Ok(done) => Ok((done, None)),
                Err(err) => Ok((false, Some(err))),
            }
        },
    )?;
    globals.set("cancel_craft", cancel_craft)?;

    Ok(())
}
