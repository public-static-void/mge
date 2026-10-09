//! Global test hooks plus asset-path resolver (M8).
//!
//! The resolver centralizes the hard-coded schema lookup lists behind one
//! pure helper honoring `MGE_SCHEMA_DIR`-style overrides; the reset hooks
//! give each mutable process-global a per-test isolation path.

use engine_core::asset_paths::resolve_asset_paths;
use std::path::PathBuf;

fn relocated_fixture(dir: &tempfile::TempDir, name: &str, contents: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn override_dir_comes_before_compiled_defaults() {
    let resolved = resolve_asset_paths(
        "skill_registry.json",
        Some(PathBuf::from("/tmp/schemas")),
        &[
            "engine/assets/schemas/skill_registry.json",
            "../engine/assets/schemas/skill_registry.json",
        ],
    );

    assert_eq!(
        resolved,
        vec![
            PathBuf::from("/tmp/schemas/skill_registry.json"),
            PathBuf::from("engine/assets/schemas/skill_registry.json"),
            PathBuf::from("../engine/assets/schemas/skill_registry.json"),
        ]
    );
}

#[test]
fn defaults_preserved_without_override() {
    let resolved = resolve_asset_paths("tech_tree.json", None, &["a.json", "b.json"]);

    assert_eq!(
        resolved,
        vec![PathBuf::from("a.json"), PathBuf::from("b.json")]
    );
}

#[test]
fn skill_registry_loads_from_relocated_directory() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = relocated_fixture(
        &dir,
        "skill_registry.json",
        r#"{"skills": [{"name": "probe_skill", "max_level": 5.0, "base_xp_per_action": 7.0}]}"#,
    );

    let registry =
        engine_core::systems::job::system::process::load_skill_registry_from_paths(&[fixture]);

    assert_eq!(
        registry.get("probe_skill").map(|e| e.base_xp_per_action),
        Some(7.0)
    );
    assert_eq!(registry.get("probe_skill").map(|e| e.max_level), Some(5.0));
}

#[test]
fn skill_registry_loader_ignores_missing_paths() {
    let registry = engine_core::systems::job::system::process::load_skill_registry_from_paths(&[
        PathBuf::from("/nonexistent/m8_probe_skill_registry.json"),
    ]);

    assert!(registry.is_empty());
}

#[test]
fn tech_tree_loads_from_relocated_directory() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = relocated_fixture(
        &dir,
        "tech_tree.json",
        r#"{"techs": [{"id": "probe_tech", "name": "Probe", "cost": 0.0}]}"#,
    );

    let nodes = engine_core::tech_tree::load_tech_tree_from_paths(std::slice::from_ref(&fixture));

    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].id, "probe_tech");
    assert_eq!(nodes[0].cost, engine_core::tech_tree::MIN_TECH_COST);
}

#[test]
fn equipment_slots_load_from_relocated_directory() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = relocated_fixture(
        &dir,
        "equipment_slots.json",
        r#"{"slots": ["head", "probe_slot"]}"#,
    );

    let slots = engine_core::systems::equipment_logic::load_equipment_slots_from_paths(&[fixture]);

    assert!(slots.contains("probe_slot"));
    assert!(slots.contains("head"));
}

#[test]
fn ai_intent_buffer_reset_clears_pending_intents() {
    use engine_core::systems::job::ai::logic::{AI_EVENT_INTENT_BUFFER, reset_for_tests};

    AI_EVENT_INTENT_BUFFER
        .lock()
        .unwrap()
        .push_back(serde_json::json!({"probe": true}));

    reset_for_tests();

    assert!(AI_EVENT_INTENT_BUFFER.lock().unwrap().is_empty());
}

#[test]
fn worldgen_global_reset_restores_builtin_algorithms() {
    use engine_core::worldgen::{GLOBAL_WORLDGEN_REGISTRY, reset_for_tests};

    reset_for_tests();

    let names = GLOBAL_WORLDGEN_REGISTRY.lock().unwrap().names();
    assert!(names.contains(&"dungeon".to_string()));
    assert!(names.contains(&"caves".to_string()));
}

#[test]
fn ui_factory_reset_drops_probe_widgets() {
    use engine_core::presentation::ui::UiEvent;
    use engine_core::presentation::ui::factory::{UI_FACTORY, reset_for_tests};
    use engine_core::presentation::ui::widget::{UiWidget, WidgetId};

    struct ProbeWidget;
    impl UiWidget for ProbeWidget {
        fn id(&self) -> WidgetId {
            1
        }
        fn render(
            &mut self,
            _renderer: &mut dyn engine_core::presentation::renderer::PresentationRenderer,
        ) {
        }
        fn handle_event(&mut self, _event: &UiEvent) {}
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
        fn widget_type(&self) -> &'static str {
            "m8_probe"
        }
        fn boxed_clone(&self) -> Box<dyn UiWidget + Send> {
            Box::new(ProbeWidget)
        }
    }

    UI_FACTORY
        .lock()
        .borrow_mut()
        .register_widget("m8_probe", Box::new(|_| Box::new(ProbeWidget)));
    assert!(UI_FACTORY.lock().borrow().has_widget_type("m8_probe"));

    reset_for_tests();

    assert!(!UI_FACTORY.lock().borrow().has_widget_type("m8_probe"));
}
