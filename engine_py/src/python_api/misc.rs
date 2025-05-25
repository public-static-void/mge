use super::PyWorld;
use pyo3::prelude::*;

pub trait MiscApi {
    fn move_entity(&self, entity_id: u32, dx: f32, dy: f32);
    fn move_all(&self, dx: i32, dy: i32);
    fn tick(&self);
    fn get_turn(&self) -> u32;
    fn set_mode(&self, mode: String);
    fn get_mode(&self) -> String;
    fn get_available_modes(&self) -> Vec<String>;
    fn damage_entity(&self, entity_id: u32, amount: f32);
    fn damage_all(&self, amount: f32);
    fn process_deaths(&self);
    fn process_decay(&self);
    fn count_entities_with_type(&self, type_str: String) -> usize;
    fn modify_stockpile_resource(&self, entity_id: u32, kind: String, delta: f64) -> PyResult<()>;
    fn save_to_file(&self, path: String) -> PyResult<()>;
    fn load_from_file(&mut self, path: String) -> PyResult<()>;
}

impl MiscApi for PyWorld {
    fn move_entity(&self, entity_id: u32, dx: f32, dy: f32) {
        let mut world = self.inner.borrow_mut();
        world.move_entity(entity_id, dx, dy);
    }

    fn move_all(&self, dx: i32, dy: i32) {
        let mut world = self.inner.borrow_mut();
        world.register_system(engine_core::systems::standard::MoveAll {
            delta: engine_core::systems::standard::MoveDelta::Square { dx, dy, dz: 0 },
        });
        world.run_system("MoveAll", None).unwrap();
    }

    fn tick(&self) {
        let mut world = self.inner.borrow_mut();
        world.run_system("MoveAll", None).unwrap();
        world.run_system("DamageAll", None).unwrap();
        world.run_system("ProcessDeaths", None).unwrap();
        world.run_system("ProcessDecay", None).unwrap();
        world.turn += 1;
    }

    fn get_turn(&self) -> u32 {
        let world = self.inner.borrow_mut();
        world.turn
    }

    fn set_mode(&self, mode: String) {
        let mut world = self.inner.borrow_mut();
        world.current_mode = mode;
    }

    fn get_mode(&self) -> String {
        let world = self.inner.borrow_mut();
        world.current_mode.clone()
    }

    fn get_available_modes(&self) -> Vec<String> {
        let world = self.inner.borrow_mut();
        world
            .registry
            .lock()
            .unwrap()
            .all_modes()
            .into_iter()
            .collect()
    }

    fn damage_entity(&self, entity_id: u32, amount: f32) {
        let mut world = self.inner.borrow_mut();
        world.damage_entity(entity_id, amount);
    }

    fn damage_all(&self, amount: f32) {
        let mut world = self.inner.borrow_mut();
        world.register_system(engine_core::systems::standard::DamageAll { amount });
        world.run_system("DamageAll", None).unwrap();
    }

    fn process_deaths(&self) {
        let mut world = self.inner.borrow_mut();
        world.register_system(engine_core::systems::standard::ProcessDeaths);
        world.run_system("ProcessDeaths", None).unwrap();
    }

    fn process_decay(&self) {
        let mut world = self.inner.borrow_mut();
        world.register_system(engine_core::systems::standard::ProcessDecay);
        world.run_system("ProcessDecay", None).unwrap();
    }

    fn count_entities_with_type(&self, type_str: String) -> usize {
        let world = self.inner.borrow_mut();
        world.count_entities_with_type(&type_str)
    }

    fn modify_stockpile_resource(&self, entity_id: u32, kind: String, delta: f64) -> PyResult<()> {
        let mut world = self.inner.borrow_mut();
        world
            .modify_stockpile_resource(entity_id, &kind, delta)
            .map_err(pyo3::exceptions::PyValueError::new_err)
    }

    fn save_to_file(&self, path: String) -> PyResult<()> {
        let world = self.inner.borrow_mut();
        world
            .save_to_file(std::path::Path::new(&path))
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    fn load_from_file(&mut self, path: String) -> PyResult<()> {
        let registry = {
            let world = self.inner.borrow_mut();
            world.registry.clone()
        };
        let loaded =
            engine_core::ecs::world::World::load_from_file(std::path::Path::new(&path), registry)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        let mut world = self.inner.borrow_mut();
        *world = loaded;
        Ok(())
    }
}
