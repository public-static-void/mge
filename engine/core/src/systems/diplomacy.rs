use crate::diplomacy::expire_due_treaties;
use crate::ecs::system::System;
use crate::ecs::world::World;

/// System: expires due diplomacy treaties once per tick.
///
/// Each run moves every `Active` treaty whose `expires_tick` has reached the
/// current turn to `Expired`, emitting one `treaty_expired` event per treaty.
/// Deterministic collect-then-apply via [`expire_due_treaties`]; no
/// wall-clock, no RNG.
pub struct DiplomacySystem;

impl System for DiplomacySystem {
    fn name(&self) -> &'static str {
        "DiplomacySystem"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["FactionReputationSystem"]
    }

    fn run(&mut self, world: &mut World) {
        let _ = expire_due_treaties(world);
    }
}
