//! Job system module root.
//!
//! This module organizes all job-related logic, handlers, operations, and registries.
//! Submodules are grouped by concern for clarity and extensibility.

/// Job AI
pub mod ai;
/// Job board
pub mod board;
/// Job core
pub mod core;
/// Job operations
pub mod ops;
/// Job registry
pub mod registry;
/// Job reservation
pub mod reservation;
/// Job states
pub mod states;
/// Job system
pub mod system;
/// Job types
pub mod types;

// Re-export the most commonly used public APIs for external use.
pub use ai::*;
pub use board::*;
pub use core::*;
pub use ops::*;
pub use registry::*;
pub use reservation::*;
pub use states::*;
pub use system::JobSystem;
pub use types::*;
