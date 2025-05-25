//! Entity management API: spawn, despawn, basic queries.

use crate::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_entity_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // spawn_entity()
    let world_spawn = world.clone();
    let spawn_entity = lua.create_function_mut(move |_, ()| {
        let mut world = world_spawn.borrow_mut();
        Ok(world.spawn_entity())
    })?;
    globals.set("spawn_entity", spawn_entity)?;

    // despawn_entity(id)
    let world_remove = world.clone();
    let despawn_entity = lua.create_function_mut(move |_, entity_id: u32| {
        let mut world = world_remove.borrow_mut();
        world.despawn_entity(entity_id);
        Ok(())
    })?;
    globals.set("despawn_entity", despawn_entity)?;

    // get_entities()
    let world_get_entities = world.clone();
    let get_entities = lua.create_function_mut(move |_, ()| {
        let world = world_get_entities.borrow();
        Ok(world.entities.clone())
    })?;
    globals.set("get_entities", get_entities)?;

    // get_entities_with_components(names)
    let world_query = world.clone();
    let get_entities_with_components = lua.create_function_mut(move |_lua, names: Table| {
        let world = world_query.borrow();
        let mut rust_names = Vec::new();
        for pair in names.sequence_values::<String>() {
            rust_names.push(pair?);
        }
        let name_refs: Vec<&str> = rust_names.iter().map(|s| s.as_str()).collect();
        Ok(world.get_entities_with_components(&name_refs))
    })?;
    globals.set("get_entities_with_components", get_entities_with_components)?;

    Ok(())
}
