use std::ffi::OsString;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::time::Duration;

use super::ConfigError;
use super::loader::{
    DEFAULT_HOST, DEFAULT_PORT, DEFAULT_SCAN_INTERVAL_SECONDS, HOST_ENV, LIBRARY_DIR_ENV, PORT_ENV,
    SCAN_INTERVAL_ENV, parse_host, parse_port, parse_scan_interval, required_directory,
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
