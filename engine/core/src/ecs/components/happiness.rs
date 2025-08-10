use engine_macros::component;

/// Happiness component
#[component(modes(Colony, Roguelike), schema, version("1.0.0"))]
pub struct Happiness {
    /// Base happiness value (0.0 - 1.0)
    pub base_value: f32,
}
