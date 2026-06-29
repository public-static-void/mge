//! Integration tests for the schema_validator CLI binary.
//!
//! These tests use `assert_cmd` to invoke the binary as a subprocess
//! and verify exit codes, stdout, and stderr.

use assert_cmd::Command;
use predicates::prelude::predicate;
use std::path::PathBuf;

/// Root of the workspace (3 levels up from `engine/tools/schema_validator/`).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("schema_validator").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"))
        .stdout(predicate::str::contains("PATH"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("schema_validator").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("schema_validator 0.1.0"));
}

#[test]
fn test_valid_schema_directory() {
    let root = workspace_root();
    let schema_dir = root.join("engine/assets/schemas");

    let mut cmd = Command::cargo_bin("schema_validator").unwrap();
    cmd.current_dir(&root)
        .arg(schema_dir.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Checked 27 files, 0 errors."));
}

#[test]
fn test_invalid_directory() {
    let root = workspace_root();

    let mut cmd = Command::cargo_bin("schema_validator").unwrap();
    cmd.current_dir(&root)
        .arg("/tmp/opencode/mge-nonexistent-schema-dir")
        .assert()
        .failure()
        .code(predicate::eq(1))
        .stderr(predicate::str::contains("Path does not exist"));
}

#[test]
fn test_empty_directory() {
    let root = workspace_root();
    let tmp = tempfile::tempdir().unwrap();

    let mut cmd = Command::cargo_bin("schema_validator").unwrap();
    cmd.current_dir(&root)
        .arg(tmp.path().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Checked 0 files, 0 errors."));
}
