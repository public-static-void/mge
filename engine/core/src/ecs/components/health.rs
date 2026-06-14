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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Component;

    #[test]
    fn test_serde_roundtrip() {
        let original = Health {
            current: 100.0,
            max: 100.0,
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Health = serde_json::from_str(&json).unwrap();
        assert_eq!(original.current, deserialized.current);
        assert_eq!(original.max, deserialized.max);
    }

    #[test]
    fn test_version() {
        assert_eq!(Health::version(), semver::Version::parse("1.0.0").unwrap());
    }
}
