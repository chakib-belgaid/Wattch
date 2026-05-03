mod client;
mod commands;
mod duration;
mod output;

use clap::Parser;
use commands::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match commands::run(cli).await {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(error) => {
            eprintln!("wattch: {error}");
            std::process::exit(1);
        }
    }
}
