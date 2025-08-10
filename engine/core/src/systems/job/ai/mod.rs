//! AI system
//!
//! AI systems are responsible for processing events and triggering
//! reactions based on their intent

/// Event intent
pub mod event_intent;
/// Event reaction
pub mod event_reaction_system;
/// AI logic
pub mod logic;

pub use event_intent::*;
pub use event_reaction_system::*;
pub use logic::*;
