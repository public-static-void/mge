use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_lua_cli_runs_script_successfully() {
    let script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../engine/scripts/lua/demos/health_demo.lua")
        .canonicalize()
        .expect("Failed to canonicalize script path");

    println!("Using script path: {:?}", script_path);

    let output = Command::new("cargo")
        .args(["run", "--bin", "mge-cli", "--"])
        .arg(script_path.to_str().unwrap())
        .output()
        .expect("Failed to execute CLI");

    println!("stdout:\n{}", String::from_utf8_lossy(&output.stdout));
    println!("stderr:\n{}", String::from_utf8_lossy(&output.stderr));

    assert!(output.status.success(), "CLI did not exit successfully");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"current\": Number(10)"),
        "Initial health not found"
    );
    assert!(
        stdout.contains("\"current\": Number(7.0)"),
        "Damaged health not found"
    );
}
