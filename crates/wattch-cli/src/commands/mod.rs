pub mod hello;
pub mod run;
pub mod sources;
pub mod stream;

use clap::{Parser, Subcommand};

use crate::duration::parse_duration_arg;
use crate::output::format::{RunFormat, SourcesFormat, StreamFormat};

const DEFAULT_STREAM_INTERVAL_MS: u64 = 100;

#[derive(Debug, Parser)]
#[command(name = "wattch")]
#[command(about = "Wattch energy measurement CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Hello,
    Sources {
        #[arg(long, value_enum, default_value_t = SourcesFormat::Table)]
        format: SourcesFormat,
    },
    Stream {
        #[arg(long, default_value_t = DEFAULT_STREAM_INTERVAL_MS)]
        interval_ms: u64,

        #[arg(long, value_parser = parse_duration_arg)]
        duration: Option<std::time::Duration>,

        #[arg(long)]
        source: Vec<u32>,

        #[arg(long, value_enum, default_value_t = StreamFormat::Table)]
        format: StreamFormat,

        #[arg(long)]
        summary: bool,
    },
    Run {
        #[arg(long, default_value_t = DEFAULT_STREAM_INTERVAL_MS)]
        interval_ms: u64,

        #[arg(long)]
        source: Vec<u32>,

        #[arg(long, value_enum, default_value_t = RunFormat::Table)]
        format: RunFormat,

        #[arg(long, default_value_t = true)]
        summary: bool,

        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
}

pub async fn run(cli: Cli) -> Result<i32, Box<dyn std::error::Error>> {
    match cli.command {
        Command::Hello => {
            hello::run().await?;
            Ok(0)
        }
        Command::Sources { format } => {
            sources::run(format).await?;
            Ok(0)
        }
        Command::Stream {
            interval_ms,
            duration,
            source,
            format,
            summary,
        } => {
            stream::run(interval_ms, duration, source, format, summary).await?;
            Ok(0)
        }
        Command::Run {
            interval_ms,
            source,
            format,
            summary,
            command,
        } => run::run(interval_ms, source, format, summary, command).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::time::Duration;

    #[test]
    fn run_command_requires_command_after_double_dash() {
        assert!(Cli::try_parse_from(["wattch", "run"]).is_err());
    }

    #[test]
    fn stream_accepts_multiple_sources() {
        let cli = Cli::parse_from(["wattch", "stream", "--source", "1", "--source", "2"]);
        match cli.command {
            Command::Stream { source, .. } => assert_eq!(source, vec![1, 2]),
            _ => panic!("expected stream command"),
        }
    }

    #[test]
    fn stream_accepts_duration() {
        let cli = Cli::parse_from(["wattch", "stream", "--duration", "100ms"]);
        match cli.command {
            Command::Stream { duration, .. } => {
                assert_eq!(duration, Some(Duration::from_millis(100)));
            }
            _ => panic!("expected stream command"),
        }
    }
}
