use std::fs;
use std::process::Command;

#[test]
fn test_schema_to_rust_component_generation() {
    let schema_path = "tests/schemas/position.json";
    let out_dir = "tests/generated";
    let expected_rust_path = "tests/expected/position.rs";

    // Ensure output directory exists
    fs::create_dir_all(out_dir).unwrap();

    // Run the codegen tool
    let status = Command::new(env!("CARGO_BIN_EXE_codegen"))
        .arg(schema_path)
        .arg(out_dir)
        .status()
        .expect("Failed to run codegen tool");
    assert!(status.success(), "Codegen tool failed");

    // Read generated Rust file
    let generated_rust_path = format!("{}/position.rs", out_dir);
    let generated =
        fs::read_to_string(&generated_rust_path).expect("Failed to read generated Rust file");

    // Read expected Rust file
    let expected =
        fs::read_to_string(expected_rust_path).expect("Failed to read expected Rust file");

    // Compare
    assert_eq!(
        generated, expected,
        "Generated Rust code does not match expected output"
    );
}
