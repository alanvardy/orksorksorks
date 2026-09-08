//! Per-user config directory resolution for the `init` write target.

use std::path::{Path, PathBuf};

use crate::errors::Error;

/// The config filename written by `init`, joined under the resolved directory.
const FILE_NAME: &str = "orksorksorks.toml";

/// How the config file path was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigPathSource {
    /// User passed `--config <path>`.
    ExplicitFlag,
    /// Resolved from the `XDG_CONFIG_HOME` environment variable.
    XdgConfigHome,
    /// Resolved from `$HOME/.config`.
    HomeDotConfig,
    /// Resolved from `%APPDATA%` (Windows).
    AppData,
}

impl std::fmt::Display for ConfigPathSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExplicitFlag => write!(f, "specified via --config"),
            Self::XdgConfigHome => write!(f, "resolved from XDG_CONFIG_HOME"),
            Self::HomeDotConfig => write!(f, "resolved from HOME/.config"),
            Self::AppData => write!(f, "resolved from %APPDATA%"),
        }
    }
}

/// Resolve the full target path for `init`.
///
/// `Some(path)` passes an explicit override through unchanged. `None` falls
/// back to `<config dir>/orksorksorks.toml`, where the config dir is resolved
/// from `$XDG_CONFIG_HOME` (absolute only), then `$HOME/.config` (Unix) or
/// `%APPDATA%` (Windows).
pub fn config_file_path(explicit: Option<&Path>) -> Result<(PathBuf, ConfigPathSource), Error> {
    match explicit {
        Some(path) => Ok((path.to_path_buf(), ConfigPathSource::ExplicitFlag)),
        None => resolve_config_dir().map(|(dir, source)| (dir.join(FILE_NAME), source)),
    }
}

/// Resolve the per-user config directory from the environment.
fn resolve_config_dir() -> Result<(PathBuf, ConfigPathSource), Error> {
    // XDG wins on every platform, but only when set to an absolute path.
    // Unset, empty, and relative values all mean "fall back" (XDG spec).
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        let dir = PathBuf::from(xdg);
        if dir.is_absolute() {
            return Ok((dir, ConfigPathSource::XdgConfigHome));
        }
    }

    if cfg!(windows) {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return Ok((PathBuf::from(appdata), ConfigPathSource::AppData));
        }
    } else if let Some(home) = std::env::var_os("HOME") {
        return Ok((
            PathBuf::from(home).join(".config"),
            ConfigPathSource::HomeDotConfig,
        ));
    }

    Err(Error::new(
        "config-dir",
        "could not determine a config directory: set XDG_CONFIG_HOME or HOME",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_xdg_is_used() {
        unsafe { std::env::set_var("XDG_CONFIG_HOME", "/tmp/ork-cfg") }
        let (path, source) = config_file_path(None).unwrap();
        assert_eq!(path, std::path::Path::new("/tmp/ork-cfg").join(FILE_NAME));
        assert_eq!(source, ConfigPathSource::XdgConfigHome);
        unsafe { std::env::remove_var("XDG_CONFIG_HOME") }
    }

    #[test]
    fn unset_xdg_falls_back_to_home_dot_config() {
        unsafe {
            std::env::remove_var("XDG_CONFIG_HOME");
            std::env::set_var("HOME", "/home/ork");
        }
        let (path, source) = config_file_path(None).unwrap();
        assert_eq!(
            path,
            std::path::Path::new("/home/ork/.config").join(FILE_NAME)
        );
        assert_eq!(source, ConfigPathSource::HomeDotConfig);
    }

    #[test]
    fn empty_xdg_falls_back_to_home() {
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", "");
            std::env::set_var("HOME", "/home/ork");
        }
        let (path, source) = config_file_path(None).unwrap();
        assert_eq!(
            path,
            std::path::Path::new("/home/ork/.config").join(FILE_NAME)
        );
        assert_eq!(source, ConfigPathSource::HomeDotConfig);
    }

    #[test]
    fn relative_xdg_falls_back_to_home() {
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", "relative/dir");
            std::env::set_var("HOME", "/home/ork");
        }
        let (path, source) = config_file_path(None).unwrap();
        assert_eq!(
            path,
            std::path::Path::new("/home/ork/.config").join(FILE_NAME)
        );
        assert_eq!(source, ConfigPathSource::HomeDotConfig);
    }

    #[test]
    fn explicit_path_passthrough() {
        let p = std::path::Path::new("/custom/override.toml");
        let (path, source) = config_file_path(Some(p)).unwrap();
        assert_eq!(path, p.to_path_buf());
        assert_eq!(source, ConfigPathSource::ExplicitFlag);
    }

    #[test]
    fn unresolved_home_yields_config_dir_error() {
        unsafe {
            std::env::remove_var("HOME");
            std::env::remove_var("XDG_CONFIG_HOME");
        }
        let err = config_file_path(None).unwrap_err();
        assert_eq!(err.source, "config-dir");
    }

    #[test]
    fn config_path_source_display_pins_phrases() {
        assert_eq!(
            ConfigPathSource::ExplicitFlag.to_string(),
            "specified via --config"
        );
        assert_eq!(
            ConfigPathSource::XdgConfigHome.to_string(),
            "resolved from XDG_CONFIG_HOME"
        );
        assert_eq!(
            ConfigPathSource::HomeDotConfig.to_string(),
            "resolved from HOME/.config"
        );
        assert_eq!(
            ConfigPathSource::AppData.to_string(),
            "resolved from %APPDATA%"
        );
    }
}
