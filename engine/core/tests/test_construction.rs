#[path = "helpers/agent.rs"]
mod agent_helper;
#[path = "helpers/world.rs"]
mod world_helper;

use agent_helper::AgentTestHelpers;
use engine_core::ecs::world::World;
use engine_core::map::{CellKey, HexGridMap, Map, ProvinceMap, SquareGridMap};
use engine_core::systems::construction::{
    ConstructionSystem, cancel_construction, demolish_building, get_construction_state,
    place_blueprint,
};
use engine_core::systems::job::JobSystem;
use engine_core::systems::job::job_board::{JobAssignmentResult, JobBoard};
use engine_core::systems::job::resource_reservation::ResourceReservationSystem;
use engine_core::systems::movement_system::MovementSystem;
use serde_json::{Value as JsonValue, json};
use world_helper::make_test_world;

fn square_world(cells: &[(i32, i32, i32)]) -> World {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = make_test_world();
    world.set_mode("colony");
    let mut grid = SquareGridMap::new();
    for (x, y, z) in cells {
        grid.add_cell(*x, *y, *z);
    }
    for window in cells.windows(2) {
        grid.add_neighbor(
            (window[0].0, window[0].1, window[0].2),
            (window[1].0, window[1].1, window[1].2),
        );
    }
    world.map = Some(Map::new(Box::new(grid)));
    world
}

fn spawn_worker(world: &mut World, x: i32, y: i32, z: i32) -> u32 {
    let agent = world.spawn_idle_agent();
    world
        .set_component(
            agent,
            "Position",
            json!({ "pos": { "Square": { "x": x, "y": y, "z": z } } }),
        )
        .unwrap();
    world
        .set_component(
            agent,
            "Inventory",
            json!({
                "max_weight": 100.0,
                "max_slots": 10,
                "max_volume": 100.0,
                "weight": 0.0,
                "slots": [],
                "volume": 0.0
            }),
        )
        .unwrap();
    agent
}

fn spawn_stockpile(world: &mut World, x: i32, y: i32, z: i32, wood: i64) -> u32 {
    let stockpile = world.spawn_entity();
    world
        .set_component(
            stockpile,
            "Stockpile",
            json!({ "resources": { "wood": wood } }),
        )
        .unwrap();
    world
        .set_component(
            stockpile,
            "Position",
            json!({ "pos": { "Square": { "x": x, "y": y, "z": z } } }),
        )
        .unwrap();
    stockpile
}

fn register_construction_loop(world: &mut World) {
    world.register_system(ResourceReservationSystem::new());
    world.register_system(JobSystem::new());
    world.register_system(MovementSystem);
    world.register_system(ConstructionSystem::new());
}

fn stockpile_wood(world: &World, stockpile: u32) -> i64 {
    world.get_component(stockpile, "Stockpile").unwrap()["resources"]["wood"]
        .as_i64()
        .unwrap()
}

/// Standard colony scenario: 3-cell row, worker at 0, stockpile at 1,
/// blueprint for 2 wood and 2 work at cell 2. Returns (agent, stockpile, site, job).
fn standard_scenario(world: &mut World) -> (u32, u32, u32, u32) {
    let agent = spawn_worker(world, 0, 0, 0);
    let stockpile = spawn_stockpile(world, 1, 0, 0, 2);
    let site = place_blueprint(
        world,
        "hut",
        &CellKey::Square { x: 2, y: 0, z: 0 },
        &[("wood".to_string(), 2)],
        2,
    )
    .expect("blueprint places on a free in-bounds cell");
    let job = world.get_component(site, "ConstructionSite").unwrap()["assigned_job"]
        .as_u64()
        .expect("site links its job") as u32;
    (agent, stockpile, site, job)
}

fn reserve_and_claim(world: &mut World, agent: u32, job: u32) {
    world.run_system("ResourceReservationSystem").unwrap();
    let mut board = JobBoard::default();
    board.update(world, 0, &[]);
    match board.claim_job(agent, world, 0) {
        JobAssignmentResult::Assigned(claimed) => assert_eq!(claimed, job),
        other => panic!("board claim failed: {other:?}"),
    }
    world.run_system("JobSystem").unwrap();
}

/// Ticks Movement + Job + Construction until the site carries Building.
/// Returns the tick count used.
fn drive_to_completion(world: &mut World, site: u32, max_ticks: u32) -> u32 {
    for tick in 1..=max_ticks {
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();
        world.run_system("ConstructionSystem").unwrap();
        if world.has_component(site, "Building") {
            return tick;
        }
    }
    panic!("site {site} did not complete within {max_ticks} ticks");
}

fn job_state(world: &World, job: u32) -> String {
    world.get_component(job, "Job").unwrap()["state"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn test_rejects_blueprint_on_out_of_bounds_cell() {
    let mut world = square_world(&[(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
    let before = world.get_entities().len();

    let result = place_blueprint(
        &mut world,
        "hut",
        &CellKey::Square { x: 9, y: 9, z: 0 },
        &[("wood".to_string(), 1)],
        1,
    );

    assert!(result.is_err());
    assert_eq!(world.get_entities().len(), before);
}

#[test]
fn test_rejects_duplicate_on_occupied_cell_but_accepts_different_z() {
    let mut world = square_world(&[(2, 0, 0), (2, 0, 1)]);

    let first = place_blueprint(
        &mut world,
        "hut",
        &CellKey::Square { x: 2, y: 0, z: 0 },
        &[("wood".to_string(), 1)],
        1,
    );
    assert!(first.is_ok());

    let duplicate = place_blueprint(
        &mut world,
        "hut",
        &CellKey::Square { x: 2, y: 0, z: 0 },
        &[("wood".to_string(), 1)],
        1,
    );
    assert!(duplicate.is_err());

    let other_level = place_blueprint(
        &mut world,
        "hut",
        &CellKey::Square { x: 2, y: 0, z: 1 },
        &[("wood".to_string(), 1)],
        1,
    );
    assert!(other_level.is_ok());
}

#[test]
fn test_rejects_topology_mismatch() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = make_test_world();
    world.set_mode("colony");
    let mut grid = HexGridMap::new();
    grid.add_cell(0, 0, 0);
    world.map = Some(Map::new(Box::new(grid)));

    let result = place_blueprint(
        &mut world,
        "hut",
        &CellKey::Square { x: 0, y: 0, z: 0 },
        &[("wood".to_string(), 1)],
        1,
    );
    assert!(result.is_err());
}

#[test]
fn test_rejects_invalid_materials_and_work() {
    let mut world = square_world(&[(2, 0, 0)]);

    assert!(
        place_blueprint(
            &mut world,
            "hut",
            &CellKey::Square { x: 2, y: 0, z: 0 },
            &[],
            1
        )
        .is_err()
    );
    assert!(
        place_blueprint(
            &mut world,
            "hut",
            &CellKey::Square { x: 2, y: 0, z: 0 },
            &[("wood".to_string(), 0)],
            1,
        )
        .is_err()
    );
    assert!(
        place_blueprint(
            &mut world,
            "hut",
            &CellKey::Square { x: 2, y: 0, z: 0 },
            &[("wood".to_string(), 1)],
            0,
        )
        .is_err()
    );
    assert!(world.get_entities().is_empty());
}

#[test]
fn test_rejects_blueprint_outside_colony_mode() {
    let mut world = square_world(&[(2, 0, 0)]);
    world.set_mode("roguelike");

    let placed = place_blueprint(
        &mut world,
        "hut",
        &CellKey::Square { x: 2, y: 0, z: 0 },
        &[("wood".to_string(), 1)],
        1,
    );
    assert!(placed.is_err());
    assert!(get_construction_state(&world, 0).is_err());
}

#[test]
fn test_successful_placement_spawns_ghost_and_linked_job() {
    let mut world = square_world(&[(2, 0, 0)]);

    let site = place_blueprint(
        &mut world,
        "hut",
        &CellKey::Square { x: 2, y: 0, z: 0 },
        &[("wood".to_string(), 2)],
        3,
    )
    .unwrap();

    let position = world
        .get_component(site, "Position")
        .expect("ghost has Position");
    assert_eq!(position["pos"]["Square"]["x"], 2);
    let ghost = world
        .get_component(site, "ConstructionSite")
        .expect("ghost has ConstructionSite");
    assert_eq!(ghost["building_type"], "hut");
    assert_eq!(ghost["state"], "pending");
    assert_eq!(ghost["progress"], 0);
    assert_eq!(ghost["required_work"], 3);

    let job_id = ghost["assigned_job"].as_u64().expect("job link") as u32;
    let job = world
        .get_component(job_id, "Job")
        .expect("linked job posted");
    assert_eq!(job["category"], "construction");
    assert_eq!(job["job_type"], "construct");
    assert_eq!(job["state"], "pending");

    let state = get_construction_state(&world, site).unwrap();
    assert_eq!(state["state"], "pending");
    assert_eq!(state["building_type"], "hut");
    assert_eq!(state["required_work"], 3);
}

#[test]
fn test_construction_schemas_allowed_in_colony_mode() {
    let world = square_world(&[(0, 0, 0)]);
    assert!(world.is_component_allowed_in_mode("ConstructionSite", &world.current_mode));
    assert!(world.is_component_allowed_in_mode("Building", &world.current_mode));
}

#[test]
fn test_reservation_flips_job_to_fetching_with_integer_amounts() {
    let mut world = square_world(&[(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
    register_construction_loop(&mut world);
    let (agent, stockpile, _site, job) = standard_scenario(&mut world);

    reserve_and_claim(&mut world, agent, job);

    assert_eq!(job_state(&world, job), "fetching_resources");
    let reserved = world.get_component(job, "Job").unwrap()["reserved_resources"].clone();
    assert_eq!(reserved, json!([{ "kind": "wood", "amount": 2 }]));
    assert_eq!(
        world.get_component(job, "Job").unwrap()["reserved_stockpile"].as_u64(),
        Some(stockpile as u64)
    );
}

#[test]
fn test_linked_job_traverses_known_delivery_states_only() {
    let mut world = square_world(&[(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
    register_construction_loop(&mut world);
    let (agent, _stockpile, site, job) = standard_scenario(&mut world);
    reserve_and_claim(&mut world, agent, job);

    let known = [
        "pending",
        "fetching_resources",
        "delivering_resources",
        "going_to_site",
        "at_site",
        "in_progress",
        "complete",
    ];
    let mut seen = vec![job_state(&world, job)];
    for _ in 0..60 {
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();
        world.run_system("ConstructionSystem").unwrap();
        seen.push(job_state(&world, job));
        if world.has_component(site, "Building") {
            break;
        }
    }

    for state in &seen {
        assert!(
            known.contains(&state.as_str()),
            "unexpected job state {state}"
        );
    }
    assert!(seen.contains(&"fetching_resources".to_string()));
    assert!(seen.contains(&"in_progress".to_string()));
    assert!(world.has_component(site, "Building"));
}

#[test]
fn test_stockpile_decrements_once_on_delivery_and_holds_across_work_ticks() {
    let mut world = square_world(&[(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
    register_construction_loop(&mut world);
    let (agent, stockpile, site, job) = standard_scenario(&mut world);
    // Extra work keeps the site in progress across several construction ticks.
    world
        .set_component(
            site,
            "ConstructionSite",
            json!({
                "building_type": "hut",
                "target_position": { "pos": { "Square": { "x": 2, "y": 0, "z": 0 } } },
                "required_materials": [{ "kind": "wood", "amount": 2 }],
                "progress": 0,
                "required_work": 5,
                "state": "pending",
                "reserved_stockpile": null,
                "assigned_job": job,
            }),
        )
        .unwrap();
    assert_eq!(stockpile_wood(&world, stockpile), 2);

    reserve_and_claim(&mut world, agent, job);
    for _ in 0..60 {
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();
        world.run_system("ConstructionSystem").unwrap();
        if get_construction_state(&world, site).unwrap()["state"] == "in_progress" {
            break;
        }
    }

    assert_eq!(stockpile_wood(&world, stockpile), 0);
    for _ in 0..3 {
        world.run_system("ConstructionSystem").unwrap();
        assert_eq!(stockpile_wood(&world, stockpile), 0);
    }
}

#[test]
fn test_progress_advances_one_per_tick_until_required_work() {
    let mut world = square_world(&[(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
    register_construction_loop(&mut world);
    let (agent, _stockpile, site, job) = standard_scenario(&mut world);
    reserve_and_claim(&mut world, agent, job);

    for _ in 0..60 {
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();
        world.run_system("ConstructionSystem").unwrap();
        if get_construction_state(&world, site).unwrap()["state"] == "in_progress" {
            break;
        }
    }

    let mut previous = get_construction_state(&world, site).unwrap()["progress"]
        .as_i64()
        .unwrap();
    while !world.has_component(site, "Building") {
        world.run_system("ConstructionSystem").unwrap();
        if world.has_component(site, "Building") {
            break;
        }
        let current = get_construction_state(&world, site).unwrap()["progress"]
            .as_i64()
            .unwrap();
        assert_eq!(current, previous + 1);
        previous = current;
    }
    assert!(world.has_component(site, "Building"));
}

#[test]
fn test_completion_emits_event_and_replaces_site_with_building() {
    let mut world = square_world(&[(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
    register_construction_loop(&mut world);
    let (agent, _stockpile, site, job) = standard_scenario(&mut world);
    reserve_and_claim(&mut world, agent, job);
    drive_to_completion(&mut world, site, 80);

    world.update_event_buses::<JsonValue>();
    let events = world.take_events("construction_completed");
    assert!(!events.is_empty());
    let event = events
        .iter()
        .find(|e| e["site_id"] == site)
        .expect("site event");
    assert_eq!(event["building_id"], site);
    assert_eq!(event["building_type"], "hut");
    assert_eq!(event["position"]["pos"]["Square"]["x"], 2);

    assert!(world.get_component(site, "ConstructionSite").is_none());
    let building = world
        .get_component(site, "Building")
        .expect("Building remains");
    assert_eq!(building["building_type"], "hut");
    assert_eq!(
        building["materials_used"],
        json!([{ "kind": "wood", "amount": 2 }])
    );
    let position = world
        .get_component(site, "Position")
        .expect("position kept");
    assert_eq!(position["pos"]["Square"]["x"], 2);

    let state = get_construction_state(&world, site).unwrap();
    assert_eq!(state["state"], "complete");
}

#[test]
fn test_completion_tick_is_deterministic() {
    fn completion_ticks() -> u32 {
        let mut world = square_world(&[(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
        register_construction_loop(&mut world);
        let (agent, _stockpile, site, job) = standard_scenario(&mut world);
        reserve_and_claim(&mut world, agent, job);
        drive_to_completion(&mut world, site, 80)
    }

    assert_eq!(completion_ticks(), completion_ticks());
}

#[test]
fn test_cancel_before_delivery_clears_reservation_and_removes_ghost() {
    let mut world = square_world(&[(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
    register_construction_loop(&mut world);
    let (agent, _stockpile, site, job) = standard_scenario(&mut world);
    reserve_and_claim(&mut world, agent, job);

    assert!(cancel_construction(&mut world, site).unwrap());
    assert!(world.get_component(site, "ConstructionSite").is_none());
    assert!(!world.get_entities().contains(&site));

    let cancelled = world.get_component(job, "Job").unwrap().clone();
    assert_eq!(cancelled["state"], "cancelled");
    assert_eq!(cancelled["reserved_resources"], json!([]));
    assert!(cancelled["reserved_stockpile"].is_null());
}

#[test]
fn test_cancel_after_delivery_refunds_stockpile_and_cancels_job() {
    let mut world = square_world(&[(0, 0, 0), (1, 0, 0), (2, 0, 0)]);
    register_construction_loop(&mut world);
    let (agent, stockpile, site, job) = standard_scenario(&mut world);
    reserve_and_claim(&mut world, agent, job);
    for _ in 0..60 {
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();
        world.run_system("ConstructionSystem").unwrap();
        if get_construction_state(&world, site).unwrap()["state"] == "in_progress" {
            break;
        }
    }
    assert_eq!(stockpile_wood(&world, stockpile), 0);

    assert!(cancel_construction(&mut world, site).unwrap());
    assert_eq!(stockpile_wood(&world, stockpile), 2);
    assert_eq!(job_state(&world, job), "cancelled");
    assert!(world.get_component(site, "ConstructionSite").is_none());
}

#[test]
fn test_cancel_after_completion_rejected_demolish_removes_building_without_refund() {
    let mut world = square_world(&[(0, 0, 0), (1, 0, 0), (2, 0, 0), (3, 0, 0)]);
    register_construction_loop(&mut world);
    let (agent, stockpile, site, job) = standard_scenario(&mut world);
    reserve_and_claim(&mut world, agent, job);
    drive_to_completion(&mut world, site, 80);
    assert_eq!(stockpile_wood(&world, stockpile), 0);

    assert!(cancel_construction(&mut world, site).is_err());

    let other = place_blueprint(
        &mut world,
        "shed",
        &CellKey::Square { x: 3, y: 0, z: 0 },
        &[("wood".to_string(), 1)],
        1,
    )
    .unwrap();
    assert!(demolish_building(&mut world, other).is_err());
    assert!(demolish_building(&mut world, 99999).is_err());

    assert!(demolish_building(&mut world, site).unwrap());
    assert!(!world.get_entities().contains(&site));
    assert_eq!(stockpile_wood(&world, stockpile), 0);
    let _ = job;
}

#[test]
fn test_ordering_slot_sits_immediately_after_economic() {
    let order = engine_core::systems::SYSTEM_EXECUTION_ORDER;
    let economic = order
        .iter()
        .position(|name| *name == "EconomicSystem")
        .unwrap();
    let construction = order
        .iter()
        .position(|name| *name == "ConstructionSystem")
        .unwrap();
    let reputation = order
        .iter()
        .position(|name| *name == "FactionReputationSystem")
        .unwrap();
    assert_eq!(construction, economic + 1);
    assert!(construction < reputation);
}

#[test]
fn test_multi_job_world_completes_within_bounded_ticks() {
    let mut world = square_world(&[(0, 0, 0), (1, 0, 0), (2, 0, 0), (3, 0, 0)]);
    register_construction_loop(&mut world);
    let first_agent = spawn_worker(&mut world, 0, 0, 0);
    let second_agent = spawn_worker(&mut world, 0, 0, 0);
    let stockpile = spawn_stockpile(&mut world, 1, 0, 0, 4);
    let first_site = place_blueprint(
        &mut world,
        "hut",
        &CellKey::Square { x: 2, y: 0, z: 0 },
        &[("wood".to_string(), 2)],
        2,
    )
    .unwrap();
    let second_site = place_blueprint(
        &mut world,
        "shed",
        &CellKey::Square { x: 3, y: 0, z: 0 },
        &[("wood".to_string(), 2)],
        2,
    )
    .unwrap();

    world.run_system("ResourceReservationSystem").unwrap();
    let mut board = JobBoard::default();
    board.update(&world, 0, &[]);
    assert!(matches!(
        board.claim_job(first_agent, &mut world, 0),
        JobAssignmentResult::Assigned(_)
    ));
    board.update(&world, 0, &[]);
    assert!(matches!(
        board.claim_job(second_agent, &mut world, 0),
        JobAssignmentResult::Assigned(_)
    ));
    world.run_system("JobSystem").unwrap();

    for _ in 0..120 {
        world.run_system("MovementSystem").unwrap();
        world.run_system("JobSystem").unwrap();
        world.run_system("ConstructionSystem").unwrap();
        if world.has_component(first_site, "Building")
            && world.has_component(second_site, "Building")
        {
            break;
        }
    }
    assert!(world.has_component(first_site, "Building"));
    assert!(world.has_component(second_site, "Building"));
    assert_eq!(stockpile_wood(&world, stockpile), 0);
}

#[test]
fn test_per_topology_placement_blocks_duplicates() {
    engine_core::systems::job::system::events::init_job_event_logger();
    let mut world = make_test_world();
    world.set_mode("colony");
    let mut hex = HexGridMap::new();
    hex.add_cell(0, 0, 0);
    hex.add_cell(1, 0, 0);
    world.map = Some(Map::new(Box::new(hex)));

    let hex_site = place_blueprint(
        &mut world,
        "hut",
        &CellKey::Hex { q: 0, r: 0, z: 0 },
        &[("wood".to_string(), 1)],
        1,
    );
    assert!(hex_site.is_ok());
    assert!(
        place_blueprint(
            &mut world,
            "hut",
            &CellKey::Hex { q: 0, r: 0, z: 0 },
            &[("wood".to_string(), 1)],
            1,
        )
        .is_err()
    );
    assert!(
        place_blueprint(
            &mut world,
            "hut",
            &CellKey::Hex { q: 1, r: 0, z: 0 },
            &[("wood".to_string(), 1)],
            1,
        )
        .is_ok()
    );

    let mut provinces = ProvinceMap::new();
    provinces.add_cell("prov_a");
    provinces.add_cell("prov_b");
    world.map = Some(Map::new(Box::new(provinces)));

    let province_site = place_blueprint(
        &mut world,
        "hall",
        &CellKey::Province {
            id: "prov_a".to_string(),
        },
        &[("wood".to_string(), 1)],
        1,
    );
    assert!(province_site.is_ok());
    assert!(
        place_blueprint(
            &mut world,
            "hall",
            &CellKey::Province {
                id: "prov_a".to_string()
            },
            &[("wood".to_string(), 1)],
            1,
        )
        .is_err()
    );
    assert!(
        place_blueprint(
            &mut world,
            "hall",
            &CellKey::Province {
                id: "prov_b".to_string()
            },
            &[("wood".to_string(), 1)],
            1,
        )
        .is_ok()
    );
}
