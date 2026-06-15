use engine_core::ecs::Component;
use engine_core::ecs::components::Health;

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
