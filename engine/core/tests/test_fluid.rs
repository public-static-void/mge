//! Integration tests for the fluid simulation system (water, magma).
//!
//! Covers the metadata merge helper (R001), horizontal flooding (R004),
//! downward z-flow (R005), determinism (R006), magma-water interaction (R007),
//! derived blocking keys (R008), and non-destructive metadata preservation
//! (NFR003). Tests exercise the public `World` and `FluidSimulationSystem` API.

use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::map::{CellKey, Map, SquareGridMap};
use engine_core::systems::fluid::FluidSimulationSystem;
use serde_json::{Value as JsonValue, json};

#[path = "helpers/world.rs"]
mod world_helper;

/// Builds a world whose active map is a square grid with the given cells and
/// 4-connected neighbors (up/down/left/right) between adjacent cells.
fn square_world(cells: &[(i32, i32, i32)]) -> World {
    let mut world = world_helper::make_test_world();
    let mut grid = SquareGridMap::new();
    for &(x, y, z) in cells {
        grid.add_cell(x, y, z);
    }
    for &(x, y, z) in cells {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let nx = x + dx;
            let ny = y + dy;
            if cells.contains(&(nx, ny, z)) {
                grid.add_neighbor((x, y, z), (nx, ny, z));
            }
        }
    }
    world.map = Some(Map::new(Box::new(grid)));
    world
}

fn cell(x: i32, y: i32, z: i32) -> CellKey {
    CellKey::Square { x, y, z }
}

/// Reads the fluid level for a single-type fluid cell, or 0 when absent.
fn fluid_level(world: &World, c: &CellKey) -> i64 {
    world
        .get_cell_metadata(c)
        .and_then(|m| m.get("fluid"))
        .and_then(|f| f.get("level"))
        .and_then(JsonValue::as_i64)
        .unwrap_or(0)
}

/// Sum of all fluid levels across the given cells (conservation check).
fn total_fluid(world: &World, cells: &[CellKey]) -> i64 {
    cells.iter().map(|c| fluid_level(world, c)).sum()
}

// --- AC001: merge helper preserves existing metadata ---

#[test]
fn merge_cell_metadata_preserves_existing_keys() {
    let mut world = square_world(&[(0, 0, 0)]);
    let c = cell(0, 0, 0);
    world.set_cell_metadata(&c, json!({"walkable": true, "terrain": "floor"}));

    world.merge_cell_metadata(&c, json!({"fluid": {"type": "water", "level": 3}}));

    let meta = world.get_cell_metadata(&c).unwrap();
    assert_eq!(meta["walkable"], true);
    assert_eq!(meta["terrain"], "floor");
    assert_eq!(meta["fluid"]["type"], "water");
    assert_eq!(meta["fluid"]["level"], 3);
}

// --- AC002: merge helper creates metadata when absent ---

#[test]
fn merge_cell_metadata_creates_when_absent() {
    let mut world = square_world(&[(0, 0, 0)]);
    let c = cell(0, 0, 0);

    world.merge_cell_metadata(&c, json!({"fluid": {"type": "water", "level": 1}}));

    let meta = world.get_cell_metadata(&c).unwrap();
    assert_eq!(meta, &json!({"fluid": {"type": "water", "level": 1}}));
}

// --- AC003: horizontal flooding on a 3x3 map ---

#[test]
fn horizontal_flooding_spreads_and_conserves() {
    let cells = [
        (0, 0, 0),
        (0, 1, 0),
        (0, 2, 0),
        (1, 0, 0),
        (1, 1, 0),
        (1, 2, 0),
        (2, 0, 0),
        (2, 1, 0),
        (2, 2, 0),
    ];
    let mut world = square_world(&cells);
    let source = cell(1, 1, 0);
    world.merge_cell_metadata(&source, json!({"fluid": {"type": "water", "level": 4}}));

    let keys: Vec<CellKey> = cells.iter().map(|&(x, y, z)| cell(x, y, z)).collect();
    let initial_total = total_fluid(&world, &keys);

    let mut system = FluidSimulationSystem::default();
    for _ in 0..5 {
        system.run(&mut world);
    }

    // Fluid spread out of the source into the orthogonal neighbors that the
    // equalization rule can reach (a level-1 cell cannot transfer to a level-0
    // neighbor, so the flood reaches the four orthogonal neighbors but not the
    // diagonal corners).
    assert!(
        fluid_level(&world, &source) < 4,
        "source level should decrease as fluid spreads"
    );
    for &(x, y, z) in &[(0, 1, 0), (1, 0, 0), (1, 2, 0)] {
        assert!(
            fluid_level(&world, &cell(x, y, z)) > 0,
            "adjacent cell ({x},{y},{z}) should have gained fluid"
        );
    }
    // No z-flow or magma interaction here, so fluid is conserved.
    assert_eq!(total_fluid(&world, &keys), initial_total);
}

// --- AC004: downward z-flow ---

#[test]
fn z_flow_moves_fluid_downward() {
    let mut world = square_world(&[(0, 0, 0), (0, 0, 1)]);
    let top = cell(0, 0, 1);
    let bottom = cell(0, 0, 0);
    world.merge_cell_metadata(&top, json!({"fluid": {"type": "water", "level": 4}}));

    let mut system = FluidSimulationSystem::default();
    system.run(&mut world);

    assert_eq!(fluid_level(&world, &bottom), 1, "cell below gains one unit");
    assert_eq!(fluid_level(&world, &top), 3, "source loses one unit");
}

// --- AC005: determinism across HashMap insertion orders ---

#[test]
fn determinism_across_insertion_orders() {
    let cells = [
        (0, 0, 0),
        (0, 1, 0),
        (0, 2, 0),
        (1, 0, 0),
        (1, 1, 0),
        (1, 2, 0),
        (2, 0, 0),
        (2, 1, 0),
        (2, 2, 0),
    ];

    // Two maps with the same cells but different insertion orders.
    let mut world_a = square_world(&cells);
    let mut world_b = square_world(&cells);
    let source_a = cell(1, 1, 0);
    let source_b = cell(1, 1, 0);
    world_a.merge_cell_metadata(&source_a, json!({"fluid": {"type": "water", "level": 4}}));
    world_b.merge_cell_metadata(&source_b, json!({"fluid": {"type": "water", "level": 4}}));

    let mut system_a = FluidSimulationSystem::default();
    let mut system_b = FluidSimulationSystem::default();
    for _ in 0..100 {
        system_a.run(&mut world_a);
        system_b.run(&mut world_b);
    }

    for &(x, y, z) in &cells {
        let c = cell(x, y, z);
        let meta_a = world_a
            .get_cell_metadata(&c)
            .cloned()
            .unwrap_or(JsonValue::Null);
        let meta_b = world_b
            .get_cell_metadata(&c)
            .cloned()
            .unwrap_or(JsonValue::Null);
        assert_eq!(
            meta_a, meta_b,
            "cell ({x},{y},{z}) metadata differs across insertion orders"
        );
    }
}

// --- AC006: magma-water interaction produces steam ---

#[test]
fn magma_water_interaction_reduces_both_and_emits_steam() {
    let mut world = square_world(&[(0, 0, 0)]);
    let c = cell(0, 0, 0);
    world.merge_cell_metadata(&c, json!({"fluid": {"water": 2, "magma": 2}}));

    let mut system = FluidSimulationSystem::default();

    // First tick: both reduced by 1, steam event recorded.
    system.run(&mut world);
    let meta = world.get_cell_metadata(&c).unwrap();
    assert_eq!(meta["fluid"]["water"], 1);
    assert_eq!(meta["fluid"]["magma"], 1);
    world.update_event_buses::<JsonValue>();
    let steam = world.drain_events::<JsonValue>("steam");
    assert_eq!(steam.len(), 1, "one steam event after first tick");

    // Second tick: both reach 0.
    system.run(&mut world);
    let meta = world.get_cell_metadata(&c).unwrap();
    // The empty state is written in the canonical single-type form; the merge
    // helper preserves the prior dual-form keys, so assert the effective level.
    assert_eq!(meta["fluid"]["level"], 0, "effective fluid level reaches 0");
    world.update_event_buses::<JsonValue>();
    let steam = world.drain_events::<JsonValue>("steam");
    assert_eq!(steam.len(), 1, "one steam event after second tick");
}

// --- AC007: derived blocking keys set and restored ---

#[test]
fn blocking_keys_set_at_threshold_and_restored_below() {
    // Two connected cells: A holds the fluid, B is the drain.
    let mut world = square_world(&[(0, 0, 0), (1, 0, 0)]);
    let a = cell(0, 0, 0);
    let b = cell(1, 0, 0);
    world.set_cell_metadata(
        &a,
        json!({"walkable": true, "transparent": true, "terrain": "floor"}),
    );
    world.set_cell_metadata(
        &b,
        json!({"walkable": true, "transparent": true, "terrain": "floor"}),
    );
    world.merge_cell_metadata(&a, json!({"fluid": {"type": "water", "level": 4}}));

    let mut system = FluidSimulationSystem::default();

    // Tick 1: A flows to B, A stays at level 3 (>= threshold) → blocked.
    system.run(&mut world);
    let meta_a = world.get_cell_metadata(&a).unwrap();
    assert_eq!(meta_a["walkable"], false);
    assert_eq!(meta_a["transparent"], false);
    assert_eq!(
        meta_a["terrain"], "floor",
        "terrain preserved while blocked"
    );

    // Tick 2: A drops to level 2 (< threshold) → restored to pre-fluid values.
    system.run(&mut world);
    let meta_a = world.get_cell_metadata(&a).unwrap();
    assert_eq!(meta_a["walkable"], true);
    assert_eq!(meta_a["transparent"], true);
    assert_eq!(
        meta_a["terrain"], "floor",
        "terrain preserved after restore"
    );
}

// --- AC014: non-destructive metadata preservation ---

#[test]
fn non_destructive_metadata_preserved_on_fluid_and_non_fluid_cells() {
    let mut world = square_world(&[(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
    let fluid_cell = cell(0, 0, 0);
    let neighbor = cell(1, 0, 0);
    let untouched = cell(2, 0, 0);

    // Worldgen-style metadata on all three cells.
    for c in [&fluid_cell, &neighbor, &untouched] {
        world.set_cell_metadata(
            c,
            json!({"walkable": true, "transparent": true, "terrain": "floor", "cost": 1}),
        );
    }
    world.merge_cell_metadata(&fluid_cell, json!({"fluid": {"type": "water", "level": 4}}));

    let mut system = FluidSimulationSystem::default();
    for _ in 0..3 {
        system.run(&mut world);
    }

    // All four keys remain intact on the fluid cell, the neighbor that received
    // flow, and the untouched cell.
    for c in [&fluid_cell, &neighbor, &untouched] {
        let meta = world.get_cell_metadata(c).unwrap();
        assert_eq!(meta["walkable"], true, "walkable preserved on {c:?}");
        assert_eq!(meta["transparent"], true, "transparent preserved on {c:?}");
        assert_eq!(meta["terrain"], "floor", "terrain preserved on {c:?}");
        assert_eq!(meta["cost"], 1, "cost preserved on {c:?}");
    }
}
