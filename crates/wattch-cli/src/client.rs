use std::path::PathBuf;

use tokio::net::UnixStream;
use wattch_core::{read_frame_async, write_frame_async, Result};
use wattch_proto::wattch::v1::{Request, Response};

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

pub async fn connect() -> Result<UnixStream> {
    Ok(UnixStream::connect(socket_path_from_env()).await?)
}

pub async fn request_response(stream: &mut UnixStream, request: &Request) -> Result<Response> {
    write_frame_async(stream, request).await?;
    read_frame_async(stream).await
}
