//! Job system module root.
//!
//! This module organizes all job-related logic, handlers, operations, and registries.
//! Submodules are grouped by concern for clarity and extensibility.

pub mod ai;
pub mod board;
pub mod core;
pub mod ops;
pub mod registry;
pub mod reservation;
pub mod states;
pub mod system;
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
