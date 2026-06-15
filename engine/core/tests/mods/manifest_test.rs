use engine_core::mods::manifest::{ModManifest, ModSystem};

#[test]
fn test_serde_roundtrip_all_fields() {
    let manifest = ModManifest {
        name: "test_mod".into(),
        version: "1.0.0".into(),
        description: "A test mod".into(),
        main_script: Some("main.lua".into()),
        dependencies: vec!["core".into()],
        schemas: vec!["schemas/foo.json".into()],
        systems: vec![ModSystem {
            file: "systems/bar.lua".into(),
            name: "BarSystem".into(),
            dependencies: vec!["baz".into()],
        }],
        scripts: vec!["util.lua".into()],
    };
    let json = serde_json::to_string(&manifest).unwrap();
    let deserialized: ModManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(manifest.name, deserialized.name);
    assert_eq!(manifest.version, deserialized.version);
    assert_eq!(manifest.description, deserialized.description);
    assert_eq!(manifest.main_script, deserialized.main_script);
    assert_eq!(manifest.dependencies, deserialized.dependencies);
    assert_eq!(manifest.schemas, deserialized.schemas);
    assert_eq!(manifest.systems.len(), deserialized.systems.len());
    assert_eq!(manifest.systems[0].file, deserialized.systems[0].file);
    assert_eq!(manifest.systems[0].name, deserialized.systems[0].name);
    assert_eq!(
        manifest.systems[0].dependencies,
        deserialized.systems[0].dependencies
    );
    assert_eq!(manifest.scripts, deserialized.scripts);
}

#[test]
fn test_missing_optionals_get_defaults() {
    let json = r#"{"name": "minimal", "version": "0.1.0"}"#;
    let manifest: ModManifest = serde_json::from_str(json).unwrap();
    assert_eq!(manifest.name, "minimal");
    assert_eq!(manifest.version, "0.1.0");
    assert_eq!(manifest.description, "");
    assert!(manifest.main_script.is_none());
    assert!(manifest.dependencies.is_empty());
    assert!(manifest.schemas.is_empty());
    assert!(manifest.systems.is_empty());
    assert!(manifest.scripts.is_empty());
}

#[test]
fn test_missing_name_errors() {
    let json = r#"{"version": "1.0.0"}"#;
    let result: Result<ModManifest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn test_missing_version_errors() {
    let json = r#"{"name": "test"}"#;
    let result: Result<ModManifest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn test_mod_system_serde_roundtrip() {
    let system = ModSystem {
        file: "systems/test.lua".into(),
        name: "TestSystem".into(),
        dependencies: vec!["dep_a".into(), "dep_b".into()],
    };
    let json = serde_json::to_string(&system).unwrap();
    let deserialized: ModSystem = serde_json::from_str(&json).unwrap();
    assert_eq!(system.file, deserialized.file);
    assert_eq!(system.name, deserialized.name);
    assert_eq!(system.dependencies, deserialized.dependencies);
}

#[test]
fn test_mod_system_missing_deps_defaults() {
    let json = r#"{"file": "systems/test.lua", "name": "TestSystem"}"#;
    let system: ModSystem = serde_json::from_str(json).unwrap();
    assert_eq!(system.file, "systems/test.lua");
    assert_eq!(system.name, "TestSystem");
    assert!(system.dependencies.is_empty());
}

#[test]
fn test_invalid_json_parse_error() {
    let json = r#"this is not json"#;
    let result: Result<ModManifest, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn test_empty_schemas_list_valid() {
    let json = r#"{
        "name": "no_schemas",
        "version": "1.0.0",
        "schemas": []
    }"#;
    let manifest: ModManifest = serde_json::from_str(json).unwrap();
    assert!(manifest.schemas.is_empty());
}

#[test]
fn test_empty_systems_list_valid() {
    let json = r#"{
        "name": "no_systems",
        "version": "1.0.0",
        "systems": []
    }"#;
    let manifest: ModManifest = serde_json::from_str(json).unwrap();
    assert!(manifest.systems.is_empty());
}
