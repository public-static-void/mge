use std::fs;
use std::process::Command;

#[test]
fn test_schema_to_lua_and_python_stub_generation() {
    let schema_path = "tests/schemas/position.json";
    let out_dir = "tests/generated";
    let expected_lua_path = "tests/expected/position.lua";
    let expected_py_path = "tests/expected/position.py";

    // Ensure output directory exists
    fs::create_dir_all(out_dir).unwrap();

    // Run the codegen tool with --lang lua,python
    let status = Command::new(env!("CARGO_BIN_EXE_codegen"))
        .arg(schema_path)
        .arg(out_dir)
        .arg("--lang")
        .arg("lua,python")
        .status()
        .expect("Failed to run codegen tool");
    assert!(status.success(), "Codegen tool failed");

    // Read generated Lua file
    let generated_lua_path = format!("{out_dir}/position.lua");
    let generated_lua =
        fs::read_to_string(&generated_lua_path).expect("Failed to read generated Lua file");
    let expected_lua =
        fs::read_to_string(expected_lua_path).expect("Failed to read expected Lua file");
    assert_eq!(
        generated_lua, expected_lua,
        "Generated Lua stub does not match expected output"
    );

    // Read generated Python file
    let generated_py_path = format!("{out_dir}/position.py");
    let generated_py =
        fs::read_to_string(&generated_py_path).expect("Failed to read generated Python file");
    let expected_py =
        fs::read_to_string(expected_py_path).expect("Failed to read expected Python file");
    assert_eq!(
        generated_py, expected_py,
        "Generated Python stub does not match expected output"
    );
}
