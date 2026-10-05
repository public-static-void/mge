//! Autonomous per-entity upkeep consumption.
//!
//! Each tick [`ConsumptionSystem`] drains every entity carrying both `Upkeep`
//! and `Stockpile`, subtracting each listed `drains` entry from the entity's
//! own `Stockpile.resources` with f64 arithmetic under the shared
//! [`crate::trade::TRANSFER_EPSILON`] dust discipline. Drains apply
//! per-entity atomically: any shortfall leaves that entity's stockpile
//! untouched and emits one `consumption_shortage` event per missing kind.
//! Shortage carries no penalty; the entity persists unchanged.

/// Per-tick upkeep drain system.
pub mod system;

pub use system::ConsumptionSystem;
