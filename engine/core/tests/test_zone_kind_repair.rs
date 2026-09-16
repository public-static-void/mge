//! Kind-join repair + WASM recursion parity for region cell queries.
//!
//! Covers the M1 defect repairs: `cells_in_region_kind` resolves kind through
//! the `Region` record table (never a `kind` field on `RegionAssignment`),
//! and the WASM mirror resolves nested `Region` cells recursively with a
//! cycle guard identically to core.

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::world::World;
use engine_core::ecs::world::wasm::WasmWorld;
use serde_json::{Value, json};

fn sorted_cells(cells: &[Value]) -> Vec<String> {
    let mut out: Vec<String> = cells.iter().map(|v| v.to_string()).collect();
    out.sort();
    out
}

fn add_region(world: &mut World, id: &str, kind: &str) {
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Region", json!({"id": id, "kind": kind}))
        .unwrap();
}

fn assign_cell(world: &mut World, cell: Value, region_id: &str) {
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "RegionAssignment",
            json!({"cell": cell, "region_id": region_id}),
        )
        .unwrap();
}

fn add_wasm_region(world: &mut WasmWorld, id: &str, kind: &str) {
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Region", &json!({"id": id, "kind": kind}).to_string())
        .unwrap();
}

fn assign_wasm_cell(world: &mut WasmWorld, cell: Value, region_id: &str) {
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "RegionAssignment",
            &json!({"cell": cell, "region_id": region_id}).to_string(),
        )
        .unwrap();
}

fn square(x: i64, y: i64) -> Value {
    json!({"Square": {"x": x, "y": y, "z": 0}})
}

// Two regions sharing one kind union their cells; other kinds are excluded.
#[test]
fn test_kind_query_unions_cells_across_same_kind_regions() {
    let mut world = make_test_world();
    add_region(&mut world, "sp_1", "stockpile");
    add_region(&mut world, "sp_2", "stockpile");
    add_region(&mut world, "room_1", "room");
    assign_cell(&mut world, square(0, 0), "sp_1");
    assign_cell(&mut world, square(1, 0), "sp_1");
    assign_cell(&mut world, square(0, 1), "sp_2");
    assign_cell(&mut world, square(5, 5), "room_1");

    let stockpile = world.cells_in_region_kind("stockpile");
    assert_eq!(stockpile.len(), 3);
    assert!(stockpile.contains(&square(0, 0)));
    assert!(stockpile.contains(&square(1, 0)));
    assert!(stockpile.contains(&square(0, 1)));
    assert!(!stockpile.contains(&square(5, 5)));

    let room = world.cells_in_region_kind("room");
    assert_eq!(room, vec![square(5, 5)]);

    assert!(world.cells_in_region_kind("vault").is_empty());
}

// Kind queries expand nested Region references and terminate on cycles,
// returning resolved physical cells only.
#[test]
fn test_kind_query_resolves_nested_cells_and_cycle() {
    let mut world = make_test_world();
    add_region(&mut world, "outer", "stockpile");
    add_region(&mut world, "inner", "stockpile");
    add_region(&mut world, "cyc_a", "room");
    add_region(&mut world, "cyc_b", "room");
    assign_cell(&mut world, square(2, 2), "inner");
    assign_cell(&mut world, json!({"Region": {"id": "inner"}}), "outer");
    assign_cell(&mut world, json!({"Region": {"id": "cyc_b"}}), "cyc_a");
    assign_cell(&mut world, json!({"Region": {"id": "cyc_a"}}), "cyc_b");

    let stockpile = world.cells_in_region_kind("stockpile");
    assert!(stockpile.contains(&square(2, 2)));
    assert!(
        stockpile.iter().all(|c| c.get("Region").is_none()),
        "kind query must never return opaque Region JSON"
    );

    let room = world.cells_in_region_kind("room");
    assert!(room.is_empty(), "pure cycle resolves to no concrete cells");
}

// Core World and the WASM mirror return identical cell sets for nested
// references, cycles, and kind queries over the same fixture.
#[test]
fn test_wasm_mirror_matches_core_on_nested_cycle_and_kind() {
    let mut core = make_test_world();
    let mut mirror = WasmWorld::new();

    for (id, kind) in [
        ("outer", "stockpile"),
        ("inner", "stockpile"),
        ("cyc_a", "room"),
    ] {
        add_region(&mut core, id, kind);
        add_wasm_region(&mut mirror, id, kind);
    }
    for (cell, region_id) in [
        (square(0, 0), "inner"),
        (square(1, 0), "inner"),
        (json!({"Region": {"id": "inner"}}), "outer"),
        (json!({"Region": {"id": "cyc_a"}}), "cyc_a"),
        (square(9, 9), "cyc_a"),
    ] {
        assign_cell(&mut core, cell.clone(), region_id);
        assign_wasm_cell(&mut mirror, cell, region_id);
    }
    // Cycle pair: cyc_a references cyc_b, cyc_b references cyc_a.
    add_region(&mut core, "cyc_b", "room");
    add_wasm_region(&mut mirror, "cyc_b", "room");
    assign_cell(&mut core, json!({"Region": {"id": "cyc_b"}}), "cyc_a");
    assign_wasm_cell(&mut mirror, json!({"Region": {"id": "cyc_b"}}), "cyc_a");
    assign_cell(&mut core, json!({"Region": {"id": "cyc_a"}}), "cyc_b");
    assign_wasm_cell(&mut mirror, json!({"Region": {"id": "cyc_a"}}), "cyc_b");

    for region_id in ["outer", "inner", "cyc_a", "cyc_b"] {
        assert_eq!(
            sorted_cells(&core.cells_in_region(region_id)),
            sorted_cells(&mirror.cells_in_region(region_id)),
            "cells_in_region diverged for {region_id}"
        );
    }
    for kind in ["stockpile", "room", "vault"] {
        assert_eq!(
            sorted_cells(&core.cells_in_region_kind(kind)),
            sorted_cells(&mirror.cells_in_region_kind(kind)),
            "cells_in_region_kind diverged for {kind}"
        );
    }

    // Spot-check resolved content: outer expands to inner's physical cells,
    // and no opaque Region JSON leaks from either implementation.
    let outer = mirror.cells_in_region("outer");
    assert!(outer.contains(&square(0, 0)));
    assert!(outer.contains(&square(1, 0)));
    assert!(outer.iter().all(|c| c.get("Region").is_none()));
    assert!(
        core.cells_in_region("outer")
            .iter()
            .all(|c| c.get("Region").is_none())
    );
}
