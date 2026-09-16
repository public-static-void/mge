use engine_core::config::GameConfig;
use engine_core::ecs::registry::ComponentRegistry;
use engine_core::ecs::schema::load_schemas_from_dir_with_modes;
use engine_core::ecs::world::World;
use engine_core::ecs::world::ZoneShape;
use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::{Arc, Mutex};

fn make_core_world() -> World {
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../game.toml");
    let config = GameConfig::load_from_file(&config_path)
        .unwrap_or_else(|_| panic!("Failed to load config from {config_path:?}"));
    let schema_dir = "../engine/assets/schemas";
    let schemas = load_schemas_from_dir_with_modes(schema_dir, &config.allowed_modes)
        .unwrap_or_else(|_| panic!("Failed to load schemas from {schema_dir:?}"));
    let mut registry = ComponentRegistry::new();
    for (_name, schema) in schemas {
        registry.register_external_schema(schema);
    }
    World::new(Arc::new(Mutex::new(registry)))
}

fn sorted_cells(cells: &[Value]) -> Vec<String> {
    let mut out: Vec<String> = cells.iter().map(|v| v.to_string()).collect();
    out.sort();
    out
}

fn square(x: i64, y: i64) -> Value {
    json!({"Square": {"x": x, "y": y, "z": 0}})
}

// Core World and the WASM mirror resolve the same nested Region fixture
// with a reference cycle to byte-identical cell sets, including zone
// rect expansion and kind-union routing through Zone records.
#[test]
fn test_zone_cells_byte_identical_core_vs_wasm_mirror() {
    let mut core = make_core_world();
    core.set_mode("colony");
    let mut mirror = WasmWorld::new();
    mirror.set_mode("colony");

    let rect = ZoneShape::Rect {
        x0: 0,
        y0: 0,
        z: 0,
        x1: 1,
        y1: 1,
    };
    let rect_json = r#"{"rect":{"x0":0,"y0":0,"z":0,"x1":1,"y1":1}}"#;
    let core_a = core
        .designate_zone("stockpile", Some("depot"), rect)
        .unwrap();
    let mirror_a = mirror
        .designate_zone("stockpile", Some("depot"), rect_json)
        .unwrap();
    assert_eq!(core_a, mirror_a);

    let core_b = core
        .designate_zone(
            "stockpile",
            None,
            ZoneShape::Cells(vec![square(5, 5), json!({"Region": {"id": core_a}})]),
        )
        .unwrap();
    let mirror_b = mirror
        .designate_zone(
            "stockpile",
            None,
            &format!(
                r#"{{"cells":[{{"Square":{{"x":5,"y":5,"z":0}}}},{{"Region":{{"id":"{mirror_a}"}}}}]}}"#
            ),
        )
        .unwrap();
    assert_eq!(core_b, mirror_b);

    core.assign_cells_to_zone(&core_a, vec![json!({"Region": {"id": core_b}})])
        .unwrap();
    mirror
        .assign_cells_to_zone(
            &mirror_a,
            &format!(r#"[{{"Region":{{"id":"{mirror_b}"}}}}]"#),
        )
        .unwrap();

    assert_eq!(
        sorted_cells(&core.cells_in_region(&core_a)),
        sorted_cells(&mirror.cells_in_region(&mirror_a)),
    );
    assert_eq!(
        sorted_cells(&core.cells_in_region(&core_b)),
        sorted_cells(&mirror.cells_in_region(&mirror_b)),
    );
    assert_eq!(
        sorted_cells(&core.cells_in_region_kind("stockpile")),
        sorted_cells(&mirror.cells_in_region_kind("stockpile")),
    );

    for cells in [
        core.cells_in_region(&core_a),
        mirror.cells_in_region(&mirror_a),
    ] {
        assert_eq!(cells.len(), 5);
        assert!(cells.iter().all(|c| c.get("Region").is_none()));
    }
}
