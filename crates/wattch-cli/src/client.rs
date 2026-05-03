use tokio::net::UnixStream;
use wattch_core::{read_frame_async, write_frame_async, Result, ServiceConfig};
use wattch_proto::wattch::v1::{
    request, response, HelloRequest, ListSourcesRequest, Request, Response, Source,
    StartStreamRequest, StopStreamRequest,
};

pub const PROTOCOL_VERSION: u32 = 1;

pub async fn connect() -> Result<UnixStream> {
    let config = ServiceConfig::load()?;
    Ok(UnixStream::connect(config.socket_path).await?)
}

pub async fn connect_with_config() -> Result<(UnixStream, ServiceConfig)> {
    let config = ServiceConfig::load()?;
    let stream = UnixStream::connect(&config.socket_path).await?;
    Ok((stream, config))
}

pub async fn request_response(stream: &mut UnixStream, request: &Request) -> Result<Response> {
    write_frame_async(stream, request).await?;
    read_frame_async(stream).await
}

pub async fn hello(stream: &mut UnixStream, request_id: u64) -> Result<Response> {
    request_response(
        stream,
        &Request {
            request_id,
            kind: Some(request::Kind::Hello(HelloRequest {
                protocol_version: PROTOCOL_VERSION,
                client_name: "wattch".to_string(),
            })),
        },
    )
    .await
}

pub async fn list_sources(stream: &mut UnixStream, request_id: u64) -> Result<Vec<Source>> {
    let response = request_response(
        stream,
        &Request {
            request_id,
            kind: Some(request::Kind::ListSources(ListSourcesRequest {})),
        },
    )
    .await?;

    match response.kind {
        Some(response::Kind::ListSources(list)) => Ok(list.sources),
        Some(response::Kind::Error(error)) => Err(wattch_core::WattchError::BadRequest(format!(
            "daemon error {}: {}",
            error.code, error.message
        ))),
        other => Err(wattch_core::WattchError::BadRequest(format!(
            "unexpected response: {other:?}"
        ))),
    }
}

pub async fn start_stream(
    stream: &mut UnixStream,
    request_id: u64,
    source_ids: Vec<u32>,
    interval_ns: u64,
) -> Result<Response> {
    request_response(
        stream,
        &Request {
            request_id,
            kind: Some(request::Kind::StartStream(StartStreamRequest {
                source_ids,
                interval_ns,
                include_raw: false,
            })),
        },
    )
    .await
}

pub async fn send_stop_stream(stream: &mut UnixStream, request_id: u64) -> Result<()> {
    write_frame_async(
        stream,
        &Request {
            request_id,
            kind: Some(request::Kind::StopStream(StopStreamRequest {})),
        },
    )
    .await
}
