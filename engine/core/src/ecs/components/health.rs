use engine_macros::component;

/// Health component for entities, used in Colony mode.
///
/// Tracks current and maximum health values.
#[component(modes(Colony, Roguelike), schema, version("1.0.0"))]
pub struct Health {
    /// Current health
    pub current: f32,
    /// Maximum health
    pub max: f32,
}
