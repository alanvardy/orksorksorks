use crate::config::{Config, Step};
use crate::errors::Error;

/// Compose the artifact-directory path from a cwd and a branch name.
///
/// Slashes in the branch name are normalized to hyphens so the branch is a
/// single path segment. Pure — no git, no I/O, no error path. The trailing
/// slash is part of the contract (see `task.md`).
pub(crate) fn artifact_dir_path(cwd: &std::path::Path, branch: &str) -> String {
    format!(
        "{}/.pi/orksorksorks/{}/",
        cwd.display(),
        branch.replace('/', "-")
    )
}

/// Reverse-iterate steps and return the *matched step* (not just its name),
/// so callers like `step` and `model` can read different fields (`name` vs
/// `model`). A step with an empty `trigger_artifact` is a default: it never
/// matches by existence, but is returned instead of erroring when no other
/// step's artifact exists. Configs loaded through `read_config` have at most
/// one default (enforced by `config:multiple-default`); this function still
/// tolerates several and the last such step in the list wins.
/// `artifact_dir` must end in a trailing slash — the same string-composition
/// convention as `artifact_dir_path`.
pub(crate) fn determine_step(config: &Config, artifact_dir: &str) -> Result<Step, Error> {
    let mut default = None;
    for step in config.steps.iter().rev() {
        if step.trigger_artifact.is_empty() {
            // Empty trigger = default step; never used while a real match
            // is possible, only as fallback. First seen in reverse = last
            // forward, which wins (reverse-priority convention).
            default.get_or_insert_with(|| step.clone());
            continue;
        }
        let path = format!("{artifact_dir}{}", step.trigger_artifact);
        if std::path::Path::new(&path).try_exists()? {
            return Ok(step.clone());
        }
    }
    if let Some(step) = default {
        return Ok(step);
    }
    Err(Error::new(
        "step",
        &format!("{artifact_dir}: no trigger artifact matched"),
    ))
}

/// Look up the named model (the current step's `model` reference) in
/// `config.models` and return the matching entry, so callers like `model`
/// and `thinking` can read different fields (`model` vs `thinking`).
pub(crate) fn resolve_model(config: &Config, name: &str) -> Result<crate::config::Model, Error> {
    for m in config.models.iter() {
        if m.name == name {
            return Ok(m.clone());
        }
    }
    Err(Error::new("model", &format!("no model named {name:?}")))
}

/// Look up the named step in `config.steps` and return the matching entry.
/// First match wins; an unknown name errors with the `"step"` tag (the same
/// tag `determine_step` uses, so callers discriminate by message).
pub(crate) fn resolve_step(config: &Config, name: &str) -> Result<Step, Error> {
    for s in config.steps.iter() {
        if s.name == name {
            return Ok(s.clone());
        }
    }
    Err(Error::new("step", &format!("no step named {name:?}")))
}

/// Look up the named prompt (a `config.prompts` key, usually a step name)
/// and return its content.
pub(crate) fn resolve_prompt(config: &Config, name: &str) -> Result<String, Error> {
    for p in config.prompts.iter() {
        if p.name == name {
            return Ok(p.content.clone());
        }
    }
    Err(Error::new("prompt", &format!("no prompt named {name:?}")))
}

/// Look up the named script (a `config.scripts` key referenced by a step's
/// `script` field) and return its raw content.
pub(crate) fn resolve_script(config: &Config, name: &str) -> Result<String, Error> {
    for s in config.scripts.iter() {
        if s.name == name {
            return Ok(s.content.clone());
        }
    }
    Err(Error::new("script", &format!("no script named {name:?}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_dir_path_appends_trailing_slash() {
        use pretty_assertions::assert_eq;
        let path = artifact_dir_path(std::path::Path::new("/repo"), "main");
        assert_eq!(path, "/repo/.pi/orksorksorks/main/");
    }

    #[test]
    fn artifact_dir_path_is_plain() {
        let path = artifact_dir_path(std::path::Path::new("/repo"), "main");
        assert!(!path.contains('\x1b'), "{path}");
    }

    #[test]
    fn artifact_dir_path_handles_branch_with_slashes() {
        use pretty_assertions::assert_eq;
        let path = artifact_dir_path(std::path::Path::new("/repo"), "feature/x");
        assert_eq!(path, "/repo/.pi/orksorksorks/feature-x/");
    }

    #[test]
    fn artifact_dir_path_replaces_all_slashes() {
        use pretty_assertions::assert_eq;
        let path = artifact_dir_path(std::path::Path::new("/repo"), "a/b/c");
        assert_eq!(path, "/repo/.pi/orksorksorks/a-b-c/");
    }

    #[test]
    fn artifact_dir_path_leaves_slashless_branch_unchanged() {
        use pretty_assertions::assert_eq;
        let path = artifact_dir_path(std::path::Path::new("/repo"), "main");
        assert_eq!(path, "/repo/.pi/orksorksorks/main/");
    }

    #[test]
    fn determine_step_returns_step_with_present_artifact() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("first.txt"), "").unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "one".to_string(),
                    trigger_artifact: "first.txt".to_string(),
                    model: "small".to_string(),
                    script: None,
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: "second.txt".to_string(),
                    model: "high".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        assert_eq!(determine_step(&config, &artifact_dir).unwrap().name, "one");
    }

    #[test]
    fn determine_step_prefers_last_step_in_reverse() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("first.txt"), "").unwrap();
        std::fs::write(dir.path().join("second.txt"), "").unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "one".to_string(),
                    trigger_artifact: "first.txt".to_string(),
                    model: "small".to_string(),
                    script: None,
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: "second.txt".to_string(),
                    model: "high".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        assert_eq!(determine_step(&config, &artifact_dir).unwrap().name, "two");
    }

    #[test]
    fn determine_step_empty_trigger_is_default_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "one".to_string(),
                    trigger_artifact: "first.txt".to_string(),
                    model: "small".to_string(),
                    script: None,
                },
                Step {
                    name: "default".to_string(),
                    trigger_artifact: String::new(),
                    model: "small".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        assert_eq!(
            determine_step(&config, &artifact_dir).unwrap().name,
            "default"
        );
    }

    #[test]
    fn determine_step_empty_trigger_does_not_shadow_real_match() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("first.txt"), "").unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "one".to_string(),
                    trigger_artifact: "first.txt".to_string(),
                    model: "small".to_string(),
                    script: None,
                },
                Step {
                    name: "default".to_string(),
                    trigger_artifact: String::new(),
                    model: "small".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        assert_eq!(determine_step(&config, &artifact_dir).unwrap().name, "one");
    }

    #[test]
    fn determine_step_empty_trigger_matches_before_later_artifact() {
        // The default is a last resort: it does not beat a later step's
        // real artifact, only the absence of any artifact.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("second.txt"), "").unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "default".to_string(),
                    trigger_artifact: String::new(),
                    model: "small".to_string(),
                    script: None,
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: "second.txt".to_string(),
                    model: "high".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        assert_eq!(determine_step(&config, &artifact_dir).unwrap().name, "two");
    }

    #[test]
    fn determine_step_latest_empty_trigger_wins() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "one".to_string(),
                    trigger_artifact: String::new(),
                    model: "small".to_string(),
                    script: None,
                },
                Step {
                    name: "two".to_string(),
                    trigger_artifact: String::new(),
                    model: "high".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        assert_eq!(determine_step(&config, &artifact_dir).unwrap().name, "two");
    }

    #[test]
    fn determine_step_no_match_errors_with_step_tag() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![Step {
                name: "one".to_string(),
                trigger_artifact: "first.txt".to_string(),
                model: "small".to_string(),
                script: None,
            }],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let artifact_dir = format!("{}/", dir.path().display());
        let err = determine_step(&config, &artifact_dir).unwrap_err();
        assert_eq!(err.source, "step");
        assert!(
            err.message.contains("no trigger artifact matched"),
            "{}",
            err.message
        );
    }

    #[test]
    fn resolve_model_returns_model_for_matching_name() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![crate::config::Model {
                name: "small".to_string(),
                model: "openrouter/deepseek/flash".to_string(),
                thinking: "high".to_string(),
            }],
            prompts: vec![],
            scripts: vec![],
        };
        assert_eq!(
            resolve_model(&config, "small").unwrap().model,
            "openrouter/deepseek/flash",
        );
        assert_eq!(resolve_model(&config, "small").unwrap().thinking, "high");
    }

    #[test]
    fn resolve_model_missing_name_errors_with_model_tag() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![crate::config::Model {
                name: "small".to_string(),
                model: "openrouter/deepseek/flash".to_string(),
                thinking: "high".to_string(),
            }],
            prompts: vec![],
            scripts: vec![],
        };
        let err = resolve_model(&config, "large").unwrap_err();
        assert_eq!(err.source, "model");
        assert!(err.message.contains("no model named"), "{}", err.message);
    }

    #[test]
    fn resolve_step_returns_matching_step() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![Step {
                name: "one".to_string(),
                trigger_artifact: "first.txt".to_string(),
                model: "small".to_string(),
                script: None,
            }],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let step = resolve_step(&config, "one").unwrap();
        assert_eq!(step.name, "one");
        assert_eq!(step.trigger_artifact, "first.txt");
        assert_eq!(step.model, "small");
    }

    #[test]
    fn resolve_step_unknown_name_tags_step_error() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![Step {
                name: "one".to_string(),
                trigger_artifact: String::new(),
                model: "small".to_string(),
                script: None,
            }],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let err = resolve_step(&config, "nope").unwrap_err();
        assert_eq!(err.source, "step");
        assert!(
            err.message.contains("no step named \"nope\""),
            "{}",
            err.message
        );
    }

    #[test]
    fn resolve_step_first_match_wins() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![
                Step {
                    name: "dup".to_string(),
                    trigger_artifact: String::new(),
                    model: "small".to_string(),
                    script: None,
                },
                Step {
                    name: "dup".to_string(),
                    trigger_artifact: String::new(),
                    model: "high".to_string(),
                    script: None,
                },
            ],
            models: vec![],
            prompts: vec![],
            scripts: vec![],
        };
        let step = resolve_step(&config, "dup").unwrap();
        assert_eq!(step.model, "small");
    }

    #[test]
    fn resolve_prompt_returns_content_for_matching_name() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![crate::config::Prompt {
                name: "questions".to_string(),
                content: "# Question — Decompose the Task\n".to_string(),
            }],
            scripts: vec![],
        };
        assert_eq!(
            resolve_prompt(&config, "questions").unwrap(),
            "# Question — Decompose the Task\n",
        );
    }

    #[test]
    fn resolve_prompt_missing_name_errors_with_prompt_tag() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![crate::config::Prompt {
                name: "questions".to_string(),
                content: String::new(),
            }],
            scripts: vec![],
        };
        let err = resolve_prompt(&config, "research").unwrap_err();
        assert_eq!(err.source, "prompt");
        assert!(err.message.contains("no prompt named"), "{}", err.message);
    }

    #[test]
    fn resolve_script_hit_returns_content() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![],
            scripts: vec![crate::config::Script {
                name: "run-one".to_string(),
                content: "#!/bin/bash\n".to_string(),
            }],
        };
        assert_eq!(resolve_script(&config, "run-one").unwrap(), "#!/bin/bash\n",);
    }

    #[test]
    fn resolve_script_miss_returns_script_tag() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![],
            scripts: vec![crate::config::Script {
                name: "run-one".to_string(),
                content: String::new(),
            }],
        };
        let err = resolve_script(&config, "nope").unwrap_err();
        assert_eq!(err.source, "script");
        assert!(err.message.contains("no script named"), "{}", err.message);
    }

    #[test]
    fn resolve_script_first_match_wins() {
        let config = Config {
            version: "0.1.0".to_string(),
            show_frontmatter: true,
            steps: vec![],
            models: vec![],
            prompts: vec![],
            scripts: vec![
                crate::config::Script {
                    name: "dup".to_string(),
                    content: "first\n".to_string(),
                },
                crate::config::Script {
                    name: "dup".to_string(),
                    content: "second\n".to_string(),
                },
            ],
        };
        assert_eq!(resolve_script(&config, "dup").unwrap(), "first\n",);
    }
}
