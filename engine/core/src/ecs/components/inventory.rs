use engine_macros::component;

/// Inventory
#[component(modes(Roguelike, Colony), schema, version("1.0.0"))]
pub struct Inventory {
    /// Number of slots
    pub slots: u32,
    /// Total weight carried
    pub weight: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Component;

    #[test]
    fn test_serde_roundtrip() {
        let original = Inventory {
            slots: 10,
            weight: 5.5,
        };
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Inventory = serde_json::from_str(&json).unwrap();
        assert_eq!(original.slots, deserialized.slots);
        assert!((original.weight - deserialized.weight).abs() < f32::EPSILON);
    }

    #[test]
    fn test_version() {
        assert_eq!(
            Inventory::version(),
            semver::Version::parse("1.0.0").unwrap()
        );
    }
}
