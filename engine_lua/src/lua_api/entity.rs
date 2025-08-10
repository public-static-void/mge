//! Entity management API: spawn, despawn, queries, state, movement, damage.

use engine_core::ecs::world::World;
use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;

/// Registers the entity API.
pub fn register_entity_api(lua: &Lua, globals: &Table, world: Rc<RefCell<World>>) -> LuaResult<()> {
    // spawn_entity()
    let world_spawn = world.clone();
    let spawn_entity = lua.create_function_mut(move |_, ()| {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut world = world_spawn.borrow_mut();
            let eid = world.spawn_entity();
            Ok(eid)
        }));
        match result {
            Ok(ok) => ok,
            Err(_) => Err(mlua::Error::external("spawn_entity panicked")),
        }
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

    // get_entities_with_component(name)
    let world_get_entities = world.clone();
    let get_entities_with_component = lua.create_function_mut(move |_lua, name: String| {
        let world = world_get_entities.borrow();
        Ok(world.get_entities_with_component(&name))
    })?;
    globals.set("get_entities_with_component", get_entities_with_component)?;

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

    // count_entities_with_type(type_str)
    let world_count_type = world.clone();
    let count_entities_with_type = lua.create_function_mut(move |_, type_str: String| {
        let world = world_count_type.borrow();
        Ok(world.count_entities_with_type(&type_str))
    })?;
    globals.set("count_entities_with_type", count_entities_with_type)?;

    // is_entity_alive(entity)
    let world_is_alive = world.clone();
    let is_entity_alive = lua.create_function_mut(move |_, entity: u32| {
        let world = world_is_alive.borrow();
        Ok(world.is_entity_alive(entity))
    })?;
    globals.set("is_entity_alive", is_entity_alive)?;

    // move_entity(entity, dx, dy)
    let world_move_entity = world.clone();
    let move_entity = lua.create_function_mut(move |_, (entity, dx, dy): (u32, f32, f32)| {
        let mut world = world_move_entity.borrow_mut();
        world.move_entity(entity, dx, dy);
        Ok(())
    })?;
    globals.set("move_entity", move_entity)?;

    // damage_entity(entity, amount)
    let world_damage_entity = world.clone();
    let damage_entity = lua.create_function_mut(move |_, (entity, amount): (u32, f32)| {
        let mut world = world_damage_entity.borrow_mut();
        world.damage_entity(entity, amount);
        Ok(())
    })?;
    globals.set("damage_entity", damage_entity)?;

    Ok(())
}
