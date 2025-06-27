use std::fs;
use std::process::Command;

#[test]
fn test_schema_to_c_header_generation() {
    let schema_path = "tests/schemas/position.json";
    let out_dir = "tests/generated";
    let expected_c_path = "tests/expected/position.h";

    // Ensure output directory exists
    fs::create_dir_all(out_dir).unwrap();

    // Run the codegen tool with --lang c
    let status = Command::new(env!("CARGO_BIN_EXE_codegen"))
        .arg(schema_path)
        .arg(out_dir)
        .arg("--lang")
        .arg("c")
        .status()
        .expect("Failed to run codegen tool");
    assert!(status.success(), "Codegen tool failed");

    // Read generated C header file
    let generated_c_path = format!("{out_dir}/position.h");
    let generated =
        fs::read_to_string(&generated_c_path).expect("Failed to read generated C header file");
    let expected =
        fs::read_to_string(expected_c_path).expect("Failed to read expected C header file");

    assert_eq!(
        generated, expected,
        "Generated C header does not match expected output"
    );
}
