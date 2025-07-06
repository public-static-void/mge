use super::PyWorld;
use engine_core::World;
use pyo3::Python;
use std::rc::Rc;

pub trait TurnApi {
    fn tick(&self);
    fn get_turn(&self) -> u32;
}

impl TurnApi for PyWorld {
    fn tick(&self) {
        World::tick(Rc::clone(&self.inner));
        // Deliver job event bus callbacks after tick
        Python::with_gil(|py| {
            let mut world = self.inner.borrow_mut();
            crate::python_api::job_events::deliver_job_event_bus_callbacks(py, &mut world).unwrap();
        });
    }

    fn get_turn(&self) -> u32 {
        let world = self.inner.borrow_mut();
        world.turn
    }
}
