//! Loot table API: define and roll loot tables from Lua.

use engine_core::ecs::world::World;
use engine_core::loot::LootEntry;
use mlua::{Lua, Result as LuaResult, Table, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the loot table API as global Lua functions.
pub fn register_loot_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // define_loot_table(name, entries)
    let world_dt = world.clone();
    let define_fn = lua.create_function_mut(
        move |_, (name, entries): (String, Table)| -> LuaResult<bool> {
            let mut entries_vec: Vec<LootEntry> = Vec::new();

            for pair in entries.pairs::<Value, Value>() {
                let (_, val) = pair.map_err(mlua::Error::external)?;
                let entry_table = match val {
                    Value::Table(t) => t,
                    _ => {
                        return Err(mlua::Error::external(
                            "each entry must be a table",
                        ))
                    }
                };

                let item_id: String = entry_table.get("item_id").map_err(|e| {
                    mlua::Error::external(format!("entry missing 'item_id': {e}"))
                })?;
                let weight: u32 = entry_table.get("weight").map_err(|e| {
                    mlua::Error::external(format!("entry missing 'weight': {e}"))
                })?;
                let min_count: u32 = entry_table.get("min_count").unwrap_or(1);
                let max_count: u32 = entry_table.get("max_count").unwrap_or(1);

                entries_vec.push(LootEntry {
                    item_id,
                    weight,
                    min_count,
                    max_count,
                });
            }

            let mut world = world_dt.borrow_mut();
            world
                .loot_tables
                .define_table(&name, entries_vec)
                .map_err(mlua::Error::external)?;
            Ok(true)
        },
    )?;
    globals.set("define_loot_table", define_fn)?;

    // roll_loot_table(name) — returns array of {item_id=, count=} tables
    let world_roll = world.clone();
    let roll_fn = lua.create_function_mut(
        move |lua: &Lua, name: String| -> LuaResult<Table> {
            let world = world_roll.borrow();
            let result = world.loot_tables.roll(&name);

            let results_table = lua.create_table()?;
            if let Ok(items) = result {
                for (i, (item_id, count)) in items.iter().enumerate() {
                    let entry = lua.create_table()?;
                    entry.set("item_id", item_id.as_str())?;
                    entry.set("count", *count)?;
                    results_table.set(i + 1, entry)?;
                }
            }
            Ok(results_table)
        },
    )?;
    globals.set("roll_loot_table", roll_fn)?;

    // has_loot_table(name) — returns boolean
    let world_ht = world.clone();
    let has_fn = lua.create_function_mut(move |_, name: String| -> LuaResult<bool> {
        let world = world_ht.borrow();
        Ok(world.loot_tables.has_table(&name))
    })?;
    globals.set("has_loot_table", has_fn)?;

    Ok(())
}
