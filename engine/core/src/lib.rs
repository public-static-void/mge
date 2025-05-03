//! Core engine library for the Modular Game Engine.
//!
//! Exposes ECS and mode management modules.

pub mod ecs;
pub mod modes;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
