use engine_macros::component;

/// Decay component for entities that will be removed after a certain number of ticks.
///
/// Used for corpses, debris, temporary effects, etc.
#[component(modes(Colony, Roguelike), schema, version("1.0.0"))]
pub struct Decay {
    /// Number of ticks remaining before the entity is removed from the world.
    pub time_remaining: u32,
}
