// The readonly tests briefly make the tempdir read-only and then restore it so
// `tempfile` can clean up. Restoring via `set_readonly(false)` is intentional
// here (tempdir, not a user file), so silence the world-writable lint.
#![allow(clippy::permissions_set_readonly_false)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn init_creates_orksorksorks_toml_with_default_content() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());

    cmd.arg("init").assert().success();

    let config_path = temp.path().join("orksorksorks.toml");
    assert!(config_path.exists(), "config file not created");

    let content = fs::read_to_string(&config_path).unwrap();
    // The crate is binary-only (no lib target), so integration tests can't
    // import `Config`; assert the exact serialization of the default value.
    let expected = "version = \"0.1.0\"\n";
    assert_eq!(content, expected);
}

#[test]
fn init_returns_success_message() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());

    cmd.arg("init")
        .assert()
        .success()
        .stdout(predicate::str::contains("✓ Created orksorksorks.toml"));
}

#[test]
fn init_in_readonly_dir_returns_error_exit_code() {
    let temp = tempfile::tempdir().unwrap();
    let mut perms = fs::metadata(temp.path()).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(temp.path(), perms).unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());

    cmd.arg("init").assert().failure();

    // Restore writable so tempfile can clean up
    let mut perms = fs::metadata(temp.path()).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(temp.path(), perms).unwrap();
}

#[test]
fn init_json_error_in_readonly_dir_shows_source_io() {
    let temp = tempfile::tempdir().unwrap();
    let mut perms = fs::metadata(temp.path()).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(temp.path(), perms).unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());

    let output = cmd.args(["init", "-j"]).output().unwrap();
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""source":"io""#), "stdout: {stdout}");

    // Restore
    let mut perms = fs::metadata(temp.path()).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(temp.path(), perms).unwrap();
}
