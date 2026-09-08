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
            "model = \"small\"\n",
            "[[steps]]\n",
            "name = \"two\"\n",
            "trigger_artifact = \"second.txt\"\n",
            "model = \"high\"\n",
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
            "model = \"small\"\n",
            "[[steps]]\n",
            "name = \"default\"\n",
            "trigger_artifact = \"\"\n",
            "model = \"small\"\n",
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
            "model = \"small\"\n",
            "[[steps]]\n",
            "name = \"two\"\n",
            "trigger_artifact = \"second.txt\"\n",
            "model = \"high\"\n",
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
            "model = \"small\"\n",
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

/// Text-mode error with explicit --config includes the path and resolution.
#[test]
fn step_with_missing_config_shows_path_in_stderr() {
    let dir = init_git_repo();
    // The CLI echoes the config path exactly as passed (no canonicalization),
    // so pass the absolute path and assert it verbatim in stderr.
    let missing = dir.path().join("nope.toml");

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .arg("step")
        .arg("--config")
        .arg(missing.display().to_string())
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&missing.display().to_string()),
        "stderr should contain path: {stderr}"
    );
    assert!(
        stderr.contains("specified via --config"),
        "stderr should contain resolution: {stderr}"
    );
    assert!(
        !stderr.contains('\x1b'),
        "stderr should have no ANSI: {stderr}"
    );
}

/// Text-mode error with XDG_CONFIG_HOME includes the resolved path and resolution.
#[test]
fn step_with_missing_xdg_config_shows_path_in_stderr() {
    let dir = init_git_repo();
    let xdg = tempfile::tempdir().unwrap();
    let config_dir = xdg.path().join("cfg");
    std::fs::create_dir_all(&config_dir).unwrap();
    // Config dir exists but no orksorksorks.toml inside it

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .arg("step")
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    let expected_path = config_dir.join("orksorksorks.toml");
    assert!(
        stderr.contains(&expected_path.display().to_string()),
        "stderr should contain path: {stderr}"
    );
    assert!(
        stderr.contains("resolved from XDG_CONFIG_HOME"),
        "stderr should contain resolution: {stderr}"
    );
    assert!(
        !stderr.contains('\x1b'),
        "stderr should have no ANSI: {stderr}"
    );
}

/// JSON error output includes the path in message, no ANSI, and no envelope shape change.
#[test]
fn step_with_missing_config_json_contains_path_in_message() {
    let dir = init_git_repo();

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["step", "--config", "nope.toml", "-j"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let msg = json["error"]["message"].as_str().unwrap();
    let src = json["error"]["source"].as_str().unwrap();

    assert_eq!(src, "io");
    assert!(
        msg.contains("nope.toml"),
        "message should contain path: {msg}"
    );
    assert!(
        msg.contains("specified via --config"),
        "message should contain resolution: {msg}"
    );
    assert!(
        !stdout.contains('\x1b'),
        "stdout should have no ANSI: {stdout}"
    );
}

#[test]
fn step_flag_works_in_non_git_dir() {
    // No `.git`: bare `step` would fail in git resolution, so success here
    // proves `--step` skips current_dir/git/artifact_dir entirely.
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args([
            "step",
            "--step",
            "one",
            "--config",
            "orksorksorks.toml",
            "-j",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["data"].as_str(), Some("one"));
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn step_flag_works_on_detached_head() {
    // A detached HEAD has no branch name (`git branch --show-current` is
    // empty), so bare `step` fails in git resolution; `--step` skips git and
    // succeeds. Directly substantiates design.md's detached-HEAD claim.
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let run = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    run(&["init"]);
    std::fs::write(dir.path().join("README"), "x").unwrap();
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
    run(&["checkout", "--detach"]);

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args([
            "step",
            "--step",
            "one",
            "--config",
            "orksorksorks.toml",
            "-j",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["data"].as_str(), Some("one"));
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn step_flag_unknown_text_error_goes_to_stderr() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["step", "--step", "nope", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty(),
        "text error must not go to stdout: {stdout}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no step named \"nope\""),
        "stderr: {stderr}"
    );
}

#[test]
fn step_flag_unknown_json_error_goes_to_stdout() {
    let dir = tempfile::tempdir().unwrap();
    write_config(dir.path());

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args([
            "step",
            "--step",
            "nope",
            "--config",
            "orksorksorks.toml",
            "-j",
        ])
        .current_dir(dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json["error"]["source"].as_str(), Some("step"));
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("no step named"),
        "{json}"
    );
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}
