use engine_macros::component;

/// Happiness component
#[component(modes(Colony, Roguelike), schema, version("1.0.0"))]
pub struct Happiness {
    /// Base happiness value (0.0 - 1.0)
    pub base_value: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Component;

    #[test]
    fn test_serde_roundtrip() {
        let original = Happiness { base_value: 0.75 };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Happiness = serde_json::from_str(&json).unwrap();
        assert!((original.base_value - deserialized.base_value).abs() < f32::EPSILON);
    }

    #[test]
    fn test_version() {
        assert_eq!(
            Happiness::version(),
            semver::Version::parse("1.0.0").unwrap()
        );
    }
}
