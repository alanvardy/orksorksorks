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

/// The path the binary should print for a repo on `feature/foo`.
///
/// Canonicalizes the tempdir so the comparison matches what the binary sees
/// through `std::env::current_dir()` (on macOS `/var` resolves to
/// `/private/var`).
fn expected_artifact_directory(dir: &std::path::Path) -> String {
    let canonical = std::fs::canonicalize(dir).unwrap();
    format!("{}/.pi/orksorksorks/feature-foo/", canonical.display())
}

#[test]
fn artifact_directory_prints_composed_path() {
    let temp = git_repo_on_branch("feature/foo");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());
    let output = cmd.arg("artifact_directory").output().unwrap();
    cmd.assert().success();

    let expected = expected_artifact_directory(temp.path());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end(), expected);
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn artifact_directory_json_returns_valid_json_with_no_ansi() {
    let temp = git_repo_on_branch("feature/foo");
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());
    let output = cmd.args(["artifact_directory", "-j"]).output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let expected = expected_artifact_directory(temp.path());
    assert_eq!(v["data"].as_str(), Some(expected.as_str()));
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

// Sad path: outside any repo, git fails → non-zero exit.
#[test]
fn artifact_directory_outside_repo_fails() {
    let temp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    cmd.current_dir(temp.path());
    cmd.arg("artifact_directory").assert().failure();
}
