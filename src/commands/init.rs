use crate::errors::Error;

/// Create a new `orksorksorks.toml` file with default configuration.
pub(crate) fn init_command(path: &std::path::Path) -> Result<String, Error> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
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
        })?;
    file.write_all(include_str!("../../templates/default.toml").as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    Ok(crate::format::green_string(&format!(
        "✓ Created {}",
        path.display()
    )))
}
