// engine/core/src/systems/job/board/mod.rs
//! Job board submodule for job system.

pub mod job_board;
pub mod priority_aging;

pub use job_board::*;
pub use priority_aging::*;
