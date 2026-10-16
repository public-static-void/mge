use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use engine_core::systems::planet_biomes::{
    PlanetBiomeConfig, PlanetBiomeGenerator, cell_climate, classify_biome,
};
use engine_core::systems::vehicle::VehicleSystem;
use engine_core::worldgen::{
    FALLBACK_MAPGEN_ALGORITHM, MapgenAlgorithm, ThreadSafeWorldgenRegistry, WorldgenRegistry,
    register_builtin_mapgen_algorithms, register_builtin_mapgen_algorithms_local,
};
use serde_json::json;
use std::collections::{HashMap, HashSet};

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

const ALLOWED_BIOMES: [&str; 11] = [
    "polar_ice",
    "tundra",
    "boreal_forest",
    "cold_steppe",
    "hot_desert",
    "tropical_rainforest",
    "savanna",
    "temperate_forest",
    "temperate_grassland",
    "mountain",
    "barren",
];

#[test]
fn generator_registers_under_planet_biomes_name() {
    assert_eq!(PlanetBiomeGenerator::NAME, "planet_biomes");
    assert_ne!(PlanetBiomeGenerator::NAME, "dungeon");
    assert_ne!(PlanetBiomeGenerator::NAME, "caves");
}

#[test]
fn low_pressure_always_yields_mountain() {
    assert_eq!(classify_biome(20.0, 0.5, 979.0), "mountain");
    assert_eq!(classify_biome(-30.0, 0.9, 900.0), "mountain");
}

#[test]
fn freezing_climate_yields_polar_ice_regardless_of_humidity() {
    assert_eq!(classify_biome(-20.0, 0.9, 1013.0), "polar_ice");
    assert_eq!(classify_biome(-20.0, 0.1, 1013.0), "polar_ice");
}

#[test]
fn near_freezing_climate_yields_tundra() {
    assert_eq!(classify_biome(-5.0, 0.5, 1013.0), "tundra");
}

#[test]
fn cool_wet_climate_yields_boreal_forest() {
    assert_eq!(classify_biome(5.0, 0.6, 1013.0), "boreal_forest");
}

#[test]
fn cool_dry_climate_yields_cold_steppe() {
    assert_eq!(classify_biome(5.0, 0.2, 1013.0), "cold_steppe");
}

#[test]
fn hot_arid_climate_yields_hot_desert() {
    assert_eq!(classify_biome(30.0, 0.1, 1013.0), "hot_desert");
}

#[test]
fn hot_humid_climate_yields_tropical_rainforest() {
    assert_eq!(classify_biome(26.0, 0.8, 1013.0), "tropical_rainforest");
}

#[test]
fn warm_dry_climate_yields_savanna() {
    assert_eq!(classify_biome(22.0, 0.3, 1013.0), "savanna");
}

#[test]
fn mild_wet_climate_yields_temperate_forest() {
    assert_eq!(classify_biome(15.0, 0.7, 1013.0), "temperate_forest");
}

#[test]
fn mild_moderate_climate_yields_temperate_grassland() {
    assert_eq!(classify_biome(15.0, 0.4, 1013.0), "temperate_grassland");
}

#[test]
fn mild_dry_leftover_yields_barren_catch_all() {
    assert_eq!(classify_biome(15.0, 0.2, 1013.0), "barren");
}

#[test]
fn pressure_boundary_at_980_splits_mountain_from_temperate() {
    assert_eq!(classify_biome(20.0, 0.5, 979.9), "mountain");
    assert_eq!(classify_biome(20.0, 0.5, 980.0), "temperate_grassland");
}

#[test]
fn freezing_boundary_at_minus_8_splits_polar_ice_from_tundra() {
    assert_eq!(classify_biome(-8.1, 0.5, 1013.0), "polar_ice");
    assert_eq!(classify_biome(-8.0, 0.5, 1013.0), "tundra");
}

#[test]
fn tundra_boundary_at_2_splits_tundra_from_cold_steppe() {
    assert_eq!(classify_biome(1.9, 0.0, 1013.0), "tundra");
    assert_eq!(classify_biome(2.0, 0.0, 1013.0), "cold_steppe");
}

#[test]
fn boreal_boundary_at_10_splits_boreal_from_temperate_forest() {
    assert_eq!(classify_biome(9.9, 0.6, 1013.0), "boreal_forest");
    assert_eq!(classify_biome(10.0, 0.6, 1013.0), "temperate_forest");
}

#[test]
fn boreal_humidity_boundary_at_045_splits_boreal_from_cold_steppe() {
    assert_eq!(classify_biome(5.0, 0.45, 1013.0), "boreal_forest");
    assert_eq!(classify_biome(5.0, 0.44, 1013.0), "cold_steppe");
}

#[test]
fn desert_boundary_at_28_splits_desert_from_savanna() {
    assert_eq!(classify_biome(28.0, 0.2, 1013.0), "hot_desert");
    assert_eq!(classify_biome(27.9, 0.2, 1013.0), "savanna");
}

#[test]
fn desert_humidity_boundary_at_025_splits_desert_from_savanna() {
    assert_eq!(classify_biome(30.0, 0.24, 1013.0), "hot_desert");
    assert_eq!(classify_biome(30.0, 0.25, 1013.0), "savanna");
}

#[test]
fn rainforest_boundary_at_24_splits_rainforest_from_temperate_forest() {
    assert_eq!(classify_biome(24.0, 0.7, 1013.0), "tropical_rainforest");
    assert_eq!(classify_biome(23.9, 0.7, 1013.0), "temperate_forest");
}

#[test]
fn rainforest_humidity_boundary_at_065_splits_rainforest_from_temperate() {
    assert_eq!(classify_biome(26.0, 0.65, 1013.0), "tropical_rainforest");
    assert_eq!(classify_biome(26.0, 0.64, 1013.0), "temperate_forest");
}

#[test]
fn savanna_boundary_at_20_splits_savanna_from_grassland() {
    assert_eq!(classify_biome(20.0, 0.3, 1013.0), "savanna");
    assert_eq!(classify_biome(19.9, 0.3, 1013.0), "temperate_grassland");
}

#[test]
fn savanna_humidity_boundary_at_04_splits_savanna_from_grassland() {
    assert_eq!(classify_biome(22.0, 0.39, 1013.0), "savanna");
    assert_eq!(classify_biome(22.0, 0.4, 1013.0), "temperate_grassland");
}

#[test]
fn temperate_forest_humidity_boundary_at_055_splits_forest_from_grassland() {
    assert_eq!(classify_biome(15.0, 0.55, 1013.0), "temperate_forest");
    assert_eq!(classify_biome(15.0, 0.54, 1013.0), "temperate_grassland");
}

#[test]
fn grassland_humidity_boundary_at_03_splits_grassland_from_barren() {
    assert_eq!(classify_biome(15.0, 0.3, 1013.0), "temperate_grassland");
    assert_eq!(classify_biome(15.0, 0.29, 1013.0), "barren");
}

#[test]
fn every_climate_triple_maps_to_a_known_biome() {
    let pressures = [930.0, 979.9, 980.0, 1013.0, 1028.0, 1035.0];
    let mut seen = std::collections::HashSet::new();
    let mut t = -70.0;
    while t <= 70.0 {
        let mut h = 0.0;
        while h <= 1.0 {
            for p in pressures {
                let biome = classify_biome(t, h, p);
                assert!(
                    ALLOWED_BIOMES.contains(&biome),
                    "unknown biome {biome} for ({t}, {h}, {p})"
                );
                assert!(!biome.is_empty());
                seen.insert(biome);
            }
            h += 0.05;
        }
        t += 2.0;
    }
    assert_eq!(seen.len(), ALLOWED_BIOMES.len());
}

#[test]
fn cell_climate_stays_within_physical_ranges() {
    for seed in [1u64, 7, 42, 12345] {
        for y in 0..16 {
            for x in 0..16 {
                let (temp, humidity, pressure) = cell_climate(seed, x, y, 16, 16);
                assert!(
                    (-60.0..=60.0).contains(&temp),
                    "temp {temp} out of range at ({x}, {y})"
                );
                assert!(
                    (0.0..=1.0).contains(&humidity),
                    "humidity {humidity} out of range at ({x}, {y})"
                );
                assert!(
                    (938.0..=1028.0).contains(&pressure),
                    "pressure {pressure} out of range at ({x}, {y})"
                );
            }
        }
    }
}

#[test]
fn cell_climate_is_deterministic_for_same_inputs() {
    let first = cell_climate(99, 3, 5, 16, 16);
    let second = cell_climate(99, 3, 5, 16, 16);
    assert_eq!(first, second);
}

#[test]
fn cell_climate_varies_across_cells_and_seeds() {
    let base = cell_climate(1, 0, 0, 16, 16);
    let neighbor = cell_climate(1, 1, 0, 16, 16);
    let other_seed = cell_climate(2, 0, 0, 16, 16);
    assert_ne!(base, neighbor);
    assert_ne!(base, other_seed);
}

#[test]
fn single_row_grid_uses_equator_climate_everywhere() {
    for x in 0..8 {
        let (temp, _, _) = cell_climate(42, x, 0, 8, 1);
        assert!(
            temp >= 25.0,
            "single-row cell ({x}, 0) should be equator-warm, got {temp}"
        );
        let (temp_c, humidity, pressure) = cell_climate(42, x, 0, 8, 1);
        let biome = classify_biome(temp_c, humidity, pressure);
        assert_ne!(biome, "polar_ice");
        assert_ne!(biome, "tundra");
    }
}

#[test]
fn missing_dims_fall_back_to_32x32() {
    let config = PlanetBiomeConfig::from_params(&json!({"seed": 7})).expect("defaults apply");
    assert_eq!(config.width, 32);
    assert_eq!(config.height, 32);
    assert_eq!(config.seed, 7);
}

#[test]
fn empty_params_fall_back_to_32x32_with_time_seed() {
    let config = PlanetBiomeConfig::from_params(&json!({})).expect("defaults apply");
    assert_eq!(config.width, 32);
    assert_eq!(config.height, 32);
}

#[test]
fn explicit_seed_and_dims_win_over_defaults() {
    let config = PlanetBiomeConfig::from_params(&json!({"seed": 7, "width": 8, "height": 8}))
        .expect("explicit params apply");
    assert_eq!(config.seed, 7);
    assert_eq!(config.width, 8);
    assert_eq!(config.height, 8);
}

#[test]
fn extra_params_are_ignored() {
    let config = PlanetBiomeConfig::from_params(
        &json!({"seed": 7, "width": 8, "height": 8, "future_knob": true}),
    )
    .expect("unknown keys ignored");
    assert_eq!((config.width, config.height), (8, 8));
}

#[test]
fn zero_width_is_rejected_by_name() {
    let err = PlanetBiomeConfig::from_params(&json!({"seed": 1, "width": 0, "height": 8}))
        .expect_err("zero width must fail");
    assert!(err.contains("width"), "error names width: {err}");
}

#[test]
fn negative_height_is_rejected_by_name() {
    let err = PlanetBiomeConfig::from_params(&json!({"seed": 1, "width": 8, "height": -3}))
        .expect_err("negative height must fail");
    assert!(err.contains("height"), "error names height: {err}");
}

#[test]
fn string_width_is_rejected_by_name() {
    let err = PlanetBiomeConfig::from_params(&json!({"seed": 1, "width": "x", "height": 8}))
        .expect_err("string width must fail");
    assert!(err.contains("width"), "error names width: {err}");
}

#[test]
fn fractional_dims_are_rejected_by_name() {
    let err = PlanetBiomeConfig::from_params(&json!({"seed": 1, "width": 8.5, "height": 8}))
        .expect_err("fractional width must fail");
    assert!(err.contains("width"), "error names width: {err}");
}

#[test]
fn oversized_grid_is_rejected_before_allocation() {
    let err = PlanetBiomeConfig::from_params(&json!({"seed": 1, "width": 2000, "height": 2000}))
        .expect_err("oversized grid must fail");
    assert!(
        err.contains("width") || err.contains("height"),
        "error names a dimension: {err}"
    );
}

// ---- Generator square output plus dual-registry integration ----

fn biome_by_coord(map: &serde_json::Value) -> HashMap<(i32, i32), String> {
    map["cells"]
        .as_array()
        .expect("cells is an array")
        .iter()
        .map(|cell| {
            (
                (
                    cell["x"].as_i64().expect("integer x") as i32,
                    cell["y"].as_i64().expect("integer y") as i32,
                ),
                cell["biome"].as_str().expect("string biome").to_string(),
            )
        })
        .collect()
}

fn neighbors_by_coord(map: &serde_json::Value) -> HashMap<(i32, i32), Vec<(i32, i32)>> {
    map["cells"]
        .as_array()
        .expect("cells is an array")
        .iter()
        .map(|cell| {
            let key = (
                cell["x"].as_i64().expect("integer x") as i32,
                cell["y"].as_i64().expect("integer y") as i32,
            );
            let neighbors = cell
                .get("neighbors")
                .and_then(|n| n.as_array())
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|n| {
                    (
                        n["x"].as_i64().expect("integer neighbor x") as i32,
                        n["y"].as_i64().expect("integer neighbor y") as i32,
                    )
                })
                .collect();
            (key, neighbors)
        })
        .collect()
}

fn find_plain_to_mountain_step(map: &serde_json::Value) -> ((i32, i32), (i32, i32)) {
    let biomes = biome_by_coord(map);
    let cells: HashSet<(i32, i32)> = biomes.keys().cloned().collect();
    for (pos, biome) in &biomes {
        if biome != "mountain" {
            continue;
        }
        for offset in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let plain = (pos.0 + offset.0, pos.1 + offset.1);
            if cells.contains(&plain) && biomes[&plain] != "mountain" {
                return (plain, *pos);
            }
        }
    }
    panic!("seed must yield a mountain cell adjacent to a plain cell");
}

fn vehicle_step(x: i32, y: i32) -> serde_json::Value {
    json!({"Square": {"x": x, "y": y, "z": 0}})
}

fn vehicle_cell(world: &World, eid: u32) -> CellKey {
    CellKey::from_position(world.get_component(eid, "Position").unwrap()).unwrap()
}

fn drain(world: &mut World, event_type: &str) -> Vec<serde_json::Value> {
    world.update_event_buses::<serde_json::Value>();
    world.drain_events::<serde_json::Value>(event_type)
}

fn spawn_vehicle(world: &mut World, x: i32, y: i32, blocked_terrains: Vec<&str>) -> u32 {
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "Position",
            json!({"pos": {"Square": {"x": x, "y": y, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(
            eid,
            "Vehicle",
            json!({
                "capacity": 2,
                "speed": 1,
                "blocked_terrains": blocked_terrains,
            }),
        )
        .unwrap();
    eid
}

fn spawn_rider(world: &mut World, x: i32, y: i32) -> u32 {
    let eid = world.spawn_entity();
    world
        .set_component(
            eid,
            "Position",
            json!({"pos": {"Square": {"x": x, "y": y, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(eid, "Agent", json!({"entity_id": eid}))
        .unwrap();
    eid
}

#[test]
fn planet_biomes_generates_square_grid_on_the_global_registry() {
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);
    let map = registry
        .invoke(
            "planet_biomes",
            &json!({"seed": 7, "width": 8, "height": 8}),
        )
        .expect("planet_biomes invokes on the global registry");
    assert_eq!(map["topology"], "square");
    assert_eq!(map["cells"].as_array().expect("cells array").len(), 64);
}

#[test]
fn planet_biomes_generates_square_grid_on_the_local_registry() {
    let mut registry = WorldgenRegistry::new();
    register_builtin_mapgen_algorithms_local(&mut registry);
    let map = registry
        .invoke(
            "planet_biomes",
            &json!({"seed": 7, "width": 8, "height": 8}),
        )
        .expect("planet_biomes invokes on the local registry");
    assert_eq!(map["topology"], "square");
    assert_eq!(map["cells"].as_array().expect("cells array").len(), 64);
}

#[test]
fn direct_generation_matches_registry_output_for_same_seed() {
    let params = json!({"seed": 7, "width": 8, "height": 8});
    let direct = PlanetBiomeGenerator
        .generate(&params)
        .expect("direct generation succeeds");
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);
    let via_registry = registry
        .invoke("planet_biomes", &params)
        .expect("registry generation succeeds");
    assert_eq!(
        serde_json::to_string(&direct).unwrap(),
        serde_json::to_string(&via_registry).unwrap()
    );
}

#[test]
fn every_cell_carries_a_known_biome_with_terrain_only_on_mountains() {
    let map = PlanetBiomeGenerator
        .generate(&json!({"seed": 42, "width": 32, "height": 32}))
        .expect("generates");
    let cells = map["cells"].as_array().expect("cells array");
    assert_eq!(cells.len(), 1024);
    let mut mountains = 0;
    for cell in cells {
        let biome = cell["biome"].as_str().expect("string biome");
        assert!(ALLOWED_BIOMES.contains(&biome), "unknown biome {biome}");
        assert_eq!(cell["z"], 0);
        if biome == "mountain" {
            mountains += 1;
            assert_eq!(cell["terrain"], "mountain");
        } else {
            assert!(
                cell.get("terrain").is_none(),
                "non-mountain cell must omit terrain: {cell}"
            );
        }
    }
    assert!(mountains >= 1, "seed 42 must surface mountain coverage");
}

#[test]
fn neighbor_lists_match_four_neighbourhood_expectations() {
    let map = PlanetBiomeGenerator
        .generate(&json!({"seed": 7, "width": 8, "height": 8}))
        .expect("generates");
    let neighbors = neighbors_by_coord(&map);
    assert_eq!(neighbors[&(0, 0)].len(), 2, "corner has 2 neighbors");
    assert_eq!(neighbors[&(7, 0)].len(), 2, "corner has 2 neighbors");
    assert_eq!(neighbors[&(0, 7)].len(), 2, "corner has 2 neighbors");
    assert_eq!(neighbors[&(7, 7)].len(), 2, "corner has 2 neighbors");
    assert_eq!(neighbors[&(4, 0)].len(), 3, "edge has 3 neighbors");
    assert_eq!(neighbors[&(0, 4)].len(), 3, "edge has 3 neighbors");
    assert_eq!(neighbors[&(4, 4)].len(), 4, "interior has 4 neighbors");
    for (pos, neighs) in &neighbors {
        for n in neighs {
            assert_ne!(*n, *pos, "no cell lists itself");
            assert!(
                (0..8).contains(&n.0) && (0..8).contains(&n.1),
                "neighbor {n:?} of {pos:?} stays in bounds"
            );
        }
        let mut unique = neighs.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), neighs.len(), "no duplicate neighbors");
    }
}

#[test]
fn tiny_grids_label_every_cell_with_correct_neighbor_counts() {
    let single = PlanetBiomeGenerator
        .generate(&json!({"seed": 3, "width": 1, "height": 1}))
        .expect("1x1 generates");
    let cells = single["cells"].as_array().expect("cells array");
    assert_eq!(cells.len(), 1);
    assert!(cells[0]["biome"].is_string());
    assert_eq!(
        cells[0]
            .get("neighbors")
            .and_then(|n| n.as_array())
            .map_or(0, |n| n.len()),
        0,
        "lone cell has no neighbors"
    );

    let row = PlanetBiomeGenerator
        .generate(&json!({"seed": 3, "width": 4, "height": 1}))
        .expect("4x1 generates");
    let neighbors = neighbors_by_coord(&row);
    assert_eq!(neighbors.len(), 4);
    assert_eq!(neighbors[&(0, 0)].len(), 1);
    assert_eq!(neighbors[&(1, 0)].len(), 2);
    assert_eq!(neighbors[&(2, 0)].len(), 2);
    assert_eq!(neighbors[&(3, 0)].len(), 1);

    let column = PlanetBiomeGenerator
        .generate(&json!({"seed": 3, "width": 1, "height": 4}))
        .expect("1x4 generates");
    let neighbors = neighbors_by_coord(&column);
    assert_eq!(neighbors.len(), 4);
    assert_eq!(neighbors[&(0, 0)].len(), 1);
    assert_eq!(neighbors[&(0, 1)].len(), 2);
    assert_eq!(neighbors[&(0, 2)].len(), 2);
    assert_eq!(neighbors[&(0, 3)].len(), 1);
}

#[test]
fn generated_metadata_never_blocks_movement_by_default() {
    let map = PlanetBiomeGenerator
        .generate(&json!({"seed": 42, "width": 32, "height": 32}))
        .expect("generates");
    for cell in map["cells"].as_array().expect("cells array") {
        if let Some(metadata) = cell.get("metadata") {
            assert!(
                metadata.get("walkable").and_then(|v| v.as_bool()) != Some(false),
                "generated metadata must not mark cells unwalkable: {cell}"
            );
            assert!(
                metadata.get("transparent").and_then(|v| v.as_bool()) != Some(false),
                "generated metadata must not mark cells opaque: {cell}"
            );
        }
    }
}

#[test]
fn caller_metadata_keys_survive_alongside_generated_terrain() {
    let map = PlanetBiomeGenerator
        .generate(&json!({"seed": 7, "width": 8, "height": 8}))
        .expect("generates");
    let (_, mountain) = find_plain_to_mountain_step(&map);
    let mut world = make_test_world();
    world.apply_generated_map(&map).expect("map applies");
    let key = CellKey::Square {
        x: mountain.0,
        y: mountain.1,
        z: 0,
    };
    world.merge_cell_metadata(&key, json!({"walkable": true, "transparent": true}));
    let metadata = world
        .get_cell_metadata(&key)
        .expect("mountain cell has metadata");
    assert_eq!(
        metadata["terrain"], "mountain",
        "generated terrain survives an additive merge"
    );
    assert_eq!(metadata["walkable"], true);
    assert_eq!(metadata["transparent"], true);
}

#[test]
fn mountain_terrain_blocks_only_mountain_averse_vehicles() {
    let map = PlanetBiomeGenerator
        .generate(&json!({"seed": 7, "width": 8, "height": 8}))
        .expect("generates");
    let (plain, mountain) = find_plain_to_mountain_step(&map);

    let mut wary = make_test_world();
    wary.apply_generated_map(&map).expect("map applies");
    let vehicle = spawn_vehicle(&mut wary, plain.0, plain.1, vec!["mountain"]);
    let rider = spawn_rider(&mut wary, plain.0, plain.1);
    wary.embark(vehicle, rider).unwrap();
    wary.set_component(
        vehicle,
        "Vehicle",
        json!({
            "capacity": 2,
            "speed": 1,
            "blocked_terrains": ["mountain"],
            "occupants": [rider],
            "move_path": [vehicle_step(mountain.0, mountain.1)],
        }),
    )
    .unwrap();
    VehicleSystem.run(&mut wary);
    assert_eq!(
        vehicle_cell(&wary, vehicle),
        CellKey::Square {
            x: plain.0,
            y: plain.1,
            z: 0
        },
        "mountain-averse vehicle holds before mountain terrain"
    );
    assert_eq!(drain(&mut wary, "vehicle_move_blocked").len(), 1);

    let mut bold = make_test_world();
    bold.apply_generated_map(&map).expect("map applies");
    let vehicle = spawn_vehicle(&mut bold, plain.0, plain.1, vec![]);
    let rider = spawn_rider(&mut bold, plain.0, plain.1);
    bold.embark(vehicle, rider).unwrap();
    bold.set_component(
        vehicle,
        "Vehicle",
        json!({
            "capacity": 2,
            "speed": 1,
            "blocked_terrains": [],
            "occupants": [rider],
            "move_path": [vehicle_step(mountain.0, mountain.1)],
        }),
    )
    .unwrap();
    VehicleSystem.run(&mut bold);
    assert_eq!(
        vehicle_cell(&bold, vehicle),
        CellKey::Square {
            x: mountain.0,
            y: mountain.1,
            z: 0
        },
        "vehicle with no blocked terrains enters mountain terrain"
    );
    assert_eq!(drain(&mut bold, "vehicle_move_blocked").len(), 0);
}

#[test]
fn default_algorithm_fallback_remains_dungeon() {
    assert_eq!(FALLBACK_MAPGEN_ALGORITHM, "dungeon");
    let mut registry = ThreadSafeWorldgenRegistry::new();
    register_builtin_mapgen_algorithms(&mut registry);
    assert!(registry.names().contains(&"planet_biomes".to_string()));
    let map = registry
        .invoke("dungeon", &json!({"seed": 7, "width": 8, "height": 8}))
        .expect("dungeon still invokes");
    assert_eq!(map["topology"], "square");
}
