use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::narrative::tick_narrative;

/// System: evaluates narrative scenarios once per tick (incident director).
///
/// Each run scans the registered scenarios in `id` order, fires at most one
/// eligible scenario into a pending decision, and forwards its fired event.
/// Collect-then-apply via [`tick_narrative`]: the snapshot is built before
/// any state mutates, so evaluation never runs under a state borrow. Runs
/// between `DiplomacySystem` and `FluidSimulationSystem` in
/// [`SYSTEM_EXECUTION_ORDER`](crate::systems::SYSTEM_EXECUTION_ORDER).
pub struct NarrativeSystem;

impl System for NarrativeSystem {
    fn name(&self) -> &'static str {
        "NarrativeSystem"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }

    fn run(&mut self, world: &mut World) {
        tick_narrative(world);
    }
}
