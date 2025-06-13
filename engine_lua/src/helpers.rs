use mlua::{Error as LuaError, Lua, Result as LuaResult, Table, Value as LuaValue};
use serde_json::{Map, Value as JsonValue, json};

/// Resolves a $ref in a JSON schema to its target definition.
fn resolve_ref<'a>(schema: &'a JsonValue, reference: &str) -> Option<&'a JsonValue> {
    // Only supports local refs like "#/definitions/BodyPart"
    if let Some(stripped) = reference.strip_prefix("#/definitions/") {
        schema.get("definitions")?.get(stripped)
    } else {
        None
    }
}

/// Ensures all specified array fields are present and are arrays (never null) recursively.
/// Now supports $ref in schema (required for nested BodyPart).
pub fn ensure_schema_arrays(value: &mut JsonValue, schema: &JsonValue) {
    let mut effective_schema = schema;
    // If schema is a $ref, resolve it
    if let Some(ref_str) = schema.get("$ref").and_then(|v| v.as_str()) {
        if let Some(resolved) = resolve_ref(schema, ref_str) {
            effective_schema = resolved;
        }
    }

    if let (JsonValue::Object(map), Some(props)) = (
        value,
        effective_schema
            .get("properties")
            .and_then(|p| p.as_object()),
    ) {
        for (key, prop_schema) in props {
            // If this property is a $ref, resolve it
            let mut field_schema = prop_schema;
            if let Some(ref_str) = prop_schema.get("$ref").and_then(|v| v.as_str()) {
                if let Some(resolved) = resolve_ref(schema, ref_str) {
                    field_schema = resolved;
                }
            }

            if let Some(field_type) = field_schema.get("type").and_then(|t| t.as_str()) {
                if field_type == "array" {
                    match map.get_mut(key) {
                        Some(JsonValue::Null) | None => {
                            map.insert(key.clone(), JsonValue::Array(vec![]));
                        }
                        Some(JsonValue::Array(_)) => {}
                        Some(_) => {}
                    }
                }
                // Recurse into objects/arrays if needed
                if field_type == "object" {
                    if let Some(child) = map.get_mut(key) {
                        ensure_schema_arrays(child, field_schema);
                    }
                } else if field_type == "array" {
                    if let Some(items_schema) = field_schema.get("items") {
                        // If items is a $ref, resolve it
                        let mut item_schema = items_schema;
                        if let Some(ref_str) = items_schema.get("$ref").and_then(|v| v.as_str()) {
                            if let Some(resolved) = resolve_ref(schema, ref_str) {
                                item_schema = resolved;
                            }
                        }
                        if let Some(JsonValue::Array(arr)) = map.get_mut(key) {
                            for item in arr {
                                ensure_schema_arrays(item, item_schema);
                            }
                        }
                    }
                }
            }
        }
    }
}

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

/// Returns true if the table has the global array metatable (`array_mt`).
fn is_marked_array(table: &Table) -> LuaResult<bool> {
    if let Some(mt) = table.metatable() {
        if let Ok(val) = mt.get::<bool>("__is_array") {
            return Ok(val);
        }
    }
    Ok(false)
}

/// Returns true if the table has only integer keys from 1..N (array-like).
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

/// Returns true if the table has any string keys.
fn has_string_keys(table: &Table) -> LuaResult<bool> {
    for pair in table.clone().pairs::<LuaValue, LuaValue>() {
        let (key, _) = pair?;
        if let LuaValue::String(_) = key {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Converts a Lua table to JSON, using metatable and structure to distinguish arrays/objects.
/// Modular, generic, and robust: never serializes as an array if there are any string keys.
pub fn lua_table_to_json(
    lua: &Lua,
    table: &Table,
    expected_type: Option<&str>,
) -> LuaResult<JsonValue> {
    // Only treat as array if:
    // - Marked as array OR array-like OR expected_type is "array"
    // - AND does NOT have any string keys
    let is_array = (is_marked_array(table)?
        || is_array_like(table)?
        || (table.len()? == 0 && expected_type == Some("array")))
        && !has_string_keys(table)?;
    if is_array {
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

/// Converts a Lua table to JSON using a schema to disambiguate empty arrays/objects.
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

pub fn json_to_lua_table(lua: &mlua::Lua, value: &JsonValue) -> mlua::Result<LuaValue> {
    match value {
        JsonValue::Null => Ok(LuaValue::Nil),
        JsonValue::Bool(b) => Ok(LuaValue::Boolean(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(LuaValue::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(LuaValue::Number(f))
            } else {
                Err(LuaError::external("Invalid number"))
            }
        }
        JsonValue::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
        JsonValue::Array(arr) => {
            let tbl = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                tbl.set(i + 1, json_to_lua_table(lua, v)?)?;
            }
            // Mark as array for roundtrip
            if let Ok(Some(array_mt)) = lua.globals().get::<Option<Table>>("array_mt") {
                tbl.set_metatable(Some(array_mt));
            }
            Ok(LuaValue::Table(tbl))
        }
        JsonValue::Object(map) => {
            let tbl = lua.create_table()?;
            for (k, v) in map {
                tbl.set(k.as_str(), json_to_lua_table(lua, v)?)?;
            }
            Ok(LuaValue::Table(tbl))
        }
    }
}
