//! Core engine library for the Modular Game Engine.
//!
//! Exposes ECS and mode management modules.

pub mod config;
pub mod ecs;
pub mod map;
pub mod modes;
pub mod mods;
pub mod plugins;
pub mod presentation;
pub mod systems;
pub mod worldgen;

pub use ecs::World;
pub use ecs::components::{Happiness, Health, Inventory, Position};
pub use modes::GameMode as Mode;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
