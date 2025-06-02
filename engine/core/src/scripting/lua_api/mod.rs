//! Scripting API bridge: orchestrates registration of all Lua API subsystems.

pub mod body;
pub mod camera;
pub mod component;
pub mod economic;
pub mod entity;
pub mod equipment;
pub mod inventory;
pub mod map;
pub mod misc;
pub mod region;
pub mod ui;
pub mod worldgen;

use crate::ecs::world::World;
use crate::scripting::input::InputProvider;
use crate::worldgen::WorldgenRegistry;

use mlua::{Lua, Result as LuaResult, Table};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// Registers all Lua API functions into the given globals table.
pub fn register_all_api_functions(
    lua: &Lua,
    globals: &Table,
    world: Rc<RefCell<World>>,
    input_provider: Arc<Mutex<Box<dyn InputProvider + Send + Sync>>>,
    worldgen_registry: Rc<RefCell<WorldgenRegistry>>,
) -> LuaResult<()> {
    entity::register_entity_api(lua, globals, world.clone())?;
    component::register_component_api(lua, globals, world.clone())?;
    inventory::register_inventory_api(lua, globals, world.clone())?;
    equipment::register_equipment_api(lua, globals, world.clone())?;
    body::register_body_api(lua, globals, world.clone())?;
    region::register_region_api(lua, globals, world.clone())?;
    camera::register_camera_api(lua, globals, world.clone())?;
    ui::register_ui_api(lua, globals)?;
    worldgen::register_worldgen_api(lua, globals, worldgen_registry)?;
    misc::register_misc_api(lua, globals, world.clone(), input_provider)?;
    map::register_map_api(lua, globals, world.clone())?;
    economic::register_economic_api(lua, globals, world.clone())?;
    Ok(())
}
