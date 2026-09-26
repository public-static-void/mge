//! Integration tests for Vehicle support (ROADMAP L56).
//!
//! Covers the M5 slice: embark/disembark, mounted co-movement at speed 1 and
//! 2, Hex variant-preserving co-move, capacity rejection, terrain guards,
//! path prefix truncation, rider path suppression, vehicle despawn handling,
//! Province no-step, 50-tick two-world determinism, and save/load round-trip.

#[path = "helpers/world.rs"]
mod world_helper;
use world_helper::make_test_world;

#[path = "helpers/world_io.rs"]
mod world_io_helper;
use world_io_helper::save_and_load_roundtrip;

use engine_core::ecs::system::System;
use engine_core::ecs::world::World;
use engine_core::map::cell_key::CellKey;
use engine_core::map::{HexGridMap, Map, ProvinceMap, SquareGridMap};
use engine_core::systems::movement_system::MovementSystem;
use engine_core::systems::vehicle::VehicleSystem;
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;

/// Open square plane with 8-directional adjacency.
fn open_plane(size: i32) -> Map {
    let mut grid = SquareGridMap::new();
    for x in -size..=size {
        for y in -size..=size {
            grid.add_cell(x, y, 0);
        }
    }
    for x in -size..=size {
        for y in -size..=size {
            for dx in [-1, 0, 1] {
                for dy in [-1, 0, 1] {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx >= -size && nx <= size && ny >= -size && ny <= size {
                        grid.add_neighbor((x, y, 0), (nx, ny, 0));
                    }
                }
            }
        }
    }
    Map::new(Box::new(grid))
}

/// Straight-line square corridor along y=0 from x=0 to x=len.
fn corridor(len: i32) -> Map {
    let mut grid = SquareGridMap::new();
    for x in 0..=len {
        grid.add_cell(x, 0, 0);
    }
    for x in 0..len {
        grid.add_neighbor((x, 0, 0), (x + 1, 0, 0));
        grid.add_neighbor((x + 1, 0, 0), (x, 0, 0));
    }
    Map::new(Box::new(grid))
}

fn vehicle_world() -> World {
    let mut world = make_test_world();
    world.map = Some(open_plane(10));
    world
}

fn square_step(x: i32, y: i32) -> serde_json::Value {
    json!({"Square": {"x": x, "y": y, "z": 0}})
}

fn spawn_vehicle(
    world: &mut World,
    x: i32,
    y: i32,
    capacity: i64,
    speed: i64,
    blocked_terrains: Vec<&str>,
) -> u32 {
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
                "capacity": capacity,
                "speed": speed,
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

fn cell_of(world: &World, eid: u32) -> CellKey {
    CellKey::from_position(world.get_component(eid, "Position").unwrap()).unwrap()
}

fn occupants_of(world: &World, vehicle: u32) -> Vec<u32> {
    world.vehicle_occupants(vehicle)
}

fn drain(world: &mut World, event_type: &str) -> Vec<serde_json::Value> {
    world.update_event_buses::<serde_json::Value>();
    world.drain_events::<serde_json::Value>(event_type)
}

fn tick_once(rc: &Rc<RefCell<World>>) {
    World::tick(Rc::clone(rc));
}

fn ticking_world(world: World) -> Rc<RefCell<World>> {
    let mut world = world;
    world.register_system(MovementSystem);
    world.register_system(VehicleSystem);
    Rc::new(RefCell::new(world))
}

#[test]
fn test_vehicle_schema_round_trip_with_defaults() {
    let mut world = vehicle_world();
    let eid = world.spawn_entity();
    world
        .set_component(eid, "Vehicle", json!({"capacity": 2, "speed": 1}))
        .unwrap();
    let stored = world.get_component(eid, "Vehicle").unwrap().clone();
    assert_eq!(stored["capacity"], json!(2));
    assert_eq!(stored["speed"], json!(1));
    assert_eq!(stored["blocked_terrains"], json!([]));
    assert_eq!(stored["occupants"], json!([]));
}

#[test]
fn test_vehicle_schema_rejects_negative_capacity_and_speed() {
    let mut world = vehicle_world();
    let eid = world.spawn_entity();
    assert!(
        world
            .set_component(eid, "Vehicle", json!({"capacity": -1, "speed": 1}))
            .is_err()
    );
    assert!(
        world
            .set_component(eid, "Vehicle", json!({"capacity": 2, "speed": 0}))
            .is_err()
    );
}

#[test]
fn test_embark_records_occupancy_clears_rider_path_and_emits_event() {
    let mut world = vehicle_world();
    let vehicle = spawn_vehicle(&mut world, 0, 0, 2, 1, vec![]);
    let rider = spawn_rider(&mut world, 0, 0);
    world
        .set_component(
            rider,
            "Agent",
            json!({"entity_id": rider, "move_path": [square_step(1, 0)]}),
        )
        .unwrap();

    world.embark(vehicle, rider).unwrap();

    assert_eq!(occupants_of(&world, vehicle), vec![rider]);
    assert!(world.is_mounted(rider));
    let agent = world.get_component(rider, "Agent").unwrap();
    let cleared = agent.get("move_path").is_none_or(|p| p == &json!([]));
    assert!(cleared, "embark must clear the rider path, got {agent}");
    let events = drain(&mut world, "vehicle_embarked");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["vehicle"], json!(vehicle));
    assert_eq!(events[0]["rider"], json!(rider));
}

#[test]
fn test_embark_rejects_unknown_ids_and_double_mount() {
    let mut world = vehicle_world();
    let vehicle = spawn_vehicle(&mut world, 0, 0, 2, 1, vec![]);
    let rider = spawn_rider(&mut world, 0, 0);
    assert_eq!(
        world.embark(9999, rider).unwrap_err(),
        "no_vehicle".to_string()
    );
    assert_eq!(
        world.embark(vehicle, 9999).unwrap_err(),
        "no_rider".to_string()
    );
    // Vehicle entities cannot ride.
    let other = spawn_vehicle(&mut world, 1, 0, 2, 1, vec![]);
    assert_eq!(
        world.embark(vehicle, other).unwrap_err(),
        "no_rider".to_string()
    );
    world.embark(vehicle, rider).unwrap();
    assert_eq!(
        world.embark(other, rider).unwrap_err(),
        "already_mounted".to_string()
    );
    assert_eq!(occupants_of(&world, other), Vec::<u32>::new());
}

#[test]
fn test_disembark_places_rider_at_vehicle_cell_and_emits_event() {
    let mut world = vehicle_world();
    let vehicle = spawn_vehicle(&mut world, 0, 0, 2, 1, vec![]);
    let rider = spawn_rider(&mut world, 0, 0);
    world.embark(vehicle, rider).unwrap();
    // Move the vehicle away while mounted, then disembark.
    world
        .set_component(
            vehicle,
            "Vehicle",
            json!({
                "capacity": 2,
                "speed": 1,
                "blocked_terrains": [],
                "occupants": [rider],
                "move_path": [square_step(2, 0)],
            }),
        )
        .unwrap();
    VehicleSystem.run(&mut world);

    world.disembark(rider).unwrap();

    assert!(!world.is_mounted(rider));
    assert_eq!(occupants_of(&world, vehicle), Vec::<u32>::new());
    assert_eq!(cell_of(&world, rider), cell_of(&world, vehicle));
    let events = drain(&mut world, "vehicle_disembarked");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["vehicle"], json!(vehicle));
    assert_eq!(events[0]["rider"], json!(rider));
    // Disembarking again is a no-op rejection.
    assert_eq!(
        world.disembark(rider).unwrap_err(),
        "not_mounted".to_string()
    );
}

#[test]
fn test_mounted_comovement_advances_one_cell_per_tick_with_riders_colocated() {
    let mut world = vehicle_world();
    let vehicle = spawn_vehicle(&mut world, 0, 0, 2, 1, vec![]);
    let rider_a = spawn_rider(&mut world, 0, 0);
    let rider_b = spawn_rider(&mut world, 0, 0);
    world.embark(vehicle, rider_a).unwrap();
    world.embark(vehicle, rider_b).unwrap();
    world
        .set_component(
            vehicle,
            "Vehicle",
            json!({
                "capacity": 2,
                "speed": 1,
                "blocked_terrains": [],
                "occupants": [rider_a, rider_b],
                "move_path": [square_step(1, 0), square_step(2, 0), square_step(3, 0)],
            }),
        )
        .unwrap();

    let rc = ticking_world(world);
    for (tick, x) in [(1, 1), (2, 2), (3, 3)] {
        tick_once(&rc);
        let world = rc.borrow();
        let vehicle_cell = CellKey::Square { x, y: 0, z: 0 };
        assert_eq!(cell_of(&world, vehicle), vehicle_cell, "tick {tick}");
        assert_eq!(cell_of(&world, rider_a), vehicle_cell, "tick {tick}");
        assert_eq!(cell_of(&world, rider_b), vehicle_cell, "tick {tick}");
        for eid in [vehicle, rider_a, rider_b] {
            assert!(
                world.entities_in_cell(&vehicle_cell).contains(&eid),
                "tick {tick}: entity {eid} must be in the vehicle cell"
            );
        }
    }
}

#[test]
fn test_mounted_comovement_at_speed_two_advances_two_cells_per_tick() {
    let mut world = vehicle_world();
    let vehicle = spawn_vehicle(&mut world, 0, 0, 2, 2, vec![]);
    let rider = spawn_rider(&mut world, 0, 0);
    world.embark(vehicle, rider).unwrap();
    world
        .set_component(
            vehicle,
            "Vehicle",
            json!({
                "capacity": 2,
                "speed": 2,
                "blocked_terrains": [],
                "occupants": [rider],
                "move_path": [square_step(1, 0), square_step(2, 0)],
            }),
        )
        .unwrap();

    let rc = ticking_world(world);
    tick_once(&rc);
    let world = rc.borrow();

    assert_eq!(
        cell_of(&world, vehicle),
        CellKey::Square { x: 2, y: 0, z: 0 }
    );
    assert_eq!(cell_of(&world, rider), cell_of(&world, vehicle));
}

/// Hex cluster: center plus the six axial neighbors, fully linked.
fn open_hex_cluster() -> Map {
    let cells = [
        (0, 0, 0),
        (1, 0, 0),
        (1, -1, 0),
        (0, -1, 0),
        (-1, 0, 0),
        (-1, 1, 0),
        (0, 1, 0),
    ];
    let mut grid = HexGridMap::new();
    for (q, r, z) in cells {
        grid.add_cell(q, r, z);
    }
    for (q, r, z) in cells {
        for (dq, dr) in [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)] {
            let neighbor = (q + dq, r + dr, z);
            if cells.contains(&neighbor) {
                grid.add_neighbor((q, r, z), neighbor);
            }
        }
    }
    Map::new(Box::new(grid))
}

#[test]
fn test_hex_vehicle_comove_preserves_hex_variant() {
    let mut world = make_test_world();
    world.map = Some(open_hex_cluster());
    let vehicle = world.spawn_entity();
    world
        .set_component(
            vehicle,
            "Position",
            json!({"pos": {"Hex": {"q": 0, "r": 0, "z": 0}}}),
        )
        .unwrap();
    world
        .set_component(
            vehicle,
            "Vehicle",
            json!({
                "capacity": 2,
                "speed": 1,
                "blocked_terrains": [],
                "occupants": [],
                "move_path": [{"Hex": {"q": 1, "r": 0, "z": 0}}],
            }),
        )
        .unwrap();
    let rider = world.spawn_entity();
    world
        .set_component(
            rider,
            "Position",
            json!({"pos": {"Hex": {"q": 0, "r": 0, "z": 0}}}),
        )
        .unwrap();
    world.embark(vehicle, rider).unwrap();

    VehicleSystem.run(&mut world);

    let expected = CellKey::Hex { q: 1, r: 0, z: 0 };
    assert_eq!(cell_of(&world, vehicle), expected);
    assert_eq!(cell_of(&world, rider), expected);
    for eid in [vehicle, rider] {
        let pos = world.get_component(eid, "Position").unwrap();
        assert!(
            pos.get("pos").and_then(|p| p.get("Hex")).is_some(),
            "entity {eid} Position must stay Hex-variant, got {pos}"
        );
    }
}

#[test]
fn test_capacity_reject_leaves_state_unchanged_and_emits_rejection() {
    let mut world = vehicle_world();
    let vehicle = spawn_vehicle(&mut world, 0, 0, 1, 1, vec![]);
    let rider_a = spawn_rider(&mut world, 0, 0);
    let rider_b = spawn_rider(&mut world, 0, 0);
    world.embark(vehicle, rider_a).unwrap();

    assert_eq!(
        world.embark(vehicle, rider_b).unwrap_err(),
        "full".to_string()
    );

    assert_eq!(occupants_of(&world, vehicle), vec![rider_a]);
    assert!(!world.is_mounted(rider_b));
    let events = drain(&mut world, "vehicle_embark_rejected");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["reason"], json!("full"));
    assert_eq!(events[0]["vehicle"], json!(vehicle));
    assert_eq!(events[0]["rider"], json!(rider_b));
}

#[test]
fn test_unwalkable_step_truncates_path_holds_vehicle_and_emits_blocked() {
    let mut world = make_test_world();
    world.map = Some(corridor(4));
    let vehicle = spawn_vehicle(&mut world, 0, 0, 2, 1, vec![]);
    let rider = spawn_rider(&mut world, 0, 0);
    world.embark(vehicle, rider).unwrap();
    world.set_cell_metadata(
        &CellKey::Square { x: 1, y: 0, z: 0 },
        json!({"walkable": false}),
    );
    world
        .set_component(
            vehicle,
            "Vehicle",
            json!({
                "capacity": 2,
                "speed": 1,
                "blocked_terrains": [],
                "occupants": [rider],
                "move_path": [square_step(1, 0), square_step(2, 0)],
            }),
        )
        .unwrap();

    VehicleSystem.run(&mut world);

    assert_eq!(
        cell_of(&world, vehicle),
        CellKey::Square { x: 0, y: 0, z: 0 }
    );
    assert_eq!(cell_of(&world, rider), cell_of(&world, vehicle));
    let vehicle_json = world.get_component(vehicle, "Vehicle").unwrap().clone();
    let truncated = vehicle_json
        .get("move_path")
        .is_none_or(|p| p == &json!([]));
    assert!(
        truncated,
        "blocked step must truncate the remaining path, got {vehicle_json}"
    );
    let events = drain(&mut world, "vehicle_move_blocked");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["vehicle"], json!(vehicle));
}

#[test]
fn test_blocked_terrain_step_is_rejected_by_name() {
    let mut world = make_test_world();
    world.map = Some(corridor(4));
    let vehicle = spawn_vehicle(&mut world, 0, 0, 2, 1, vec!["water"]);
    let rider = spawn_rider(&mut world, 0, 0);
    world.embark(vehicle, rider).unwrap();
    world.set_cell_metadata(
        &CellKey::Square { x: 1, y: 0, z: 0 },
        json!({"terrain": "water"}),
    );
    world
        .set_component(
            vehicle,
            "Vehicle",
            json!({
                "capacity": 2,
                "speed": 1,
                "blocked_terrains": ["water"],
                "occupants": [rider],
                "move_path": [square_step(1, 0), square_step(2, 0)],
            }),
        )
        .unwrap();

    VehicleSystem.run(&mut world);

    assert_eq!(
        cell_of(&world, vehicle),
        CellKey::Square { x: 0, y: 0, z: 0 }
    );
    assert_eq!(drain(&mut world, "vehicle_move_blocked").len(), 1);
}

#[test]
fn test_assign_path_stores_prefix_before_blocked_cell() {
    let mut world = make_test_world();
    world.map = Some(corridor(4));
    // Terrain-named blocks are invisible to A* (only the vehicle guard sees
    // them), so find_path routes through and the stored path truncates.
    let vehicle = spawn_vehicle(&mut world, 0, 0, 2, 1, vec!["water"]);
    world.set_cell_metadata(
        &CellKey::Square { x: 2, y: 0, z: 0 },
        json!({"terrain": "water"}),
    );

    let steps = world
        .assign_vehicle_path(vehicle, &CellKey::Square { x: 3, y: 0, z: 0 })
        .unwrap();

    assert_eq!(steps, 1);
    let stored = world.get_component(vehicle, "Vehicle").unwrap().clone();
    assert_eq!(stored["move_path"], json!([square_step(1, 0)]));
    assert_eq!(drain(&mut world, "vehicle_move_blocked").len(), 1);
}

#[test]
fn test_assign_path_fully_blocked_stores_empty_and_unreachable_keeps_path() {
    let mut world = make_test_world();
    world.map = Some(corridor(4));
    let vehicle = spawn_vehicle(&mut world, 0, 0, 2, 1, vec!["water"]);
    world.set_cell_metadata(
        &CellKey::Square { x: 1, y: 0, z: 0 },
        json!({"terrain": "water"}),
    );
    let steps = world
        .assign_vehicle_path(vehicle, &CellKey::Square { x: 3, y: 0, z: 0 })
        .unwrap();
    assert_eq!(steps, 0);
    let stored = world.get_component(vehicle, "Vehicle").unwrap().clone();
    assert!(
        stored.get("move_path").is_none_or(|p| p == &json!([])),
        "fully-blocked path must store empty, got {stored}"
    );

    // Seed a stored path, then assign an unreachable goal: the path is kept.
    world
        .set_component(
            vehicle,
            "Vehicle",
            json!({
                "capacity": 2,
                "speed": 1,
                "blocked_terrains": [],
                "occupants": [],
                "move_path": [square_step(3, 0)],
            }),
        )
        .unwrap();
    let mut island = SquareGridMap::new();
    island.add_cell(0, 0, 0);
    world.map = Some(Map::new(Box::new(island)));
    // Vehicle at (0,0) can no longer path anywhere off its island... the map
    // holds only its own cell, so any distant goal is unreachable.
    assert_eq!(
        world
            .assign_vehicle_path(vehicle, &CellKey::Square { x: 9, y: 9, z: 0 })
            .unwrap_err(),
        "no_path".to_string()
    );
    let kept = world.get_component(vehicle, "Vehicle").unwrap().clone();
    assert_eq!(kept["move_path"], json!([square_step(3, 0)]));
}

#[test]
fn test_mounted_rider_path_has_no_visible_effect_while_mounted() {
    let mut world = vehicle_world();
    let vehicle = spawn_vehicle(&mut world, 0, 0, 2, 1, vec![]);
    let rider = spawn_rider(&mut world, 0, 0);
    world.embark(vehicle, rider).unwrap();
    world
        .set_component(
            vehicle,
            "Vehicle",
            json!({
                "capacity": 2,
                "speed": 1,
                "blocked_terrains": [],
                "occupants": [rider],
                "move_path": [square_step(1, 0), square_step(2, 0), square_step(3, 0)],
            }),
        )
        .unwrap();

    let rc = ticking_world(world);
    for _ in 0..3 {
        // A rider path re-added mid-ride must not move the rider independently.
        rc.borrow_mut()
            .set_component(
                rider,
                "Agent",
                json!({"entity_id": rider, "move_path": [square_step(0, 5)]}),
            )
            .unwrap();
        tick_once(&rc);
        let world = rc.borrow();
        assert_eq!(
            cell_of(&world, rider),
            cell_of(&world, vehicle),
            "rider must track the vehicle while mounted"
        );
    }
}

#[test]
fn test_despawned_vehicle_leaves_rider_alive_at_last_cell() {
    let mut world = vehicle_world();
    let vehicle = spawn_vehicle(&mut world, 0, 0, 2, 1, vec![]);
    let rider = spawn_rider(&mut world, 0, 0);
    world.embark(vehicle, rider).unwrap();
    let vehicle_cell = cell_of(&world, vehicle);

    world.despawn_entity(vehicle);
    VehicleSystem.run(&mut world);

    assert!(world.entity_exists(rider));
    assert_eq!(cell_of(&world, rider), vehicle_cell);
    assert!(!world.is_mounted(rider));
}

#[test]
fn test_province_vehicle_holds_position_with_riders_synced() {
    let mut world = make_test_world();
    let mut grid = ProvinceMap::new();
    grid.add_cell("prov_a");
    grid.add_cell("prov_b");
    grid.add_neighbor("prov_a", "prov_b");
    world.map = Some(Map::new(Box::new(grid)));
    let vehicle = world.spawn_entity();
    world
        .set_component(
            vehicle,
            "Position",
            json!({"pos": {"Province": {"id": "prov_a"}}}),
        )
        .unwrap();
    world
        .set_component(
            vehicle,
            "Vehicle",
            json!({
                "capacity": 2,
                "speed": 1,
                "blocked_terrains": [],
                "occupants": [],
                "move_path": [{"Province": {"id": "prov_b"}}],
            }),
        )
        .unwrap();
    let rider = world.spawn_entity();
    world
        .set_component(
            rider,
            "Position",
            json!({"pos": {"Province": {"id": "prov_a"}}}),
        )
        .unwrap();
    world.embark(vehicle, rider).unwrap();

    VehicleSystem.run(&mut world);

    let expected = CellKey::Province {
        id: "prov_a".to_string(),
    };
    assert_eq!(cell_of(&world, vehicle), expected);
    assert_eq!(cell_of(&world, rider), expected);
}

fn deterministic_vehicle_world() -> World {
    let mut world = make_test_world();
    world.map = Some(open_plane(10));
    let vehicle = spawn_vehicle(&mut world, -5, 0, 2, 1, vec![]);
    let rider_a = spawn_rider(&mut world, -5, 0);
    let rider_b = spawn_rider(&mut world, -5, 0);
    world.embark(vehicle, rider_a).unwrap();
    world.embark(vehicle, rider_b).unwrap();
    world
        .assign_vehicle_path(vehicle, &CellKey::Square { x: 5, y: 0, z: 0 })
        .unwrap();
    world.register_system(MovementSystem);
    world.register_system(VehicleSystem);
    world
}

type VehicleSnapshot = (Vec<(u32, CellKey, Vec<u32>)>, Vec<String>);

fn snapshot_vehicle(world: &mut World) -> VehicleSnapshot {
    let mut vehicles = world.get_entities_with_component("Vehicle");
    vehicles.sort_unstable();
    let mut positions: Vec<(u32, CellKey, Vec<u32>)> = Vec::new();
    let mut ids = world.get_entities();
    ids.sort_unstable();
    for eid in ids {
        if let Some(pos) = world.get_component(eid, "Position") {
            let cell = CellKey::from_position(pos).unwrap();
            positions.push((eid, cell, world.vehicle_occupants(eid)));
        }
    }
    let events = world
        .drain_events::<serde_json::Value>("vehicle_move_blocked")
        .into_iter()
        .map(|e| e.to_string())
        .collect();
    (positions, events)
}

#[test]
fn test_fifty_tick_two_world_mounted_determinism() {
    let world_a = Rc::new(RefCell::new(deterministic_vehicle_world()));
    let world_b = Rc::new(RefCell::new(deterministic_vehicle_world()));

    for tick in 0..50 {
        World::tick(Rc::clone(&world_a));
        World::tick(Rc::clone(&world_b));
        let (a_pos, a_events) = snapshot_vehicle(&mut world_a.borrow_mut());
        let (b_pos, b_events) = snapshot_vehicle(&mut world_b.borrow_mut());
        assert_eq!(a_pos, b_pos, "positions/occupants diverged on tick {tick}");
        assert_eq!(a_events, b_events, "events diverged on tick {tick}");
        assert_eq!(
            world_a.borrow().turn,
            world_b.borrow().turn,
            "turn diverged on tick {tick}"
        );
    }
}

#[test]
fn test_save_load_roundtrip_preserves_mounted_vehicle() {
    let mut world = make_test_world();
    world.map = Some(open_plane(10));
    let vehicle = spawn_vehicle(&mut world, 0, 0, 2, 1, vec![]);
    let rider_a = spawn_rider(&mut world, 0, 0);
    let rider_b = spawn_rider(&mut world, 0, 0);
    world.embark(vehicle, rider_a).unwrap();
    world.embark(vehicle, rider_b).unwrap();
    world
        .assign_vehicle_path(vehicle, &CellKey::Square { x: 4, y: 0, z: 0 })
        .unwrap();
    world.register_system(MovementSystem);
    world.register_system(VehicleSystem);
    let world_rc = Rc::new(RefCell::new(world));
    World::tick(Rc::clone(&world_rc));
    World::tick(Rc::clone(&world_rc));
    let live = world_rc.borrow();
    let live_vehicle = live.get_component(vehicle, "Vehicle").unwrap().clone();
    let live_positions: Vec<CellKey> = [vehicle, rider_a, rider_b]
        .iter()
        .map(|eid| CellKey::from_position(live.get_component(*eid, "Position").unwrap()).unwrap())
        .collect();

    let registry = live.registry.clone();
    let loaded = save_and_load_roundtrip(&live, registry);

    assert_eq!(
        loaded.get_component(vehicle, "Vehicle").unwrap(),
        &live_vehicle,
        "Vehicle component must survive round-trip"
    );
    assert_eq!(loaded.vehicle_occupants(vehicle), vec![rider_a, rider_b]);
    for (eid, expected) in [vehicle, rider_a, rider_b]
        .iter()
        .zip(live_positions.iter())
    {
        let cell = CellKey::from_position(loaded.get_component(*eid, "Position").unwrap()).unwrap();
        assert_eq!(&cell, expected, "position mismatch for entity {eid}");
    }
    assert!(loaded.is_mounted(rider_a));
    assert!(loaded.is_mounted(rider_b));
}
