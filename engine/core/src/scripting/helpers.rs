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

pub fn json_to_lua_table<'lua>(
    lua: &'lua mlua::Lua,
    value: &serde_json::Value,
) -> mlua::Result<mlua::Value<'lua>> {
    match value {
        serde_json::Value::Null => Ok(mlua::Value::Nil),
        serde_json::Value::Bool(b) => Ok(mlua::Value::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(mlua::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(mlua::Value::Number(f))
            } else {
                Err(mlua::Error::external("Invalid number"))
            }
        }
        serde_json::Value::String(s) => Ok(mlua::Value::String(lua.create_string(s)?)),
        serde_json::Value::Array(arr) => {
            let tbl = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                tbl.set(i + 1, json_to_lua_table(lua, v)?)?;
            }
            Ok(mlua::Value::Table(tbl))
        }
        serde_json::Value::Object(map) => {
            let tbl = lua.create_table()?;
            for (k, v) in map {
                tbl.set(k.as_str(), json_to_lua_table(lua, v)?)?;
            }
            Ok(mlua::Value::Table(tbl))
        }
    }
}
