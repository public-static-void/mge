use engine_core::worldgen::{ThreadSafeWorldgenRegistry, register_builtin_mapgen_algorithms};
use serde_json::json;

fn switch_params() -> serde_json::Value {
    json!({"seed": 42, "width": 20, "height": 15})
}

#[test]
fn wasm_registry_path_invokes_both_algorithms_by_name() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);

    let names = registry.names();
    assert!(names.contains(&"dungeon".to_string()));
    assert!(names.contains(&"caves".to_string()));

    let params = switch_params();
    let dungeon_map = registry
        .invoke("dungeon", &params)
        .expect("dungeon invokes");
    let caves_map = registry.invoke("caves", &params).expect("caves invokes");
    assert_eq!(dungeon_map["topology"], "square");
    assert_eq!(caves_map["topology"], "square");
}

#[test]
fn wasm_registry_path_caves_output_is_deterministic() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);

    let params = json!({"seed": 7, "width": 20, "height": 15});
    let first = registry.invoke("caves", &params).expect("generates");
    let second = registry.invoke("caves", &params).expect("generates");
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );

    let other = registry
        .invoke("caves", &json!({"seed": 99, "width": 20, "height": 15}))
        .expect("generates");
    assert_ne!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&other).unwrap()
    );
}
