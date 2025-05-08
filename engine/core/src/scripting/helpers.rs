use mlua::LuaSerdeExt;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use serde_json::Value as JsonValue;

/// Convert a Lua table to a serde_json::Value
pub fn lua_table_to_json(lua: &Lua, table: &Table) -> LuaResult<JsonValue> {
    lua.from_value(LuaValue::Table(table.clone()))
}

/// Convert a serde_json::Value to a Lua table
pub fn json_to_lua_table<'lua>(lua: &'lua Lua, value: &JsonValue) -> LuaResult<Table<'lua>> {
    let lua_value = lua.to_value(value)?;
    if let LuaValue::Table(tbl) = lua_value {
        Ok(tbl)
    } else {
        lua.create_table()
    }
}
