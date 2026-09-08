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

/// Config with two steps (`one`, `two`) whose names each have a matching
/// `[[prompts]]` entry (multi-line TOML content).
///
/// The `show_frontmatter` key is omitted, so this exercises the serde
/// default of `true` (see `prompt_frontmatter_shown_by_default`).
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
        ),
    )
    .unwrap();
}

/// The same config with `show_frontmatter = false`, exercising the disabled
/// frontmatter code path end to end.
fn write_config_hidden(dir: &std::path::Path) {
    std::fs::write(
        dir.join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "show_frontmatter = false\n",
            "[[steps]]\n",
            "name = \"one\"\n",
            "trigger_artifact = \"first.txt\"\n",
            "model = \"small\"\n",
            "[[steps]]\n",
            "name = \"two\"\n",
            "trigger_artifact = \"second.txt\"\n",
            "model = \"high\"\n",
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
        ),
    )
    .unwrap();
}

#[test]
fn prompt_without_step_name_infers_current_step() {
    let dir = init_git_repo();
    write_config(dir.path());
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("second.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["prompt", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Prompt for step two"), "stdout: {stdout}");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn prompt_without_step_name_no_artifacts_fails() {
    let dir = init_git_repo();
    write_config(dir.path());

    Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["prompt", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn prompt_with_explicit_step_name_flag_overrides_step() {
    // Even with a later artifact present, an explicit --step name wins.
    let dir = init_git_repo();
    write_config(dir.path());
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("second.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["prompt", "--step", "one", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Prompt for step one"), "stdout: {stdout}");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
    // The frontmatter `step` is the effective prompt name: the explicit
    // override, not the (different) derived step from the present artifact.
    assert!(
        stdout.starts_with("## Important variables\n"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.contains(
            "## Important variables\nUse these everywhere you see $<variable>\nstep = one\n"
        ),
        "stdout: {stdout}",
    );
}

#[test]
fn prompt_json_returns_valid_json_with_data_field() {
    let dir = init_git_repo();
    write_config(dir.path());
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("first.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["prompt", "--config", "orksorksorks.toml", "-j"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let data = v["data"].as_str().unwrap();
    // The frontmatter block is part of the prompt output in JSON mode too.
    assert!(data.starts_with("## Important variables\n"), "data: {data}");
    assert!(data.contains("step = one\n"), "data: {data}");
    assert!(data.contains("# Prompt for step one"), "data: {data}");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn prompt_step_flag_unknown_name_fails_with_step_source() {
    let dir = tempfile::tempdir().unwrap();
    // No [[steps]]: `--step nope` fails in resolve_step, not resolve_prompt.
    std::fs::write(
        dir.path().join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[prompts]]\n",
            "name = \"research\"\n",
            "content = \"\"\"\n",
            "# Research — Answer the Questions\n",
            "\"\"\"\n",
        ),
    )
    .unwrap();

    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["prompt", "--step", "nope", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no step named \"nope\""),
        "stderr: {stderr}"
    );
}

/// Without `--config`, `prompt` resolves the config through the config
/// directory — mirroring `step`/`model`/`init`.
#[test]
fn prompt_without_flag_reads_config_dir() {
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
            "[[prompts]]\n",
            "name = \"one\"\n",
            "content = \"\"\"\n",
            "# Question — Decompose the Task\n",
            "\"\"\"\n",
        ),
    )
    .unwrap();
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("first.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .arg("prompt")
        .current_dir(dir.path())
        .env("XDG_CONFIG_HOME", &config_dir)
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("# Question — Decompose the Task"),
        "stdout: {stdout}"
    );
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn prompt_frontmatter_shown_by_default() {
    let dir = init_git_repo();
    // No `show_frontmatter` key in the config: the serde default is `true`,
    // so the frontmatter block must appear above the prompt output.
    write_config(dir.path());
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("first.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["prompt", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("## Important variables\n"),
        "stdout: {stdout}"
    );
    assert!(
        stdout.starts_with("## Important variables\nUse these everywhere you see $<variable>\n"),
        "stdout: {stdout}",
    );
    assert!(stdout.contains("step = one\n"), "stdout: {stdout}");
    assert!(stdout.contains("branch = main\n"), "stdout: {stdout}");
    // Canonicalize so the comparison matches what the child process sees
    // through `std::env::current_dir()` (on macOS `/var` → `/private/var`);
    // same convention as `expected_artifact_directory` in artifact_directory.rs.
    let canonical = std::fs::canonicalize(dir.path()).unwrap();
    assert!(
        stdout.contains(&format!(
            "artifact_directory = {}/.pi/orksorksorks/main/\n",
            canonical.display()
        )),
        "stdout: {stdout}",
    );
    assert!(stdout.contains("# Prompt for step one"), "stdout: {stdout}");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn prompt_frontmatter_hidden_when_show_frontmatter_false() {
    let dir = init_git_repo();
    write_config_hidden(dir.path());
    let artifact_dir = artifact_dir(dir.path());
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("first.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["prompt", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Prompt for step one"), "stdout: {stdout}");
    assert!(
        !stdout.contains("## Important variables"),
        "stdout: {stdout}"
    );
    assert!(
        !stdout.contains("Use these everywhere you see"),
        "stdout: {stdout}"
    );
    assert!(!stdout.contains("step = "), "stdout: {stdout}");
    assert!(!stdout.contains("branch = "), "stdout: {stdout}");
    assert!(
        !stdout.contains("artifact_directory = "),
        "stdout: {stdout}"
    );
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn prompt_flag_returns_content() {
    // `--step` skips artifact derivation. The prompt output carries the
    // frontmatter block (default `show_frontmatter`, needs git for the
    // branch), so run in a git repo and assert containment, not equality.
    let dir = init_git_repo();
    write_config(dir.path());

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args([
            "prompt",
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
    let data = v["data"].as_str().unwrap();
    assert!(data.contains("# Prompt for step one"), "data: {data}");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}

#[test]
fn prompt_rejects_positional_step_name() {
    let dir = init_git_repo();
    write_config(dir.path());

    Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["prompt", "one", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .assert()
        .code(2); // clap usage-error exit code
}
