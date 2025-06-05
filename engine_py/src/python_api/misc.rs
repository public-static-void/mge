use super::PyWorld;

pub trait MiscApi {
    fn process_deaths(&self);
    fn process_decay(&self);
    fn count_entities_with_type(&self, type_str: String) -> usize;
}

impl MiscApi for PyWorld {
    fn process_deaths(&self) {
        let mut world = self.inner.borrow_mut();
        world.register_system(engine_core::systems::death_decay::ProcessDeaths);
        world.run_system("ProcessDeaths", None).unwrap();
    }

    fn process_decay(&self) {
        let mut world = self.inner.borrow_mut();
        world.register_system(engine_core::systems::death_decay::ProcessDecay);
        world.run_system("ProcessDecay", None).unwrap();
    }

    fn count_entities_with_type(&self, type_str: String) -> usize {
        let world = self.inner.borrow_mut();
        world.count_entities_with_type(&type_str)
    }
}
