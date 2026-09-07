use assert_cmd::Command;

#[test]
fn init_json_returns_valid_json_with_data_field() {
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.args(["init", "-j"]).output().unwrap();

    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""data""#), "stdout: {stdout}");
    assert!(
        stdout.contains("✓ Created orksorksorks.toml"),
        "stdout: {stdout}"
    );
    // Valid JSON
    let _: serde_json::Value = serde_json::from_str(&stdout).unwrap();
}

#[test]
fn init_no_json_prints_plain_text() {
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.arg("init").output().unwrap();

    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("✓ Created orksorksorks.toml"),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains('\x1b'),
        "stdout should have no ANSI: {stdout}"
    );
}
