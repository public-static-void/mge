use engine_macros::component;

/// Happiness component for colony simulation mode.
/// Tracks the base happiness value of a colony.
#[component(modes(Colony), schema, version("1.0.0"))]
pub struct ColonyHappiness {
    /// Base happiness value (0.0 - 1.0)
    pub base_value: f32,
}
