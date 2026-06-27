//! Dungeon generation API: `generate_dungeon(config)` Lua global.
//!
//! Provides a stateless function that delegates to `DungeonGenerator::generate()`
//! and returns the result as a Lua table compatible with `world:apply_generated_map()`.

use engine_core::systems::dungeon::{DungeonConfig, DungeonGenerator};
use mlua::{Lua, Result as LuaResult, Table};

/// Registers the dungeon generation API.
pub fn register_dungeon_api(lua: &Lua, globals: &Table) -> LuaResult<()> {
    let generate_dungeon = lua.create_function_mut(move |lua, config: Option<Table>| {
        // Parse config from Lua table or use defaults
        let mut dungeon_config = DungeonConfig::default();

        if let Some(table) = config {
            // Only apply user-provided values; zero or non-numeric → error or default
            if let Ok(val) = table.get::<f64>("width") {
                let w = val as u32;
                if w == 0 {
                    return Err(mlua::Error::external(
                        "Width must be positive".to_string(),
                    ));
                }
                dungeon_config.width = w;
            }
            if let Ok(val) = table.get::<f64>("height") {
                let h = val as u32;
                if h == 0 {
                    return Err(mlua::Error::external(
                        "Height must be positive".to_string(),
                    ));
                }
                dungeon_config.height = h;
            }
            if let Ok(val) = table.get::<f64>("seed") {
                dungeon_config.seed = val as u64;
            }
            if let Ok(val) = table.get::<f64>("min_room_size") {
                dungeon_config.min_room_size = val as u32;
            }
            if let Ok(val) = table.get::<f64>("max_room_size") {
                dungeon_config.max_room_size = val as u32;
            }
            if let Ok(val) = table.get::<f64>("max_rooms") {
                dungeon_config.max_rooms = val as u32;
            }
        }

        // Generate the dungeon map
        let map = DungeonGenerator::generate(&dungeon_config)
            .map_err(|e| mlua::Error::external(e))?;

        // Convert to worldgen JSON format
        let json_value = map.to_worldgen_json();

        // Convert JSON to Lua table using existing helper
        crate::helpers::json_to_lua_table(lua, &json_value)
    })?;

    globals.set("generate_dungeon", generate_dungeon)?;
    Ok(())
}
