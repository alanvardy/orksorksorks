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
    // The config lives in the repo and must be passed explicitly: without
    // `--config` the path resolves through the config directory, not the CWD.
    let output = cmd
        .args(["step", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end(), "two");
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
        .args(["step", "--config", "orksorksorks.toml", "-j"])
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
        .args(["step", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn step_with_default_returns_default_when_no_artifacts() {
    let dir = init_git_repo();
    // A step with an empty trigger_artifact is the default: it matches when
    // no other step's artifact exists, so the command succeeds (no error).
    std::fs::write(
        dir.path().join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\n",
            "name = \"one\"\n",
            "trigger_artifact = \"first.txt\"\n",
            "[[steps]]\n",
            "name = \"default\"\n",
            "trigger_artifact = \"\"\n",
        ),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["step", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end(), "default");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn step_real_artifact_beats_default_step() {
    let dir = init_git_repo();
    std::fs::write(
        dir.path().join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\n",
            "name = \"default\"\n",
            "trigger_artifact = \"\"\n",
            "[[steps]]\n",
            "name = \"two\"\n",
            "trigger_artifact = \"second.txt\"\n",
        ),
    )
    .unwrap();
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("second.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["step", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end(), "two");
}

/// Without `--config`, `step` resolves the config through the config
/// directory (`$XDG_CONFIG_HOME` here) — mirroring `init` — not the CWD.
#[test]
fn step_without_flag_reads_config_dir() {
    let dir = init_git_repo();
    let xdg = tempfile::tempdir().unwrap();
    let config_dir = xdg.path().join("cfg");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\n",
            "name = \"one\"\n",
            "trigger_artifact = \"first.txt\"\n",
        ),
    )
    .unwrap();
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("first.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .arg("step")
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end(), "one");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

/// Regression guard: without `--config`, a config sitting in the CWD is no
/// longer picked up — the command must fail rather than fall back to it.
#[test]
fn step_without_flag_ignores_cwd_config() {
    let dir = init_git_repo();
    write_config(dir.path());
    let xdg = tempfile::tempdir().unwrap();
    let config_dir = xdg.path().join("cfg");
    std::fs::create_dir_all(&config_dir).unwrap();

    Command::cargo_bin("orksorksorks")
        .unwrap()
        .arg("step")
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .assert()
        .failure();
}
