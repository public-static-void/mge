//! Stockpile first-consumer fixture + zone save/load round-trip (M6).
//!
//! P015 (AC012): a `kind=stockpile` zone exposes its cells through
//! `cells_in_region_kind("stockpile")` as hauling targets.
//! P016 (AC013): all zone state (records, rects, assignments, label/kind
//! edits) round-trips through save/load with zero loss.

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

#[path = "helpers/world_io.rs"]
mod world_io_helper;
use world_io_helper::save_and_load_roundtrip;

use engine_core::ecs::world::ZoneShape;
use serde_json::{Value, json};

fn square(x: i64, y: i64) -> Value {
    json!({"Square": {"x": x, "y": y, "z": 0}})
}

// A `kind=stockpile` zone makes its cells visible via the kind query, so
// hauling logic can treat the returned cells as targets. Non-stockpile
// zones stay out of the set, and every hauling target also resolves
// through the zone's own id path.
#[test]
fn test_stockpile_zone_cells_visible_as_hauling_targets() {
    let mut world = make_test_world();
    let depot = world
        .designate_zone(
            "stockpile",
            Some("depot"),
            ZoneShape::Rect {
                x0: 0,
                y0: 0,
                z: 0,
                x1: 1,
                y1: 0,
            },
        )
        .unwrap();
    world
        .assign_cells_to_zone(&depot, vec![square(7, 7)])
        .unwrap();
    world
        .designate_zone("farm", None, ZoneShape::Cells(vec![square(5, 5)]))
        .unwrap();

    let hauling_targets = world.cells_in_region_kind("stockpile");
    assert_eq!(hauling_targets.len(), 3);
    assert!(hauling_targets.contains(&square(0, 0)));
    assert!(hauling_targets.contains(&square(1, 0)));
    assert!(hauling_targets.contains(&square(7, 7)));
    assert!(!hauling_targets.contains(&square(5, 5)));

    // Every hauling target is also a member of the depot zone by id.
    let members = world.cells_in_region(&depot);
    for target in &hauling_targets {
        assert!(
            members.contains(target),
            "target {target} not in depot zone"
        );
    }
}

// Designations, rects, explicit assignments, and label/kind edits survive
// a save/load round-trip identically, and the id counter continues without
// collision after reload.
#[test]
fn test_zone_save_load_round_trips_all_state() {
    let mut world = make_test_world();
    let rect_zone = world
        .designate_zone(
            "stockpile",
            Some("depot"),
            ZoneShape::Rect {
                x0: 0,
                y0: 0,
                z: 0,
                x1: 2,
                y1: 1,
            },
        )
        .unwrap();
    let cells_zone = world
        .designate_zone("farm", None, ZoneShape::Cells(vec![square(9, 9)]))
        .unwrap();

    world.rename_zone(&rect_zone, "store").unwrap();
    world.set_zone_kind(&cells_zone, "pasture").unwrap();
    world
        .assign_cells_to_zone(&rect_zone, vec![square(7, 7)])
        .unwrap();
    world
        .assign_cells_to_zone(&cells_zone, vec![square(8, 8)])
        .unwrap();

    let before_rect = world.get_zone(&rect_zone).unwrap();
    let before_cells = world.get_zone(&cells_zone).unwrap();
    let before_list = world.list_zones();
    let before_kind_cells = world.cells_in_region_kind("stockpile");
    assert_eq!(before_kind_cells.len(), 7);

    let registry = world.registry.clone();
    let mut loaded = save_and_load_roundtrip(&world, registry);

    // Explicit-cell order is insertion order, which serde round-trip does
    // not preserve; compare zone records with sorted cell arrays so the
    // assertion covers membership (zero loss) rather than ordering.
    fn sorted_cells(zone: &Value) -> Vec<String> {
        let mut cells: Vec<String> = zone["cells"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|c| c.to_string())
            .collect();
        cells.sort();
        cells
    }
    for zone_id in [&rect_zone, &cells_zone] {
        let before = world.get_zone(zone_id).unwrap();
        let after = loaded.get_zone(zone_id).unwrap();
        assert_eq!(after["id"], before["id"]);
        assert_eq!(after["label"], before["label"]);
        assert_eq!(after["kind"], before["kind"]);
        assert_eq!(after["rects"], before["rects"]);
        assert_eq!(sorted_cells(&after), sorted_cells(&before));
    }
    assert_eq!(
        loaded.get_zone(&rect_zone).unwrap()["rects"],
        before_rect["rects"]
    );
    assert_eq!(
        loaded.get_zone(&cells_zone).unwrap()["kind"],
        before_cells["kind"]
    );
    assert_eq!(loaded.list_zones(), before_list);
    assert_eq!(loaded.cells_in_region(&rect_zone).len(), 7);
    let mut after_kind_cells = loaded.cells_in_region_kind("stockpile");
    let mut expected_kind_cells = before_kind_cells;
    after_kind_cells.sort_by_key(|c| c.to_string());
    expected_kind_cells.sort_by_key(|c| c.to_string());
    assert_eq!(after_kind_cells, expected_kind_cells);
    let mut after_pasture = loaded.cells_in_region_kind("pasture");
    let mut expected_pasture = world.cells_in_region_kind("pasture");
    after_pasture.sort_by_key(|c| c.to_string());
    expected_pasture.sort_by_key(|c| c.to_string());
    assert_eq!(after_pasture, expected_pasture);

    // The id counter survived: the next designation is fresh and unique.
    let fresh = loaded
        .designate_zone("room", None, ZoneShape::Cells(vec![square(1, 1)]))
        .unwrap();
    assert_ne!(fresh, rect_zone);
    assert_ne!(fresh, cells_zone);
    assert!(loaded.get_zone(&fresh).is_some());
}
