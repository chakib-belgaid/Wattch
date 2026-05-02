use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use tokio::net::{UnixListener, UnixStream};
use wattch_core::{Result, WattchError};

pub async fn bind_socket(
    path: &Path,
    socket_mode: u32,
    socket_uid: Option<u32>,
    socket_gid: Option<u32>,
) -> Result<UnixListener> {
    if let Some(parent) = path.parent() {
        let parent_existed = parent.exists();
        fs::create_dir_all(parent)?;
        if !parent_existed {
            fs::set_permissions(parent, fs::Permissions::from_mode(0o755))?;
        }
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
    set_socket_owner(path, socket_uid, socket_gid)?;
    fs::set_permissions(path, fs::Permissions::from_mode(socket_mode))?;
    Ok(listener)
}

fn set_socket_owner(path: &Path, socket_uid: Option<u32>, socket_gid: Option<u32>) -> Result<()> {
    if socket_uid.is_none() && socket_gid.is_none() {
        return Ok(());
    }

    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| WattchError::BadRequest("socket path contains a NUL byte".to_string()))?;
    let uid = socket_uid
        .map(|uid| uid as libc::uid_t)
        .unwrap_or(!0 as libc::uid_t);
    let gid = socket_gid
        .map(|gid| gid as libc::gid_t)
        .unwrap_or(!0 as libc::gid_t);

    // libc::chown is the small, direct primitive needed to hand the root-created
    // socket to the non-root user that will run the CLI.
    let result = unsafe { libc::chown(path.as_ptr(), uid, gid) };
    if result == -1 {
        return Err(std::io::Error::last_os_error().into());
    }

    Ok(())
}
