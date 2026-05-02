use std::fs;
use std::path::{Path, PathBuf};

use crate::errors::{Result, WattchError};
use crate::sources::powercap::PRODUCTION_POWER_CAP_ROOT;

pub const DEFAULT_CONFIG_PATH: &str = "/etc/wattch/wattch.conf";
pub const DEFAULT_SOCKET_PATH: &str = "/run/wattch/wattch.sock";
pub const DEFAULT_SOCKET_MODE: u32 = 0o600;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceConfig {
    pub socket_path: PathBuf,
    pub socket_mode: u32,
    pub socket_uid: Option<u32>,
    pub socket_gid: Option<u32>,
    pub powercap_root: PathBuf,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from(DEFAULT_SOCKET_PATH),
            socket_mode: DEFAULT_SOCKET_MODE,
            socket_uid: sudo_env_id("SUDO_UID"),
            socket_gid: sudo_env_id("SUDO_GID"),
            powercap_root: PathBuf::from(PRODUCTION_POWER_CAP_ROOT),
        }
    }
}

impl ServiceConfig {
    pub fn load() -> Result<Self> {
        let config_path = std::env::var_os("WATTCH_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH));
        Self::load_from_path(&config_path)
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        let mut config = Self::default();

        match fs::read_to_string(path) {
            Ok(contents) => apply_config_file(&mut config, &contents)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        if let Some(socket_path) = std::env::var_os("WATTCH_SOCKET") {
            config.socket_path = PathBuf::from(socket_path);
        }

        if let Some(powercap_root) = std::env::var_os("WATTCH_POWER_CAP_ROOT") {
            config.powercap_root = PathBuf::from(powercap_root);
        }

        Ok(config)
    }
}

fn apply_config_file(config: &mut ServiceConfig, contents: &str) -> Result<()> {
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line
            .split_once('#')
            .map_or(raw_line, |(before, _)| before);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            return Err(WattchError::BadRequest(format!(
                "invalid config line {}: expected key = value",
                line_index + 1
            )));
        };

        let key = key.trim();
        let value = unquote(value.trim());

        match key {
            "socket_path" => config.socket_path = PathBuf::from(value),
            "socket_mode" => config.socket_mode = parse_socket_mode(value)?,
            "socket_uid" => config.socket_uid = Some(parse_id("socket_uid", value)?),
            "socket_gid" => config.socket_gid = Some(parse_id("socket_gid", value)?),
            "powercap_root" => config.powercap_root = PathBuf::from(value),
            _ => {}
        }
    }

    Ok(())
}

fn unquote(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
}

fn parse_socket_mode(value: &str) -> Result<u32> {
    let value = value
        .strip_prefix("0o")
        .or_else(|| value.strip_prefix("0O"))
        .unwrap_or(value);
    let value = value.strip_prefix('0').unwrap_or(value);

    u32::from_str_radix(value, 8).map_err(|error| {
        WattchError::BadRequest(format!("invalid socket_mode value {value:?}: {error}"))
    })
}

fn parse_id(name: &str, value: &str) -> Result<u32> {
    value.parse::<u32>().map_err(|error| {
        WattchError::BadRequest(format!("invalid {name} value {value:?}: {error}"))
    })
}

fn sudo_env_id(name: &str) -> Option<u32> {
    std::env::var(name).ok()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_to_root_service_socket() {
        let config = ServiceConfig {
            socket_uid: None,
            socket_gid: None,
            ..ServiceConfig::default()
        };

        assert_eq!(config.socket_path, PathBuf::from("/run/wattch/wattch.sock"));
        assert_eq!(config.socket_mode, 0o600);
    }

    #[test]
    fn config_parses_socket_path_owner_and_mode() {
        let mut config = ServiceConfig::default();
        apply_config_file(
            &mut config,
            r#"
            # service socket for root daemon and user CLI
            socket_path = "/tmp/custom-wattch.sock"
            socket_mode = 0600
            socket_uid = 1000
            socket_gid = 1000
            powercap_root = "/tmp/fake-powercap"
            "#,
        )
        .expect("parse config");

        assert_eq!(config.socket_path, PathBuf::from("/tmp/custom-wattch.sock"));
        assert_eq!(config.socket_mode, 0o600);
        assert_eq!(config.socket_uid, Some(1000));
        assert_eq!(config.socket_gid, Some(1000));
        assert_eq!(config.powercap_root, PathBuf::from("/tmp/fake-powercap"));
    }
}
