//! Integration tests for the weather and climate system.
//!
//! Covers initial state (AC020), tick advancement, transition on expiry,
//! season-weighted probabilities, the visibility modifier formula (R008),
//! direct weather override, `"weather_changed"` events (AC014), determinism
//! (AC015), save/load round-trip (AC016), and backward-compatible
//! deserialization (AC017).

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

#[path = "helpers/world_io.rs"]
mod world_io_helper;
use world_io_helper::save_and_load_roundtrip;

use engine_core::ecs::system::System;
use engine_core::ecs::world::{WeatherCondition, WeatherState, World};
use engine_core::map::{Map, SquareGridMap};
use engine_core::systems::weather::{WeatherSystem, compute_visibility_modifier};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Build a world with a minimal square map so `WeatherSystem` runs
/// (the system is a no-op while `world.map` is `None`).
fn weather_world() -> World {
    let mut world = make_test_world();
    let mut grid = SquareGridMap::new();
    grid.add_cell(0, 0, 0);
    world.map = Some(Map::new(Box::new(grid)));
    world
}

#[test]
fn test_weather_initial_state() {
    // R020: default state is Clear, intensity 0.0, duration 0, zeroed seed.
    let state = WeatherState::default();
    assert_eq!(state.condition, WeatherCondition::Clear);
    assert_eq!(state.intensity, 0.0);
    assert_eq!(state.duration_remaining, 0);
    assert_eq!(state.rng_state, [0u8; 32]);

    // World::new() initializes weather to the default state (R020).
    let world = weather_world();
    assert_eq!(world.weather.condition, WeatherCondition::Clear);
    assert_eq!(world.weather.intensity, 0.0);
    assert_eq!(world.weather.duration_remaining, 0);
    assert_eq!(world.weather.rng_state, [0u8; 32]);
    assert_eq!(world.visibility_modifier, 1.0);
}

#[test]
fn test_weather_advances_on_tick() {
    let mut world = weather_world();
    let mut system = WeatherSystem;

    // Fresh world has duration 0 — the first run transitions immediately
    // (edge case: "Duration reaches 0 on first tick").
    system.run(&mut world);
    assert!(
        (50..=200).contains(&world.weather.duration_remaining),
        "first tick should set a fresh duration in 50..=200, got {}",
        world.weather.duration_remaining
    );
    assert!(
        (0.0..=1.0).contains(&world.weather.intensity),
        "intensity must stay in 0.0..=1.0"
    );

    // The next run decrements the duration without transitioning.
    let before = world.weather.duration_remaining;
    system.run(&mut world);
    assert_eq!(world.weather.duration_remaining, before - 1);
}

#[test]
fn test_weather_transitions_on_expiry() {
    let mut world = weather_world();
    // Snow is an attractor condition: it can only transition to Storm or
    // Clear (R006), so the new condition is guaranteed to differ from Snow.
    world.weather.condition = WeatherCondition::Snow;
    world.weather.duration_remaining = 1;

    let mut system = WeatherSystem;
    system.run(&mut world);

    assert_ne!(
        world.weather.condition,
        WeatherCondition::Snow,
        "Snow must transition away from Snow (attractor rule)"
    );
    assert!(
        (50..=200).contains(&world.weather.duration_remaining),
        "transition should set a fresh duration in 50..=200"
    );
    assert!((0.0..=1.0).contains(&world.weather.intensity));
}

#[test]
fn test_weather_season_probabilities() {
    // Winter base weights (R006): Cloudy 25, Snow 30, Clear 25, Storm 20.
    // Force the current condition to Clear (a general condition that uses the
    // base weights) and reset the duration to 0 before every draw so each run
    // samples the same distribution. The persisted rng_state evolves between
    // draws, producing a deterministic pseudo-random sequence.
    let mut world = weather_world();
    world.time_of_day.day = 90; // Winter (days 90–119)
    let mut system = WeatherSystem;

    let mut counts: HashMap<&'static str, u32> = HashMap::new();
    const DRAWS: u32 = 2000;
    for _ in 0..DRAWS {
        world.weather.condition = WeatherCondition::Clear;
        world.weather.duration_remaining = 0;
        system.run(&mut world);
        *counts.entry(world.weather.condition.as_str()).or_insert(0) += 1;
    }

    let expected: &[(&str, f64)] = &[
        ("cloudy", 0.25),
        ("snow", 0.30),
        ("clear", 0.25),
        ("storm", 0.20),
    ];
    for (name, p) in expected {
        let observed = *counts.get(name).unwrap_or(&0) as f64 / DRAWS as f64;
        assert!(
            (observed - p).abs() < 0.05,
            "Winter transition to {name}: observed {observed:.3}, expected ~{p:.2}"
        );
    }
}

#[test]
fn test_weather_visibility_modifier() {
    // R008 formula: Clear 1.0, Cloudy 0.9, Rain 0.7×i, Snow 0.6×i,
    // Storm 0.5×i, Fog 0.4×i.
    assert_eq!(
        compute_visibility_modifier(WeatherCondition::Clear, 0.5),
        1.0
    );
    assert_eq!(
        compute_visibility_modifier(WeatherCondition::Cloudy, 0.5),
        0.9
    );
    assert!((compute_visibility_modifier(WeatherCondition::Rain, 0.5) - 0.35).abs() < 1e-9);
    assert!((compute_visibility_modifier(WeatherCondition::Snow, 0.5) - 0.30).abs() < 1e-9);
    assert!((compute_visibility_modifier(WeatherCondition::Storm, 0.5) - 0.25).abs() < 1e-9);
    assert!((compute_visibility_modifier(WeatherCondition::Fog, 0.5) - 0.20).abs() < 1e-9);

    // Intensity bounds: 0 → no reduction for precipitation types, 1 → base factor.
    assert_eq!(
        compute_visibility_modifier(WeatherCondition::Rain, 0.0),
        0.0
    );
    assert!((compute_visibility_modifier(WeatherCondition::Rain, 1.0) - 0.7).abs() < 1e-9);

    // The system writes the computed value to world.visibility_modifier.
    let mut world = weather_world();
    world.weather.condition = WeatherCondition::Fog;
    world.weather.intensity = 0.5;
    world.weather.duration_remaining = 10;
    let mut system = WeatherSystem;
    system.run(&mut world);
    assert!((world.visibility_modifier - 0.2).abs() < 1e-9);
}

#[test]
fn test_weather_set_override() {
    // Direct override of condition, intensity, and duration. The bridge-level
    // set_weather() (Lua/Python/WASM) writes these same fields; Rust tests
    // exercise the underlying state directly.
    let mut world = weather_world();
    world.weather.condition = WeatherCondition::Storm;
    world.weather.intensity = 0.8;
    world.weather.duration_remaining = 120;

    assert_eq!(world.weather.condition, WeatherCondition::Storm);
    assert_eq!(world.weather.intensity, 0.8);
    assert_eq!(world.weather.duration_remaining, 120);

    // The override is respected by the system: the next run decrements the
    // overridden duration rather than transitioning.
    let mut system = WeatherSystem;
    system.run(&mut world);
    assert_eq!(world.weather.condition, WeatherCondition::Storm);
    assert_eq!(world.weather.duration_remaining, 119);
    // Visibility modifier reflects the overridden state: 0.5 × 0.8 = 0.4.
    assert!((world.visibility_modifier - 0.4).abs() < 1e-9);
}

#[test]
fn test_weather_emits_events() {
    let mut world = weather_world();
    world.weather.condition = WeatherCondition::Rain;
    world.weather.duration_remaining = 1;

    let mut system = WeatherSystem;
    system.run(&mut world);

    // Advance the event bus so the emitted event moves to the read buffer.
    world.update_event_buses::<serde_json::Value>();
    let events = world.drain_events::<serde_json::Value>("weather_changed");
    assert_eq!(
        events.len(),
        1,
        "a transition should emit exactly one weather_changed event"
    );
    assert_eq!(events[0]["old_condition"], "rain");
    assert_eq!(events[0]["new_condition"], world.weather.condition.as_str());
    assert_eq!(
        events[0]["intensity"].as_f64(),
        Some(world.weather.intensity)
    );
}

#[test]
fn test_weather_determinism() {
    // Two identical worlds run through the same turn sequence must produce
    // identical WeatherState sequences (NFR001, AC015).
    let mut world1 = weather_world();
    world1.register_system(WeatherSystem);
    let mut world2 = weather_world();
    world2.register_system(WeatherSystem);

    let world1_rc = Rc::new(RefCell::new(world1));
    let world2_rc = Rc::new(RefCell::new(world2));

    for _ in 0..50 {
        World::tick(Rc::clone(&world1_rc));
        World::tick(Rc::clone(&world2_rc));
        let w1 = world1_rc.borrow();
        let w2 = world2_rc.borrow();
        assert_eq!(w1.weather.condition, w2.weather.condition);
        assert_eq!(w1.weather.intensity, w2.weather.intensity);
        assert_eq!(w1.weather.duration_remaining, w2.weather.duration_remaining);
        assert_eq!(w1.weather.rng_state, w2.weather.rng_state);
        assert_eq!(w1.visibility_modifier, w2.visibility_modifier);
    }
}

#[test]
fn test_weather_save_load() {
    let mut world = weather_world();
    world.weather.condition = WeatherCondition::Snow;
    world.weather.intensity = 0.6;
    world.weather.duration_remaining = 77;
    world.weather.rng_state = [7u8; 32];

    let registry = world.registry.clone();
    let loaded = save_and_load_roundtrip(&world, registry);

    assert_eq!(loaded.weather.condition, WeatherCondition::Snow);
    assert_eq!(loaded.weather.intensity, 0.6);
    assert_eq!(loaded.weather.duration_remaining, 77);
    assert_eq!(loaded.weather.rng_state, [7u8; 32]);
}

#[test]
fn test_weather_default_backward_compat() {
    let mut world = weather_world();
    world.weather.condition = WeatherCondition::Storm;
    world.weather.intensity = 0.9;
    world.weather.duration_remaining = 150;
    world.weather.rng_state = [42u8; 32];

    let mut json: serde_json::Value = serde_json::to_value(&world).unwrap();
    // Simulate an old save: strip the weather field entirely (NFR002).
    json.as_object_mut().unwrap().remove("weather");

    let loaded: World = serde_json::from_value(json).unwrap();
    assert_eq!(loaded.weather.condition, WeatherCondition::Clear);
    assert_eq!(loaded.weather.intensity, 0.0);
    assert_eq!(loaded.weather.duration_remaining, 0);
    assert_eq!(loaded.weather.rng_state, [0u8; 32]);
}
