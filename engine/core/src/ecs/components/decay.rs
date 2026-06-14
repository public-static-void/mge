use engine_macros::component;

/// Decay component for entities that will be removed after a certain number of ticks.
///
/// Used for corpses, debris, temporary effects, etc.
#[component(modes(Colony, Roguelike), schema, version("1.0.0"))]
pub struct Decay {
    /// Number of ticks remaining before the entity is removed from the world.
    pub time_remaining: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Component;

    #[test]
    fn test_serde_roundtrip() {
        let original = Decay { time_remaining: 42 };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Decay = serde_json::from_str(&json).unwrap();
        assert_eq!(original.time_remaining, deserialized.time_remaining);
    }

    #[test]
    fn test_version() {
        assert_eq!(Decay::version(), semver::Version::parse("1.0.0").unwrap());
    }
}
