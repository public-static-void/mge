use engine_core::mods::manifest::ModManifest;

#[test]
fn test_validate_valid_manifest() {
    let manifest = ModManifest {
        name: "valid_mod".into(),
        version: "1.0.0".into(),
        description: "".into(),
        main_script: Some("main.lua".into()),
        dependencies: vec![],
        schemas: vec![],
        systems: vec![],
        scripts: vec![],
    };
    assert!(manifest.validate().is_ok());
}

#[test]
fn test_validate_missing_main_script() {
    let manifest = ModManifest {
        name: "no_main".into(),
        version: "1.0.0".into(),
        description: "".into(),
        main_script: None,
        dependencies: vec![],
        schemas: vec![],
        systems: vec![],
        scripts: vec![],
    };
    let result = manifest.validate();
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("main_script")));
}

#[test]
fn test_validate_with_empty_schemas_systems() {
    let manifest = ModManifest {
        name: "empty_ok".into(),
        version: "1.0.0".into(),
        description: "".into(),
        main_script: Some("run.lua".into()),
        dependencies: vec![],
        schemas: vec![],
        systems: vec![],
        scripts: vec![],
    };
    assert!(manifest.validate().is_ok());
}
