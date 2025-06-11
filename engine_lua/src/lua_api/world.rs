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
        map_validators: RefCell::new(Vec::new()),
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
    pub map_validators: RefCell<Vec<RegistryKey>>,
    pub self_userdata: RefCell<Option<AnyUserData>>,
}

impl UserData for WorldWrapper {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // Map Validator Registration
        methods.add_method("register_map_validator", |lua, this, func: Function| {
            let key = lua.create_registry_value(func)?;
            this.map_validators.borrow_mut().push(key);
            Ok(())
        });

        methods.add_method("clear_map_validators", |_, this, ()| {
            this.map_validators.borrow_mut().clear();
            Ok(())
        });

        // Map Postprocessor Registration
        methods.add_method("register_map_postprocessor", |lua, this, func: Function| {
            let key = lua.create_registry_value(func)?;
            this.map_postprocessors.borrow_mut().push(key);
            Ok(())
        });

        methods.add_method("clear_map_postprocessors", |_, this, ()| {
            this.map_postprocessors.borrow_mut().clear();
            Ok(())
        });

        // Apply Generated Map: run validators, then apply, then postprocessors
        methods.add_method("apply_generated_map", |lua, this, map_table: Table| {
            let map_json = crate::helpers::lua_table_to_json(lua, &map_table, None)?;

            // Run all validators first, fail fast on error
            let validators = this.map_validators.borrow();
            for key in validators.iter() {
                let func: Function = lua.registry_value(key)?;
                let ok: bool = func.call(map_table.clone())?;
                if !ok {
                    return Err(mlua::Error::external("Map validator failed"));
                }
            }

            // Apply map
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
                func.call::<()>(world_userdata.clone())?;
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

        methods.add_method("apply_chunk", |lua, this, chunk_table: Table| {
            let chunk_json = crate::helpers::lua_table_to_json(lua, &chunk_table, None)?;
            let mut world = this.world.borrow_mut();
            world
                .apply_chunk(&chunk_json)
                .map_err(mlua::Error::external)?;
            Ok(())
        });
    }
}
