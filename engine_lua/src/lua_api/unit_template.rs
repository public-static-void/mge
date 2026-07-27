//! Unit template scripting API for Lua.

use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers unit template API functions into the Lua globals.
pub fn register_unit_template_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // load_unit_templates(dir: string)
    let w = world.clone();
    let load_unit_templates = lua.create_function_mut(move |_, dir: String| {
        let path = std::path::Path::new(&dir);
        let mut world = w.borrow_mut();
        world
            .template_registry
            .load_templates_from_dir(path)
            .map_err(mlua::Error::runtime)
    })?;
    globals.set("load_unit_templates", load_unit_templates)?;

    // register_unit_template(name: string, template_json: string)
    let w = world.clone();
    let register_unit_template =
        lua.create_function_mut(move |_, (_name, template_json): (String, String)| {
            let template: engine_core::ecs::template::UnitTemplate =
                serde_json::from_str(&template_json)
                    .map_err(|e| mlua::Error::runtime(format!("Invalid template JSON: {e}")))?;
            let mut world = w.borrow_mut();
            world.template_registry.register_template(template);
            Ok(())
        })?;
    globals.set("register_unit_template", register_unit_template)?;

    // spawn_from_template(template_name: string, overrides?: table) -> integer
    let w = world.clone();
    let spawn_from_template = lua.create_function_mut(
        move |lua, (template_name, overrides): (String, Option<Table>)| {
            let overrides_map = match overrides {
                Some(t) => {
                    let val = crate::helpers::lua_table_to_json(lua, &t, None)?;
                    match val {
                        serde_json::Value::Object(map) => Some(map),
                        _ => return Err(mlua::Error::runtime("Overrides must be a table")),
                    }
                }
                None => None,
            };
            let mut world = w.borrow_mut();
            world
                .spawn_from_template(&template_name, overrides_map)
                .map_err(mlua::Error::runtime)
        },
    )?;
    globals.set("spawn_from_template", spawn_from_template)?;

    // get_unit_template(name: string) -> table | nil
    let w = world.clone();
    let get_unit_template = lua.create_function_mut(move |lua, name: String| {
        let world = w.borrow();
        match world.template_registry.get_template(&name) {
            Some(tmpl) => {
                let json =
                    serde_json::to_value(tmpl).map_err(|e| mlua::Error::runtime(e.to_string()))?;
                crate::helpers::json_to_lua_table(lua, &json).map(Some)
            }
            None => Ok(None),
        }
    })?;
    globals.set("get_unit_template", get_unit_template)?;

    // list_unit_templates() -> table (array of strings)
    let w = world.clone();
    let list_unit_templates = lua.create_function_mut(move |_, ()| {
        let world = w.borrow();
        let names = world.template_registry.list_templates();
        Ok(names)
    })?;
    globals.set("list_unit_templates", list_unit_templates)?;

    Ok(())
}
