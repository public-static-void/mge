use engine_macros::component;

#[component(modes(Colony), schema, version("1.0.0"))]
pub struct Health {
    pub current: f32,
    pub max: f32,
}
