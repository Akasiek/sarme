use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing::info;

const LIBRARY_DIR_ENV: &str = "LIBRARY_DIR";

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

#[derive(Debug)]
pub(crate) struct AppConfig {
    library_dir: PathBuf,
}

impl AppConfig {
    pub(crate) fn library_dir(&self) -> &Path {
        &self.library_dir
    }

    fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            library_dir: required_directory(LIBRARY_DIR_ENV, env::var_os(LIBRARY_DIR_ENV))?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("{name} is not set or is empty")]
    MissingVariable { name: &'static str },

    #[error("{name} must be an absolute path, got {}", path.display())]
    DirectoryNotAbsolute { name: &'static str, path: PathBuf },

    #[error("directory configured by {name} at {} is not accessible: {source}", path.display())]
    DirectoryUnavailable {
        name: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("path configured by {name} at {} is not a directory", path.display())]
    PathNotDirectory { name: &'static str, path: PathBuf },

    #[error("application configuration could not be initialized")]
    InitializationFailed,
}

fn required_directory(name: &'static str, value: Option<OsString>) -> Result<PathBuf, ConfigError> {
    let path = value
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or(ConfigError::MissingVariable { name })?;

    if !path.is_absolute() {
        return Err(ConfigError::DirectoryNotAbsolute { name, path });
    }

    let metadata = path
        .metadata()
        .map_err(|source| ConfigError::DirectoryUnavailable {
            name,
            path: path.clone(),
            source,
        })?;

    if !metadata.is_dir() {
        return Err(ConfigError::PathNotDirectory { name, path });
    }

    Ok(path)
}

pub(crate) fn init() -> Result<&'static AppConfig, ConfigError> {
    if let Some(config) = CONFIG.get() {
        return Ok(config);
    }

    let config = AppConfig::from_env()?;
    let _already_initialized = CONFIG.set(config).is_err();

    let config = CONFIG.get().ok_or(ConfigError::InitializationFailed)?;
    info!("Application configuration initialized");

    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::{ConfigError, LIBRARY_DIR_ENV, required_directory};

    #[test]
    fn rejects_missing_library_directory() {
        let result = required_directory(LIBRARY_DIR_ENV, None);

        assert!(matches!(
            result,
            Err(ConfigError::MissingVariable {
                name: LIBRARY_DIR_ENV
            })
        ));
    }

    #[test]
    fn rejects_empty_library_directory() {
        let result = required_directory(LIBRARY_DIR_ENV, Some(OsString::new()));

        assert!(matches!(
            result,
            Err(ConfigError::MissingVariable {
                name: LIBRARY_DIR_ENV
            })
        ));
    }

    #[test]
    fn rejects_relative_library_directory() {
        let result = required_directory(LIBRARY_DIR_ENV, Some(OsString::from("music")));

        assert!(matches!(
            result,
            Err(ConfigError::DirectoryNotAbsolute {
                name: LIBRARY_DIR_ENV,
                path
            }) if path == PathBuf::from("music")
        ));
    }

    #[test]
    fn accepts_existing_library_directory() {
        let directory = std::env::temp_dir();
        let result = required_directory(LIBRARY_DIR_ENV, Some(directory.clone().into_os_string()));

        assert!(matches!(result, Ok(path) if path == directory));
    }

    #[test]
    fn rejects_unavailable_library_directory() {
        let path = std::env::temp_dir().join("sarme-config-test-directory-that-does-not-exist");
        let result = required_directory(LIBRARY_DIR_ENV, Some(path.clone().into_os_string()));

        assert!(matches!(
            result,
            Err(ConfigError::DirectoryUnavailable {
                name: LIBRARY_DIR_ENV,
                path: error_path,
                ..
            }) if error_path == path
        ));
    }

    #[test]
    fn rejects_file_as_library_directory() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let result = required_directory(LIBRARY_DIR_ENV, Some(path.clone().into_os_string()));

        assert!(matches!(
            result,
            Err(ConfigError::PathNotDirectory {
                name: LIBRARY_DIR_ENV,
                path: error_path
            }) if error_path == path
        ));
    }
}
