use engine_macros::component;

#[component(
    modes(Colony, Roguelike, Editor),
    schema,
    version("2.1.0"),
    migration(from = "1.0.0", convert = "LegacyPosition")
)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}
