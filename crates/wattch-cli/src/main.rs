mod client;

use clap::{Parser, Subcommand};
use wattch_core::{read_frame_async, write_frame_async};
use wattch_proto::wattch::v1::{
    request, response, HelloRequest, ListSourcesRequest, Request, Response, StartStreamRequest,
};

const PROTOCOL_VERSION: u32 = 1;
const DEFAULT_STREAM_INTERVAL_MS: u64 = 100;

#[derive(Debug, Parser)]
#[command(name = "wattch-cli")]
#[command(about = "Minimal Wattch client")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Hello,
    Sources,
    Stream {
        #[arg(long, default_value_t = DEFAULT_STREAM_INTERVAL_MS)]
        interval_ms: u64,
    },
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("wattch-cli: {error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Hello => hello().await?,
        Command::Sources => sources().await?,
        Command::Stream { interval_ms } => stream(interval_ms).await?,
    }

    Ok(())
}

async fn hello() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = client::connect().await?;
    let response = client::request_response(
        &mut stream,
        &Request {
            request_id: 1,
            kind: Some(request::Kind::Hello(HelloRequest {
                protocol_version: PROTOCOL_VERSION,
                client_name: "wattch-cli".to_string(),
            })),
        },
    )
    .await?;

    match response.kind {
        Some(response::Kind::Hello(hello)) => {
            println!("daemon version {}", hello.daemon_version);
            Ok(())
        }
        Some(response::Kind::Error(error)) => {
            Err(format!("daemon error {}: {}", error.code, error.message).into())
        }
        other => Err(format!("unexpected response: {other:?}").into()),
    }
}

async fn sources() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = client::connect().await?;
    let response = client::request_response(
        &mut stream,
        &Request {
            request_id: 1,
            kind: Some(request::Kind::ListSources(ListSourcesRequest {})),
        },
    )
    .await?;

    match response.kind {
        Some(response::Kind::ListSources(list)) => {
            for source in list.sources {
                let availability = if source.available {
                    "available"
                } else {
                    "unavailable"
                };
                println!(
                    "{} {} {} {} {}",
                    source.source_id, source.name, source.kind, source.unit, availability
                );
            }
            Ok(())
        }
        Some(response::Kind::Error(error)) => {
            Err(format!("daemon error {}: {}", error.code, error.message).into())
        }
        other => Err(format!("unexpected response: {other:?}").into()),
    }
}

async fn stream(interval_ms: u64) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = client::connect().await?;

    let list_response = client::request_response(
        &mut stream,
        &Request {
            request_id: 1,
            kind: Some(request::Kind::ListSources(ListSourcesRequest {})),
        },
    )
    .await?;

    let source_ids = match list_response.kind {
        Some(response::Kind::ListSources(list)) => list
            .sources
            .into_iter()
            .filter(|source| source.available)
            .map(|source| source.source_id)
            .collect::<Vec<_>>(),
        Some(response::Kind::Error(error)) => {
            return Err(format!("daemon error {}: {}", error.code, error.message).into());
        }
        other => return Err(format!("unexpected response: {other:?}").into()),
    };

    if source_ids.is_empty() {
        println!("no available sources");
        return Ok(());
    }

    let interval_ns = interval_ms.saturating_mul(1_000_000);
    write_frame_async(
        &mut stream,
        &Request {
            request_id: 2,
            kind: Some(request::Kind::StartStream(StartStreamRequest {
                source_ids,
                interval_ns,
                include_raw: false,
            })),
        },
    )
    .await?;

    loop {
        let response: Response = read_frame_async(&mut stream).await?;
        match response.kind {
            Some(response::Kind::StartStream(start)) => {
                println!("stream started interval_ns={}", start.effective_interval_ns);
            }
            Some(response::Kind::Sample(sample)) => {
                println!(
                    "source={} monotonic_ns={} energy_j={:.6} delta_j={:.6} power_w={:.6} interval_ns={} wrap={}",
                    sample.source_id,
                    sample.monotonic_ns,
                    sample.energy_j,
                    sample.delta_j,
                    sample.power_w,
                    sample.interval_ns,
                    sample.counter_wrap
                );
            }
            Some(response::Kind::Error(error)) => {
                return Err(format!("daemon error {}: {}", error.code, error.message).into());
            }
            other => return Err(format!("unexpected response: {other:?}").into()),
        }
    }
}
