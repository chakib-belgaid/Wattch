use std::path::{Path, PathBuf};
use std::time::Duration;

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;
use tokio::net::UnixListener;
use wattch_core::{read_frame_async, write_frame_async};
use wattch_proto::wattch::v1::{
    request, response, HelloResponse, ListSourcesResponse, Request, Response, Sample, Source,
    StartStreamResponse, StopStreamResponse,
};

fn socket_path(temp: &TempDir) -> PathBuf {
    temp.path().join("wattch.sock")
}

fn sources() -> Vec<Source> {
    vec![
        Source {
            source_id: 1,
            name: "rapl:package-0".to_string(),
            kind: "rapl".to_string(),
            unit: "joule".to_string(),
            available: true,
        },
        Source {
            source_id: 2,
            name: "rapl:core".to_string(),
            kind: "rapl".to_string(),
            unit: "joule".to_string(),
            available: true,
        },
    ]
}

async fn spawn_fake_daemon(socket: &Path) -> tokio::task::JoinHandle<()> {
    let listener = UnixListener::bind(socket).expect("bind fake daemon socket");
    let sources = sources();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept cli");
        loop {
            let request: Request = match read_frame_async(&mut stream).await {
                Ok(request) => request,
                Err(_) => break,
            };

            match request.kind {
                Some(request::Kind::Hello(_)) => {
                    write_frame_async(
                        &mut stream,
                        &Response {
                            request_id: request.request_id,
                            kind: Some(response::Kind::Hello(HelloResponse {
                                protocol_version: 1,
                                daemon_version: "0.1.0".to_string(),
                            })),
                        },
                    )
                    .await
                    .expect("write hello");
                }
                Some(request::Kind::ListSources(_)) => {
                    write_frame_async(
                        &mut stream,
                        &Response {
                            request_id: request.request_id,
                            kind: Some(response::Kind::ListSources(ListSourcesResponse {
                                sources: sources.clone(),
                            })),
                        },
                    )
                    .await
                    .expect("write sources");
                }
                Some(request::Kind::StartStream(_)) => {
                    write_frame_async(
                        &mut stream,
                        &Response {
                            request_id: request.request_id,
                            kind: Some(response::Kind::StartStream(StartStreamResponse {
                                started: true,
                                effective_interval_ns: 10_000_000,
                            })),
                        },
                    )
                    .await
                    .expect("write start");

                    for index in 0..5 {
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        if write_frame_async(
                            &mut stream,
                            &Response {
                                request_id: request.request_id,
                                kind: Some(response::Kind::Sample(Sample {
                                    source_id: 1,
                                    monotonic_ns: (index + 1) * 100_000_000,
                                    energy_j: 100.0 + index as f64,
                                    delta_j: 0.5,
                                    power_w: 5.0 + index as f64,
                                    interval_ns: 100_000_000,
                                    counter_wrap: false,
                                })),
                            },
                        )
                        .await
                        .is_err()
                        {
                            break;
                        }
                    }
                }
                Some(request::Kind::StopStream(_)) => {
                    let _ = write_frame_async(
                        &mut stream,
                        &Response {
                            request_id: request.request_id,
                            kind: Some(response::Kind::StopStream(StopStreamResponse {
                                stopped: true,
                            })),
                        },
                    )
                    .await;
                    break;
                }
                None => break,
            }
        }
    })
}

fn wattch(socket: &Path) -> Command {
    let mut command = Command::cargo_bin("wattch").expect("cli binary");
    command.env("WATTCH_SOCKET", socket);
    command
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_hello_prints_daemon_info() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let fake_daemon = spawn_fake_daemon(&socket).await;

    wattch(&socket)
        .arg("hello")
        .assert()
        .success()
        .stdout(contains("Wattch daemon: 0.1.0"))
        .stdout(contains("Protocol: 1"))
        .stdout(contains(socket.display().to_string()));

    fake_daemon.await.expect("fake daemon task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_sources_prints_table() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let fake_daemon = spawn_fake_daemon(&socket).await;

    wattch(&socket)
        .arg("sources")
        .assert()
        .success()
        .stdout(contains("ID"))
        .stdout(contains("rapl:package-0"));

    fake_daemon.await.expect("fake daemon task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_sources_prints_csv() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let fake_daemon = spawn_fake_daemon(&socket).await;

    wattch(&socket)
        .args(["sources", "--format", "csv"])
        .assert()
        .success()
        .stdout(contains("id,name,kind,unit,available"))
        .stdout(contains("1,rapl:package-0,rapl,joule,true"));

    fake_daemon.await.expect("fake daemon task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_stream_prints_csv_samples() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let fake_daemon = spawn_fake_daemon(&socket).await;

    wattch(&socket)
        .args(["stream", "--format", "csv", "--duration", "60ms"])
        .assert()
        .success()
        .stdout(contains(
            "elapsed_s,source_id,source_name,energy_j,delta_j,power_w,interval_ns,counter_wrap",
        ))
        .stdout(contains("rapl:package-0"));

    fake_daemon.await.expect("fake daemon task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_stream_stops_after_duration() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let fake_daemon = spawn_fake_daemon(&socket).await;

    wattch(&socket)
        .args(["stream", "--duration", "30ms"])
        .assert()
        .success()
        .stdout(contains("TIME"));

    fake_daemon.await.expect("fake daemon task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_stream_rejects_invalid_source() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let fake_daemon = spawn_fake_daemon(&socket).await;

    wattch(&socket)
        .args(["stream", "--source", "99", "--duration", "10ms"])
        .assert()
        .failure()
        .stderr(contains("source not found: 99"));

    fake_daemon.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_run_executes_command_and_prints_summary() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let fake_daemon = spawn_fake_daemon(&socket).await;

    wattch(&socket)
        .args(["run", "--", "sh", "-c", "sleep 0.08"])
        .assert()
        .success()
        .stdout(contains("Command: sh -c sleep 0.08"))
        .stdout(contains("SOURCE"))
        .stdout(contains("rapl:package-0"));

    fake_daemon.await.expect("fake daemon task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_run_prints_csv_summary() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let fake_daemon = spawn_fake_daemon(&socket).await;

    wattch(&socket)
        .args(["run", "--format", "csv", "--", "sh", "-c", "sleep 0.08"])
        .assert()
        .success()
        .stdout(contains("command,exit_code,duration_s"))
        .stdout(contains("rapl:package-0"));

    fake_daemon.await.expect("fake daemon task");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_run_preserves_child_exit_code() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let fake_daemon = spawn_fake_daemon(&socket).await;

    wattch(&socket)
        .args(["run", "--", "sh", "-c", "exit 7"])
        .assert()
        .code(7);

    fake_daemon.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cli_run_stops_stream_after_child_exit() {
    let temp = TempDir::new().expect("tempdir");
    let socket = socket_path(&temp);
    let fake_daemon = spawn_fake_daemon(&socket).await;

    wattch(&socket)
        .args(["run", "--", "sh", "-c", "sleep 0.02"])
        .assert()
        .success()
        .stdout(contains("Exit code: 0"));

    fake_daemon.await.expect("fake daemon task");
}
