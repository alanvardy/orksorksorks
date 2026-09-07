use crate::errors::Error;

/// Resolve the current git branch by invoking `git branch --show-current` in
/// the process working directory.
///
/// A failed spawn (git not on PATH) rides the existing `From<std::io::Error>`
/// → `"io"` tag; git-level failures use `Error::new("git", ...)`.
pub fn current_branch() -> Result<String, Error> {
    current_branch_in(&std::env::current_dir()?)
}

/// Resolve the git branch by running `git branch --show-current` in `dir`.
///
/// Testable in isolation: callers pass an explicit directory so unit tests can
/// point at a disposable repo instead of the ambient checkout (CI checks out
/// in detached HEAD, where `--show-current` is empty).
pub fn current_branch_in(dir: &std::path::Path) -> Result<String, Error> {
    let output = std::process::Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(dir)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_branch_output(output.status.success(), &stdout, &stderr)
}

/// Pure decision: map `git branch --show-current` output to a branch or error.
fn parse_branch_output(exit_ok: bool, stdout: &str, stderr: &str) -> Result<String, Error> {
    if !exit_ok {
        return Err(Error::new("git", stderr.trim()));
    }
    let stdout = stdout.trim();
    if stdout.is_empty() {
        return Err(Error::new("git", "not on a branch (detached HEAD)"));
    }
    Ok(stdout.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a disposable git repo with one commit on `branch`.
    ///
    /// Keeps git-dependent tests hermetic: CI's `actions/checkout` leaves the
    /// tree in detached HEAD (empty `branch --show-current`), so the ambient
    /// checkout cannot be used as a source of truth.
    fn init_git_branch(dir: &std::path::Path, branch: &str) {
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
    }

    #[test]
    fn current_branch_in_returns_configured_branch() {
        use pretty_assertions::assert_eq;
        let temp = tempfile::tempdir().unwrap();
        init_git_branch(temp.path(), "feature/foo");
        assert_eq!(current_branch_in(temp.path()).unwrap(), "feature/foo");
    }

    #[test]
    fn current_branch_in_detached_head_errors() {
        let temp = tempfile::tempdir().unwrap();
        init_git_branch(temp.path(), "main");
        let detach = std::process::Command::new("git")
            .args(["checkout", "--detach"])
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(detach.status.success());
        let err = current_branch_in(temp.path()).unwrap_err();
        assert_eq!(err.source, "git");
        assert!(err.message.contains("detached HEAD"), "{}", err.message);
    }

    #[test]
    fn parse_branch_output_trims_trailing_newline() {
        use pretty_assertions::assert_eq;
        let branch = parse_branch_output(true, "main\n", "").unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn parse_branch_output_detached_head_errors() {
        let err = parse_branch_output(true, "", "").unwrap_err();
        assert_eq!(err.source, "git");
        assert!(err.message.contains("detached HEAD"), "{}", err.message);
    }

    #[test]
    fn parse_branch_output_nonzero_exit_uses_stderr() {
        use pretty_assertions::assert_eq;
        let err = parse_branch_output(false, "", "fatal: not a git repository\n").unwrap_err();
        assert_eq!(err.source, "git");
        assert_eq!(err.message, "fatal: not a git repository");
    }
}
