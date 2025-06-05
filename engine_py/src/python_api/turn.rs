use super::PyWorld;
use engine_core::World;
use std::rc::Rc;

pub trait TurnApi {
    fn tick(&self);
    fn get_turn(&self) -> u32;
}

impl TurnApi for PyWorld {
    fn tick(&self) {
        World::tick(Rc::clone(&self.inner));
    }

    fn get_turn(&self) -> u32 {
        let world = self.inner.borrow_mut();
        world.turn
    }
}
