use std::collections::BTreeMap;
use std::time::Duration;

use tokio::net::UnixStream;
use wattch_core::{read_frame_async, SourceSummary, SummaryAggregator, WattchError};
use wattch_proto::wattch::v1::{response, Response, Sample, Source};

use crate::client;
use crate::output::csv::{format_sample_csv_header, format_sample_csv_row};
use crate::output::format::StreamFormat;
use crate::output::jsonl::format_sample_jsonl_row;
use crate::output::line::format_sample_line;
use crate::output::summary::format_summary_table;
use crate::output::table::{format_sample_table_header, format_sample_table_row};
use crate::output::SampleRow;

pub async fn run(
    interval_ms: u64,
    duration: Option<Duration>,
    requested_sources: Vec<u32>,
    format: StreamFormat,
    summary: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = client::connect().await?;
    let sources = client::list_sources(&mut stream, 1).await?;
    let selected = select_sources(&sources, &requested_sources)?;
    if selected.is_empty() {
        println!("no available sources");
        return Ok(());
    }

    let source_ids = selected.iter().map(|source| source.source_id).collect();
    let source_names = source_name_map(&selected);
    let interval_ns = interval_ms.saturating_mul(1_000_000);
    let response = client::start_stream(&mut stream, 2, source_ids, interval_ns).await?;
    expect_start_stream(response)?;

    stream_samples(&mut stream, duration, format, summary, &source_names).await
}

pub async fn stream_samples(
    stream: &mut UnixStream,
    duration: Option<Duration>,
    format: StreamFormat,
    summary_enabled: bool,
    source_names: &BTreeMap<u32, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if format == StreamFormat::Csv {
        println!("{}", format_sample_csv_header());
    } else if format == StreamFormat::Table {
        println!("{}", format_sample_table_header());
    }

    let mut first_monotonic_ns = None;
    let mut summaries = SummaryAggregator::new();
    let sleep = duration.map(tokio::time::sleep);
    tokio::pin!(sleep);

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                let _ = client::send_stop_stream(stream, 3).await;
                break;
            }
            _ = async {
                if let Some(sleep) = sleep.as_mut().as_pin_mut() {
                    sleep.await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                let _ = client::send_stop_stream(stream, 3).await;
                break;
            }
            response = read_frame_async::<_, Response>(stream) => {
                let response = match response {
                    Ok(response) => response,
                    Err(error) => {
                        eprintln!("wattch: daemon disconnected: {error}");
                        break;
                    }
                };

                match response.kind {
                    Some(response::Kind::Sample(sample)) => {
                        print_sample(&sample, format, source_names, &mut first_monotonic_ns, &mut summaries);
                    }
                    Some(response::Kind::Error(error)) => {
                        return Err(format!("daemon error {}: {}", error.code, error.message).into());
                    }
                    Some(response::Kind::StopStream(_)) => break,
                    _ => {}
                }
            }
        }
    }

    if summary_enabled {
        print_summary(&summaries.summaries());
    }

    Ok(())
}

pub fn select_sources(
    sources: &[Source],
    requested_sources: &[u32],
) -> Result<Vec<Source>, WattchError> {
    if requested_sources.is_empty() {
        return Ok(sources
            .iter()
            .filter(|source| source.available)
            .cloned()
            .collect());
    }

    let mut selected = Vec::new();
    for source_id in requested_sources {
        let source = sources
            .iter()
            .find(|source| source.source_id == *source_id)
            .ok_or(WattchError::SourceNotFound(*source_id))?;
        if !source.available {
            return Err(WattchError::SourceUnavailable(*source_id));
        }
        selected.push(source.clone());
    }

    Ok(selected)
}

pub fn source_name_map(sources: &[Source]) -> BTreeMap<u32, String> {
    sources
        .iter()
        .map(|source| (source.source_id, source.name.clone()))
        .collect()
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

fn print_sample(
    sample: &Sample,
    format: StreamFormat,
    source_names: &BTreeMap<u32, String>,
    first_monotonic_ns: &mut Option<u64>,
    summaries: &mut SummaryAggregator,
) {
    let first = *first_monotonic_ns.get_or_insert(sample.monotonic_ns);
    let elapsed_s = sample.monotonic_ns.saturating_sub(first) as f64 / 1_000_000_000.0;
    let source_name = source_names
        .get(&sample.source_id)
        .cloned()
        .unwrap_or_else(|| sample.source_id.to_string());
    summaries.observe(sample.source_id, source_name.clone(), sample);
    let row = SampleRow::from_sample(elapsed_s, source_name, sample);

    match format {
        StreamFormat::Table => println!("{}", format_sample_table_row(&row)),
        StreamFormat::Line => println!("{}", format_sample_line(&row)),
        StreamFormat::Csv => println!("{}", format_sample_csv_row(&row)),
        StreamFormat::Jsonl => println!("{}", format_sample_jsonl_row(&row)),
    }
}

fn print_summary(summaries: &[SourceSummary]) {
    println!();
    println!("{}", format_summary_table(summaries));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(source_id: u32, available: bool) -> Source {
        Source {
            source_id,
            name: format!("rapl:{source_id}"),
            kind: "rapl".to_string(),
            unit: "joule".to_string(),
            available,
        }
    }

    #[test]
    fn select_all_sources_when_none_requested() {
        let selected =
            select_sources(&[source(1, true), source(2, false)], &[]).expect("select sources");

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].source_id, 1);
    }

    #[test]
    fn select_requested_sources() {
        let selected =
            select_sources(&[source(1, true), source(2, true)], &[2]).expect("select sources");

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].source_id, 2);
    }

    #[test]
    fn select_rejects_missing_source() {
        let error = select_sources(&[source(1, true)], &[2]).expect_err("missing source");

        assert!(matches!(error, WattchError::SourceNotFound(2)));
        assert!(error.to_string().contains('2'));
    }
}
