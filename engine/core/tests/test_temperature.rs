//! Integration tests for temperature and environment simulation v1.
//!
//! Covers the ambient derivation formula (season bases, weather deltas,
//! diurnal cycle, physical bounds), persistent state defaults and
//! backward-compatible deserialization, the no-map guard, per-part heat
//! exchange math (insulation scaling, null initialization, nested children),
//! the script override path (clamp, hold, single event), cold/heat stress
//! thresholds with per-part re-arming, extreme-exposure damage routing
//! (append shape, entry preservation, next-tick application), execution
//! ordering after weather, and multi-tick determinism with a mid-sequence
//! save/load round-trip.

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

#[path = "helpers/world_io.rs"]
mod world_io_helper;
use world_io_helper::save_and_load_roundtrip;

use engine_core::ecs::system::System;
use engine_core::ecs::world::{Season, WeatherCondition, World};
use engine_core::map::{Map, SquareGridMap};
use engine_core::systems::SYSTEM_EXECUTION_ORDER;
use engine_core::systems::body_part_damage::BodyPartDamageSystem;
use engine_core::systems::temperature::{
    TemperatureState, TemperatureSystem, compute_ambient_temperature,
};
use engine_core::systems::weather::WeatherSystem;
use serde_json::{Value as JsonValue, json};
use std::cell::RefCell;
use std::rc::Rc;

/// Build a world with a minimal square map so `TemperatureSystem` runs
/// (the system is a no-op while `world.map` is `None`).
fn temperature_world() -> World {
    let mut world = make_test_world();
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    world.map = Some(Map::new(Box::new(grid)));
    world
}

/// Hold ambient at a fixed value without emitting an override event
/// (direct field write, for tests that need a known ambient).
fn hold_ambient(world: &mut World, ambient: f64) {
    world.temperature.ambient = ambient;
    world.temperature.manual_override = Some(ambient);
}

/// Build one body part carrying every schema-required field.
fn part(name: &str, temperature: JsonValue, ideal: JsonValue, insulation: JsonValue) -> JsonValue {
    json!({
        "name": name,
        "kind": "torso",
        "status": "healthy",
        "hp": 25.0,
        "max_hp": 25.0,
        "temperature": temperature,
        "ideal_temperature": ideal,
        "insulation": insulation,
        "heat_loss": null,
        "children": [],
        "equipped": []
    })
}

/// Spawn an entity carrying a `Body` made of the given top-level parts.
fn spawn_with_parts(world: &mut World, parts: Vec<JsonValue>) -> u32 {
    let entity = world.spawn_entity();
    world
        .set_component(entity, "Body", json!({ "parts": parts }))
        .unwrap();
    entity
}

/// Move emitted events into the read buffer, then drain one bus.
fn take_events(world: &mut World, name: &str) -> Vec<JsonValue> {
    world.update_event_buses::<JsonValue>();
    world.drain_events::<JsonValue>(name)
}

fn part_temperature(world: &World, entity: u32, index: usize) -> f64 {
    world.get_component(entity, "Body").unwrap()["parts"][index]["temperature"]
        .as_f64()
        .unwrap()
}

fn part_heat_loss(world: &World, entity: u32, index: usize) -> f64 {
    world.get_component(entity, "Body").unwrap()["parts"][index]["heat_loss"]
        .as_f64()
        .unwrap()
}

#[test]
fn ambient_formula_matches_reference_values() {
    // Summer noon under clear skies: 25 base + 0 weather + 5 diurnal peak.
    let noon = compute_ambient_temperature(Season::Summer, WeatherCondition::Clear, 0.0, 14, 0);
    assert!((noon - 30.0).abs() < 1e-9, "expected 30.0, got {noon}");

    // Winter night in heavy snow: -10 base - 12 weather - 5 diurnal trough.
    let night = compute_ambient_temperature(Season::Winter, WeatherCondition::Snow, 1.0, 2, 0);
    assert!(
        (night - (-27.0)).abs() < 1e-9,
        "expected -27.0, got {night}"
    );
}

#[test]
fn ambient_formula_encodes_each_season_base() {
    // At 14:00 under clear skies the diurnal term is exactly +5,
    // so ambient equals the season base plus five.
    let cases = [
        (Season::Winter, -5.0),
        (Season::Spring, 15.0),
        (Season::Summer, 30.0),
        (Season::Autumn, 17.0),
    ];
    for (season, expected) in cases {
        let ambient = compute_ambient_temperature(season, WeatherCondition::Clear, 0.0, 14, 0);
        assert!(
            (ambient - expected).abs() < 1e-9,
            "{season:?}: expected {expected}, got {ambient}"
        );
    }
}

#[test]
fn ambient_weather_deltas_scale_linearly_with_intensity() {
    // Zero intensity means no weather penalty for precipitation types.
    for condition in [
        WeatherCondition::Rain,
        WeatherCondition::Snow,
        WeatherCondition::Storm,
        WeatherCondition::Fog,
    ] {
        let calm = compute_ambient_temperature(Season::Summer, condition, 0.0, 14, 0);
        assert!(
            (calm - 30.0).abs() < 1e-9,
            "{condition:?} at zero intensity should add nothing, got {calm}"
        );
    }

    // Full intensity applies the whole penalty; half intensity applies half.
    let full_and_half = [
        (WeatherCondition::Rain, -5.0),
        (WeatherCondition::Snow, -12.0),
        (WeatherCondition::Storm, -8.0),
        (WeatherCondition::Fog, -3.0),
    ];
    for (condition, delta) in full_and_half {
        let full = compute_ambient_temperature(Season::Summer, condition, 1.0, 14, 0);
        assert!(
            (full - (30.0 + delta)).abs() < 1e-9,
            "{condition:?}: expected {}, got {full}",
            30.0 + delta
        );
        let half = compute_ambient_temperature(Season::Summer, condition, 0.5, 14, 0);
        assert!(
            (half - (30.0 + delta / 2.0)).abs() < 1e-9,
            "{condition:?}: expected {}, got {half}",
            30.0 + delta / 2.0
        );
    }

    // Cloudy and Clear ignore intensity entirely.
    for intensity in [0.0, 0.5, 1.0] {
        let cloudy =
            compute_ambient_temperature(Season::Summer, WeatherCondition::Cloudy, intensity, 14, 0);
        assert!(
            (cloudy - 28.0).abs() < 1e-9,
            "cloudy should always sit 2 below clear, got {cloudy}"
        );
    }
}

#[test]
fn ambient_diurnal_cycle_peaks_at_mid_afternoon() {
    // Peak +5 at 14:00, trough -5 at 02:00, neutral near 08:00 and 20:00.
    let peak = compute_ambient_temperature(Season::Spring, WeatherCondition::Clear, 0.0, 14, 0);
    assert!((peak - 15.0).abs() < 1e-9);
    let trough = compute_ambient_temperature(Season::Spring, WeatherCondition::Clear, 0.0, 2, 0);
    assert!((trough - 5.0).abs() < 1e-9);
    for (hour, minute) in [(8, 0), (20, 0)] {
        let neutral =
            compute_ambient_temperature(Season::Spring, WeatherCondition::Clear, 0.0, hour, minute);
        assert!(
            (neutral - 10.0).abs() < 1e-9,
            "{hour:02}:{minute:02} should equal the bare season base, got {neutral}"
        );
    }
}

#[test]
fn ambient_output_stays_within_physical_bounds() {
    // The formula range is far inside the clamp, so a sweep over every
    // season, condition, intensity, and hour must never leave [-60, 60].
    let seasons = [
        Season::Spring,
        Season::Summer,
        Season::Autumn,
        Season::Winter,
    ];
    let conditions = [
        WeatherCondition::Clear,
        WeatherCondition::Cloudy,
        WeatherCondition::Rain,
        WeatherCondition::Snow,
        WeatherCondition::Storm,
        WeatherCondition::Fog,
    ];
    for season in seasons {
        for condition in conditions {
            for intensity in [0.0, 0.5, 1.0] {
                for hour in [0, 2, 8, 14, 20] {
                    let ambient =
                        compute_ambient_temperature(season, condition, intensity, hour, 30);
                    assert!(
                        (-60.0..=60.0).contains(&ambient),
                        "{season:?}/{condition:?} i={intensity} at {hour}:30 gave {ambient}"
                    );
                }
            }
        }
    }
}

#[test]
fn new_world_defaults_to_neutral_ambient_without_override() {
    let state = TemperatureState::default();
    assert_eq!(state.ambient, 15.0);
    assert_eq!(state.manual_override, None);

    let world = temperature_world();
    assert_eq!(world.temperature.ambient, 15.0);
    assert_eq!(world.temperature.manual_override, None);
}

#[test]
fn stripped_temperature_field_loads_with_default_ambient() {
    let mut world = temperature_world();
    world.temperature.ambient = 21.5;
    world.temperature.manual_override = Some(7.0);

    let mut json: serde_json::Value = serde_json::to_value(&world).unwrap();
    // Simulate an old save: strip the temperature field entirely.
    json.as_object_mut().unwrap().remove("temperature");

    let loaded: World = serde_json::from_value(json).unwrap();
    assert_eq!(loaded.temperature.ambient, 15.0);
    assert_eq!(loaded.temperature.manual_override, None);
}

#[test]
fn system_does_nothing_without_a_loaded_map() {
    let mut world = make_test_world();
    assert!(world.map.is_none());
    world.temperature.ambient = 20.0;
    let entity = spawn_with_parts(
        &mut world,
        vec![part("torso", json!(30.0), json!(30.0), json!(0.0))],
    );

    let mut system = TemperatureSystem;
    for _ in 0..10 {
        system.run(&mut world);
    }

    assert_eq!(world.temperature.ambient, 20.0);
    let body = world.get_component(entity, "Body").unwrap();
    assert_eq!(body["parts"][0]["temperature"], json!(30.0));
    assert!(body["parts"][0]["heat_loss"].is_null());
    assert!(!world.has_component(entity, "PendingDamage"));
    assert!(take_events(&mut world, "temperature_changed").is_empty());
    assert!(take_events(&mut world, "cold_stress").is_empty());
    assert!(take_events(&mut world, "heat_stress").is_empty());
}

#[test]
fn body_parts_drift_toward_ambient_at_insulation_scaled_rate() {
    let mut world = temperature_world();
    hold_ambient(&mut world, 0.0);
    let entity = spawn_with_parts(
        &mut world,
        vec![
            part("torso", json!(37.0), json!(37.0), json!(0.0)),
            part("arm", json!(37.0), json!(37.0), json!(4.0)),
        ],
    );

    TemperatureSystem.run(&mut world);

    // Uninsulated part moves 5% of the way: 37 - 37*0.05 = 35.15.
    let torso = part_temperature(&world, entity, 0);
    assert!(
        (torso - 35.15).abs() < 1e-9,
        "torso should be 35.15, got {torso}"
    );
    let torso_loss = part_heat_loss(&world, entity, 0);
    assert!(
        (torso_loss - 1.85).abs() < 1e-9,
        "torso heat_loss should be 1.85, got {torso_loss}"
    );
    // Insulation 4 slows the rate to k = 0.05/5 = 0.01: 37 - 0.37 = 36.63.
    let arm = part_temperature(&world, entity, 1);
    assert!((arm - 36.63).abs() < 1e-9, "arm should be 36.63, got {arm}");
    let arm_loss = part_heat_loss(&world, entity, 1);
    assert!(
        (arm_loss - 0.37).abs() < 1e-9,
        "arm heat_loss should be 0.37, got {arm_loss}"
    );
}

#[test]
fn missing_temperatures_initialize_silently() {
    let mut world = temperature_world();
    hold_ambient(&mut world, 0.0);
    let entity = spawn_with_parts(
        &mut world,
        vec![
            part("torso", JsonValue::Null, json!(37.0), json!(0.0)),
            part("leg", JsonValue::Null, JsonValue::Null, json!(0.0)),
        ],
    );

    TemperatureSystem.run(&mut world);

    // Null temperature starts at the ideal (or ambient when ideal is null).
    let body = world.get_component(entity, "Body").unwrap();
    assert_eq!(body["parts"][0]["temperature"], json!(37.0));
    assert_eq!(body["parts"][0]["heat_loss"], json!(0.0));
    assert_eq!(body["parts"][1]["temperature"], json!(0.0));
    assert_eq!(body["parts"][1]["heat_loss"], json!(0.0));
    // Initialization emits nothing.
    assert!(take_events(&mut world, "cold_stress").is_empty());
    assert!(take_events(&mut world, "heat_stress").is_empty());
    assert!(take_events(&mut world, "temperature_changed").is_empty());
    assert!(!world.has_component(entity, "PendingDamage"));
}

#[test]
fn parts_without_ideal_drift_without_events() {
    let mut world = temperature_world();
    hold_ambient(&mut world, 0.0);
    let entity = spawn_with_parts(
        &mut world,
        vec![part("torso", json!(37.0), JsonValue::Null, json!(0.0))],
    );

    TemperatureSystem.run(&mut world);

    let torso = part_temperature(&world, entity, 0);
    assert!((torso - 35.15).abs() < 1e-9);
    assert!(take_events(&mut world, "cold_stress").is_empty());
    assert!(take_events(&mut world, "heat_stress").is_empty());
    assert!(!world.has_component(entity, "PendingDamage"));
}

#[test]
fn nested_child_parts_exchange_heat() {
    let mut world = temperature_world();
    hold_ambient(&mut world, 0.0);
    let mut torso = part("torso", json!(37.0), json!(37.0), json!(0.0));
    torso["children"] = json!([part("hand", json!(20.0), json!(20.0), json!(0.0))]);
    let entity = spawn_with_parts(&mut world, vec![torso]);

    TemperatureSystem.run(&mut world);

    let body = world.get_component(entity, "Body").unwrap();
    let torso_temp = body["parts"][0]["temperature"].as_f64().unwrap();
    assert!((torso_temp - 35.15).abs() < 1e-9);
    let hand_temp = body["parts"][0]["children"][0]["temperature"]
        .as_f64()
        .unwrap();
    assert!(
        (hand_temp - 19.0).abs() < 1e-9,
        "hand should be 19.0, got {hand_temp}"
    );
}

#[test]
fn insulation_values_are_never_written_back() {
    let mut world = temperature_world();
    hold_ambient(&mut world, 0.0);
    let entity = spawn_with_parts(
        &mut world,
        vec![
            part("bare", json!(37.0), json!(37.0), json!(0.0)),
            part("clothed", json!(37.0), json!(37.0), json!(2.5)),
            part("unknown", json!(37.0), json!(37.0), JsonValue::Null),
        ],
    );

    let mut system = TemperatureSystem;
    for _ in 0..5 {
        system.run(&mut world);
    }

    let body = world.get_component(entity, "Body").unwrap();
    assert_eq!(body["parts"][0]["insulation"], json!(0.0));
    assert_eq!(body["parts"][1]["insulation"], json!(2.5));
    assert!(body["parts"][2]["insulation"].is_null());
}

#[test]
fn script_override_clamps_holds_and_emits_a_single_event() {
    let mut world = temperature_world();
    let mut system = TemperatureSystem;

    world.set_temperature(100.0);
    assert_eq!(world.temperature.ambient, 60.0);
    assert_eq!(world.temperature.manual_override, Some(60.0));
    let changed = take_events(&mut world, "temperature_changed");
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0]["old_ambient"], json!(15.0));
    assert_eq!(changed[0]["new_ambient"], json!(60.0));

    // Hostile derivation weather cannot move a held override.
    world.time_of_day.day = 100; // winter
    world.time_of_day.hour = 2;
    world.weather.condition = WeatherCondition::Snow;
    world.weather.intensity = 1.0;
    system.run(&mut world);
    assert_eq!(world.temperature.ambient, 60.0);
    system.run(&mut world);
    assert_eq!(world.temperature.ambient, 60.0);
    assert!(take_events(&mut world, "temperature_changed").is_empty());

    // The lower clamp behaves symmetrically.
    world.set_temperature(-100.0);
    assert_eq!(world.temperature.ambient, -60.0);
    let changed = take_events(&mut world, "temperature_changed");
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0]["old_ambient"], json!(60.0));
    assert_eq!(changed[0]["new_ambient"], json!(-60.0));
}

#[test]
fn derived_drift_recomputes_ambient_without_override_events() {
    let mut world = temperature_world();
    world.time_of_day.day = 40; // summer
    world.time_of_day.hour = 14;
    world.time_of_day.minute = 0;
    let mut system = TemperatureSystem;

    world.weather.condition = WeatherCondition::Clear;
    world.weather.intensity = 0.0;
    system.run(&mut world);
    assert!((world.temperature.ambient - 30.0).abs() < 1e-9);

    world.weather.condition = WeatherCondition::Storm;
    world.weather.intensity = 1.0;
    system.run(&mut world);
    assert!((world.temperature.ambient - 22.0).abs() < 1e-9);

    assert!(take_events(&mut world, "temperature_changed").is_empty());
}

#[test]
fn cold_stress_fires_once_per_crossing_with_payload() {
    let mut world = temperature_world();
    hold_ambient(&mut world, -60.0);
    let entity = spawn_with_parts(
        &mut world,
        vec![part("torso", json!(23.0), json!(37.0), json!(0.0))],
    );
    let mut system = TemperatureSystem;

    system.run(&mut world);
    // 23 + (-60 - 23) * 0.05 = 18.85, below the 22-degree cold line.
    let events = take_events(&mut world, "cold_stress");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["entity"], json!(entity));
    assert_eq!(events[0]["part"], json!("torso"));
    assert_eq!(events[0]["ideal"], json!(37.0));
    let reported = events[0]["temperature"].as_f64().unwrap();
    assert!((reported - 18.85).abs() < 1e-9);

    // Staying out in the cold does not repeat the event.
    system.run(&mut world);
    assert!(take_events(&mut world, "cold_stress").is_empty());
}

#[test]
fn heat_stress_fires_once_per_crossing_with_payload() {
    let mut world = temperature_world();
    hold_ambient(&mut world, 60.0);
    let entity = spawn_with_parts(
        &mut world,
        vec![part("torso", json!(55.0), json!(37.0), json!(0.0))],
    );
    let mut system = TemperatureSystem;

    system.run(&mut world);
    // 55 + (60 - 55) * 0.05 = 55.25, above the 52-degree heat line.
    let events = take_events(&mut world, "heat_stress");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["entity"], json!(entity));
    assert_eq!(events[0]["part"], json!("torso"));
    assert_eq!(events[0]["ideal"], json!(37.0));
    let reported = events[0]["temperature"].as_f64().unwrap();
    assert!((reported - 55.25).abs() < 1e-9);

    system.run(&mut world);
    assert!(take_events(&mut world, "heat_stress").is_empty());
}

#[test]
fn stress_flags_rearm_after_returning_inside_the_band() {
    let mut world = temperature_world();
    hold_ambient(&mut world, -60.0);
    let entity = spawn_with_parts(
        &mut world,
        vec![part("torso", json!(23.0), json!(37.0), json!(0.0))],
    );
    let mut system = TemperatureSystem;

    system.run(&mut world);
    assert_eq!(take_events(&mut world, "cold_stress").len(), 1);

    // Warm back inside the comfort band; the crossing flag clears quietly.
    world.temperature.ambient = 60.0;
    world.temperature.manual_override = Some(60.0);
    for _ in 0..5 {
        system.run(&mut world);
    }
    assert!(take_events(&mut world, "cold_stress").is_empty());
    assert!(take_events(&mut world, "heat_stress").is_empty());
    let warmed = part_temperature(&world, entity, 0);
    assert!(
        warmed > 22.0 && warmed < 52.0,
        "part should rest inside the band, got {warmed}"
    );

    // A fresh cold snap fires again once the part leaves the band.
    hold_ambient(&mut world, -60.0);
    let mut refires = 0;
    for _ in 0..5 {
        system.run(&mut world);
        refires += take_events(&mut world, "cold_stress").len();
    }
    assert_eq!(refires, 1);
}

#[test]
fn extreme_exposure_queues_unit_damage_preserving_existing_entries() {
    let mut world = temperature_world();
    hold_ambient(&mut world, -60.0);
    let entity = spawn_with_parts(
        &mut world,
        vec![part("torso", json!(10.0), json!(37.0), json!(0.0))],
    );
    world.damage_entity(entity, 2.0);
    let mut system = TemperatureSystem;

    system.run(&mut world);
    let pending = world.get_component(entity, "PendingDamage").unwrap();
    let damages = pending["damages"].as_array().unwrap();
    assert_eq!(damages.len(), 2);
    assert_eq!(damages[0], json!({ "amount": 2.0, "target_part": null }));
    assert_eq!(damages[1], json!({ "amount": 1.0, "target_part": "torso" }));

    // Exposure persists while the deviation holds, appending each tick.
    system.run(&mut world);
    let pending = world.get_component(entity, "PendingDamage").unwrap();
    assert_eq!(pending["damages"].as_array().unwrap().len(), 3);
}

#[test]
fn mild_deviation_queues_no_damage() {
    let mut world = temperature_world();
    hold_ambient(&mut world, 30.0);
    let entity = spawn_with_parts(
        &mut world,
        vec![part("torso", json!(30.0), json!(37.0), json!(0.0))],
    );

    let mut system = TemperatureSystem;
    for _ in 0..3 {
        system.run(&mut world);
    }

    assert!(!world.has_component(entity, "PendingDamage"));
}

#[test]
fn queued_damage_reduces_part_hp_on_the_following_tick() {
    let mut world = temperature_world();
    hold_ambient(&mut world, -60.0);
    let entity = spawn_with_parts(
        &mut world,
        vec![part("torso", json!(10.0), json!(37.0), json!(0.0))],
    );

    // First tick queues damage; the part itself is untouched yet.
    TemperatureSystem.run(&mut world);
    assert!(world.has_component(entity, "PendingDamage"));
    let body = world.get_component(entity, "Body").unwrap();
    assert_eq!(body["parts"][0]["hp"], json!(25.0));

    // Second tick applies the queued unit of damage to the named part.
    let mut damage = BodyPartDamageSystem;
    damage.run(&mut world);
    let body = world.get_component(entity, "Body").unwrap();
    assert_eq!(body["parts"][0]["hp"], json!(24.0));
    assert!(!world.has_component(entity, "PendingDamage"));
}

#[test]
fn temperature_runs_immediately_after_weather_in_execution_order() {
    let weather_pos = SYSTEM_EXECUTION_ORDER
        .iter()
        .position(|name| *name == "WeatherSystem")
        .unwrap();
    let temperature_pos = SYSTEM_EXECUTION_ORDER
        .iter()
        .position(|name| *name == "TemperatureSystem")
        .unwrap();
    assert_eq!(temperature_pos, weather_pos + 1);
}

#[test]
fn identical_worlds_produce_identical_temperature_sequences() {
    fn deterministic_world() -> World {
        let mut world = temperature_world();
        world.time_of_day.day = 30; // summer
        world.time_of_day.hour = 0;
        world.time_of_day.minute = 0;
        world.weather.condition = WeatherCondition::Clear;
        world.weather.intensity = 0.0;
        world.weather.duration_remaining = 200;
        let entity = world.spawn_entity();
        world
            .set_component(
                entity,
                "Body",
                json!({ "parts": [part("torso", json!(20.0), json!(20.0), json!(1.0)) ]}),
            )
            .unwrap();
        world.register_system(WeatherSystem);
        world.register_system(TemperatureSystem);
        world
    }

    let world_a = Rc::new(RefCell::new(deterministic_world()));
    let world_b = Rc::new(RefCell::new(deterministic_world()));

    for tick in 0..50 {
        World::tick(Rc::clone(&world_a));
        World::tick(Rc::clone(&world_b));

        let a = world_a.borrow();
        let b = world_b.borrow();
        assert_eq!(
            a.temperature.ambient, b.temperature.ambient,
            "ambient diverged on tick {tick}"
        );
        assert_eq!(
            a.get_component(1, "Body"),
            b.get_component(1, "Body"),
            "body temperatures diverged on tick {tick}"
        );

        // Mid-sequence save/load preserves the live state: float fields
        // cross a JSON text round-trip that can shift the last ulp, so the
        // comparison uses a tight tolerance instead of bitwise equality.
        if tick == 25 {
            let registry = a.registry.clone();
            drop(a);
            drop(b);
            let live = world_a.borrow();
            let loaded = save_and_load_roundtrip(&live, registry);
            assert!(
                (loaded.temperature.ambient - live.temperature.ambient).abs() < 1e-12,
                "round-trip must preserve ambient"
            );
            assert_eq!(
                loaded.temperature.manual_override,
                live.temperature.manual_override
            );
            let live_body = live.get_component(1, "Body").unwrap();
            let loaded_body = loaded.get_component(1, "Body").unwrap();
            for (index, live_part) in live_body["parts"].as_array().unwrap().iter().enumerate() {
                let loaded_part = &loaded_body["parts"][index];
                assert_eq!(loaded_part["name"], live_part["name"]);
                assert_eq!(
                    loaded_part["ideal_temperature"],
                    live_part["ideal_temperature"]
                );
                assert_eq!(loaded_part["insulation"], live_part["insulation"]);
                for field in ["temperature", "heat_loss"] {
                    let before = live_part[field].as_f64().unwrap();
                    let after = loaded_part[field].as_f64().unwrap();
                    assert!(
                        (before - after).abs() < 1e-9,
                        "round-trip must preserve {field}: {before} vs {after}"
                    );
                }
            }
            drop(live);
        }
    }
}
