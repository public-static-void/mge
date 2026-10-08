//! Tests for the shared noise propagation kernel.
//!
//! The kernel computes BFS spread with linear falloff over a caller-supplied
//! neighbor lookup, so the core system (typed topology) and the WASM bridge
//! mirror (JSON adjacency) share one propagation shape instead of two
//! hand-synced loops.

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::map::cell_key::CellKey;
use engine_core::map::{Map, SquareGridMap};
use engine_core::systems::noise::{NoiseSystem, propagate_noise_kernel};
use serde_json::json;
use std::collections::{HashMap, HashSet};

/// Line of 5 cells (x = 0..=4) with bidirectional adjacency.
fn line_cells() -> (HashSet<CellKey>, HashMap<CellKey, Vec<CellKey>>) {
    let mut cells = HashSet::new();
    let mut adjacency: HashMap<CellKey, Vec<CellKey>> = HashMap::new();
    for x in 0..=4 {
        let cell = CellKey::Square { x, y: 0, z: 0 };
        cells.insert(cell.clone());
        let mut neighbors = Vec::new();
        if x > 0 {
            neighbors.push(CellKey::Square {
                x: x - 1,
                y: 0,
                z: 0,
            });
        }
        if x < 4 {
            neighbors.push(CellKey::Square {
                x: x + 1,
                y: 0,
                z: 0,
            });
        }
        adjacency.insert(cell, neighbors);
    }
    (cells, adjacency)
}

fn open_neighbors(
    adjacency: &HashMap<CellKey, Vec<CellKey>>,
) -> impl Fn(&CellKey) -> Vec<CellKey> + '_ {
    move |cell| adjacency.get(cell).cloned().unwrap_or_default()
}

fn transparent(_: &CellKey) -> bool {
    false
}

fn noise_at(world: &World, x: i32) -> Option<f64> {
    world.get_noise_at(&CellKey::Square { x, y: 0, z: 0 })
}

fn wasm_noise_at(world: &WasmWorld, x: i32) -> Option<f64> {
    world.get_noise_at(&CellKey::Square { x, y: 0, z: 0 })
}

#[test]
fn kernel_applies_linear_falloff_along_a_line() {
    let (_cells, adjacency) = line_cells();
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };

    let result = propagate_noise_kernel(&origin, 1.0, 5, open_neighbors(&adjacency), transparent);

    assert_eq!(result.get(&origin), Some(&1.0));
    for (x, expected) in [(1, 0.8), (2, 0.6), (3, 0.4), (4, 0.2)] {
        let cell = CellKey::Square { x, y: 0, z: 0 };
        assert_eq!(
            result.get(&cell),
            Some(&expected),
            "distance {x} should receive {expected}"
        );
    }
}

#[test]
fn kernel_marks_opaque_cells_but_stops_behind_them() {
    let (_cells, adjacency) = line_cells();
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let wall = CellKey::Square { x: 1, y: 0, z: 0 };

    let result = propagate_noise_kernel(&origin, 1.0, 5, open_neighbors(&adjacency), |cell| {
        *cell == wall
    });

    assert_eq!(result.get(&wall), Some(&0.8));
    assert_eq!(
        result.get(&CellKey::Square { x: 2, y: 0, z: 0 }),
        None,
        "cells behind an opaque cell stay silent"
    );
}

#[test]
fn kernel_agrees_with_core_system_and_wasm_mirror() {
    // Arrange: the same line map in all three shapes.
    let (_cells, adjacency) = line_cells();
    let origin = CellKey::Square { x: 0, y: 0, z: 0 };
    let expected = propagate_noise_kernel(&origin, 1.0, 5, open_neighbors(&adjacency), transparent);

    let mut grid = SquareGridMap::new();
    for x in 0..=4 {
        grid.add_cell(x, 0, 0);
    }
    for x in 0..4 {
        grid.add_neighbor((x, 0, 0), (x + 1, 0, 0));
    }
    let mut core = make_test_world();
    core.current_mode = "roguelike".to_string();
    core.map = Some(Map::new(Box::new(grid)));
    let emitter = core.spawn_entity();
    core.set_component(
        emitter,
        "NoiseEmitter",
        json!({"intensity": 1.0, "radius": 5, "active": true}),
    )
    .unwrap();
    core.set_component(
        emitter,
        "Position",
        json!({"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}),
    )
    .unwrap();

    let mut wasm = WasmWorld::new();
    for x in 0..=4 {
        wasm.add_cell(x, 0, 0);
    }
    for x in 0..4 {
        let a = serde_json::to_string(&CellKey::Square { x, y: 0, z: 0 }).unwrap();
        let b = serde_json::to_string(&CellKey::Square {
            x: x + 1,
            y: 0,
            z: 0,
        })
        .unwrap();
        wasm.add_neighbor(&a, &b).unwrap();
    }
    let wemitter = wasm.spawn_entity();
    wasm.set_component(
        wemitter,
        "Position",
        "{\"pos\":{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}}",
    )
    .unwrap();
    wasm.set_component(
        wemitter,
        "NoiseEmitter",
        "{\"intensity\": 1.0, \"radius\": 5, \"active\": true}",
    )
    .unwrap();

    // Act
    NoiseSystem.run(&mut core);
    wasm.tick();

    // Assert: both sides match the kernel cell by cell.
    for x in 0..=4 {
        let cell = CellKey::Square { x, y: 0, z: 0 };
        assert_eq!(
            noise_at(&core, x),
            expected.get(&cell).copied(),
            "core system differs from kernel at x={x}"
        );
        assert_eq!(
            wasm_noise_at(&wasm, x),
            expected.get(&cell).copied(),
            "wasm mirror differs from kernel at x={x}"
        );
    }
}
