//! ECS component definitions.
//!
//! This module re-exports all built-in components.

mod health;
mod position;

pub use self::health::Health;
pub use self::position::Position;
