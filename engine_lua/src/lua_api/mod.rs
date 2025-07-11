//! Scripting API bridge: orchestrates registration of all Lua API subsystems.

pub mod body;
pub mod camera;
pub mod component;
pub mod death_decay;
pub mod economic;
pub mod entity;
pub mod equipment;
pub mod event_bus;
pub mod input;
pub mod inventory;
pub mod job_board;
pub mod job_cancel;
pub mod job_events;
pub mod job_mutation;
pub mod job_query;
pub mod job_system;
pub mod map;
pub mod mode;
pub mod region;
pub mod save_load;
pub mod system;
pub mod time_of_day;
pub mod turn;
pub mod ui;
pub mod world;
pub mod worldgen;

use crate::input::InputProvider;
use engine_core::ecs::world::World;
use engine_core::worldgen::WorldgenRegistry;

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
    input::register_input_api(lua, globals, input_provider)?;
    inventory::register_inventory_api(lua, globals, world.clone())?;
    equipment::register_equipment_api(lua, globals, world.clone())?;
    body::register_body_api(lua, globals, world.clone())?;
    region::register_region_api(lua, globals, world.clone())?;
    camera::register_camera_api(lua, globals, world.clone())?;
    ui::register_ui_api(lua, globals)?;
    worldgen::register_worldgen_api(lua, globals, worldgen_registry)?;
    mode::register_mode_api(lua, globals, world.clone())?;
    turn::register_turn_api(lua, globals, world.clone())?;
    save_load::register_save_load_api(lua, globals, world.clone())?;
    death_decay::register_death_decay_api(lua, globals, world.clone())?;
    time_of_day::register_time_of_day_api(lua, globals, world.clone())?;
    map::register_map_api(lua, globals, world.clone())?;
    economic::register_economic_api(lua, globals, world.clone())?;
    Ok(())
}
