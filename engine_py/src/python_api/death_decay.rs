use super::PyWorld;

pub trait DeathDecayApi {
    fn process_deaths(&self);
    fn process_decay(&self);
}

impl DeathDecayApi for PyWorld {
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
}
