use mlua::LuaSerdeExt;
use mlua::{Lua, Result as LuaResult, Table, Value as LuaValue};
use serde_json::{Map, Value as JsonValue};

pub fn lua_table_to_json(lua: &Lua, table: &Table) -> LuaResult<JsonValue> {
    if is_array_like(table)? {
        let mut vec = Vec::new();
        for i in 1..=table.len()? {
            let val = table.get::<_, LuaValue>(i)?;
            vec.push(lua_value_to_json(lua, val)?);
        }
        Ok(JsonValue::Array(vec))
    } else {
        let mut map = Map::new();
        for pair in table.clone().pairs::<LuaValue, LuaValue>() {
            let (key, val) = pair?;
            let key_str = lua_value_to_string(&key)?;
            map.insert(key_str, lua_value_to_json(lua, val)?);
        }
        Ok(JsonValue::Object(map))
    }
}

fn is_array_like(table: &Table) -> LuaResult<bool> {
    let len = table.len()?;
    let mut count = 0;
    for pair in table.clone().pairs::<LuaValue, LuaValue>() {
        let (key, _) = pair?;
        match key {
            LuaValue::Integer(i) if i >= 1 && i <= len => count += 1,
            _ => return Ok(false),
        }
    }
    Ok(count == len)
}

fn lua_value_to_json(lua: &Lua, val: LuaValue) -> LuaResult<JsonValue> {
    match val {
        LuaValue::Nil => Ok(JsonValue::Null),
        LuaValue::Boolean(b) => Ok(JsonValue::Bool(b)),
        LuaValue::Integer(i) => Ok(JsonValue::Number(i.into())),
        LuaValue::Number(n) => serde_json::Number::from_f64(n)
            .map(JsonValue::Number)
            .ok_or_else(|| mlua::Error::external("Invalid number")),
        LuaValue::String(s) => Ok(JsonValue::String(s.to_str()?.to_string())),
        LuaValue::Table(t) => lua_table_to_json(lua, &t),
        _ => Err(mlua::Error::external(
            "Unsupported Lua value for JSON conversion",
        )),
    }
}

fn lua_value_to_string(val: &LuaValue) -> LuaResult<String> {
    match val {
        LuaValue::String(s) => Ok(s.to_str()?.to_string()),
        LuaValue::Integer(i) => Ok(i.to_string()),
        LuaValue::Number(n) => Ok(n.to_string()),
        LuaValue::Boolean(b) => Ok(b.to_string()),
        _ => Err(mlua::Error::external(
            "Unsupported Lua key type for JSON object",
        )),
    }
}

pub fn json_to_lua_table<'lua>(lua: &'lua Lua, value: &JsonValue) -> LuaResult<Table<'lua>> {
    let lua_value = lua.to_value(value)?;
    if let LuaValue::Table(tbl) = lua_value {
        Ok(tbl)
    } else {
        lua.create_table()
    }
}
