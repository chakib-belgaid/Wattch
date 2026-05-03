use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use tempfile::TempDir;
use tokio::net::UnixStream;
use wattch_core::{read_frame_async, write_frame_async};
use wattch_proto::wattch::v1::{
    request, response, HelloRequest, ListSourcesRequest, Request, Response, StartStreamRequest,
    StopStreamRequest,
};

struct DaemonProcess {
    child: Child,
}

impl Drop for DaemonProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

async fn spawn_daemon(socket_path: &Path, powercap_root: &Path) -> DaemonProcess {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rapl-wattchd"))
        .env("WATTCH_SOCKET", socket_path)
        .env("WATTCH_POWER_CAP_ROOT", powercap_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon");

    for _ in 0..100 {
        if UnixStream::connect(socket_path).await.is_ok() {
            return DaemonProcess { child };
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let _ = child.kill();
    let _ = child.wait();
    panic!("daemon did not create socket at {}", socket_path.display());
}

fn socket_path(temp: &TempDir) -> PathBuf {
    temp.path().join("wattch.sock")
}

fn write_source(root: &Path, relative: &str, name: &str, energy_uj: u64, max_uj: u64) -> PathBuf {
    let dir = root.join(relative);
    fs::create_dir_all(&dir).expect("create fake source dir");
    fs::write(dir.join("name"), name).expect("write name");
    fs::write(dir.join("energy_uj"), energy_uj.to_string()).expect("write energy");
    fs::write(dir.join("max_energy_range_uj"), max_uj.to_string()).expect("write max");
    dir
}

fn replace_energy_uj(source_dir: &Path, energy_uj: u64) {
    let temp_path = source_dir.join("energy_uj.tmp");
    fs::write(&temp_path, energy_uj.to_string()).expect("write temp energy");
    fs::rename(temp_path, source_dir.join("energy_uj")).expect("replace energy");
}

async fn roundtrip(stream: &mut UnixStream, request: &Request) -> Response {
    write_frame_async(stream, request)
        .await
        .expect("write request");
    read_frame_async(stream).await.expect("read response")
}

#[tokio::test]
async fn daemon_hello_roundtrip_over_unix_socket() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let _daemon = spawn_daemon(&socket, temp.path()).await;

    let mut stream = UnixStream::connect(&socket).await.expect("connect daemon");
    let response = roundtrip(
        &mut stream,
        &Request {
            request_id: 1,
            kind: Some(request::Kind::Hello(HelloRequest {
                protocol_version: 1,
                client_name: "integration-test".to_string(),
            })),
        },
    )
    .await;

    match response.kind {
        Some(response::Kind::Hello(hello)) => {
            assert_eq!(hello.protocol_version, 1);
            assert_eq!(hello.daemon_version, "0.1.0");
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[tokio::test]
async fn daemon_list_sources_over_unix_socket_with_fake_powercap_root() {
    let temp = TempDir::new().expect("tempdir");
    write_source(
        temp.path(),
        "intel-rapl/intel-rapl:0",
        "package-0",
        1_000_000,
        262_143_000_000,
    );
    write_source(
        temp.path(),
        "intel-rapl/intel-rapl:0/intel-rapl:0:0",
        "core",
        500_000,
        262_143_000_000,
    );
    let socket = socket_path(&temp);
    let _daemon = spawn_daemon(&socket, temp.path()).await;

    let mut stream = UnixStream::connect(&socket).await.expect("connect daemon");
    let response = roundtrip(
        &mut stream,
        &Request {
            request_id: 2,
            kind: Some(request::Kind::ListSources(ListSourcesRequest {})),
        },
    )
    .await;

    match response.kind {
        Some(response::Kind::ListSources(list)) => {
            assert_eq!(list.sources.len(), 2);
            assert_eq!(list.sources[0].name, "rapl:package-0");
            assert_eq!(list.sources[1].name, "rapl:core");
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[tokio::test]
async fn daemon_start_stream_emits_samples_with_fake_powercap_root() {
    let temp = TempDir::new().expect("tempdir");
    let source_dir = write_source(
        temp.path(),
        "intel-rapl/intel-rapl:0",
        "package-0",
        1_000_000,
        262_143_000_000,
    );
    let socket = socket_path(&temp);
    let _daemon = spawn_daemon(&socket, temp.path()).await;

    let mut stream = UnixStream::connect(&socket).await.expect("connect daemon");
    let response = roundtrip(
        &mut stream,
        &Request {
            request_id: 3,
            kind: Some(request::Kind::StartStream(StartStreamRequest {
                source_ids: Vec::new(),
                interval_ns: 10_000_000,
                include_raw: false,
            })),
        },
    )
    .await;
    assert!(matches!(
        response.kind,
        Some(response::Kind::StartStream(_))
    ));

    replace_energy_uj(&source_dir, 2_000_000);
    let sample_response: Response = read_frame_async(&mut stream).await.expect("read sample");
    match sample_response.kind {
        Some(response::Kind::Sample(sample)) => {
            assert_eq!(sample.source_id, 1);
            assert_eq!(sample.energy_j, 2.0);
            assert_eq!(sample.interval_ns, 10_000_000);
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[tokio::test]
async fn daemon_rejects_second_active_stream() {
    let temp = TempDir::new().expect("tempdir");
    write_source(
        temp.path(),
        "intel-rapl/intel-rapl:0",
        "package-0",
        1_000_000,
        262_143_000_000,
    );
    let socket = socket_path(&temp);
    let _daemon = spawn_daemon(&socket, temp.path()).await;

    let mut first = UnixStream::connect(&socket).await.expect("connect daemon");
    let first_response = roundtrip(
        &mut first,
        &Request {
            request_id: 4,
            kind: Some(request::Kind::StartStream(StartStreamRequest {
                source_ids: Vec::new(),
                interval_ns: 100_000_000,
                include_raw: false,
            })),
        },
    )
    .await;
    assert!(matches!(
        first_response.kind,
        Some(response::Kind::StartStream(_))
    ));

    let mut second = UnixStream::connect(&socket).await.expect("connect daemon");
    let second_response = roundtrip(
        &mut second,
        &Request {
            request_id: 5,
            kind: Some(request::Kind::StartStream(StartStreamRequest {
                source_ids: Vec::new(),
                interval_ns: 100_000_000,
                include_raw: false,
            })),
        },
    )
    .await;

    match second_response.kind {
        Some(response::Kind::Error(error)) => assert_eq!(error.code, 6),
        other => panic!("unexpected response: {other:?}"),
    }
}

#[tokio::test]
async fn daemon_stop_without_stream_returns_error() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let _daemon = spawn_daemon(&socket, temp.path()).await;

    let mut stream = UnixStream::connect(&socket).await.expect("connect daemon");
    let response = roundtrip(
        &mut stream,
        &Request {
            request_id: 6,
            kind: Some(request::Kind::StopStream(StopStreamRequest {})),
        },
    )
    .await;

    match response.kind {
        Some(response::Kind::Error(error)) => assert_eq!(error.code, 7),
        other => panic!("unexpected response: {other:?}"),
    }
}
