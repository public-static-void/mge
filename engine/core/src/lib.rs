//! Core engine library for the Modular Game Engine.
//!
//! Exposes ECS and mode management modules.

pub mod ecs;
pub mod modes;
pub mod scripting;

pub use ecs::components::{Happiness, Health, Position, Inventory};
pub use ecs::{EcsWorld, Error};
pub use modes::GameMode as Mode;
pub use scripting::World;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
