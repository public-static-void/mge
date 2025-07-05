#[path = "helpers/job_type.rs"]
mod job_type_helper;
#[path = "helpers/world.rs"]
mod world_helper;

use engine_core::systems::job::JobTypeRegistry;
use std::fs::File;
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_can_load_and_lookup_job_types_from_json() {
    // Create a temp directory for job definitions
    let temp_dir = TempDir::new().unwrap();
    let jobs_dir = temp_dir.path();

    // Write a sample job type JSON file
    let dig_job_json = r#"
    {
        "name": "DigTunnel",
        "requirements": ["Tool:Pickaxe"],
        "duration": 5,
        "effects": [{ "action": "ModifyTerrain", "from": "rock", "to": "tunnel" }]
    }
    "#;
    let dig_job_path = jobs_dir.join("dig_tunnel.json");
    let mut file = File::create(&dig_job_path).unwrap();
    file.write_all(dig_job_json.as_bytes()).unwrap();

    // Load job types from the directory
    let registry = JobTypeRegistry::load_from_dir(jobs_dir).unwrap();

    // Lookup the job type by name (case-insensitive, normalized)
    let dig = registry.get_data("DigTunnel").expect("job type exists");
    assert_eq!(dig.name, "DigTunnel");
    assert_eq!(dig.duration, Some(5.0));
    assert_eq!(dig.requirements, vec!["Tool:Pickaxe"]);
    assert_eq!(dig.effects.len(), 1);
    assert_eq!(dig.effects[0]["action"], "ModifyTerrain");
    assert_eq!(dig.effects[0]["from"], "rock");
    assert_eq!(dig.effects[0]["to"], "tunnel");
}

#[test]
fn test_job_type_asset_is_registered() {
    let mut world = world_helper::make_test_world();

    // Load the real job type JSON from disk (e.g., DigTunnel)
    let job_type = job_type_helper::load_job_type_from_assets("dig_tunnel");
    world.job_types.register_job_type(job_type);

    let dig = world
        .job_types
        .get_data("DigTunnel")
        .expect("DigTunnel should be registered");
    assert_eq!(dig.name, "DigTunnel");
    assert_eq!(dig.requirements, vec!["Tool:Pickaxe"]);
    assert_eq!(dig.duration, Some(5.0));
    assert_eq!(dig.effects[0]["action"], "ModifyTerrain");
    assert_eq!(dig.effects[0]["from"], "rock");
    assert_eq!(dig.effects[0]["to"], "tunnel");
}
