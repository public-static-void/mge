//! Job system module root.
//!
//! This module organizes all job-related logic, handlers, operations, and registries.
//! Submodules are grouped by concern for clarity and extensibility.

pub mod ai;
pub mod ai_event_reaction_system;
pub mod board;
pub mod builtin_handlers;
pub mod children;
pub mod dependencies;
mod job_type;
pub mod loader;
pub mod ops;
pub mod registry;
pub mod requirements;
pub mod resource_reservation;
pub mod state_utils;
pub mod states;
pub mod system;

// Re-export the most commonly used public APIs for external use.
pub use ai::*;
pub use ai_event_reaction_system::AiEventReactionSystem;
pub use board::*;
pub use builtin_handlers::*;
pub use children::*;
pub use dependencies::*;
pub use job_type::{JobEffect, JobLogicKind, JobTypeData, JobTypeRegistry};
pub use loader::*;
pub use ops::*;
pub use registry::*;
pub use requirements::*;
pub use resource_reservation::*;
pub use state_utils::*;
pub use states::*;
pub use system::JobSystem;
