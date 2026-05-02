use tokio::net::UnixStream;
use wattch_core::{read_frame_async, write_frame_async, Result, ServiceConfig};
use wattch_proto::wattch::v1::{Request, Response};

pub async fn connect() -> Result<UnixStream> {
    let config = ServiceConfig::load()?;
    Ok(UnixStream::connect(config.socket_path).await?)
}

pub async fn request_response(stream: &mut UnixStream, request: &Request) -> Result<Response> {
    write_frame_async(stream, request).await?;
    read_frame_async(stream).await
}
