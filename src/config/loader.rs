use std::env;
use std::ffi::OsString;
use std::net::{IpAddr, SocketAddr};
use std::num::ParseIntError;
use std::path::PathBuf;
use std::time::Duration;

use tracing::info;

use super::{AppConfig, ConfigError};

pub(super) const LIBRARY_DIR_ENV: &str = "LIBRARY_DIR";
const DATA_DIR_ENV: &str = "DATA_DIR";
const DATABASE_URL_ENV: &str = "DATABASE_URL";
pub(super) const HOST_ENV: &str = "HOST";
pub(super) const PORT_ENV: &str = "PORT";
pub(super) const SCAN_INTERVAL_ENV: &str = "SCAN_INTERVAL_SECONDS";

const DEFAULT_DATA_DIR: &str = "./data";
pub(super) const DEFAULT_HOST: &str = "0.0.0.0";
pub(super) const DEFAULT_PORT: u16 = 8080;
pub(super) const DEFAULT_SCAN_INTERVAL_SECONDS: u64 = 3_600;
const DEFAULT_DATABASE_FILENAME: &str = "sarme.db";

impl AppConfig {
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

pub(super) fn required_directory(
    name: &'static str,
    value: Option<OsString>,
) -> Result<PathBuf, ConfigError> {
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

pub(super) fn parse_host(value: Option<OsString>) -> Result<IpAddr, ConfigError> {
    let value = optional_string(HOST_ENV, value, DEFAULT_HOST)?;

    value.parse().map_err(|source| ConfigError::InvalidHost {
        name: HOST_ENV,
        value,
        source,
    })
}

pub(super) fn parse_port(value: Option<OsString>) -> Result<u16, ConfigError> {
    parse_positive_integer(PORT_ENV, value, DEFAULT_PORT)
}

pub(super) fn parse_scan_interval(value: Option<OsString>) -> Result<Duration, ConfigError> {
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
