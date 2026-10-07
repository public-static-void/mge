//! Unit tests for the WASM-bridge noise mirror (`WasmWorld::tick_noise`).
//!
//! Pins Lua/Python parity values plus the mirror's own branches (flat
//! positions, stealth scaling, inactive emitters, missing data) without
//! crossing the WASM transport — transport coverage lives in
//! `engine_wasm/tests/wasm_noise_api.rs`.

use engine_core::ecs::world::wasm::WasmWorld;
use engine_core::map::CellKey;

fn line_world() -> WasmWorld {
    let mut world = WasmWorld::new();
    world.add_cell(0, 0, 0);
    world.add_cell(1, 0, 0);
    world.add_cell(2, 0, 0);
    world.add_cell(6, 0, 0);
    let a = "{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}";
    let b = "{\"Square\":{\"x\":1,\"y\":0,\"z\":0}}";
    let c = "{\"Square\":{\"x\":2,\"y\":0,\"z\":0}}";
    world.add_neighbor(a, b).unwrap();
    world.add_neighbor(b, c).unwrap();
    world
}

fn emit(world: &mut WasmWorld, entity: u32, intensity: f64, radius: i32) {
    let json = serde_json::json!({
        "intensity": intensity,
        "radius": radius,
        "active": true,
    })
    .to_string();
    world.set_component(entity, "NoiseEmitter", &json).unwrap();
}

fn noise_at(world: &WasmWorld, x: i32, y: i32, z: i32) -> f64 {
    world
        .get_noise_at(&CellKey::Square { x, y, z })
        .unwrap_or(0.0)
}

#[test]
fn propagated_values_match_core_falloff() {
    let mut world = line_world();
    let emitter = world.spawn_entity();
    world
        .set_component(
            emitter,
            "Position",
            "{\"pos\":{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}}",
        )
        .unwrap();
    emit(&mut world, emitter, 1.0, 5);

    world.tick();

    assert_eq!(noise_at(&world, 0, 0, 0), 1.0);
    assert_eq!(noise_at(&world, 1, 0, 0), 0.8);
    assert_eq!(noise_at(&world, 2, 0, 0), 0.6);
    assert_eq!(noise_at(&world, 6, 0, 0), 0.0);
}

#[test]
fn flat_bridge_position_resolves() {
    let mut world = line_world();
    let emitter = world.spawn_entity();
    world
        .set_component(emitter, "Position", "{\"x\":0.0,\"y\":0.0,\"z\":0.0}")
        .unwrap();
    emit(&mut world, emitter, 1.0, 5);

    world.tick();

    assert_eq!(noise_at(&world, 0, 0, 0), 1.0);
    assert_eq!(noise_at(&world, 1, 0, 0), 0.8);
}

#[test]
fn stealth_scales_propagated_noise() {
    let mut world = line_world();
    let emitter = world.spawn_entity();
    world
        .set_component(
            emitter,
            "Position",
            "{\"pos\":{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}}",
        )
        .unwrap();
    emit(&mut world, emitter, 1.0, 5);
    world
        .set_component(emitter, "Stealth", "{\"noise_modifier\":0.5}")
        .unwrap();

    world.tick();

    assert_eq!(noise_at(&world, 0, 0, 0), 0.5);
    assert_eq!(noise_at(&world, 1, 0, 0), 0.4);
}

#[test]
fn inactive_emitter_stays_silent() {
    let mut world = line_world();
    let emitter = world.spawn_entity();
    world
        .set_component(
            emitter,
            "Position",
            "{\"pos\":{\"Square\":{\"x\":0,\"y\":0,\"z\":0}}}",
        )
        .unwrap();
    let json = serde_json::json!({
        "intensity": 1.0,
        "radius": 5,
        "active": false,
    })
    .to_string();
    world.set_component(emitter, "NoiseEmitter", &json).unwrap();

    world.tick();

    assert_eq!(noise_at(&world, 0, 0, 0), 0.0);
}

#[test]
fn emitter_without_position_is_skipped() {
    let mut world = line_world();
    let emitter = world.spawn_entity();
    emit(&mut world, emitter, 1.0, 5);

    world.tick();

    assert_eq!(noise_at(&world, 0, 0, 0), 0.0);
    assert_eq!(noise_at(&world, 1, 0, 0), 0.0);
}

#[test]
fn tick_without_map_leaves_noise_empty() {
    let mut world = WasmWorld::new();
    let emitter = world.spawn_entity();
    emit(&mut world, emitter, 1.0, 5);

    world.tick();

    assert_eq!(noise_at(&world, 0, 0, 0), 0.0);
}
