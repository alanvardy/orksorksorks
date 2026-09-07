use assert_cmd::Command;

fn git_branch() -> String {
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

#[test]
fn branch_prints_current_branch() {
    let expected = git_branch();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.arg("branch").output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end_matches('\x07').trim_end(), expected);
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn branch_json_returns_valid_json_with_no_ansi() {
    let expected = git_branch();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.args(["branch", "-j"]).output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["data"].as_str(), Some(expected.as_str()));
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

// Optional sad path: outside any repo, git fails → non-zero exit, "git" source.
#[test]
fn branch_outside_repo_fails() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());
    cmd.arg("branch").assert().failure();
}
