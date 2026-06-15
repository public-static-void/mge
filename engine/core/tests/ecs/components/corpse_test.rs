use engine_core::ecs::Component;
use engine_core::ecs::components::Corpse;

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
