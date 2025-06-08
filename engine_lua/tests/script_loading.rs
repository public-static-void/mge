//! Tests for loading and executing Lua scripts via both the ScriptEngine API and the CLI.
//! - API-level: Ensures the Rust scripting bridge loads and runs scripts from files.
//! - CLI-level: Ensures the user-facing CLI can execute Lua scripts from disk.

use engine_core::ecs::registry::ComponentRegistry;
use engine_lua::ScriptEngine;
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

#[test]
fn test_scriptengine_loads_lua_script_from_file() {
    let mut registry = ComponentRegistry::new();
    let schema_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../engine/assets/schemas");
    assert!(
        schema_dir.exists(),
        "Schema directory does not exist: {:?}",
        schema_dir
    );

    for entry in std::fs::read_dir(&schema_dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let json = std::fs::read_to_string(&path).unwrap();
            registry.register_external_schema_from_json(&json).unwrap();
        }
    }
    let registry = Arc::new(Mutex::new(registry));
    let mut engine = ScriptEngine::new();
    let world = Rc::new(RefCell::new(engine_core::ecs::World::new(registry.clone())));
    world.borrow_mut().current_mode = "roguelike".to_string();
    engine.register_world(world.clone()).unwrap();

    // Write a temp Lua script file
    let mut file = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;
    writeln!(
        file,
        r#"
        local id = spawn_entity()
        set_component(id, "Position", {{ pos = {{ Square = {{ x = 9, y = 10, z = 0 }} }} }})
        local pos = get_component(id, "Position")
        assert(pos.pos.Square.x == 9)
        assert(pos.pos.Square.y == 10)
        "#
    )
    .unwrap();

    let script_str = std::fs::read_to_string(file.path()).unwrap();
    engine.run_script(&script_str).unwrap();
}

#[test]
fn test_cli_executes_lua_script_file() {
    let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../engine/scripts/lua/demos/hello.lua")
        .canonicalize()
        .expect("Failed to canonicalize script path: check that engine/scripts/lua/demos/hello.lua exists");

    assert!(
        script_path.exists(),
        "Lua script file does not exist: {:?}",
        script_path
    );

    let output = Command::new("cargo")
        .args(["run", "--bin", "mge_cli", "--"])
        .arg(script_path.to_str().unwrap())
        .output()
        .expect("Failed to execute CLI");

    if !output.status.success() {
        eprintln!(
            "CLI failed!\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(output.status.success(), "CLI did not exit successfully");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hello from Lua scripting bridge!"),
        "Did not find expected output"
    );
}
