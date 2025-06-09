use engine_core::ecs::world::World;
use mlua::{
    AnyUserData, Function, Lua, RegistryKey, Result as LuaResult, Table, UserData, UserDataMethods,
};
use std::cell::RefCell;
use std::rc::Rc;

pub fn register_world_api(
    lua: &Lua,
    globals: &mlua::Table,
    world: Rc<RefCell<World>>,
) -> LuaResult<()> {
    // Create the world userdata and store a reference to itself for postprocessor calls
    let world_userdata = lua.create_userdata(WorldWrapper {
        world: world.clone(),
        map_postprocessors: RefCell::new(Vec::new()),
        self_userdata: RefCell::new(None),
    })?;
    // Store a reference to itself
    {
        let any_ud: AnyUserData = world_userdata.clone();
        world_userdata
            .borrow_mut::<WorldWrapper>()?
            .self_userdata
            .replace(Some(any_ud));
    }
    globals.set("world", world_userdata)?;
    Ok(())
}

pub struct WorldWrapper {
    pub world: Rc<RefCell<World>>,
    pub map_postprocessors: RefCell<Vec<RegistryKey>>,
    pub self_userdata: RefCell<Option<AnyUserData>>,
}

impl UserData for WorldWrapper {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // Register a map postprocessor (called after apply_generated_map)
        methods.add_method("register_map_postprocessor", |lua, this, func: Function| {
            let key = lua.create_registry_value(func)?;
            this.map_postprocessors.borrow_mut().push(key);
            Ok(())
        });

        // Clear all registered map postprocessors
        methods.add_method("clear_map_postprocessors", |_, this, ()| {
            this.map_postprocessors.borrow_mut().clear();
            Ok(())
        });

        // Apply generated map and run all postprocessors
        methods.add_method("apply_generated_map", |lua, this, map_table: Table| {
            let map_json = crate::helpers::lua_table_to_json(lua, &map_table, None)?;
            {
                let mut world = this.world.borrow_mut();
                world
                    .apply_generated_map(&map_json)
                    .map_err(mlua::Error::external)?;
            }
            // Run all postprocessors, fail fast on error
            let postprocessors = this.map_postprocessors.borrow();
            let world_userdata = this
                .self_userdata
                .borrow()
                .as_ref()
                .expect("self_userdata not set")
                .clone();
            for key in postprocessors.iter() {
                let func: Function = lua.registry_value(key)?;
                func.call::<()>(world_userdata.clone())?; // pass world userdata
            }
            Ok(())
        });

        methods.add_method("get_map_topology_type", |_, this, ()| {
            let world = this.world.borrow();
            Ok(world
                .map
                .as_ref()
                .map(|m| m.topology_type().to_string())
                .unwrap_or_else(|| "none".to_string()))
        });

        methods.add_method("get_map_cell_count", |_, this, ()| {
            let world = this.world.borrow();
            Ok(world.map.as_ref().map(|m| m.all_cells().len()).unwrap_or(0))
        });
    }
}
