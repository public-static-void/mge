use engine_macros::component;

/// Corpse component for dead entities.
///
/// Marks an entity as a corpse after death; can be used for decay, looting, etc.
#[component(modes(Colony, Roguelike), schema, version("1.0.0"))]
pub struct Corpse {
    // Optionally, add fields like cause_of_death, time_of_death, etc.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Component;

    #[test]
    fn test_serde_roundtrip() {
        let original = Corpse {};
        let json = serde_json::to_string(&original).unwrap();
        // Serialize as struct with 0 fields
        assert_eq!(json, "{}");
        let deserialized: Corpse = serde_json::from_str(&json).unwrap();
        // Verify deserialization succeeded for unit struct
        let _ = deserialized;
    }

    #[test]
    fn test_version() {
        assert_eq!(Corpse::version(), semver::Version::parse("1.0.0").unwrap());
    }
}
