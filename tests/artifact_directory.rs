use assert_cmd::Command;

fn expected_artifact_directory() -> String {
    let cwd = std::env::current_dir().unwrap();
    let branch = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap();
    format!(
        "{}/.pi/orksorksorks/{}/",
        cwd.display(),
        branch.replace('/', "-")
    )
}

#[test]
fn artifact_directory_prints_composed_path() {
    let expected = expected_artifact_directory();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.arg("artifact_directory").output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end_matches('\x07').trim_end(), expected);
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn artifact_directory_json_returns_valid_json_with_no_ansi() {
    let expected = expected_artifact_directory();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.args(["artifact_directory", "-j"]).output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["data"].as_str(), Some(expected.as_str()));
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

// Optional sad path: outside any repo, git fails → non-zero exit.
#[test]
fn artifact_directory_outside_repo_fails() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());
    cmd.arg("artifact_directory").assert().failure();
}
