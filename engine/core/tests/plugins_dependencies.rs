#[test]
fn test_plugin_dependency_resolution_cycle() {
    use engine_core::plugins::loader::resolve_plugin_load_order;
    use engine_core::plugins::types::PluginManifest;

    let a = (
        "a.json".to_string(),
        PluginManifest {
            name: "A".to_string(),
            version: "1.0.0".to_string(),
            description: "".to_string(),
            authors: vec![],
            dependencies: vec!["B".to_string()],
            dynamic_library: "liba.so".to_string(),
        },
    );
    let b = (
        "b.json".to_string(),
        PluginManifest {
            name: "B".to_string(),
            version: "1.0.0".to_string(),
            description: "".to_string(),
            authors: vec![],
            dependencies: vec!["A".to_string()],
            dynamic_library: "libb.so".to_string(),
        },
    );
    let err = resolve_plugin_load_order(&[a, b]).unwrap_err();
    assert!(err.contains("Cycle detected"));
}

#[test]
fn test_plugin_dependency_resolution_missing_dep() {
    use engine_core::plugins::loader::resolve_plugin_load_order;
    use engine_core::plugins::types::PluginManifest;

    let a = (
        "a.json".to_string(),
        PluginManifest {
            name: "A".to_string(),
            version: "1.0.0".to_string(),
            description: "".to_string(),
            authors: vec![],
            dependencies: vec!["B".to_string(), "C".to_string()],
            dynamic_library: "liba.so".to_string(),
        },
    );
    let b = (
        "b.json".to_string(),
        PluginManifest {
            name: "B".to_string(),
            version: "1.0.0".to_string(),
            description: "".to_string(),
            authors: vec![],
            dependencies: vec![],
            dynamic_library: "libb.so".to_string(),
        },
    );
    // C is missing
    let err = resolve_plugin_load_order(&[a, b]).unwrap_err();
    assert!(
        err.starts_with("Missing dependencies:"),
        "Expected missing dependency error, got: {err}"
    );
}
