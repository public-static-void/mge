use engine_core::map::{CellKey, Map, ProvinceMap, SquareGridMap};

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

/// Build a square-topology map with a couple of cells.
fn square_map() -> Map {
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    grid.add_cell(1, 0, 0);
    Map::new(Box::new(grid))
}

/// Build a province-topology (overmap) map with a couple of provinces.
fn province_map() -> Map {
    let mut grid = ProvinceMap::new();
    grid.add_cell("prov_a");
    grid.add_cell("prov_b");
    Map::new(Box::new(grid))
}

// AC001 (R001): two named maps registered, both in get_map_names() and World.maps.
#[test]
fn test_register_two_maps_appear_in_registry() {
    let mut world = make_test_world();

    world.register_map("overmap", province_map()).unwrap();
    world.register_map("local", square_map()).unwrap();

    let names = world.get_map_names();
    assert!(names.contains(&"overmap".to_string()));
    assert!(names.contains(&"local".to_string()));
    assert_eq!(names.len(), 2);

    assert!(world.maps.contains_key("overmap"));
    assert!(world.maps.contains_key("local"));
    assert_eq!(world.maps["overmap"].topology_type(), "province");
    assert_eq!(world.maps["local"].topology_type(), "square");
}

// AC002 (R002): after set_active_map, world.map reflects the selected map's topology.
#[test]
fn test_set_active_map_reflects_topology() {
    let mut world = make_test_world();
    world.register_map("overmap", province_map()).unwrap();
    world.register_map("local", square_map()).unwrap();

    world.set_active_map("local").unwrap();
    assert_eq!(world.map.as_ref().unwrap().topology_type(), "square");

    world.set_active_map("overmap").unwrap();
    assert_eq!(world.map.as_ref().unwrap().topology_type(), "province");
}

// AC003 (R002): set_active_map on an unknown name returns Err and leaves active map unchanged.
#[test]
fn test_set_active_map_unknown_name_errors_and_unchanged() {
    let mut world = make_test_world();
    world.register_map("local", square_map()).unwrap();
    world.set_active_map("local").unwrap();

    let err = world.set_active_map("nonexistent");
    assert!(err.is_err());

    // Active map unchanged: still "local" with square topology.
    assert_eq!(world.get_active_map_name(), "local");
    assert_eq!(world.map.as_ref().unwrap().topology_type(), "square");
}

// AC004 (R003): get_active_map_name returns the last successfully selected map;
// get_map_names contains exactly the registered names.
#[test]
fn test_active_map_name_and_exact_names() {
    let mut world = make_test_world();
    world.register_map("overmap", province_map()).unwrap();
    world.register_map("local", square_map()).unwrap();

    // No map selected yet -> empty active name.
    assert_eq!(world.get_active_map_name(), "");

    world.set_active_map("local").unwrap();
    assert_eq!(world.get_active_map_name(), "local");

    world.set_active_map("overmap").unwrap();
    assert_eq!(world.get_active_map_name(), "overmap");

    let mut names = world.get_map_names();
    names.sort();
    assert_eq!(names, vec!["local".to_string(), "overmap".to_string()]);
}

// AC017 (NFR004): duplicate register_map returns Err (no panic).
#[test]
fn test_register_duplicate_name_errors() {
    let mut world = make_test_world();
    world.register_map("local", square_map()).unwrap();

    let err = world.register_map("local", square_map());
    assert!(err.is_err());

    // Registry still holds the original map.
    assert_eq!(world.maps.len(), 1);
    assert_eq!(world.maps["local"].topology_type(), "square");
}

// --- M2: scale-generic link-based transitions + topology-generic coordinate mapping ---

// A square-topology map can serve as an overmap (Dwarf Fortress/Cataclysm style):
// topology and map type are decoupled.
#[test]
fn test_square_topology_named_overmap() {
    let mut world = make_test_world();
    world.register_map("overmap", square_map()).unwrap();
    assert_eq!(world.maps["overmap"].topology_type(), "square");
}

// A province-topology map can serve as a strategic map (Hearts of Iron style).
#[test]
fn test_province_topology_named_strategic() {
    let mut world = make_test_world();
    world.register_map("strategic", province_map()).unwrap();
    assert_eq!(world.maps["strategic"].topology_type(), "province");
}

// enter_map switches the active map to the target and positions the camera at the entry cell.
#[test]
fn test_enter_map_switches_active_and_positions_camera() {
    let mut world = make_test_world();
    world.register_map("overmap", province_map()).unwrap();
    world.register_map("field", square_map()).unwrap();
    world.set_active_map("overmap").unwrap();

    let source_cell = CellKey::Province {
        id: "prov_a".to_string(),
    };
    let entry_cell = CellKey::Square { x: 1, y: 0, z: 0 };
    world
        .link_maps("overmap", source_cell, "field", entry_cell.clone())
        .unwrap();

    world.enter_map("field", entry_cell.clone()).unwrap();

    assert_eq!(world.get_active_map_name(), "field");
    assert_eq!(world.map.as_ref().unwrap().topology_type(), "square");

    // Camera position equals the entry cell.
    let camera_id = world
        .get_entities_with_component("Camera")
        .first()
        .cloned()
        .unwrap();
    let pos = world.get_component(camera_id, "Position").unwrap();
    assert_eq!(
        pos,
        &serde_json::json!({ "pos": { "Square": { "x": 1, "y": 0, "z": 0 } } })
    );
}

// exit_map returns to the previously active map.
#[test]
fn test_exit_map_returns_to_previous_map() {
    let mut world = make_test_world();
    world.register_map("overmap", province_map()).unwrap();
    world.register_map("field", square_map()).unwrap();
    world.set_active_map("overmap").unwrap();

    world
        .enter_map("field", CellKey::Square { x: 0, y: 0, z: 0 })
        .unwrap();
    assert_eq!(world.get_active_map_name(), "field");

    world.exit_map().unwrap();
    assert_eq!(world.get_active_map_name(), "overmap");
    assert_eq!(world.map.as_ref().unwrap().topology_type(), "province");
}

// enter_map on an unknown map name errors; exit_map with an empty stack errors.
#[test]
fn test_enter_unknown_map_and_exit_empty_stack_error() {
    let mut world = make_test_world();
    world.register_map("field", square_map()).unwrap();

    let err = world.enter_map("nonexistent", CellKey::Square { x: 0, y: 0, z: 0 });
    assert!(err.is_err());

    let err = world.exit_map();
    assert!(err.is_err());
}

// map_cell/unmap_cell round-trip a linked source cell to its target cell and back.
#[test]
fn test_map_cell_unmap_cell_round_trip() {
    let mut world = make_test_world();
    world.register_map("overmap", province_map()).unwrap();
    world.register_map("field", square_map()).unwrap();

    let source_cell = CellKey::Province {
        id: "prov_a".to_string(),
    };
    let target_cell = CellKey::Square { x: 2, y: 1, z: 0 };
    world
        .link_maps("overmap", source_cell.clone(), "field", target_cell.clone())
        .unwrap();

    assert_eq!(
        world.map_cell("overmap", &source_cell),
        Some(target_cell.clone())
    );
    assert_eq!(
        world.unmap_cell("field", &target_cell),
        Some(source_cell.clone())
    );
}

// map_cell/unmap_cell return None for unlinked source/target cells.
#[test]
fn test_map_cell_unmap_cell_unlinked_returns_none() {
    let mut world = make_test_world();
    world.register_map("overmap", province_map()).unwrap();
    world.register_map("field", square_map()).unwrap();

    let source_cell = CellKey::Province {
        id: "prov_a".to_string(),
    };
    let target_cell = CellKey::Square { x: 2, y: 1, z: 0 };
    world
        .link_maps("overmap", source_cell, "field", target_cell)
        .unwrap();

    // Unlinked source cell on a linked map.
    assert_eq!(
        world.map_cell(
            "overmap",
            &CellKey::Province {
                id: "prov_b".to_string()
            }
        ),
        None
    );
    // Unlinked map entirely.
    assert_eq!(
        world.map_cell("field", &CellKey::Square { x: 0, y: 0, z: 0 }),
        None
    );
    // Unlinked target cell.
    assert_eq!(
        world.unmap_cell("field", &CellKey::Square { x: 9, y: 9, z: 0 }),
        None
    );
    // Unlinked target map.
    assert_eq!(
        world.unmap_cell(
            "overmap",
            &CellKey::Province {
                id: "prov_a".to_string()
            }
        ),
        None
    );
}

// Coordinate mapping is topology-generic: a province source cell round-trips to a
// square target cell across two different topologies.
#[test]
fn test_cross_topology_round_trip_province_to_square() {
    let mut world = make_test_world();
    world.register_map("strategic", province_map()).unwrap();
    world.register_map("tactical", square_map()).unwrap();

    let source_cell = CellKey::Province {
        id: "prov_b".to_string(),
    };
    let target_cell = CellKey::Square { x: 3, y: 4, z: 0 };
    world
        .link_maps(
            "strategic",
            source_cell.clone(),
            "tactical",
            target_cell.clone(),
        )
        .unwrap();

    assert_eq!(
        world.map_cell("strategic", &source_cell),
        Some(target_cell.clone())
    );
    assert_eq!(
        world.unmap_cell("tactical", &target_cell),
        Some(source_cell.clone())
    );
}

// link_maps with an unknown map name errors (no panic).
#[test]
fn test_link_maps_unknown_map_errors() {
    let mut world = make_test_world();
    world.register_map("field", square_map()).unwrap();

    let err = world.link_maps(
        "nonexistent",
        CellKey::Square { x: 0, y: 0, z: 0 },
        "field",
        CellKey::Square { x: 1, y: 0, z: 0 },
    );
    assert!(err.is_err());

    let err = world.link_maps(
        "field",
        CellKey::Square { x: 0, y: 0, z: 0 },
        "nonexistent",
        CellKey::Square { x: 1, y: 0, z: 0 },
    );
    assert!(err.is_err());
}

// --- M1: per-(source_map, source_cell) link keying, conflict semantics ---
// AC001 (R001): fan-out — two source cells on one parent map link to two
// distinct child maps, and both resolve independently. Fails on the pre-fix
// single-link-per-source-map model, where the second link discards the first.
#[test]
fn test_link_maps_fans_out_two_cells_to_two_children() {
    let mut world = make_test_world();
    world.register_map("planet", square_map()).unwrap();
    world.register_map("region_a", square_map()).unwrap();
    world.register_map("region_b", square_map()).unwrap();

    let cell_a = CellKey::Square { x: 0, y: 0, z: 0 };
    let cell_b = CellKey::Square { x: 1, y: 0, z: 0 };
    let target_a = CellKey::Square { x: 5, y: 5, z: 0 };
    let target_b = CellKey::Square { x: 7, y: 7, z: 0 };

    world
        .link_maps("planet", cell_a.clone(), "region_a", target_a.clone())
        .unwrap();
    world
        .link_maps("planet", cell_b.clone(), "region_b", target_b.clone())
        .unwrap();

    // Both links coexist: each source cell resolves to its own target.
    assert_eq!(world.map_cell("planet", &cell_a), Some(target_a.clone()));
    assert_eq!(world.map_cell("planet", &cell_b), Some(target_b.clone()));

    // Reverse lookup stays per-target-map too.
    assert_eq!(world.unmap_cell("region_a", &target_a), Some(cell_a));
    assert_eq!(world.unmap_cell("region_b", &target_b), Some(cell_b));
}

// AC003 (R002): a link from an already-linked source cell to a *different*
// target returns Err, and the original link is retained unchanged.
#[test]
fn test_link_maps_conflicting_target_errors_and_retains_original() {
    let mut world = make_test_world();
    world.register_map("planet", square_map()).unwrap();
    world.register_map("region_a", square_map()).unwrap();
    world.register_map("region_b", square_map()).unwrap();

    let source = CellKey::Square { x: 0, y: 0, z: 0 };
    let original_target = CellKey::Square { x: 1, y: 1, z: 0 };
    let other_target = CellKey::Square { x: 2, y: 2, z: 0 };

    world
        .link_maps(
            "planet",
            source.clone(),
            "region_a",
            original_target.clone(),
        )
        .unwrap();

    let err = world.link_maps("planet", source.clone(), "region_b", other_target.clone());
    assert!(
        err.is_err(),
        "a conflicting target for the same source cell must return Err"
    );

    // Original link retained; conflicting target was not inserted.
    assert_eq!(
        world.map_cell("planet", &source),
        Some(original_target.clone())
    );
    assert_eq!(
        world.unmap_cell("region_a", &original_target),
        Some(source.clone())
    );
    assert_eq!(world.unmap_cell("region_b", &other_target), None);
}

// AC004 (R002): re-issuing the *identical* link is an idempotent Ok and leaves
// the link table unchanged.
#[test]
fn test_link_maps_identical_relink_is_idempotent_ok() {
    let mut world = make_test_world();
    world.register_map("planet", square_map()).unwrap();
    world.register_map("region_a", square_map()).unwrap();

    let source = CellKey::Square { x: 0, y: 0, z: 0 };
    let target = CellKey::Square { x: 1, y: 1, z: 0 };

    world
        .link_maps("planet", source.clone(), "region_a", target.clone())
        .unwrap();
    // Identical re-link: Ok, no change.
    world
        .link_maps("planet", source.clone(), "region_a", target.clone())
        .unwrap();

    assert_eq!(world.map_cell("planet", &source), Some(target.clone()));
    assert_eq!(world.unmap_cell("region_a", &target), Some(source));
}

// --- M2: deterministic links_from enumeration and unmap_cell reverse lookup ---

use engine_core::ecs::world::MapLink;

// AC005 (R004): when several links share the same (target_map, target_cell),
// unmap_cell returns the link whose (source_map, source_cell) is minimum under
// the documented total order (source_map UTF-8 byte order, then canonical
// cell-key string byte order), and repeated calls stay stable regardless of
// HashMap iteration order.
#[test]
fn test_unmap_cell_tie_break_returns_documented_minimum() {
    let mut world = make_test_world();
    world.register_map("planet", square_map()).unwrap();
    world.register_map("region", square_map()).unwrap();

    let cell_min = CellKey::Square { x: 0, y: 0, z: 0 };
    let cell_other = CellKey::Square { x: 5, y: 5, z: 0 };
    let shared_target = CellKey::Square { x: 2, y: 2, z: 0 };

    // Insert the larger key first so a truncating scan cannot rely on insertion
    // order for the documented minimum.
    world
        .link_maps(
            "planet",
            cell_other.clone(),
            "region",
            shared_target.clone(),
        )
        .unwrap();
    world
        .link_maps("planet", cell_min.clone(), "region", shared_target.clone())
        .unwrap();

    // Canonical key strings: {"Square":{"x":0,...}} < {"Square":{"x":5,...}}.
    assert_eq!(
        world.unmap_cell("region", &shared_target),
        Some(cell_min.clone())
    );
    for _ in 0..32 {
        assert_eq!(
            world.unmap_cell("region", &shared_target),
            Some(cell_min.clone())
        );
    }
}

// AC005 (R004): source_map is the primary key of the documented total order
// (UTF-8 byte order), so a link from the lexicographically smaller source map
// wins the tie-break regardless of insertion order.
#[test]
fn test_unmap_cell_tie_break_orders_by_source_map_first() {
    let mut world = make_test_world();
    world.register_map("alpha", square_map()).unwrap();
    world.register_map("beta", square_map()).unwrap();
    world.register_map("region", square_map()).unwrap();

    let shared_target = CellKey::Square { x: 3, y: 3, z: 0 };
    let cell_beta = CellKey::Square { x: 9, y: 9, z: 0 };
    let cell_alpha = CellKey::Square { x: 9, y: 9, z: 0 };

    // "beta" inserted first; "alpha" must still win because "alpha" < "beta".
    world
        .link_maps("beta", cell_beta, "region", shared_target.clone())
        .unwrap();
    world
        .link_maps("alpha", cell_alpha.clone(), "region", shared_target.clone())
        .unwrap();

    assert_eq!(world.unmap_cell("region", &shared_target), Some(cell_alpha));
    for _ in 0..32 {
        assert_eq!(
            world.unmap_cell("region", &shared_target),
            Some(CellKey::Square { x: 9, y: 9, z: 0 })
        );
    }
}

// AC006 (R005): links_from returns every outbound link of a source map in
// deterministic canonical source-cell-key byte order; unknown and link-less
// source maps return an empty Vec.
#[test]
fn test_links_from_canonical_order_and_empty_for_unknown() {
    let mut world = make_test_world();
    world.register_map("planet", square_map()).unwrap();
    world.register_map("region_1", square_map()).unwrap();
    world.register_map("region_2", square_map()).unwrap();
    world.register_map("region_3", square_map()).unwrap();

    let cell_x1 = CellKey::Square { x: 1, y: 0, z: 0 };
    let cell_x0y0 = CellKey::Square { x: 0, y: 0, z: 0 };
    let cell_x0y1 = CellKey::Square { x: 0, y: 1, z: 0 };

    // Insert in scrambled order: canonical order is by serialized key bytes,
    // not insertion order.
    world
        .link_maps(
            "planet",
            cell_x1.clone(),
            "region_1",
            CellKey::Square { x: 1, y: 1, z: 0 },
        )
        .unwrap();
    world
        .link_maps(
            "planet",
            cell_x0y0.clone(),
            "region_2",
            CellKey::Square { x: 0, y: 0, z: 0 },
        )
        .unwrap();
    world
        .link_maps(
            "planet",
            cell_x0y1.clone(),
            "region_3",
            CellKey::Square { x: 0, y: 3, z: 0 },
        )
        .unwrap();

    // Canonical strings: {"Square":{"x":0,"y":0,..}} < {"Square":{"x":0,"y":1,..}}
    // < {"Square":{"x":1,"y":0,..}}.
    let expected = vec![
        MapLink {
            target_map: "region_2".to_string(),
            source_cell: cell_x0y0.clone(),
            target_cell: CellKey::Square { x: 0, y: 0, z: 0 },
        },
        MapLink {
            target_map: "region_3".to_string(),
            source_cell: cell_x0y1.clone(),
            target_cell: CellKey::Square { x: 0, y: 3, z: 0 },
        },
        MapLink {
            target_map: "region_1".to_string(),
            source_cell: cell_x1.clone(),
            target_cell: CellKey::Square { x: 1, y: 1, z: 0 },
        },
    ];
    assert_eq!(world.links_from("planet"), expected);

    // Unknown source map and a registered-but-link-less map both yield empty.
    assert!(world.links_from("unknown").is_empty());
    assert!(world.links_from("region_1").is_empty());
}

// --- M1: R008 runtime-only hierarchy (no save/load persistence) ---

#[path = "helpers/world_io.rs"]
mod world_io_helper;
use world_io_helper::save_and_load_roundtrip;

// AC011 (R008): core `MapLink` has no Serialize/Deserialize derive.
//
// Compile-level guard (the `static_assertions::assert_not_impl_any!` technique):
// a blanket `AmbiguousIfImpl<()>` applies to every type, and a second impl with a
// distinct marker trait-arg applies only when the bound holds. If `MapLink`
// implemented `Serialize` or `Deserialize`, two candidate impls would apply and
// inferring `_` in the probe below would be ambiguous (E0283), failing
// compilation. It compiles only while MapLink implements neither.
#[test]
fn test_map_link_has_no_serde_derives() {
    use engine_core::ecs::world::MapLink;

    trait AmbiguousIfImpl<A> {
        fn some_item() {}
    }
    impl<T: ?Sized> AmbiguousIfImpl<()> for T {}
    struct IfSerialize;
    struct IfDeserialize;
    impl<T: ?Sized + serde::Serialize> AmbiguousIfImpl<IfSerialize> for T {}
    impl<T: for<'de> serde::Deserialize<'de>> AmbiguousIfImpl<IfDeserialize> for T {}

    let _ = <MapLink as AmbiguousIfImpl<_>>::some_item;
}

// AC011 (R008): the multi-map fields stay #[serde(skip)]; a save->load
// round-trip completes without error and the hierarchy is absent after load.
#[test]
fn test_map_hierarchy_is_runtime_only_and_absent_after_load() {
    let mut world = make_test_world();
    world.register_map("planet", square_map()).unwrap();
    world.register_map("region_a", square_map()).unwrap();
    world.set_active_map("planet").unwrap();

    let source = CellKey::Square { x: 0, y: 0, z: 0 };
    let target = CellKey::Square { x: 1, y: 1, z: 0 };
    world
        .link_maps("planet", source.clone(), "region_a", target.clone())
        .unwrap();

    // Serialized form carries no hierarchy state.
    let json = serde_json::to_string(&world).unwrap();
    assert!(
        !json.contains("\"map_links\""),
        "map_links must be #[serde(skip)]"
    );
    assert!(!json.contains("\"maps\""), "maps must be #[serde(skip)]");

    // Save -> load completes without error and the hierarchy is gone.
    let loaded = save_and_load_roundtrip(&world, world.registry.clone());
    assert!(
        loaded.get_map_names().is_empty(),
        "map registry is runtime-only"
    );
    assert_eq!(loaded.get_active_map_name(), "");
    assert!(loaded.map_links.is_empty(), "links are runtime-only");
    assert_eq!(loaded.map_cell("planet", &source), None);
}
