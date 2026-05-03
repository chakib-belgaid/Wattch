use wattch_proto::wattch::v1::response;

use crate::client;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let (mut stream, config) = client::connect_with_config().await?;
    let response = client::hello(&mut stream, 1).await?;

    match response.kind {
        Some(response::Kind::Hello(hello)) => {
            println!("Wattch daemon: {}", hello.daemon_version);
            println!("Protocol: {}", hello.protocol_version);
            println!("Socket: {}", config.socket_path.display());
            Ok(())
        }
        Some(response::Kind::Error(error)) => {
            Err(format!("daemon error {}: {}", error.code, error.message).into())
        }
        other => Err(format!("unexpected response: {other:?}").into()),
    }
}
