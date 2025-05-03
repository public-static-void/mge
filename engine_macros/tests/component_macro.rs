// engine_macros/tests/component_macro.rs

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
        #[derive(Debug, PartialEq)]
        pub enum MigrationError {
            UnsupportedVersion(Version),
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

    assert_eq!(
        result,
        Err(ecs::error::MigrationError::UnsupportedVersion(
            dummy_version
        ))
    );
}
