//! Material system Lua helpers: lookup, attach, query.

use engine_core::ecs::world::World;
use engine_core::material;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the material API functions into the Lua globals.
pub fn register_material_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // get_material_properties(name) -> table or nil
    let w = world.clone();
    let get_material_properties = lua.create_function_mut(move |lua, name: String| {
        let world = w.borrow();
        let def = material::get_material_properties(&world, &name);
        // If the name wasn't found, default_material() is returned.
        // Return nil only if the material_definitions map is empty and
        // the caller asked for something that truly doesn't exist.
        if world.material_definitions.contains_key(&name) {
            crate::helpers::json_to_lua_table(lua, &def)
        } else if def.get("name").and_then(|v| v.as_str()) == Some("default") {
            Ok(mlua::Value::Nil)
        } else {
            crate::helpers::json_to_lua_table(lua, &def)
        }
    })?;
    globals.set("get_material_properties", get_material_properties)?;

    // set_entity_material(eid, name) -> nil or error string
    let w = world.clone();
    let set_entity_material = lua.create_function_mut(move |lua, (eid, name): (u32, String)| {
        let mut world = w.borrow_mut();
        match material::set_entity_material(&mut world, eid, &name) {
            Ok(()) => Ok(mlua::Value::Nil),
            Err(e) => Ok(mlua::Value::String(lua.create_string(e)?)),
        }
    })?;
    globals.set("set_entity_material", set_entity_material)?;

    // get_entity_material(eid) -> table or nil
    let w = world.clone();
    let get_entity_material = lua.create_function_mut(move |lua, eid: u32| {
        let world = w.borrow();
        match material::get_entity_material(&world, eid) {
            Some(val) => crate::helpers::json_to_lua_table(lua, &val),
            None => Ok(mlua::Value::Nil),
        }
    })?;
    globals.set("get_entity_material", get_entity_material)?;

    // get_material_names() -> list
    let w = world.clone();
    let get_material_names = lua.create_function_mut(move |lua, ()| {
        let world = w.borrow();
        let names = material::get_material_names(&world);
        let arr =
            serde_json::Value::Array(names.into_iter().map(serde_json::Value::String).collect());
        crate::helpers::json_to_lua_table(lua, &arr)
    })?;
    globals.set("get_material_names", get_material_names)?;

    Ok(())
}
