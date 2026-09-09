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

fn artifact_dir(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join(".pi").join("orksorksorks").join(BRANCH)
}

/// Config with two steps (`one`, `two`) each referencing a matching
/// `[[scripts]]` entry via their `script` field (multi-line TOML content).
/// `[[models]]` and `[[prompts]]` entries are required by the config
/// validator for every step, so they mirror `prompt.rs`'s harness exactly.
fn write_config(dir: &std::path::Path) {
    std::fs::write(
        dir.join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\n",
            "name = \"one\"\n",
            "trigger_artifact = \"first.txt\"\n",
            "model = \"small\"\n",
            "script = \"run-one\"\n",
            "[[steps]]\n",
            "name = \"two\"\n",
            "trigger_artifact = \"second.txt\"\n",
            "model = \"high\"\n",
            "script = \"run-two\"\n",
            "[[models]]\n",
            "name = \"small\"\n",
            "model = \"openrouter/deepseek/flash\"\n",
            "thinking = \"high\"\n",
            "[[models]]\n",
            "name = \"high\"\n",
            "model = \"openrouter/deepseek/pro\"\n",
            "thinking = \"high\"\n",
            "[[prompts]]\n",
            "name = \"one\"\n",
            "content = \"\"\"\n",
            "# Prompt for step one\n",
            "\"\"\"\n",
            "[[prompts]]\n",
            "name = \"two\"\n",
            "content = \"\"\"\n",
            "# Prompt for step two\n",
            "\"\"\"\n",
            "[[scripts]]\n",
            "name = \"run-one\"\n",
            "content = \"\"\"\n",
            "# Script for step one\n",
            "\"\"\"\n",
            "[[scripts]]\n",
            "name = \"run-two\"\n",
            "content = \"\"\"\n",
            "# Script for step two\n",
            "\"\"\"\n",
        ),
    )
    .unwrap();
}

/// The same config, but step `two` has **no** `script` field (and only a
/// `run-one` `[[scripts]]` entry exists) so the no-script sad path fails in
/// `script_command`, not in config validation — hence both steps still get
/// their required `[[models]]`/`[[prompts]]` entries.
fn write_config_step_without_script(dir: &std::path::Path) {
    std::fs::write(
        dir.join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\n",
            "name = \"one\"\n",
            "trigger_artifact = \"first.txt\"\n",
            "model = \"small\"\n",
            "script = \"run-one\"\n",
            "[[steps]]\n",
            "name = \"two\"\n",
            "trigger_artifact = \"second.txt\"\n",
            "model = \"high\"\n",
            "[[models]]\n",
            "name = \"small\"\n",
            "model = \"openrouter/deepseek/flash\"\n",
            "thinking = \"high\"\n",
            "[[models]]\n",
            "name = \"high\"\n",
            "model = \"openrouter/deepseek/pro\"\n",
            "thinking = \"high\"\n",
            "[[prompts]]\n",
            "name = \"one\"\n",
            "content = \"none\"\n",
            "[[prompts]]\n",
            "name = \"two\"\n",
            "content = \"none\"\n",
            "[[scripts]]\n",
            "name = \"run-one\"\n",
            "content = \"\"\"\n",
            "# Script for step one\n",
            "\"\"\"\n",
        ),
    )
    .unwrap();
}

#[test]
fn script_without_step_name_infers_current_step() {
    let dir = init_git_repo();
    write_config(dir.path());
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("second.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["script", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Script for step two"), "stdout: {stdout}");
    assert!(
        !stdout.contains("## Important variables"),
        "script output carries no frontmatter: {stdout}"
    );
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn script_with_explicit_step_name_overrides_step() {
    // Even with a later artifact present (which would derive step `two`),
    // the explicit positional step name wins.
    let dir = init_git_repo();
    write_config(dir.path());
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("second.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["script", "one", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Script for step one"), "stdout: {stdout}");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn script_json_returns_valid_json_with_data_field() {
    let dir = init_git_repo();
    write_config(dir.path());
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("first.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["script", "--config", "orksorksorks.toml", "-j"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let data = v["data"].as_str().unwrap();
    // The raw script content, with no frontmatter prefix.
    assert!(data.contains("# Script for step one"), "data: {data}");
    assert!(
        !data.starts_with("## Important variables"),
        "script output carries no frontmatter: {data}"
    );
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn script_unknown_script_name_fails() {
    // Step `one` references `script = "nope"` but no `[[scripts]]` entry
    // matches: validation passes (it has its model + prompt), so the
    // failure surfaces at runtime in `resolve_script` with the `"script"`
    // tag. The explicit step name avoids git, so no repo is needed.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\n",
            "name = \"one\"\n",
            "trigger_artifact = \"first.txt\"\n",
            "model = \"small\"\n",
            "script = \"nope\"\n",
            "[[models]]\n",
            "name = \"small\"\n",
            "model = \"openrouter/deepseek/flash\"\n",
            "thinking = \"high\"\n",
            "[[prompts]]\n",
            "name = \"one\"\n",
            "content = \"none\"\n",
            "[[scripts]]\n",
            "name = \"run-other\"\n",
            "content = \"echo other\"\n",
        ),
    )
    .unwrap();

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["script", "one", "--config", "orksorksorks.toml", "-j"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["error"]["source"].as_str().unwrap(), "script");
    let message = v["error"]["message"].as_str().unwrap();
    assert!(message.contains("no script named"), "message: {message}");
}

#[test]
fn script_step_without_script_field_fails() {
    // Step `two` has no `script` key: validation passes, then the
    // explicit-name path resolves the step and fails on the missing script
    // reference with the `"script"` tag (`has no script configured`).
    let dir = tempfile::tempdir().unwrap();
    write_config_step_without_script(dir.path());

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["script", "two", "--config", "orksorksorks.toml", "-j"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["error"]["source"].as_str().unwrap(), "script");
    let message = v["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("has no script configured"),
        "message: {message}"
    );
}

/// Without `--config`, `script` resolves the config through the config
/// directory — mirroring `step`/`model`/`prompt`.
#[test]
fn script_without_flag_reads_config_dir() {
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
            "script = \"run-one\"\n",
            "[[models]]\n",
            "name = \"small\"\n",
            "model = \"openrouter/deepseek/flash\"\n",
            "thinking = \"high\"\n",
            "[[prompts]]\n",
            "name = \"one\"\n",
            "content = \"none\"\n",
            "[[scripts]]\n",
            "name = \"run-one\"\n",
            "content = \"\"\"\n",
            "# Script for step one\n",
            "\"\"\"\n",
        ),
    )
    .unwrap();
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("first.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .arg("script")
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Script for step one"), "stdout: {stdout}");
    assert!(
        !stdout.contains("## Important variables"),
        "script output carries no frontmatter: {stdout}"
    );
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}
