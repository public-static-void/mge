// engine/core/src/systems/job/board/mod.rs
//! Job board submodule for job system.

/// Job board module.
pub mod job_board;
/// Priority aging module.
pub mod priority_aging;

pub use job_board::*;
pub use priority_aging::*;
