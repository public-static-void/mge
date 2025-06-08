use mlua::{Error as LuaError, Lua, Result as LuaResult, Table, Value as LuaValue};
use serde_json::{Map, Value as JsonValue, json};

pub fn luaunit_style_error(_lua: &Lua, msg: &str) -> LuaError {
    let err_json = json!({
        "msg": msg,
        "trace": "",
        "status": null
    });
    LuaError::RuntimeError(err_json.to_string())
}

pub fn lua_error_msg(lua: &Lua, msg: &str) -> LuaError {
    luaunit_style_error(lua, msg)
}

pub fn lua_error_from_any<E: std::fmt::Display>(lua: &mlua::Lua, err: E) -> mlua::Error {
    luaunit_style_error(lua, &format!("{}", err))
}

fn is_marked_array(table: &Table) -> LuaResult<bool> {
    if let Some(mt) = table.metatable() {
        if let Ok(val) = mt.get::<bool>("__is_array") {
            return Ok(val);
        }
    }
    Ok(false)
}

pub fn lua_table_to_json(
    lua: &Lua,
    table: &Table,
    expected_type: Option<&str>,
) -> LuaResult<JsonValue> {
    if is_marked_array(table)?
        || is_array_like(table)?
        || (table.len()? == 0 && expected_type == Some("array"))
    {
        let mut vec = Vec::new();
        for i in 1..=table.len()? {
            let val = table.get::<LuaValue>(i)?;
            vec.push(lua_value_to_json(lua, val, None)?);
        }
        Ok(JsonValue::Array(vec))
    } else {
        let mut map = Map::new();
        for pair in table.clone().pairs::<LuaValue, LuaValue>() {
            let (key, val) = pair?;
            let key_str = lua_value_to_string(&key)?;
            let json_val = match val {
                LuaValue::Nil => JsonValue::Null,
                LuaValue::Table(ref t) => lua_table_to_json(lua, t, None)?,
                _ => lua_value_to_json(lua, val, None)?,
            };
            map.insert(key_str, json_val);
        }
        Ok(JsonValue::Object(map))
    }
}

/// Recursively convert a Lua table to JSON, using a JSON schema (or fragment) to determine
/// whether empty tables should be arrays or objects.
pub fn lua_table_to_json_with_schema(
    lua: &Lua,
    table: &Table,
    schema: &JsonValue,
) -> LuaResult<JsonValue> {
    let props = schema.get("properties").and_then(|p| p.as_object());

    let mut map = Map::new();
    for pair in table.clone().pairs::<LuaValue, LuaValue>() {
        let (key, val) = pair?;
        let key_str = lua_value_to_string(&key)?;
        // Look up type in schema for this property
        let expected_type = props
            .and_then(|props| props.get(&key_str))
            .and_then(|field| field.get("type"))
            .and_then(|t| t.as_str());

        let json_val = match val {
            LuaValue::Table(ref t) => lua_table_to_json(lua, t, expected_type)?,
            _ => lua_value_to_json(lua, val, expected_type)?,
        };
        map.insert(key_str, json_val);
    }
    Ok(JsonValue::Object(map))
}

fn is_array_like(table: &Table) -> LuaResult<bool> {
    let mut has_integer = false;
    let mut has_string = false;
    let mut max_index = 0;
    let mut count = 0;
    for pair in table.clone().pairs::<LuaValue, LuaValue>() {
        let (key, _) = pair?;
        match key {
            LuaValue::Integer(i) => {
                has_integer = true;
                if i > max_index {
                    max_index = i;
                }
                count += 1;
            }
            LuaValue::String(_) => has_string = true,
            _ => {}
        }
    }
    if count == 0 {
        // Empty table: treat as object unless marked as array!
        return Ok(false);
    }
    Ok(has_integer && !has_string && count == max_index)
}

pub fn lua_value_to_json(
    lua: &Lua,
    val: LuaValue,
    expected_type: Option<&str>,
) -> LuaResult<JsonValue> {
    match val {
        LuaValue::Nil => Ok(JsonValue::Null),
        LuaValue::Boolean(b) => Ok(JsonValue::Bool(b)),
        LuaValue::Integer(i) => Ok(JsonValue::Number(i.into())),
        LuaValue::Number(n) => serde_json::Number::from_f64(n)
            .map(JsonValue::Number)
            .ok_or_else(|| mlua::Error::external("Invalid number")),
        LuaValue::String(s) => Ok(JsonValue::String(s.to_str()?.to_string())),
        LuaValue::Table(t) => lua_table_to_json(lua, &t, expected_type),
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

pub fn json_to_lua_table(lua: &mlua::Lua, value: &serde_json::Value) -> mlua::Result<mlua::Value> {
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
