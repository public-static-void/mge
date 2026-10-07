use engine_core::worldgen::{
    DuplicateName, ScriptingWorldgenPlugin, ThreadSafeScriptingWorldgenPlugin,
    ThreadSafeWorldgenPlugin, ThreadSafeWorldgenRegistry, WorldgenError, WorldgenPlugin,
    WorldgenRegistry,
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn valid_map() -> Value {
    json!({
        "topology": "square",
        "cells": [
            { "x": 0, "y": 0, "z": 0, "neighbors": [] }
        ]
    })
}

fn cabi_plugin(name: &str, map: Value) -> WorldgenPlugin {
    WorldgenPlugin::CAbi {
        name: name.to_string(),
        generate: Arc::new(move |_| map.clone()),
        _lib: None,
    }
}

fn threadsafe_cabi_plugin(name: &str, map: Value) -> ThreadSafeWorldgenPlugin {
    ThreadSafeWorldgenPlugin::CAbi {
        name: name.to_string(),
        generate: Arc::new(move |_| map.clone()),
        _lib: None,
    }
}

#[derive(Clone)]
struct DummyScriptingPlugin;

impl ScriptingWorldgenPlugin for DummyScriptingPlugin {
    fn invoke(&self, _params: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        Ok(valid_map())
    }
    fn backend(&self) -> &str {
        "test"
    }
}

#[derive(Clone)]
struct DummyThreadSafePlugin;

impl ThreadSafeScriptingWorldgenPlugin for DummyThreadSafePlugin {
    fn invoke(&self, _params: &Value) -> Result<Value, Box<dyn std::error::Error>> {
        Ok(valid_map())
    }
    fn backend(&self) -> &str {
        "test"
    }
}

#[test]
fn rejects_duplicate_registration_without_replace() {
    let mut registry = WorldgenRegistry::new();
    registry
        .register(cabi_plugin("dungeon", valid_map()))
        .expect("first registration succeeds");
    let err = registry
        .register(cabi_plugin("dungeon", valid_map()))
        .expect_err("second registration without replace must fail");
    assert_eq!(err, DuplicateName("dungeon".to_string()));
    assert_eq!(registry.names().len(), 1);
}

#[test]
fn rejects_duplicate_registration_on_threadsafe_registry() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    registry
        .register(threadsafe_cabi_plugin("dungeon", valid_map()))
        .expect("first registration succeeds");
    let err = registry
        .register(threadsafe_cabi_plugin("dungeon", valid_map()))
        .expect_err("second registration without replace must fail");
    assert_eq!(err, DuplicateName("dungeon".to_string()));
    assert_eq!(registry.names().len(), 1);
}

#[test]
fn replace_registers_new_entry_under_same_name() {
    let mut registry = WorldgenRegistry::new();
    registry
        .register(cabi_plugin("dungeon", valid_map()))
        .expect("first registration succeeds");
    let other = json!({
        "topology": "square",
        "cells": [
            { "x": 1, "y": 2, "z": 3, "neighbors": [] }
        ]
    });
    registry.register_or_replace(cabi_plugin("dungeon", other));
    assert_eq!(registry.names().len(), 1);
    let map = registry
        .invoke("dungeon", &json!({}))
        .expect("replaced plugin invokes");
    assert_eq!(map["cells"][0]["x"], 1);
}

#[test]
fn replace_registers_new_entry_on_threadsafe_registry() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    registry
        .register(threadsafe_cabi_plugin("dungeon", valid_map()))
        .expect("first registration succeeds");
    let other = json!({
        "topology": "square",
        "cells": [
            { "x": 4, "y": 5, "z": 6, "neighbors": [] }
        ]
    });
    registry.register_or_replace(threadsafe_cabi_plugin("dungeon", other));
    assert_eq!(registry.names().len(), 1);
    let map = registry
        .invoke("dungeon", &json!({}))
        .expect("replaced plugin invokes");
    assert_eq!(map["cells"][0]["x"], 4);
}

#[test]
fn unregister_removes_entry_so_invoke_misses() {
    let mut registry = WorldgenRegistry::new();
    registry
        .register(cabi_plugin("dungeon", valid_map()))
        .expect("registration succeeds");
    assert!(registry.unregister("dungeon"));
    assert!(!registry.names().contains(&"dungeon".to_string()));
    assert!(matches!(
        registry.invoke("dungeon", &json!({})),
        Err(WorldgenError::NotFound { .. })
    ));
}

#[test]
fn unregister_roundtrip_on_threadsafe_registry() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    registry
        .register(threadsafe_cabi_plugin("dungeon", valid_map()))
        .expect("registration succeeds");
    assert!(registry.unregister("dungeon"));
    assert!(!registry.names().contains(&"dungeon".to_string()));
    assert!(matches!(
        registry.invoke("dungeon", &json!({})),
        Err(WorldgenError::NotFound { .. })
    ));
}

#[test]
fn unregister_missing_name_reports_false() {
    let mut local = WorldgenRegistry::new();
    assert!(!local.unregister("absent"));
    let mut shared = ThreadSafeWorldgenRegistry::new();
    assert!(!shared.unregister("absent"));
}

#[test]
fn unknown_name_error_lists_available_names() {
    let mut registry = WorldgenRegistry::new();
    registry
        .register(cabi_plugin("dungeon", valid_map()))
        .expect("registration succeeds");
    registry
        .register(WorldgenPlugin::Scripting {
            name: "caves".to_string(),
            backend: "test".to_string(),
            opaque: Box::new(DummyScriptingPlugin),
        })
        .expect("registration succeeds");
    match registry.invoke("nope", &json!({})) {
        Err(WorldgenError::NotFound { name, available }) => {
            assert_eq!(name, "nope");
            assert!(available.contains(&"dungeon".to_string()));
            assert!(available.contains(&"caves".to_string()));
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn unknown_name_error_lists_available_on_threadsafe_registry() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    registry
        .register(threadsafe_cabi_plugin("dungeon", valid_map()))
        .expect("registration succeeds");
    registry
        .register(ThreadSafeWorldgenPlugin::ThreadSafeScripting {
            name: "caves".to_string(),
            backend: "test".to_string(),
            opaque: Box::new(DummyThreadSafePlugin),
        })
        .expect("registration succeeds");
    match registry.invoke("nope", &json!({})) {
        Err(WorldgenError::NotFound { name, available }) => {
            assert_eq!(name, "nope");
            assert!(available.contains(&"dungeon".to_string()));
            assert!(available.contains(&"caves".to_string()));
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn invoke_runs_postprocess_then_validators_exactly_once() {
    let mut registry = WorldgenRegistry::new();
    registry
        .register(cabi_plugin("dungeon", valid_map()))
        .expect("registration succeeds");
    let post_count = Arc::new(AtomicUsize::new(0));
    let valid_count = Arc::new(AtomicUsize::new(0));
    let validator_saw_marker = Arc::new(AtomicUsize::new(0));
    let post_count_in = Arc::clone(&post_count);
    registry.register_postprocessor(move |map| {
        post_count_in.fetch_add(1, Ordering::SeqCst);
        map.as_object_mut()
            .expect("map is an object")
            .insert("postprocessed".to_string(), json!(true));
    });
    let valid_count_in = Arc::clone(&valid_count);
    let marker_in = Arc::clone(&validator_saw_marker);
    registry.register_validator(move |map| {
        valid_count_in.fetch_add(1, Ordering::SeqCst);
        if map.get("postprocessed") == Some(&json!(true)) {
            marker_in.fetch_add(1, Ordering::SeqCst);
        }
        Ok(())
    });
    let map = registry
        .invoke("dungeon", &json!({}))
        .expect("invoke succeeds");
    assert_eq!(map["postprocessed"], true);
    assert_eq!(post_count.load(Ordering::SeqCst), 1);
    assert_eq!(valid_count.load(Ordering::SeqCst), 1);
    assert_eq!(
        validator_saw_marker.load(Ordering::SeqCst),
        1,
        "validator runs after postprocessing through the shared pipeline"
    );
}

#[test]
fn invoke_runs_pipeline_once_on_threadsafe_registry() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    registry
        .register(threadsafe_cabi_plugin("dungeon", valid_map()))
        .expect("registration succeeds");
    let post_count = Arc::new(AtomicUsize::new(0));
    let valid_count = Arc::new(AtomicUsize::new(0));
    let post_count_in = Arc::clone(&post_count);
    registry.register_postprocessor(move |map| {
        post_count_in.fetch_add(1, Ordering::SeqCst);
        map.as_object_mut()
            .expect("map is an object")
            .insert("postprocessed".to_string(), json!(true));
    });
    let valid_count_in = Arc::clone(&valid_count);
    registry.register_validator(move |_| {
        valid_count_in.fetch_add(1, Ordering::SeqCst);
        Ok(())
    });
    let map = registry
        .invoke("dungeon", &json!({}))
        .expect("invoke succeeds");
    assert_eq!(map["postprocessed"], true);
    assert_eq!(post_count.load(Ordering::SeqCst), 1);
    assert_eq!(valid_count.load(Ordering::SeqCst), 1);
}
