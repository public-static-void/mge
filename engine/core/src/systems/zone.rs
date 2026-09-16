use crate::World;
use crate::ecs::system::System;

/// Zone state validation system.
///
/// Runs in the explicit `SYSTEM_EXECUTION_ORDER` slot immediately after
/// `ConstructionSystem`. Each tick it drops zone state that points at
/// removed zones: orphan `ZoneRect` records and `RegionAssignment`
/// references to zone ids with no live `Zone` record. Array-form
/// `region_id` values are stripped of the stale id (entity kept while
/// other ids remain); string-form matches are despawned. Assignments that
/// reference plain regions (non-`zone-` ids) are never touched, and an
/// empty zone set is a safe no-op. The tick never errors.
#[derive(Default)]
pub struct ZoneSystem;

impl ZoneSystem {
    /// Creates a new zone system.
    pub fn new() -> Self {
        Self
    }
}

impl System for ZoneSystem {
    fn name(&self) -> &'static str {
        "ZoneSystem"
    }

    fn run(&mut self, world: &mut World) {
        world.validate_zones();
    }
}
