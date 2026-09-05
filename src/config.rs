use std::env;
use std::ffi::OsString;
use std::net::{AddrParseError, IpAddr, SocketAddr};
use std::num::ParseIntError;
use std::path::PathBuf;
use std::time::Duration;

use tracing::info;

const LIBRARY_DIR_ENV: &str = "LIBRARY_DIR";
const DATA_DIR_ENV: &str = "DATA_DIR";
const DATABASE_URL_ENV: &str = "DATABASE_URL";
const HOST_ENV: &str = "HOST";
const PORT_ENV: &str = "PORT";
const SCAN_INTERVAL_ENV: &str = "SCAN_INTERVAL_SECONDS";

const DEFAULT_DATA_DIR: &str = "./data";
const DEFAULT_HOST: &str = "0.0.0.0";
const DEFAULT_PORT: u16 = 8080;
const DEFAULT_SCAN_INTERVAL_SECONDS: u64 = 3_600;
const DEFAULT_DATABASE_FILENAME: &str = "sarme.db";

#[derive(Debug)]
pub(crate) struct AppConfig {
    #[expect(dead_code, reason = "used by the upcoming library scanning service")]
    library_dir: PathBuf,
    #[expect(dead_code, reason = "used by the upcoming persistence initialization")]
    data_dir: PathBuf,
    #[expect(
        dead_code,
        reason = "used by the upcoming SQLx persistence initialization"
    )]
    database_url: String,
    listen_address: SocketAddr,
    #[expect(dead_code, reason = "used by the upcoming scheduled scanning service")]
    scan_interval: Duration,
}

impl AppConfig {
    pub(crate) const fn listen_address(&self) -> SocketAddr {
        self.listen_address
    }

    fn from_env() -> Result<Self, ConfigError> {
        let library_dir = required_directory(LIBRARY_DIR_ENV, env::var_os(LIBRARY_DIR_ENV))?;
        let data_dir = optional_path(DATA_DIR_ENV, env::var_os(DATA_DIR_ENV), DEFAULT_DATA_DIR)?;
        let database_url = optional_string(
            DATABASE_URL_ENV,
            env::var_os(DATABASE_URL_ENV),
            format!(
                "sqlite://{}",
                data_dir.join(DEFAULT_DATABASE_FILENAME).display()
            ),
        )?;
        let host = parse_host(env::var_os(HOST_ENV))?;
        let port = parse_port(env::var_os(PORT_ENV))?;
        let scan_interval = parse_scan_interval(env::var_os(SCAN_INTERVAL_ENV))?;

        Ok(Self {
            library_dir,
            data_dir,
            database_url,
            listen_address: SocketAddr::new(host, port),
            scan_interval,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("{name} is not set or is empty")]
    MissingVariable { name: &'static str },

    #[error("{name} contains non-UTF-8 data")]
    InvalidUnicode { name: &'static str },

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

    #[error("{name} must be a valid IP address, got {value}: {source}")]
    InvalidHost {
        name: &'static str,
        value: String,
        #[source]
        source: AddrParseError,
    },

    #[error("{name} must be a valid integer, got {value}: {source}")]
    InvalidInteger {
        name: &'static str,
        value: String,
        #[source]
        source: ParseIntError,
    },

    #[error("{name} must be greater than zero")]
    ZeroNotAllowed { name: &'static str },
}

fn required_directory(name: &'static str, value: Option<OsString>) -> Result<PathBuf, ConfigError> {
    let path = parse_path(name, value)?;

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

fn optional_path(
    name: &'static str,
    value: Option<OsString>,
    default: impl Into<PathBuf>,
) -> Result<PathBuf, ConfigError> {
    match value {
        Some(value) => parse_path(name, Some(value)),
        None => Ok(default.into()),
    }
}

fn parse_path(name: &'static str, value: Option<OsString>) -> Result<PathBuf, ConfigError> {
    value
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or(ConfigError::MissingVariable { name })
}

fn parse_host(value: Option<OsString>) -> Result<IpAddr, ConfigError> {
    let value = optional_string(HOST_ENV, value, DEFAULT_HOST)?;

    value.parse().map_err(|source| ConfigError::InvalidHost {
        name: HOST_ENV,
        value,
        source,
    })
}

fn parse_port(value: Option<OsString>) -> Result<u16, ConfigError> {
    parse_positive_integer(PORT_ENV, value, DEFAULT_PORT)
}

fn parse_scan_interval(value: Option<OsString>) -> Result<Duration, ConfigError> {
    let seconds = parse_positive_integer(SCAN_INTERVAL_ENV, value, DEFAULT_SCAN_INTERVAL_SECONDS)?;

    Ok(Duration::from_secs(seconds))
}

fn optional_string(
    name: &'static str,
    value: Option<OsString>,
    default: impl Into<String>,
) -> Result<String, ConfigError> {
    match value {
        Some(value) if value.is_empty() => Err(ConfigError::MissingVariable { name }),
        Some(value) => value
            .into_string()
            .map_err(|_| ConfigError::InvalidUnicode { name }),
        None => Ok(default.into()),
    }
}

fn parse_positive_integer<T>(
    name: &'static str,
    value: Option<OsString>,
    default: T,
) -> Result<T, ConfigError>
where
    T: Copy + Default + PartialEq + std::str::FromStr<Err = ParseIntError>,
{
    let Some(value) = value else {
        return Ok(default);
    };
    let value = value
        .into_string()
        .map_err(|_| ConfigError::InvalidUnicode { name })?;
    let parsed = value
        .parse()
        .map_err(|source| ConfigError::InvalidInteger {
            name,
            value,
            source,
        })?;

    if parsed == T::default() {
        return Err(ConfigError::ZeroNotAllowed { name });
    }

    Ok(parsed)
}

pub(crate) fn load() -> Result<AppConfig, ConfigError> {
    let config = AppConfig::from_env()?;
    info!("Application configuration initialized");

    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::net::{IpAddr, Ipv4Addr};
    use std::path::PathBuf;
    use std::time::Duration;

    use super::{
        ConfigError, DEFAULT_HOST, DEFAULT_PORT, DEFAULT_SCAN_INTERVAL_SECONDS, HOST_ENV,
        LIBRARY_DIR_ENV, PORT_ENV, SCAN_INTERVAL_ENV, parse_host, parse_port, parse_scan_interval,
        required_directory,
    };

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

    #[test]
    fn uses_network_defaults() {
        assert!(matches!(
            parse_host(None),
            Ok(address) if address == IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        ));
        assert!(matches!(parse_port(None), Ok(port) if port == DEFAULT_PORT));
        assert_eq!(DEFAULT_HOST, "0.0.0.0");
    }

    #[test]
    fn rejects_invalid_host() {
        let result = parse_host(Some(OsString::from("localhost")));

        assert!(matches!(
            result,
            Err(ConfigError::InvalidHost { name: HOST_ENV, .. })
        ));
    }

    #[test]
    fn rejects_invalid_port() {
        let result = parse_port(Some(OsString::from("70000")));

        assert!(matches!(
            result,
            Err(ConfigError::InvalidInteger { name: PORT_ENV, .. })
        ));
    }

    #[test]
    fn rejects_zero_scan_interval() {
        let result = parse_scan_interval(Some(OsString::from("0")));

        assert!(matches!(
            result,
            Err(ConfigError::ZeroNotAllowed {
                name: SCAN_INTERVAL_ENV
            })
        ));
    }

    #[test]
    fn uses_default_scan_interval() {
        assert!(matches!(
            parse_scan_interval(None),
            Ok(interval)
                if interval == Duration::from_secs(DEFAULT_SCAN_INTERVAL_SECONDS)
        ));
    }
}
