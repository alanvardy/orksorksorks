use assert_cmd::Command;

#[test]
fn init_json_returns_valid_json_with_data_field() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    let output = cmd.args(["init", "-j"]).output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""data""#), "stdout: {stdout}");
    let resolved = config_dir.join("orksorksorks.toml");
    assert!(
        stdout.contains(&format!("✓ Created {}", resolved.display())),
        "stdout: {stdout}"
    );
    // Valid JSON
    let _: serde_json::Value = serde_json::from_str(&stdout).unwrap();
}

#[test]
fn init_no_json_prints_plain_text() {
    let temp = tempfile::tempdir().unwrap();
    let config_dir = temp.path().join("config");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.env("XDG_CONFIG_HOME", &config_dir);

    let output = cmd.arg("init").output().unwrap();
    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let resolved = config_dir.join("orksorksorks.toml");
    assert!(
        stdout.contains(&format!("✓ Created {}", resolved.display())),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains('\x1b'),
        "stdout should have no ANSI: {stdout}"
    );
}
