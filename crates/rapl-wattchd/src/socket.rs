use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use tokio::net::{UnixListener, UnixStream};
use wattch_core::{Result, WattchError};

pub fn socket_path_from_env() -> PathBuf {
    if let Some(path) = std::env::var_os("WATTCH_SOCKET") {
        return PathBuf::from(path);
    }

    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir).join("wattch.sock");
    }

    let uid = std::env::var("UID").unwrap_or_else(|_| "0".to_string());
    PathBuf::from(format!("/tmp/wattch-{uid}.sock"))
}

pub async fn bind_socket(path: &Path) -> Result<UnixListener> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    if path.exists() {
        match UnixStream::connect(path).await {
            Ok(_) => {
                return Err(WattchError::Internal(format!(
                    "server already listening at {}",
                    path.display()
                )));
            }
            Err(_) => fs::remove_file(path)?,
        }
    }

    let listener = UnixListener::bind(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}
