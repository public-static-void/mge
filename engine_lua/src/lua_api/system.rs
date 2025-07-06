use crate::helpers::{lua_error_from_any, lua_error_msg};
use engine_core::ecs::world::World;
use mlua::{Function, Lua, RegistryKey, Result as LuaResult, Table};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Registers all system functions and Lua bridge APIs except job system APIs.
pub fn register_system_functions(
    lua: Rc<Lua>,
    globals: &Table,
    world: Rc<RefCell<World>>,
    lua_systems: Rc<RefCell<HashMap<String, RegistryKey>>>,
) -> LuaResult<()> {
    let lua_systems_outer = Rc::clone(&lua_systems);
    let world_rc = world.clone();
    let lua_outer = lua.clone();
    let register_system = lua.create_function_mut(
        move |_, (name, func, opts): (String, Function, Option<Table>)| {
            let key = lua_outer.create_registry_value(func)?;
            lua_systems_outer.borrow_mut().insert(name.clone(), key);

            let mut dependencies = Vec::new();
            if let Some(opts) = opts {
                if let Ok(dep_table) = opts.get::<Table>("dependencies") {
                    for dep in dep_table.sequence_values::<String>() {
                        dependencies.push(dep?);
                    }
                }
            }

            let system_name_for_closure = name.clone();
            let system_name_for_fn = system_name_for_closure.clone();
            let lua_systems_inner = Rc::clone(&lua_systems_outer);
            let lua_inner = lua_outer.clone();

            world_rc.borrow_mut().register_dynamic_system_with_deps(
                &system_name_for_closure,
                dependencies,
                move |_world, dt| {
                    let binding = lua_systems_inner.borrow();
                    let key = binding
                        .get(&system_name_for_fn)
                        .expect("Lua system not found");
                    let func: Function = lua_inner
                        .registry_value(key)
                        .expect("Invalid Lua registry key");
                    let _ = func.call::<()>(dt);
                },
            );

            Ok(())
        },
    )?;
    globals.set("register_system", register_system)?;

    let lua_systems_ref = Rc::clone(&lua_systems);
    let run_system = lua.create_function_mut(move |lua, name: String| {
        let systems = lua_systems_ref.borrow();
        if let Some(key) = systems.get(&name) {
            let func: Function = lua.registry_value(key)?;
            func.call::<()>(())?;
            Ok(())
        } else {
            Err(lua_error_msg(lua, "system not found"))
        }
    })?;
    globals.set("run_system", run_system)?;

    let world_native_run = world.clone();
    let lua_for_native = lua.clone();
    let run_native_system = lua.create_function_mut(move |_, name: String| {
        let mut world = world_native_run.borrow_mut();
        world
            .run_system(&name, None)
            .map_err(|e| lua_error_from_any(&lua_for_native, e))
    })?;
    globals.set("run_native_system", run_native_system)?;

    Ok(())
}
