use std::fs;
use std::process::Command;

/// Normalize Markdown for robust comparison:
/// - Normalize line endings
/// - Remove trailing whitespace
/// - Remove leading/trailing blank lines
/// - Ignore trailing blank lines at EOF
fn normalize_markdown(s: &str) -> String {
    let normalized = s.replace("\r\n", "\n");
    let mut result = Vec::new();
    let mut prev_blank = false;
    let mut prev_was_variant_header = false;

    for line in normalized.split('\n') {
        let trimmed = line.trim_end();
        let is_blank = trimmed.is_empty();
        let is_variant_header = trimmed.starts_with("- **") && trimmed.ends_with("**:");

        if is_blank {
            if prev_was_variant_header {
                // skip blank lines immediately after variant headers
                continue;
            }
            if !prev_blank {
                result.push(""); // Only push one blank line
            }
        } else {
            result.push(trimmed);
        }
        prev_blank = is_blank;
        prev_was_variant_header = is_variant_header;
    }

    // Remove leading blank lines
    while result.first().is_some_and(|l| l.is_empty()) {
        result.remove(0);
    }
    // Remove trailing blank lines
    while result.last().is_some_and(|l| l.is_empty()) {
        result.pop();
    }

    let mut joined = result.join("\n");
    joined.push('\n'); // Always end with a single newline
    joined
}

#[test]
fn test_schema_to_markdown_doc_generation() {
    let schema_path = "tests/schemas/position.json";
    let out_dir = "tests/generated";
    let expected_md_path = "tests/expected/position.md";

    // Ensure output directory exists
    fs::create_dir_all(out_dir).unwrap();

    // Run the codegen tool with --lang md
    let status = Command::new(env!("CARGO_BIN_EXE_codegen"))
        .arg(schema_path)
        .arg(out_dir)
        .arg("--lang")
        .arg("md")
        .status()
        .expect("Failed to run codegen tool");
    assert!(status.success(), "Codegen tool failed");

    // Read and normalize generated and expected Markdown files
    let generated_md_path = format!("{}/position.md", out_dir);
    let generated =
        fs::read_to_string(&generated_md_path).expect("Failed to read generated Markdown file");
    let expected =
        fs::read_to_string(expected_md_path).expect("Failed to read expected Markdown file");

    if normalize_markdown(&generated) != normalize_markdown(&expected) {
        println!("=== GENERATED ===\n{:?}", normalize_markdown(&generated));
        println!("=== EXPECTED ===\n{:?}", normalize_markdown(&expected));
    }
    assert_eq!(
        normalize_markdown(&generated),
        normalize_markdown(&expected),
        "Generated Markdown doc does not match expected output (ignoring whitespace and blank lines)"
    );
}
