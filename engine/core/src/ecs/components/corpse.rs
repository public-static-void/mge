use engine_macros::component;

/// Corpse component for dead entities.
///
/// Marks an entity as a corpse after death; can be used for decay, looting, etc.
#[component(modes(Colony, Roguelike), schema, version("1.0.0"))]
pub struct Corpse {
    // Optionally, add fields like cause_of_death, time_of_death, etc.
}
