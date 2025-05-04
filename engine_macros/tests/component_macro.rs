// engine_macros/tests/component_macro.rs

use crate::error::MigrationError;
use engine_macros::component;

mod ecs {
    use semver::Version;
    pub trait Component {
        fn version() -> Version;
        fn migrate(_from: Version, _data: &[u8]) -> Result<Self, error::MigrationError>
        where
            Self: Sized,
        {
            unimplemented!()
        }
        fn generate_schema() -> Option<schemars::schema::RootSchema> {
            None
        }
    }
    pub mod error {
        use semver::Version;
        use thiserror::Error;

        #[derive(Debug, Error)]
        pub enum MigrationError {
            #[error("Unsupported version: {0}")]
            UnsupportedVersion(Version),
            #[error("Deserialization failed: {0}")]
            DeserializationError(#[from] bson::de::Error),
        }
    }
}

mod modes {
    #[derive(Debug, PartialEq)]
    pub enum GameMode {
        Single,
        Multi,
    }
    pub trait ModeRestrictedComponent {
        fn supported_modes() -> Vec<GameMode>;
    }
}

use ecs::*;
use modes::*;

#[component(modes(Single, Multi), version = "1.2.3", schema)]
#[derive(Debug, PartialEq)]
struct TestComponent {
    x: i32,
    y: i32,
}

#[test]
fn test_component_macro_basic() {
    assert_eq!(TestComponent::version().to_string(), "1.2.3");
    assert_eq!(
        TestComponent::supported_modes(),
        vec![GameMode::Single, GameMode::Multi]
    );
    assert!(TestComponent::generate_schema().is_some());

    // Test migrate (should return Err for unknown version)
    let dummy_data: &[u8] = &[];
    let dummy_version = semver::Version::parse("0.0.0").unwrap();
    let result = TestComponent::migrate(dummy_version.clone(), dummy_data);

    match result {
        Err(MigrationError::UnsupportedVersion(ver)) => {
            assert_eq!(ver, semver::Version::parse("0.0.0").unwrap());
        }
        _ => panic!("Expected UnsupportedVersion error"),
    }
}

#[test]
fn test_component_serde_roundtrip() {
    let comp = TestComponent { x: 123, y: -456 };
    let json = serde_json::to_string(&comp).unwrap();
    let de: TestComponent = serde_json::from_str(&json).unwrap();
    assert_eq!(comp, de);
}

#[test]
fn test_component_schema_fields() {
    let schema = TestComponent::generate_schema().unwrap();
    let props = &schema.schema.object.as_ref().unwrap().properties;
    assert!(props.contains_key("x"));
    assert!(props.contains_key("y"));
}

#[component(
    modes(Single),
    version = "2.0.0",
    migration(from = "1.0.0", convert = "LegacyComponent")
)]
#[derive(Debug, PartialEq)]
struct MigratedComponent {
    x: i32,
    y: i32,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct LegacyComponent {
    x: i32,
    y: i32,
}

#[test]
fn test_component_migration() {
    let legacy = LegacyComponent { x: 1, y: 2 };
    let legacy_bytes = bson::to_vec(&legacy).unwrap();
    let migrated =
        MigratedComponent::migrate(semver::Version::parse("1.0.0").unwrap(), &legacy_bytes)
            .unwrap();
    assert_eq!(migrated, MigratedComponent { x: 1, y: 2 });
}

#[component(modes(Single), version = "1.0.0", schema)]
#[derive(Debug, PartialEq)]
struct EmptyComponent {}

#[test]
fn test_empty_component_serde() {
    let comp = EmptyComponent {};
    let json = serde_json::to_string(&comp).unwrap();
    let de: EmptyComponent = serde_json::from_str(&json).unwrap();
    assert_eq!(comp, de);
}

#[component(modes(Single), version = "1.0.0", schema)]
#[derive(Debug, PartialEq)]
struct OptionComponent {
    value: Option<i32>,
}

#[test]
fn test_option_component_serde() {
    let comp = OptionComponent { value: Some(42) };
    let json = serde_json::to_string(&comp).unwrap();
    let de: OptionComponent = serde_json::from_str(&json).unwrap();
    assert_eq!(comp, de);

    let comp_none = OptionComponent { value: None };
    let json = serde_json::to_string(&comp_none).unwrap();
    let de: OptionComponent = serde_json::from_str(&json).unwrap();
    assert_eq!(comp_none, de);
}

#[component(modes(Single), version = "1.0.0", schema)]
#[derive(Debug, PartialEq)]
struct Inner {
    z: i32,
}

#[component(modes(Single), version = "1.0.0", schema)]
#[derive(Debug, PartialEq)]
struct Outer {
    inner: Inner,
}

#[test]
fn test_nested_component_serde() {
    let comp = Outer {
        inner: Inner { z: 7 },
    };
    let json = serde_json::to_string(&comp).unwrap();
    let de: Outer = serde_json::from_str(&json).unwrap();
    assert_eq!(comp, de);
}

#[component(modes(Single), version = "1.0.0")]
#[derive(Debug, PartialEq)]
struct SingleModeComponent {
    a: i32,
}

#[test]
fn test_single_mode_component_modes() {
    assert_eq!(
        SingleModeComponent::supported_modes(),
        vec![GameMode::Single]
    );
}
