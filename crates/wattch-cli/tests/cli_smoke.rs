use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;
use tokio::net::UnixListener;
use wattch_core::{read_frame_async, write_frame_async};
use wattch_proto::wattch::v1::{
    response, Error as ProtoError, HelloResponse, ListSourcesResponse, Request, Response, Source,
};

fn socket_path(temp: &TempDir) -> PathBuf {
    temp.path().join("wattch.sock")
}

async fn spawn_fake_daemon(socket: &Path, response: Response) -> tokio::task::JoinHandle<()> {
    let listener = UnixListener::bind(socket).expect("bind fake daemon socket");
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept cli");
        let request: Request = read_frame_async(&mut stream).await.expect("read request");
        let response = Response {
            request_id: request.request_id,
            kind: response.kind,
        };
        write_frame_async(&mut stream, &response)
            .await
            .expect("write response");
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_hello_smoke_test() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let fake_daemon = spawn_fake_daemon(
        &socket,
        Response {
            request_id: 0,
            kind: Some(response::Kind::Hello(HelloResponse {
                protocol_version: 1,
                daemon_version: "0.1.0".to_string(),
            })),
        },
    )
    .await;

    Command::cargo_bin("wattch-cli")
        .expect("cli binary")
        .arg("hello")
        .env("WATTCH_SOCKET", &socket)
        .assert()
        .success()
        .stdout(contains("daemon version 0.1.0"));

    fake_daemon.await.expect("fake daemon task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_sources_smoke_test_with_fake_daemon_or_fake_powercap() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let fake_daemon = spawn_fake_daemon(
        &socket,
        Response {
            request_id: 0,
            kind: Some(response::Kind::ListSources(ListSourcesResponse {
                sources: vec![Source {
                    source_id: 1,
                    name: "rapl:package-0".to_string(),
                    kind: "rapl".to_string(),
                    unit: "joule".to_string(),
                    available: true,
                }],
            })),
        },
    )
    .await;

    Command::cargo_bin("wattch-cli")
        .expect("cli binary")
        .arg("sources")
        .env("WATTCH_SOCKET", &socket)
        .assert()
        .success()
        .stdout(contains("rapl:package-0"));

    fake_daemon.await.expect("fake daemon task");
}

#[allow(dead_code)]
fn _example_error_response() -> Response {
    Response {
        request_id: 0,
        kind: Some(response::Kind::Error(ProtoError {
            code: 9,
            message: "internal".to_string(),
        })),
    }
}
