use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;
use wattch_core::{compute_delta_j, time, PowercapSource, Result};
use wattch_proto::wattch::v1::{response, Response, Sample};

use crate::daemon::{error_response, send_response, DaemonState, SharedWriter, CODE_INTERNAL};

pub async fn run_stream(
    state: Arc<DaemonState>,
    stream_id: u64,
    writer: SharedWriter,
    sources: Vec<PowercapSource>,
    interval_ns: u64,
    request_id: u64,
    stop_rx: watch::Receiver<bool>,
) {
    if let Err(error) = stream_loop(&writer, &sources, interval_ns, request_id, stop_rx).await {
        let _ = send_response(
            &writer,
            &error_response(request_id, CODE_INTERNAL, error.to_string()),
        )
        .await;
    }

    state.finish_stream(stream_id).await;
}

async fn stream_loop(
    writer: &SharedWriter,
    sources: &[PowercapSource],
    interval_ns: u64,
    request_id: u64,
    mut stop_rx: watch::Receiver<bool>,
) -> Result<()> {
    let mut previous_energy = Vec::with_capacity(sources.len());
    for source in sources {
        previous_energy.push(source.read_energy_j()?);
    }

    let interval = Duration::from_nanos(interval_ns);
    loop {
        tokio::select! {
            changed = stop_rx.changed() => {
                if changed.is_err() || *stop_rx.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(interval) => {
                for (source, previous) in sources.iter().zip(previous_energy.iter_mut()) {
                    let current = source.read_energy_j()?;
                    let (delta_j, counter_wrap) =
                        compute_delta_j(*previous, current, source.max_energy_j);
                    *previous = current;

                    let response = Response {
                        request_id,
                        kind: Some(response::Kind::Sample(Sample {
                            source_id: source.source_id,
                            monotonic_ns: time::monotonic_ns(),
                            energy_j: current,
                            delta_j,
                            power_w: delta_j / (interval_ns as f64 / 1_000_000_000.0),
                            interval_ns,
                            counter_wrap,
                        })),
                    };

                    send_response(writer, &response).await?;
                }
            }
        }
    }

    Ok(())
}
