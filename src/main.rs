mod config;
mod connection;
mod cache;

use std::io::Error as IoError;

use config::Settings;
use tracing::{info, debug, warn};

use tokio::net::{TcpListener, TcpStream};

// ---------------------------------------------------------

// TODO
// struct ProxyMetrics {
//     cache_hits: AtomicU64,
//     cache_misses: AtomicU64,
//     relay_errors: AtomicU64,
// }

// TODO
// struct RelayPool {
//     connections: HashMap<String, WebSocketStream>,
//     max_retries: usize,
// }

async fn handle_connection(raw_stream: TcpStream, relay_urls: Vec<String>) {
    if let Ok(conn) = connection::Connection::new(raw_stream, &relay_urls).await {
        if let Err(e) = conn.run().await {
            warn!("Connection error: {}", e);
        }
    }
}

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
        info!("Incoming TCP connection from: {}", addr);

        let sources = settings.sources.clone();

        tokio::spawn(async move {
            debug!("main: handle_connection");
            handle_connection(stream, sources.relays).await;
            debug!("main: handle_connection end");
        });
    }

    Ok(())
}
