//! Miscellaneous API: tick, turn, user input, etc.

use crate::ecs::world::World;
use crate::scripting::helpers::{lua_error_from_any, lua_error_msg};
use crate::scripting::input::InputProvider;
use crate::systems::standard::{DamageAll, MoveAll, MoveDelta, ProcessDeaths, ProcessDecay};
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub fn register_misc_api(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
    input_provider: Arc<Mutex<Box<dyn InputProvider + Send + Sync>>>,
) -> LuaResult<()> {
    // set_mode(mode: String)
    let world_set_mode = world.clone();
    let set_mode = lua.create_function_mut(move |_, mode: String| {
        let mut world = world_set_mode.borrow_mut();
        world.current_mode = mode;
        Ok(())
    })?;
    globals.set("set_mode", set_mode)?;

    // get_mode()
    let world_get_mode = world.clone();
    let get_mode = lua.create_function_mut(move |_, ()| {
        let world = world_get_mode.borrow();
        Ok(world.current_mode.clone())
    })?;
    globals.set("get_mode", get_mode)?;

    // get_available_modes()
    let world_get_modes = world.clone();
    let get_available_modes = lua.create_function_mut(move |_, ()| {
        let world = world_get_modes.borrow();
        let modes = world.registry.lock().unwrap().all_modes();
        Ok(modes.into_iter().collect::<Vec<String>>())
    })?;
    globals.set("get_available_modes", get_available_modes)?;

    // is_entity_alive(entity)
    let world_is_alive = world.clone();
    let is_entity_alive = lua.create_function_mut(move |_, entity: u32| {
        let world = world_is_alive.borrow();
        Ok(world.is_entity_alive(entity))
    })?;
    globals.set("is_entity_alive", is_entity_alive)?;

    // get_user_input(prompt)
    let input_provider_clone = input_provider.clone();
    let get_user_input = lua.create_function(move |lua, prompt: String| {
        let mut provider = input_provider_clone
            .lock()
            .map_err(|_| lua_error_msg(lua, "Input provider lock poisoned"))?;
        provider
            .read_line(&prompt)
            .map_err(|e| lua_error_msg(lua, &format!("Input error: {e}")))
    })?;
    globals.set("get_user_input", get_user_input)?;

    // count_entities_with_type(type_str)
    let world_count_type = world.clone();
    let count_entities_with_type = lua.create_function_mut(move |_, type_str: String| {
        let world = world_count_type.borrow();
        Ok(world.count_entities_with_type(&type_str))
    })?;
    globals.set("count_entities_with_type", count_entities_with_type)?;

    // modify_stockpile_resource(entity, kind, delta)
    let world_modify_stockpile = world.clone();
    let modify_stockpile_resource =
        lua.create_function_mut(move |lua, (entity, kind, delta): (u32, String, f64)| {
            let mut world = world_modify_stockpile.borrow_mut();
            world
                .modify_stockpile_resource(entity, &kind, delta)
                .map_err(|e| lua_error_from_any(lua, e))
        })?;
    globals.set("modify_stockpile_resource", modify_stockpile_resource)?;

    // save_to_file(filename)
    let world_save = world.clone();
    let save_to_file = lua.create_function_mut(move |lua, filename: String| {
        let world = world_save.borrow();
        world
            .save_to_file(std::path::Path::new(&filename))
            .map_err(|e| lua_error_from_any(lua, e))
    })?;
    globals.set("save_to_file", save_to_file)?;

    // load_from_file(filename)
    let world_load = world.clone();
    let registry = world.borrow().registry.clone();
    let load_from_file = lua.create_function_mut(move |lua, filename: String| {
        let mut world = world_load.borrow_mut();
        let loaded = World::load_from_file(std::path::Path::new(&filename), registry.clone())
            .map_err(|e| lua_error_from_any(lua, e))?;
        *world = loaded;
        Ok(())
    })?;
    globals.set("load_from_file", load_from_file)?;

    // move_entity(entity, dx, dy)
    let world_move_entity = world.clone();
    let move_entity = lua.create_function_mut(move |_, (entity, dx, dy): (u32, f32, f32)| {
        let mut world = world_move_entity.borrow_mut();
        world.move_entity(entity, dx, dy);
        Ok(())
    })?;
    globals.set("move_entity", move_entity)?;

    // move_all(dx, dy)
    let world_move = world.clone();
    let move_all = lua.create_function_mut(move |_, (dx, dy): (i32, i32)| {
        let mut world = world_move.borrow_mut();
        world.register_system(MoveAll {
            delta: MoveDelta::Square { dx, dy, dz: 0 },
        });
        world.run_system("MoveAll", None).unwrap();
        Ok(())
    })?;
    globals.set("move_all", move_all)?;

    // damage_entity(entity, amount)
    let world_damage_entity = world.clone();
    let damage_entity = lua.create_function_mut(move |_, (entity, amount): (u32, f32)| {
        let mut world = world_damage_entity.borrow_mut();
        world.damage_entity(entity, amount);
        Ok(())
    })?;
    globals.set("damage_entity", damage_entity)?;

    // damage_all(amount)
    let world_damage = world.clone();
    let damage_all = lua.create_function_mut(move |_, amount: f32| {
        let mut world = world_damage.borrow_mut();
        world.register_system(DamageAll { amount });
        world.run_system("DamageAll", None).unwrap();
        Ok(())
    })?;
    globals.set("damage_all", damage_all)?;

    // tick()
    let world_tick = world.clone();
    let tick = lua.create_function_mut(move |_, ()| {
        let mut world = world_tick.borrow_mut();
        world.tick();
        Ok(())
    })?;
    globals.set("tick", tick)?;

    // get_turn()
    let world_get_turn = world.clone();
    let get_turn = lua.create_function_mut(move |_, ()| {
        let world = world_get_turn.borrow();
        Ok(world.turn)
    })?;
    globals.set("get_turn", get_turn)?;

    // process_deaths()
    let world_deaths = world.clone();
    let process_deaths = lua.create_function_mut(move |_, ()| {
        let mut world = world_deaths.borrow_mut();
        world.register_system(ProcessDeaths);
        world.run_system("ProcessDeaths", None).unwrap();
        Ok(())
    })?;
    globals.set("process_deaths", process_deaths)?;

    // process_decay()
    let world_decay = world.clone();
    let process_decay = lua.create_function_mut(move |_, ()| {
        let mut world = world_decay.borrow_mut();
        world.register_system(ProcessDecay);
        world.run_system("ProcessDecay", None).unwrap();
        Ok(())
    })?;
    globals.set("process_decay", process_decay)?;

    // get_time_of_day()
    let world_time = world.clone();
    let get_time_of_day = lua.create_function_mut(move |lua, ()| {
        let world = world_time.borrow();
        let time = world.get_time_of_day();
        let tbl = lua.create_table()?;
        tbl.set("hour", time.hour)?;
        tbl.set("minute", time.minute)?;
        Ok(tbl)
    })?;
    globals.set("get_time_of_day", get_time_of_day)?;

    Ok(())
}
