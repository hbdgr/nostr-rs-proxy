mod config;
mod connection;

use std::io::Error as IoError;

use config::Settings;
use tracing::{info, warn};

use tokio::net::TcpListener;

// ---------------------------------------------------------

#[tokio::main]
async fn main() -> Result<(), IoError> {
    tracing_subscriber::fmt::try_init().unwrap(); // standard logging

    let settings = Settings::new(&None);
    let addr = format!(
        "{}:{}",
        settings.network.address.trim(),
        settings.network.port
    );

    // Create the event loop and TCP listener we'll accept connections on
    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task
    while let Ok((stream, addr)) = listener.accept().await {
        let sources = settings.sources.clone();

        tokio::spawn(async move {
            if let Err(e) = connection::handle_connection(sources, stream, addr).await {
                warn!("Connection error: {}", e);
            }
        });
    }

    Ok(())
}
