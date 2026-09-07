use assert_cmd::Command;

const BRANCH: &str = "main";

/// `git init -b <name>` yields an unborn branch, and modern git (>= 2.22)
/// still reports its name from `git branch --show-current`.
fn init_git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-b", BRANCH])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "git init failed");
    dir
}

fn write_config(dir: &std::path::Path) {
    std::fs::write(
        dir.join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\n",
            "name = \"one\"\n",
            "trigger_artifact = \"first.txt\"\n",
            "[[steps]]\n",
            "name = \"two\"\n",
            "trigger_artifact = \"second.txt\"\n",
        ),
    )
    .unwrap();
}

fn artifact_dir(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join(".pi").join("orksorksorks").join(BRANCH)
}

#[test]
fn step_prints_name_of_step_with_present_artifact() {
    let dir = init_git_repo();
    write_config(dir.path());
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("second.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd.arg("step").current_dir(dir.path()).output().unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end_matches('\x07').trim_end(), "two");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn step_json_returns_valid_json_with_no_ansi() {
    let dir = init_git_repo();
    write_config(dir.path());
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("second.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["step", "-j"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["data"].as_str(), Some("two"));
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn step_no_artifacts_present_fails() {
    let dir = init_git_repo();
    write_config(dir.path());

    Command::cargo_bin("orksorksorks")
        .unwrap()
        .arg("step")
        .current_dir(dir.path())
        .assert()
        .failure();
}
