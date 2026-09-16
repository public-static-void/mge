//! Zone designate/remove/list/get with compact rect persistence.
//!
//! Covers the M2 core surface: unique ids for rect and cell-list zones,
//! removal semantics, list/get shapes, and O(1) rect storage with
//! query-time expansion.

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::world::ZoneShape;
use serde_json::{Value, json};

fn square(x: i64, y: i64) -> Value {
    json!({"Square": {"x": x, "y": y, "z": 0}})
}

fn rect_shape(x0: i64, y0: i64, x1: i64, y1: i64) -> ZoneShape {
    ZoneShape::Rect {
        x0,
        y0,
        z: 0,
        x1,
        y1,
    }
}

// Rect and cell-list designations return unique non-empty ids.
#[test]
fn test_designate_returns_unique_non_empty_ids() {
    let mut world = make_test_world();
    let rect_id = world
        .designate_zone("farm", Some("north field"), rect_shape(0, 0, 2, 2))
        .unwrap();
    let cells_id = world
        .designate_zone(
            "room",
            None,
            ZoneShape::Cells(vec![square(9, 9), square(9, 10)]),
        )
        .unwrap();

    assert!(!rect_id.is_empty());
    assert!(!cells_id.is_empty());
    assert_ne!(rect_id, cells_id);
    assert!(world.get_zone(&rect_id).is_some());
    assert!(world.get_zone(&cells_id).is_some());
}

// Removal deletes the zone and its assignments; unknown ids return false.
#[test]
fn test_remove_zone_drops_record_and_cells_keeps_others() {
    let mut world = make_test_world();
    let doomed = world
        .designate_zone("farm", None, rect_shape(0, 0, 1, 1))
        .unwrap();
    let survivor = world
        .designate_zone("room", None, ZoneShape::Cells(vec![square(7, 7)]))
        .unwrap();

    assert!(world.remove_zone(&doomed).unwrap());
    assert!(world.get_zone(&doomed).is_none());
    assert!(world.cells_in_region(&doomed).is_empty());
    assert!(world.list_zones().iter().all(|z| z["id"] != doomed));

    // Survivor keeps its cell and stays listed.
    assert_eq!(world.cells_in_region(&survivor), vec![square(7, 7)]);
    assert!(world.get_zone(&survivor).is_some());

    // Unknown id: false, no panic, no new zone.
    let before = world.list_zones().len();
    assert!(!world.remove_zone("zone-does-not-exist").unwrap());
    assert_eq!(world.list_zones().len(), before);
}

// list_zones entries carry exactly {id,label,kind,cell_count} with correct
// counts; get_zone on unknown id returns None.
#[test]
fn test_list_and_get_shapes_and_counts() {
    let mut world = make_test_world();
    let rect_id = world
        .designate_zone("farm", Some("field"), rect_shape(0, 0, 2, 1))
        .unwrap();
    let cells_id = world
        .designate_zone("room", None, ZoneShape::Cells(vec![square(5, 5)]))
        .unwrap();

    let listed = world.list_zones();
    assert_eq!(listed.len(), 2);
    for entry in &listed {
        let obj = entry.as_object().unwrap();
        assert_eq!(obj.len(), 4);
        assert!(obj.contains_key("id"));
        assert!(obj.contains_key("label"));
        assert!(obj.contains_key("kind"));
        assert!(obj.contains_key("cell_count"));
    }
    let rect_entry = listed.iter().find(|z| z["id"] == rect_id).unwrap();
    assert_eq!(rect_entry["label"], json!("field"));
    assert_eq!(rect_entry["kind"], json!("farm"));
    assert_eq!(rect_entry["cell_count"], json!(6));
    let cells_entry = listed.iter().find(|z| z["id"] == cells_id).unwrap();
    assert_eq!(cells_entry["cell_count"], json!(1));

    let zone = world.get_zone(&rect_id).unwrap();
    assert_eq!(zone["id"], json!(rect_id));
    assert_eq!(zone["label"], json!("field"));
    assert_eq!(zone["kind"], json!("farm"));
    assert_eq!(
        zone["rects"],
        json!([{"x0": 0, "y0": 0, "z": 0, "x1": 2, "y1": 1}])
    );
    assert_eq!(zone["cells"], json!([]));

    let cell_zone = world.get_zone(&cells_id).unwrap();
    assert_eq!(cell_zone["rects"], json!([]));
    assert_eq!(cell_zone["cells"], json!([square(5, 5)]));

    assert!(world.get_zone("zone-does-not-exist").is_none());
}

// A 50x50 rect stores O(1) records (one Zone + one ZoneRect, zero
// per-cell assignments) while expanding to 2500 cells at query time.
#[test]
fn test_large_rect_stores_constant_records_expands_at_query() {
    let mut world = make_test_world();
    let zone_id = world
        .designate_zone("farm", None, rect_shape(0, 0, 49, 49))
        .unwrap();

    let zone_rects = world
        .get_entities_with_component("ZoneRect")
        .into_iter()
        .filter(|&eid| {
            world
                .get_component(eid, "ZoneRect")
                .and_then(|v| v.get("zone_id"))
                .and_then(|v| v.as_str())
                == Some(zone_id.as_str())
        })
        .count();
    assert_eq!(zone_rects, 1);
    let assignments = world
        .get_entities_with_component("RegionAssignment")
        .into_iter()
        .filter(|&eid| {
            world
                .get_component(eid, "RegionAssignment")
                .and_then(|v| v.get("region_id"))
                .and_then(|v| v.as_str())
                == Some(zone_id.as_str())
        })
        .count();
    assert_eq!(assignments, 0);

    let cells = world.cells_in_region(&zone_id);
    assert_eq!(cells.len(), 2500);
    assert!(cells.contains(&square(0, 0)));
    assert!(cells.contains(&square(49, 49)));
    assert!(cells.contains(&square(25, 25)));

    let listed = world.list_zones();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["cell_count"], json!(2500));
}

// Mutating ops outside colony mode fail with the colony-gate error;
// validation rejects empty kinds and malformed rects naming the field.
#[test]
fn test_colony_gate_and_validation_errors() {
    let mut world = make_test_world();
    world.set_mode("roguelike");
    let err = world
        .designate_zone("farm", None, rect_shape(0, 0, 1, 1))
        .unwrap_err();
    assert!(err.contains("requires 'colony' mode"), "{err}");
    let err = world.remove_zone("zone-1").unwrap_err();
    assert!(err.contains("requires 'colony' mode"), "{err}");
    // Gated reads stay available: empty list, unknown get.
    assert!(world.list_zones().is_empty());
    assert!(world.get_zone("zone-1").is_none());

    world.set_mode("colony");
    let err = world
        .designate_zone("", None, rect_shape(0, 0, 1, 1))
        .unwrap_err();
    assert!(err.contains("'kind'"), "{err}");
    let err = world
        .designate_zone("farm", None, rect_shape(3, 0, 1, 1))
        .unwrap_err();
    assert!(err.contains("'x0'"), "{err}");
    assert!(err.contains("'x1'"), "{err}");
    let err = world
        .designate_zone("farm", None, rect_shape(0, 4, 1, 2))
        .unwrap_err();
    assert!(err.contains("'y0'"), "{err}");
    assert!(err.contains("'y1'"), "{err}");
}

// Invalid explicit cells are rejected, naming the offending index.
#[test]
fn test_designate_rejects_invalid_cells() {
    let mut world = make_test_world();
    let err = world
        .designate_zone(
            "room",
            None,
            ZoneShape::Cells(vec![square(0, 0), json!({"nope": 1})]),
        )
        .unwrap_err();
    assert!(err.contains("cells[1]"), "{err}");
    assert!(world.list_zones().is_empty());
}
