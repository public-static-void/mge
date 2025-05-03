use engine_macros::component;

/// Position component for entities, used in Colony, Roguelike, and Editor modes.
///
/// Supports migration from `LegacyPosition` (version 1.0.0).
#[component(
    modes(Colony, Roguelike, Editor),
    schema,
    version("2.1.0"),
    migration(from = "1.0.0", convert = "LegacyPosition")
)]
pub struct Position {
    /// X coordinate
    pub x: f32,
    /// Y coordinate
    pub y: f32,
}
