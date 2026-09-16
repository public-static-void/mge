//! ZoneSystem ordering slot and tick validation.
//!
//! Covers the M4 slice: `SYSTEM_EXECUTION_ORDER` places `ZoneSystem`
//! immediately after `ConstructionSystem`, an empty zone set ticks safely,
//! and stale assignments pointing at removed zones are dropped without
//! error while live zones and plain region assignments are untouched.

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::world::ZoneShape;
use engine_core::systems::zone::ZoneSystem;
use engine_core::systems::{SYSTEM_EXECUTION_ORDER, order_systems};
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;

fn square(x: i64, y: i64) -> serde_json::Value {
    json!({"Square": {"x": x, "y": y, "z": 0}})
}

// ZoneSystem sits immediately after ConstructionSystem in the order table,
// ahead of FactionReputationSystem.
#[test]
fn test_zone_system_slot_follows_construction() {
    let order: Vec<&str> = SYSTEM_EXECUTION_ORDER.to_vec();
    let construction = order
        .iter()
        .position(|name| *name == "ConstructionSystem")
        .expect("ConstructionSystem is ordered");
    let zone = order
        .iter()
        .position(|name| *name == "ZoneSystem")
        .expect("ZoneSystem is ordered");
    let faction = order
        .iter()
        .position(|name| *name == "FactionReputationSystem")
        .expect("FactionReputationSystem is ordered");
    assert_eq!(zone, construction + 1);
    assert_eq!(faction, zone + 1);

    let names = vec![
        "FactionReputationSystem".to_string(),
        "ZoneSystem".to_string(),
        "ConstructionSystem".to_string(),
    ];
    assert_eq!(
        order_systems(&names),
        vec![
            "ConstructionSystem".to_string(),
            "ZoneSystem".to_string(),
            "FactionReputationSystem".to_string(),
        ]
    );
}

// An empty zone set ticks cleanly through both the direct run path and the
// deterministic simulation tick.
#[test]
fn test_zone_tick_with_empty_set_passes() {
    let mut world = make_test_world();
    world.register_system(ZoneSystem::new());
    world.run_system("ZoneSystem").expect("empty tick runs");
    assert!(world.list_zones().is_empty());

    let mut world = make_test_world();
    world.register_system(ZoneSystem::new());
    let world_rc = Rc::new(RefCell::new(world));
    engine_core::World::simulation_tick(Rc::clone(&world_rc));
    assert!(world_rc.borrow().list_zones().is_empty());
}

// Dangling references to a removed zone are dropped without error: orphan
// rects despawned, string-form assignments despawned, array-form values
// stripped of just the stale id. Live zones and plain region assignments
// survive the same tick.
#[test]
fn test_zone_tick_drops_stale_assignments_without_error() {
    let mut world = make_test_world();
    let live_id = world
        .designate_zone(
            "farm",
            Some("kept"),
            ZoneShape::Rect {
                x0: 0,
                y0: 0,
                z: 0,
                x1: 1,
                y1: 1,
            },
        )
        .unwrap();
    let gone_id = world
        .designate_zone("room", None, ZoneShape::Cells(vec![square(9, 9)]))
        .unwrap();

    // Plain region assignment that must never be touched by zone cleanup.
    let region_eid = world.spawn_entity();
    world
        .set_component(
            region_eid,
            "RegionAssignment",
            json!({"cell": square(4, 4), "region_id": "meadow"}),
        )
        .unwrap();
    // Array-form assignment mixing the live zone, the soon-stale zone, and
    // a plain region id: only the stale zone id is stripped.
    let mixed_eid = world.spawn_entity();
    world
        .set_component(
            mixed_eid,
            "RegionAssignment",
            json!({"cell": square(5, 5), "region_id": [live_id, gone_id, "meadow"]}),
        )
        .unwrap();

    // Simulate an external removal that bypasses remove_zone cleanup by
    // despawning the Zone record directly, leaving rects and assignments
    // dangling.
    let record = world
        .get_entities_with_component("Zone")
        .into_iter()
        .find(|&eid| {
            world
                .get_component(eid, "Zone")
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_str())
                == Some(gone_id.as_str())
        })
        .expect("gone zone record exists");
    world.despawn_entity(record);

    world.register_system(ZoneSystem::new());
    world.run_system("ZoneSystem").expect("stale tick runs");

    assert!(world.get_zone(&gone_id).is_none());
    assert_eq!(
        world
            .get_entities_with_component("ZoneRect")
            .into_iter()
            .filter(|&eid| {
                world
                    .get_component(eid, "ZoneRect")
                    .and_then(|v| v.get("zone_id"))
                    .and_then(|v| v.as_str())
                    == Some(gone_id.as_str())
            })
            .count(),
        0
    );
    assert!(
        world.cells_in_region(&gone_id).is_empty(),
        "stale zone id resolves to no cells after cleanup"
    );

    let live = world.get_zone(&live_id).expect("live zone survives");
    assert_eq!(live["label"], json!("kept"));
    // 4 rect cells plus the mixed-assignment cell, which still names the
    // live zone after the stale id is stripped.
    assert_eq!(world.cells_in_region(&live_id).len(), 5);

    let mixed = world
        .get_component(mixed_eid, "RegionAssignment")
        .expect("mixed assignment survives with stale id stripped");
    assert_eq!(mixed["region_id"], json!([live_id, "meadow"]));
    assert!(
        world
            .get_component(region_eid, "RegionAssignment")
            .is_some(),
        "plain region assignment untouched"
    );
}
