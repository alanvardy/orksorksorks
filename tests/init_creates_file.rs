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
    let config_dir = temp.path().join("config");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    cmd.arg("init").assert().success();

    let config_path = config_dir.join("orksorksorks.toml");
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
    let config_dir = temp.path().join("config");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    let resolved = config_dir.join("orksorksorks.toml");
    cmd.arg("init")
        .assert()
        .success()
        // Under cfg!(test) the child still applies color, so assert the path
        // substring that survives the ANSI-free stdout.
        .stdout(predicate::str::contains(format!(
            "✓ Created {}",
            resolved.display()
        )));
}

#[test]
fn init_in_readonly_dir_returns_error_exit_code() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    fs::create_dir(&config_dir).unwrap();
    let mut perms = fs::metadata(&config_dir).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&config_dir, perms).unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    cmd.arg("init").assert().failure();

    // Restore writable so tempfile can clean up
    let mut perms = fs::metadata(&config_dir).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(&config_dir, perms).unwrap();
}

#[test]
fn init_json_error_in_readonly_dir_shows_source_io() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    fs::create_dir(&config_dir).unwrap();
    let mut perms = fs::metadata(&config_dir).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&config_dir, perms).unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    let output = cmd.args(["init", "-j"]).output().unwrap();
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""source":"io""#), "stdout: {stdout}");

    // Restore
    let mut perms = fs::metadata(&config_dir).unwrap().permissions();
    perms.set_readonly(false);
    fs::set_permissions(&config_dir, perms).unwrap();
}
