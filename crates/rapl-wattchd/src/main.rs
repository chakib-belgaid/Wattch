mod daemon;
mod sampler;
mod socket;

#[tokio::main]
async fn main() {
    if let Err(error) = daemon::run_from_env().await {
        eprintln!("rapl-wattchd: {error}");
        std::process::exit(1);
    }
}
