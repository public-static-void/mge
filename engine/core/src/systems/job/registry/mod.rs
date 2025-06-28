//! Job registry submodule for job system.

pub mod effect_processor_registry;
pub mod job_handler_registry;

// Re-export core job type logic/data/registry
pub use crate::systems::job::types::{JobLogicKind, JobTypeData, JobTypeRegistry};

pub use effect_processor_registry::*;
pub use job_handler_registry::*;
