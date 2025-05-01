use crate::component;

component! {
    /// Health component for damage tracking
    Health {
        /// Current hit points
        #[schemars(
            description = "Current health value",
            range(min = 0.0)
        )]
        current: f32,

        /// Maximum hit points
        #[schemars(
            description = "Maximum health capacity",
            range(min = 1.0)
        )]
        max: f32
    }
}
