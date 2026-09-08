use assert_cmd::Command;

/// Create a disposable git repo (with one commit) on `branch`.
///
/// Keeps git-dependent tests hermetic: CI's `actions/checkout` leaves the tree
/// in detached HEAD (empty `branch --show-current`), so the ambient checkout
/// cannot be used as a source of truth.
fn git_repo_on_branch(branch: &str) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let run = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?} failed");
    };
    run(&["init"]);
    run(&["checkout", "-b", branch]);
    std::fs::write(dir.join("README"), "x").unwrap();
    run(&["add", "."]);
    run(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-qm",
        "init",
    ]);
    temp
}

#[test]
fn branch_prints_current_branch() {
    let temp = git_repo_on_branch("feature/foo");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());
    let output = cmd.arg("branch").output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end(), "feature/foo");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn branch_json_returns_valid_json_with_no_ansi() {
    let temp = git_repo_on_branch("feature/foo");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());
    let output = cmd.args(["branch", "-j"]).output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["data"].as_str(), Some("feature/foo"));
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

// Sad path: outside any repo, git fails → non-zero exit, "git" source.
#[test]
fn branch_outside_repo_fails() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());
    cmd.arg("branch").assert().failure();
}
