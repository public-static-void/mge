use engine_macros::component;

/// Inventory component for roguelike mode.
/// Tracks inventory slots and total weight.
#[component(modes(Roguelike), schema, version("1.0.0"))]
pub struct RoguelikeInventory {
    /// Number of slots
    pub slots: u32,
    /// Total weight carried
    pub weight: f32,
}
