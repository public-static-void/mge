use engine_core::systems::planet_biomes::{
    PlanetBiomeConfig, PlanetBiomeGenerator, cell_climate, classify_biome,
};
use serde_json::json;

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
