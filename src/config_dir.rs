//! Per-user config directory resolution for the `init` write target.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::errors::Error;

/// The config filename written by `init`, joined under the resolved directory.
const FILE_NAME: &str = "orksorksorks.toml";

/// Environment variables consulted by config-directory resolution.
///
/// All fields are `None` by default so tests get a clean slate. Production
/// code populates this via [`ConfigEnv::from_env`]; tests construct it
/// directly with the values they want to inject.
pub(crate) struct ConfigEnv {
    pub xdg_config_home: Option<OsString>,
    pub home: Option<OsString>,
    pub appdata: Option<OsString>,
}

impl Default for ConfigEnv {
    fn default() -> Self {
        Self {
            xdg_config_home: None,
            home: None,
            appdata: None,
        }
    }
}

impl ConfigEnv {
    /// Read the config-directory env vars from the process environment,
    /// populating only the fields the current platform consults.
    pub(crate) fn from_env() -> Self {
        Self {
            xdg_config_home: std::env::var_os("XDG_CONFIG_HOME"),
            home: if cfg!(windows) {
                None
            } else {
                std::env::var_os("HOME")
            },
            appdata: if cfg!(windows) {
                std::env::var_os("APPDATA")
            } else {
                None
            },
        }
    }
}

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

/// Pure resolution core — every env read is a field access on `env`.
///
/// Platform-agnostic: `ConfigEnv::from_env` populates only the fields the
/// current platform consults, so production behavior is unchanged while the
/// logic is testable on any host.
pub(crate) fn resolve_config_dir_with_env(
    env: &ConfigEnv,
) -> Result<(PathBuf, ConfigPathSource), Error> {
    // XDG wins on every platform, but only when set to an absolute path.
    // Unset, empty, and relative values all mean "fall back" (XDG spec).
    if let Some(xdg) = &env.xdg_config_home {
        let dir = PathBuf::from(xdg);
        if dir.is_absolute() {
            return Ok((dir, ConfigPathSource::XdgConfigHome));
        }
    }

    if let Some(appdata) = &env.appdata {
        return Ok((PathBuf::from(appdata), ConfigPathSource::AppData));
    }

    if let Some(home) = &env.home {
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

/// Read ambient env and resolve the config directory (thin wrapper).
fn resolve_config_dir() -> Result<(PathBuf, ConfigPathSource), Error> {
    resolve_config_dir_with_env(&ConfigEnv::from_env())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_xdg_is_used() {
        let (path, source) = resolve_config_dir_with_env(&ConfigEnv {
            xdg_config_home: Some("/tmp/ork-cfg".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(path, std::path::PathBuf::from("/tmp/ork-cfg"));
        assert_eq!(source, ConfigPathSource::XdgConfigHome);
    }

    #[test]
    fn unset_xdg_falls_back_to_home_dot_config() {
        let (path, source) = resolve_config_dir_with_env(&ConfigEnv {
            home: Some("/home/ork".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(path, std::path::PathBuf::from("/home/ork/.config"));
        assert_eq!(source, ConfigPathSource::HomeDotConfig);
    }

    #[test]
    fn empty_xdg_falls_back_to_home() {
        let (path, source) = resolve_config_dir_with_env(&ConfigEnv {
            xdg_config_home: Some("".into()),
            home: Some("/home/ork".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(path, std::path::PathBuf::from("/home/ork/.config"));
        assert_eq!(source, ConfigPathSource::HomeDotConfig);
    }

    #[test]
    fn relative_xdg_falls_back_to_home() {
        let (path, source) = resolve_config_dir_with_env(&ConfigEnv {
            xdg_config_home: Some("relative/dir".into()),
            home: Some("/home/ork".into()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(path, std::path::PathBuf::from("/home/ork/.config"));
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
    fn appdata_used_on_windows() {
        let (path, source) = resolve_config_dir_with_env(&ConfigEnv {
            appdata: Some("C:\\Users\\ork\\AppData\\Roaming".into()),
            ..Default::default()
        })
        .unwrap();
        // APPDATA returns raw: no absolute check, no .join (current behavior).
        assert_eq!(
            path,
            std::path::PathBuf::from("C:\\Users\\ork\\AppData\\Roaming")
        );
        assert_eq!(source, ConfigPathSource::AppData);
    }

    #[test]
    fn unresolved_home_yields_config_dir_error() {
        let err = resolve_config_dir_with_env(&ConfigEnv::default()).unwrap_err();
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
