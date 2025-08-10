use engine_macros::component;

/// Inventory
#[component(modes(Roguelike, Colony), schema, version("1.0.0"))]
pub struct Inventory {
    /// Number of slots
    pub slots: u32,
    /// Total weight carried
    pub weight: f32,
}
