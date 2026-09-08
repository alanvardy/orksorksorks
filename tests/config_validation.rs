//! CLI integration pinning for the `config:*` validation surface: every
//! rule family fails fast with exit 1, no ANSI, a stable text phrase on
//! stderr, and a JSON `error.source == "config:<tag>"` on stdout.

use assert_cmd::Command;

const BRANCH: &str = "main";

fn git_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "-b", BRANCH])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success(), "git init failed");
    dir
}

/// Run `step --config <file>` in text and JSON modes against an invalid
/// config, pinning exit 1, a text phrase, no ANSI, and the JSON source tag.
fn assert_invalid_config(config: &str, text_phrase: &str, json_source: &str) {
    let dir = git_repo();
    std::fs::write(dir.path().join("orksorksorks.toml"), config).unwrap();

    // Text mode: exit 1, stderr phrase, no ANSI.
    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["step", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "expected exit 1 for {json_source}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(text_phrase), "stderr: {stderr}");
    assert!(!stderr.contains('\x1b'), "stderr had ANSI: {stderr}");

    // JSON mode: source tag pin on stdout.
    let output = Command::cargo_bin("orksorksorks")
        .unwrap()
        .args(["step", "--config", "orksorksorks.toml", "-j"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["error"]["source"].as_str(), Some(json_source));
    assert!(!stdout.contains('\x1b'), "stdout had ANSI: {stdout}");
}

#[test]
fn config_bad_version_fails() {
    assert_invalid_config(
        "version = \"9.9.9\"\n",
        "unsupported config version",
        "config:version",
    );
}

#[test]
fn config_duplicate_step_name_fails() {
    assert_invalid_config(
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"b.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ),
        "duplicate name",
        "config:duplicate-name",
    );
}

#[test]
fn config_empty_name_fails() {
    assert_invalid_config(
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ),
        "empty name",
        "config:empty-name",
    );
}

#[test]
fn config_empty_model_fails() {
    assert_invalid_config(
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ),
        "empty model",
        "config:empty-model",
    );
}

#[test]
fn config_duplicate_trigger_fails() {
    assert_invalid_config(
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[steps]]\nname = \"two\"\ntrigger_artifact = \"a.txt\"\nmodel = \"high\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[models]]\nname = \"high\"\nmodel = \"openrouter/deepseek/pro\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
            "[[prompts]]\nname = \"two\"\ncontent = \"two\"\n",
        ),
        "used by more than one step",
        "config:duplicate-trigger",
    );
}

#[test]
fn config_multiple_defaults_fail() {
    assert_invalid_config(
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"\"\nmodel = \"small\"\n",
            "[[steps]]\nname = \"two\"\ntrigger_artifact = \"\"\nmodel = \"high\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[models]]\nname = \"high\"\nmodel = \"openrouter/deepseek/pro\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
            "[[prompts]]\nname = \"two\"\ncontent = \"two\"\n",
        ),
        "empty trigger artifact",
        "config:multiple-default",
    );
}

#[test]
fn config_missing_prompt_fails() {
    assert_invalid_config(
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
        ),
        "no matching prompt",
        "config:missing-prompt",
    );
}

#[test]
fn config_missing_model_fails() {
    assert_invalid_config(
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"a.txt\"\nmodel = \"nope\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ),
        "references unknown model",
        "config:missing-model",
    );
}

#[test]
fn valid_config_succeeds() {
    let dir = git_repo();
    std::fs::write(
        dir.path().join("orksorksorks.toml"),
        concat!(
            "version = \"0.1.0\"\n",
            "[[steps]]\nname = \"one\"\ntrigger_artifact = \"first.txt\"\nmodel = \"small\"\n",
            "[[models]]\nname = \"small\"\nmodel = \"openrouter/deepseek/flash\"\nthinking = \"high\"\n",
            "[[prompts]]\nname = \"one\"\ncontent = \"one\"\n",
        ),
    )
    .unwrap();
    let artifact_dir = dir.path().join(".pi").join("orksorksorks").join(BRANCH);
    std::fs::create_dir_all(&artifact_dir).unwrap();
    std::fs::write(artifact_dir.join("first.txt"), "").unwrap();

    let mut cmd = Command::cargo_bin("orksorksorks").unwrap();
    let output = cmd
        .args(["step", "--config", "orksorksorks.toml"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.trim_end(), "one");
    assert!(!stdout.contains('\x1b'), "stdout: {stdout}");
}
