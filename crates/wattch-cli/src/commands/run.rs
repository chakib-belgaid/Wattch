use std::collections::BTreeMap;
use std::process::Command;
use std::time::{Duration, Instant};

use tokio::net::UnixStream;
use wattch_core::{read_frame_async, SourceSummary, SummaryAggregator};
use wattch_proto::wattch::v1::{response, Response, Sample};

use crate::client;
use crate::commands::stream::{select_sources, source_name_map};
use crate::output::format::RunFormat;
use crate::output::summary::{format_run_summary_csv, format_summary_json, format_summary_table};

pub async fn run(
    interval_ms: u64,
    requested_sources: Vec<u32>,
    format: RunFormat,
    summary_enabled: bool,
    command: Vec<String>,
) -> Result<i32, Box<dyn std::error::Error>> {
    let mut stream = client::connect().await?;
    let sources = client::list_sources(&mut stream, 1).await?;
    let selected = select_sources(&sources, &requested_sources)?;
    let source_ids = selected.iter().map(|source| source.source_id).collect();
    let source_names = source_name_map(&selected);
    let interval_ns = interval_ms.saturating_mul(1_000_000);
    let response = client::start_stream(&mut stream, 2, source_ids, interval_ns).await?;
    expect_start_stream(response)?;

    let started = Instant::now();
    let mut child = Command::new(&command[0]).args(&command[1..]).spawn()?;
    let (exit_code, summaries) =
        collect_while_child_runs(&mut stream, &mut child, &source_names).await?;
    let duration_s = started.elapsed().as_secs_f64();
    let _ = client::send_stop_stream(&mut stream, 3).await;

    print_run_output(
        &command,
        exit_code,
        duration_s,
        &summaries,
        format,
        summary_enabled,
    );
    Ok(exit_code)
}

async fn collect_while_child_runs(
    stream: &mut UnixStream,
    child: &mut std::process::Child,
    source_names: &BTreeMap<u32, String>,
) -> Result<(i32, Vec<SourceSummary>), Box<dyn std::error::Error>> {
    let mut summaries = SummaryAggregator::new();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok((130, summaries.summaries()));
            }
            response = read_frame_async::<_, Response>(stream) => {
                match response {
                    Ok(response) => handle_stream_response(response, source_names, &mut summaries)?,
                    Err(error) => {
                        eprintln!("wattch: daemon disconnected: {error}");
                        break;
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(10)) => {
                if let Some(status) = child.try_wait()? {
                    let code = status.code().unwrap_or(1);
                    return Ok((code, summaries.summaries()));
                }
            }
        }
    }

    let status = child.wait()?;
    Ok((status.code().unwrap_or(1), summaries.summaries()))
}

fn handle_stream_response(
    response: Response,
    source_names: &BTreeMap<u32, String>,
    summaries: &mut SummaryAggregator,
) -> Result<(), Box<dyn std::error::Error>> {
    match response.kind {
        Some(response::Kind::Sample(sample)) => observe_sample(&sample, source_names, summaries),
        Some(response::Kind::Error(error)) => {
            return Err(format!("daemon error {}: {}", error.code, error.message).into());
        }
        _ => {}
    }
    Ok(())
}

fn observe_sample(
    sample: &Sample,
    source_names: &BTreeMap<u32, String>,
    summaries: &mut SummaryAggregator,
) {
    let source_name = source_names
        .get(&sample.source_id)
        .cloned()
        .unwrap_or_else(|| sample.source_id.to_string());
    summaries.observe(sample.source_id, source_name, sample);
}

fn expect_start_stream(response: Response) -> Result<(), Box<dyn std::error::Error>> {
    match response.kind {
        Some(response::Kind::StartStream(_)) => Ok(()),
        Some(response::Kind::Error(error)) => {
            Err(format!("daemon error {}: {}", error.code, error.message).into())
        }
        other => Err(format!("unexpected response: {other:?}").into()),
    }
}

fn print_run_output(
    command: &[String],
    exit_code: i32,
    duration_s: f64,
    summaries: &[SourceSummary],
    format: RunFormat,
    summary_enabled: bool,
) {
    match format {
        RunFormat::Table => {
            println!("Command: {}", command.join(" "));
            println!("Exit code: {exit_code}");
            println!("Duration: {duration_s:.2}s");
            if summary_enabled {
                println!();
                println!("{}", format_summary_table(summaries));
            }
        }
        RunFormat::Json => {
            println!(
                "{{\"command\":\"{}\",\"exit_code\":{},\"duration_s\":{},\"summary\":{}}}",
                crate::output::json_escape(&command.join(" ")),
                exit_code,
                duration_s,
                if summary_enabled {
                    format_summary_json(summaries)
                } else {
                    "[]".to_string()
                }
            );
        }
        RunFormat::Csv => {
            if summary_enabled {
                println!(
                    "{}",
                    format_run_summary_csv(&command.join(" "), exit_code, duration_s, summaries)
                );
            } else {
                println!("command,exit_code,duration_s");
                println!("{},{},{}", command.join(" "), exit_code, duration_s);
            }
        }
    }
}
