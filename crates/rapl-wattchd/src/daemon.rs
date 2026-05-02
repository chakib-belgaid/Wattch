use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::net::unix::OwnedWriteHalf;
use tokio::net::UnixStream;
use tokio::sync::{watch, Mutex};
use wattch_core::{
    discover_powercap_sources, read_frame_async, validate_interval_ns, validate_source_ids,
    write_frame_async, PowercapSource, Result, WattchError, PRODUCTION_POWER_CAP_ROOT,
};
use wattch_proto::wattch::v1::{
    request, response, Error as ProtoError, HelloResponse, ListSourcesResponse, Request, Response,
    StartStreamResponse, StopStreamResponse,
};

use crate::sampler;
use crate::socket::{bind_socket, socket_path_from_env};

pub const PROTOCOL_VERSION: u32 = 1;
pub const DAEMON_VERSION: &str = "0.1.0";

pub const CODE_UNKNOWN: u32 = 1;
pub const CODE_BAD_REQUEST: u32 = 2;
pub const CODE_UNSUPPORTED_VERSION: u32 = 3;
pub const CODE_SOURCE_NOT_FOUND: u32 = 4;
pub const CODE_SOURCE_UNAVAILABLE: u32 = 5;
pub const CODE_STREAM_ALREADY_RUNNING: u32 = 6;
pub const CODE_STREAM_NOT_RUNNING: u32 = 7;
pub const CODE_INTERVAL_TOO_LOW: u32 = 8;
pub const CODE_INTERNAL: u32 = 9;

pub(crate) type SharedWriter = Arc<Mutex<OwnedWriteHalf>>;

#[derive(Debug)]
pub struct DaemonConfig {
    pub socket_path: PathBuf,
    pub powercap_root: PathBuf,
}

impl DaemonConfig {
    pub fn from_env() -> Self {
        let powercap_root = std::env::var_os("WATTCH_POWER_CAP_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(PRODUCTION_POWER_CAP_ROOT));

        Self {
            socket_path: socket_path_from_env(),
            powercap_root,
        }
    }
}

pub struct DaemonState {
    config: DaemonConfig,
    active_stream: Mutex<Option<ActiveStream>>,
    next_stream_id: AtomicU64,
}

struct ActiveStream {
    id: u64,
    stop_tx: watch::Sender<bool>,
}

impl DaemonState {
    fn new(config: DaemonConfig) -> Self {
        Self {
            config,
            active_stream: Mutex::new(None),
            next_stream_id: AtomicU64::new(1),
        }
    }

    async fn is_stream_active(&self) -> bool {
        self.active_stream.lock().await.is_some()
    }

    pub(crate) async fn finish_stream(&self, stream_id: u64) {
        let mut active_stream = self.active_stream.lock().await;
        if active_stream
            .as_ref()
            .is_some_and(|active| active.id == stream_id)
        {
            *active_stream = None;
        }
    }
}

pub async fn run_from_env() -> Result<()> {
    run(DaemonConfig::from_env()).await
}

pub async fn run(config: DaemonConfig) -> Result<()> {
    let listener = bind_socket(&config.socket_path).await?;
    let state = Arc::new(DaemonState::new(config));

    loop {
        let (stream, _) = listener.accept().await?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(error) = handle_client(stream, state).await {
                eprintln!("rapl-wattchd client error: {error}");
            }
        });
    }
}

async fn handle_client(stream: UnixStream, state: Arc<DaemonState>) -> Result<()> {
    let (mut reader, writer) = stream.into_split();
    let writer = Arc::new(Mutex::new(writer));

    loop {
        let request = match read_frame_async::<_, Request>(&mut reader).await {
            Ok(request) => request,
            Err(WattchError::Io(error)) if error.kind() == ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(error),
        };

        handle_request(request, Arc::clone(&state), Arc::clone(&writer)).await?;
    }

    Ok(())
}

async fn handle_request(
    request: Request,
    state: Arc<DaemonState>,
    writer: SharedWriter,
) -> Result<()> {
    let request_id = request.request_id;
    match request.kind {
        Some(request::Kind::Hello(hello)) => {
            if hello.protocol_version != PROTOCOL_VERSION {
                return send_response(
                    &writer,
                    &error_response(
                        request_id,
                        CODE_UNSUPPORTED_VERSION,
                        "unsupported protocol version",
                    ),
                )
                .await;
            }

            send_response(
                &writer,
                &Response {
                    request_id,
                    kind: Some(response::Kind::Hello(HelloResponse {
                        protocol_version: PROTOCOL_VERSION,
                        daemon_version: DAEMON_VERSION.to_string(),
                    })),
                },
            )
            .await
        }
        Some(request::Kind::ListSources(_)) => handle_list_sources(request_id, state, writer).await,
        Some(request::Kind::StartStream(start)) => {
            if state.is_stream_active().await {
                return send_wattch_error(&writer, request_id, WattchError::StreamAlreadyRunning)
                    .await;
            }

            if let Err(error) = validate_interval_ns(start.interval_ns) {
                return send_wattch_error(&writer, request_id, error).await;
            }

            let sources = match discover_powercap_sources(&state.config.powercap_root) {
                Ok(sources) => sources,
                Err(error) => return send_wattch_error(&writer, request_id, error).await,
            };

            if let Err(error) = validate_source_ids(&start.source_ids, &sources) {
                return send_wattch_error(&writer, request_id, error).await;
            }

            let selected_sources = select_sources(&start.source_ids, &sources);
            handle_start_stream(
                request_id,
                start.interval_ns,
                selected_sources,
                state,
                writer,
            )
            .await
        }
        Some(request::Kind::StopStream(_)) => handle_stop_stream(request_id, state, writer).await,
        None => {
            send_response(
                &writer,
                &error_response(request_id, CODE_BAD_REQUEST, "missing request kind"),
            )
            .await
        }
    }
}

async fn handle_list_sources(
    request_id: u64,
    state: Arc<DaemonState>,
    writer: SharedWriter,
) -> Result<()> {
    let sources = match discover_powercap_sources(&state.config.powercap_root) {
        Ok(sources) => sources,
        Err(error) => return send_wattch_error(&writer, request_id, error).await,
    };

    send_response(
        &writer,
        &Response {
            request_id,
            kind: Some(response::Kind::ListSources(ListSourcesResponse {
                sources: sources.iter().map(PowercapSource::to_proto).collect(),
            })),
        },
    )
    .await
}

async fn handle_start_stream(
    request_id: u64,
    interval_ns: u64,
    sources: Vec<PowercapSource>,
    state: Arc<DaemonState>,
    writer: SharedWriter,
) -> Result<()> {
    let stream_id = state.next_stream_id.fetch_add(1, Ordering::Relaxed);
    let (stop_tx, stop_rx) = watch::channel(false);

    {
        let mut active_stream = state.active_stream.lock().await;
        if active_stream.is_some() {
            return send_wattch_error(&writer, request_id, WattchError::StreamAlreadyRunning).await;
        }
        *active_stream = Some(ActiveStream {
            id: stream_id,
            stop_tx,
        });
    }

    if let Err(error) = send_response(
        &writer,
        &Response {
            request_id,
            kind: Some(response::Kind::StartStream(StartStreamResponse {
                started: true,
                effective_interval_ns: interval_ns,
            })),
        },
    )
    .await
    {
        state.finish_stream(stream_id).await;
        return Err(error);
    }

    tokio::spawn(sampler::run_stream(
        Arc::clone(&state),
        stream_id,
        writer,
        sources,
        interval_ns,
        request_id,
        stop_rx,
    ));

    Ok(())
}

async fn handle_stop_stream(
    request_id: u64,
    state: Arc<DaemonState>,
    writer: SharedWriter,
) -> Result<()> {
    let active = {
        let mut active_stream = state.active_stream.lock().await;
        active_stream.take()
    };

    let Some(active) = active else {
        return send_wattch_error(&writer, request_id, WattchError::StreamNotRunning).await;
    };

    let _ = active.stop_tx.send(true);
    send_response(
        &writer,
        &Response {
            request_id,
            kind: Some(response::Kind::StopStream(StopStreamResponse {
                stopped: true,
            })),
        },
    )
    .await
}

fn select_sources(source_ids: &[u32], sources: &[PowercapSource]) -> Vec<PowercapSource> {
    if source_ids.is_empty() {
        return sources
            .iter()
            .filter(|source| source.available)
            .cloned()
            .collect();
    }

    source_ids
        .iter()
        .filter_map(|source_id| {
            sources
                .iter()
                .find(|source| source.source_id == *source_id && source.available)
        })
        .cloned()
        .collect()
}

pub(crate) async fn send_response(writer: &SharedWriter, response: &Response) -> Result<()> {
    let mut writer = writer.lock().await;
    write_frame_async(&mut *writer, response).await
}

async fn send_wattch_error(
    writer: &SharedWriter,
    request_id: u64,
    error: WattchError,
) -> Result<()> {
    let code = error_code(&error);
    send_response(writer, &error_response(request_id, code, error.to_string())).await
}

pub(crate) fn error_response(request_id: u64, code: u32, message: impl Into<String>) -> Response {
    Response {
        request_id,
        kind: Some(response::Kind::Error(ProtoError {
            code,
            message: message.into(),
        })),
    }
}

fn error_code(error: &WattchError) -> u32 {
    match error {
        WattchError::BadRequest(_) => CODE_BAD_REQUEST,
        WattchError::SourceNotFound(_) => CODE_SOURCE_NOT_FOUND,
        WattchError::SourceUnavailable(_) => CODE_SOURCE_UNAVAILABLE,
        WattchError::StreamAlreadyRunning => CODE_STREAM_ALREADY_RUNNING,
        WattchError::StreamNotRunning => CODE_STREAM_NOT_RUNNING,
        WattchError::IntervalTooLow { .. } => CODE_INTERVAL_TOO_LOW,
        WattchError::FrameTooLarge { .. }
        | WattchError::TruncatedPayload { .. }
        | WattchError::Internal(_)
        | WattchError::Io(_)
        | WattchError::Decode(_)
        | WattchError::Encode(_) => CODE_INTERNAL,
    }
}

#[allow(dead_code)]
fn _keep_unknown_code_available() -> u32 {
    CODE_UNKNOWN
}
