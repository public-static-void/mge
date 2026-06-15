use engine_core::ecs::Component;
use engine_core::ecs::components::Decay;

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
