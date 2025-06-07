use crate::ecs::world::World;
use mlua::{Lua, Result as LuaResult, UserData, UserDataMethods};
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_world_api(
    lua: &Lua,
    globals: &mlua::Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    let world_userdata = lua.create_userdata(WorldWrapper(world.clone()))?;
    globals.set("world", world_userdata)?;
    Ok(())
}

#[derive(Clone)]
pub struct WorldWrapper(pub Rc<RefCell<World>>);

impl UserData for WorldWrapper {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "apply_generated_map",
            |lua, this, map_table: mlua::Table| {
                let map_json = crate::scripting::helpers::lua_table_to_json(lua, &map_table, None)?;
                let mut world = this.0.borrow_mut();
                world
                    .apply_generated_map(&map_json)
                    .map_err(mlua::Error::external)
            },
        );

        methods.add_method("get_map_topology_type", |_, this, ()| {
            let world = this.0.borrow();
            Ok(world
                .map
                .as_ref()
                .map(|m| m.topology_type().to_string())
                .unwrap_or_else(|| "none".to_string()))
        });

        methods.add_method("get_map_cell_count", |_, this, ()| {
            let world = this.0.borrow();
            Ok(world.map.as_ref().map(|m| m.all_cells().len()).unwrap_or(0))
        });
    }
}
