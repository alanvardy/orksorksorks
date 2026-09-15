use crate::errors::Error;

/// Content of the default config template, embedded at compile time.
const DEFAULT_TEMPLATE: &str = include_str!("../../templates/default.toml");

/// Create a new `orksorksorks.toml` file with default configuration.
pub(crate) fn init_command(path: &std::path::Path) -> Result<String, Error> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = create_new_file(path)?;
    file.write_all(DEFAULT_TEMPLATE.as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    Ok(crate::format::green_string(&format!(
        "✓ Created {}",
        path.display()
    )))
}

/// Open `path` with create-new semantics (fails on any existing path — file
/// or symlink — without following), mapping an already-existing path to the
/// `config-exists` error instead of a raw I/O failure so callers can
/// distinguish "refused to overwrite" from other open errors.
fn create_new_file(path: &std::path::Path) -> Result<std::fs::File, Error> {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Error::new(
                    "config-exists",
                    &format!("{} already exists; not overwriting", path.display()),
                )
            } else {
                Error::from(e)
            }
        })
}
