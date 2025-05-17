//! Core engine library for the Modular Game Engine.
//!
//! Exposes ECS and mode management modules.

pub mod ecs;
pub mod map;
pub mod modes;
pub mod mods;
pub mod plugins;
pub mod scripting;
pub mod systems;
pub mod worldgen;

pub use ecs::components::{Happiness, Health, Inventory, Position};
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
