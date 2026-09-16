//! Zone mutators, membership, query union, gates, and error semantics.
//!
//! Covers the M3 core surface: rename/rekind, idempotent assign/unassign,
//! nested Region-reference resolution with cycle guard through zone ids,
//! entity membership via the region id path, colony gates on all mutators,
//! and uniform error semantics.

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::world::ZoneShape;
use serde_json::{Value, json};

fn square(x: i64, y: i64) -> Value {
    json!({"Square": {"x": x, "y": y, "z": 0}})
}

fn region_ref(id: &str) -> Value {
    json!({"Region": {"id": id}})
}

fn add_region(world: &mut engine_core::ecs::world::World, id: &str, kind: &str) {
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Region", json!({"id": id, "kind": kind}))
        .unwrap();
}

// Rename and rekind are reflected in listings; empty kinds and unknown ids
// are rejected without side effects.
#[test]
fn test_rename_and_rekind_update_listing_and_reject_bad_input() {
    let mut world = make_test_world();
    let zone_id = world
        .designate_zone(
            "farm",
            Some("old field"),
            ZoneShape::Cells(vec![square(0, 0)]),
        )
        .unwrap();

    assert!(world.rename_zone(&zone_id, "new field").unwrap());
    assert!(world.set_zone_kind(&zone_id, "pasture").unwrap());

    let listed = world.list_zones();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["label"], json!("new field"));
    assert_eq!(listed[0]["kind"], json!("pasture"));
    let zone = world.get_zone(&zone_id).unwrap();
    assert_eq!(zone["label"], json!("new field"));
    assert_eq!(zone["kind"], json!("pasture"));

    // Empty kind is rejected naming the field; the stored kind is unchanged.
    let err = world.set_zone_kind(&zone_id, "").unwrap_err();
    assert!(err.contains("'kind'"), "{err}");
    assert_eq!(world.get_zone(&zone_id).unwrap()["kind"], json!("pasture"));

    // Unknown ids return false and create nothing.
    let before = world.entities.len();
    assert!(!world.rename_zone("zone-does-not-exist", "x").unwrap());
    assert!(!world.set_zone_kind("zone-does-not-exist", "farm").unwrap());
    assert_eq!(world.entities.len(), before);
    assert_eq!(world.list_zones().len(), 1);
}

// Duplicate assigns and unassigns of unmembered cells are idempotent;
// unknown ids return false without creating entities.
#[test]
fn test_assign_idempotent_and_unassign_ignores_unmembered() {
    let mut world = make_test_world();
    let zone_id = world
        .designate_zone("room", None, ZoneShape::Cells(vec![square(1, 1)]))
        .unwrap();

    assert!(
        world
            .assign_cells_to_zone(&zone_id, vec![square(1, 1), square(2, 2)])
            .unwrap()
    );
    assert_eq!(world.cells_in_region(&zone_id).len(), 2);

    // Assigning the same cells again changes nothing.
    assert!(
        world
            .assign_cells_to_zone(&zone_id, vec![square(1, 1), square(2, 2)])
            .unwrap()
    );
    let cells = world.cells_in_region(&zone_id);
    assert_eq!(cells.len(), 2);
    assert!(cells.contains(&square(1, 1)));
    assert!(cells.contains(&square(2, 2)));

    // Unassigning a cell that was never a member is a no-op success.
    assert!(
        world
            .unassign_cells_from_zone(&zone_id, vec![square(9, 9)])
            .unwrap()
    );
    assert_eq!(world.cells_in_region(&zone_id).len(), 2);

    // Unassigning a member drops exactly that cell; rect-free zone keeps rest.
    assert!(
        world
            .unassign_cells_from_zone(&zone_id, vec![square(1, 1)])
            .unwrap()
    );
    assert_eq!(world.cells_in_region(&zone_id), vec![square(2, 2)]);

    // Unknown ids return false and create no entities.
    let before = world.entities.len();
    assert!(
        !world
            .assign_cells_to_zone("zone-does-not-exist", vec![square(0, 0)])
            .unwrap()
    );
    assert!(
        !world
            .unassign_cells_from_zone("zone-does-not-exist", vec![square(0, 0)])
            .unwrap()
    );
    assert_eq!(world.entities.len(), before);
}

// Zone ids resolve nested Region references recursively, terminate on
// reference cycles, and never leak opaque Region JSON; entities carrying the
// zone id resolve through the region id path, including kind queries.
#[test]
fn test_zone_queries_resolve_nested_refs_cycles_and_members() {
    let mut world = make_test_world();
    add_region(&mut world, "inner", "room");
    let assign = world.spawn_entity();
    world
        .set_component(
            assign,
            "RegionAssignment",
            json!({"cell": square(3, 3), "region_id": "inner"}),
        )
        .unwrap();

    let zone_id = world
        .designate_zone(
            "pasture",
            None,
            ZoneShape::Cells(vec![square(0, 0), region_ref("inner")]),
        )
        .unwrap();

    let cells = world.cells_in_region(&zone_id);
    assert!(cells.contains(&square(0, 0)));
    assert!(cells.contains(&square(3, 3)));
    assert!(
        cells.iter().all(|c| c.get("Region").is_none()),
        "zone query must never return opaque Region JSON"
    );

    // Self-referential zone terminates with no concrete cells.
    let loopy = world
        .designate_zone("room", None, ZoneShape::Cells(vec![]))
        .unwrap();
    assert!(
        world
            .assign_cells_to_zone(&loopy, vec![region_ref(&loopy)])
            .unwrap()
    );
    let resolved = world.cells_in_region(&loopy);
    assert!(resolved.is_empty());
    assert!(resolved.iter().all(|c| c.get("Region").is_none()));

    // Entities carrying the zone id are zone members by id and by kind join.
    let member = world.spawn_entity();
    world
        .set_component(member, "Region", json!({"id": zone_id, "kind": "room"}))
        .unwrap();
    let outsider = world.spawn_entity();
    world
        .set_component(outsider, "Region", json!({"id": "inner", "kind": "room"}))
        .unwrap();
    assert!(world.entities_in_region(&zone_id).contains(&member));
    assert!(!world.entities_in_region(&zone_id).contains(&outsider));
    // The member's own kind is "room", so matching "pasture" proves the join
    // through the zone record rather than the entity's own kind.
    assert!(world.entities_in_region_kind("pasture").contains(&member));
    assert!(!world.entities_in_region_kind("pasture").contains(&outsider));

    // Rekind moves kind-query membership with the zone record.
    world.set_zone_kind(&zone_id, "meadow").unwrap();
    assert!(world.entities_in_region_kind("pasture").is_empty());
    assert!(world.entities_in_region_kind("meadow").contains(&member));
}

// All mutating zone ops fail outside colony mode and succeed inside it;
// reads stay available in every mode.
#[test]
fn test_all_mutators_require_colony_mode() {
    let mut world = make_test_world();
    let zone_id = world
        .designate_zone("farm", None, ZoneShape::Cells(vec![square(0, 0)]))
        .unwrap();

    world.set_mode("roguelike");
    for err in [
        world.rename_zone(&zone_id, "x").unwrap_err(),
        world.set_zone_kind(&zone_id, "room").unwrap_err(),
        world
            .assign_cells_to_zone(&zone_id, vec![square(1, 1)])
            .unwrap_err(),
        world
            .unassign_cells_from_zone(&zone_id, vec![square(0, 0)])
            .unwrap_err(),
    ] {
        assert!(err.contains("requires 'colony' mode"), "{err}");
    }
    // Reads are ungated: listing, detail, and cell expansion still work.
    assert_eq!(world.list_zones().len(), 1);
    assert!(world.get_zone(&zone_id).is_some());
    assert_eq!(world.cells_in_region(&zone_id), vec![square(0, 0)]);

    world.set_mode("colony");
    assert!(world.rename_zone(&zone_id, "x").unwrap());
    assert!(world.set_zone_kind(&zone_id, "room").unwrap());
    assert!(
        world
            .assign_cells_to_zone(&zone_id, vec![square(1, 1)])
            .unwrap()
    );
    assert!(
        world
            .unassign_cells_from_zone(&zone_id, vec![square(0, 0)])
            .unwrap()
    );
}

// Unknown ids yield false/None, malformed rects name the offending field,
// invalid cells name the index — and no error path panics or creates
// entities.
#[test]
fn test_error_paths_have_no_side_effects() {
    let mut world = make_test_world();

    assert!(world.get_zone("zone-does-not-exist").is_none());
    assert!(world.cells_in_region("zone-does-not-exist").is_empty());
    assert!(world.entities_in_region("zone-does-not-exist").is_empty());

    let before = world.entities.len();
    assert!(!world.rename_zone("zone-does-not-exist", "x").unwrap());
    assert!(!world.set_zone_kind("zone-does-not-exist", "x").unwrap());
    assert!(
        !world
            .assign_cells_to_zone("zone-does-not-exist", vec![square(0, 0)])
            .unwrap()
    );
    assert!(
        !world
            .unassign_cells_from_zone("zone-does-not-exist", vec![square(0, 0)])
            .unwrap()
    );
    assert!(!world.remove_zone("zone-does-not-exist").unwrap());
    assert_eq!(world.entities.len(), before);

    // Malformed rects name the offending field and create no zone.
    for (shape, field_a, field_b) in [
        (
            ZoneShape::Rect {
                x0: 3,
                y0: 0,
                z: 0,
                x1: 1,
                y1: 1,
            },
            "'x0'",
            "'x1'",
        ),
        (
            ZoneShape::Rect {
                x0: 0,
                y0: 4,
                z: 0,
                x1: 1,
                y1: 2,
            },
            "'y0'",
            "'y1'",
        ),
    ] {
        let err = world.designate_zone("farm", None, shape).unwrap_err();
        assert!(err.contains(field_a), "{err}");
        assert!(err.contains(field_b), "{err}");
    }
    assert!(world.list_zones().is_empty());

    // Invalid assign cells name the index and store nothing.
    let zone_id = world
        .designate_zone("room", None, ZoneShape::Cells(vec![square(0, 0)]))
        .unwrap();
    let err = world
        .assign_cells_to_zone(&zone_id, vec![square(1, 1), json!({"nope": 1})])
        .unwrap_err();
    assert!(err.contains("cells[1]"), "{err}");
    assert_eq!(world.cells_in_region(&zone_id), vec![square(0, 0)]);
}

// Kind queries union the expanded cells of every zone carrying the kind.
#[test]
fn test_kind_query_unions_cells_across_zones() {
    let mut world = make_test_world();
    let rect_zone = world
        .designate_zone(
            "stockpile",
            None,
            ZoneShape::Rect {
                x0: 0,
                y0: 0,
                z: 0,
                x1: 1,
                y1: 0,
            },
        )
        .unwrap();
    let cells_zone = world
        .designate_zone("stockpile", None, ZoneShape::Cells(vec![square(7, 7)]))
        .unwrap();
    world
        .designate_zone("farm", None, ZoneShape::Cells(vec![square(5, 5)]))
        .unwrap();

    let stockpile = world.cells_in_region_kind("stockpile");
    assert_eq!(stockpile.len(), 3);
    assert!(stockpile.contains(&square(0, 0)));
    assert!(stockpile.contains(&square(1, 0)));
    assert!(stockpile.contains(&square(7, 7)));
    assert!(!stockpile.contains(&square(5, 5)));

    // Removing one zone shrinks the union to the survivor.
    world.remove_zone(&rect_zone).unwrap();
    assert_eq!(world.cells_in_region_kind("stockpile"), vec![square(7, 7)]);
    assert!(world.get_zone(&cells_zone).is_some());
}
