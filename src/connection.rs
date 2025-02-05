use std::net::SocketAddr;

use crate::config::Sources;
use tracing::{info,debug, warn};

use futures_util::{future, StreamExt, SinkExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Error};

// ---------------------------------------------------------

pub async fn handle_connection(
    sources: Sources,
    raw_stream: TcpStream,
    addr: SocketAddr
) -> Result<(), Error> {
    info!("Incoming TCP connection from: {}", addr);

    let client_ws = tokio_tungstenite::accept_async(raw_stream).await?;
    let (mut client_tx, mut client_rx) = client_ws.split();
    debug!("WebSocket connection established: {}", addr);

    // Connect to all configured relays
    let relay_urls = sources.relays.as_ref().unwrap_or(&vec![]).to_vec();
    let mut relays_tx = Vec::new();
    let mut relays_rx = Vec::new();

    for url in relay_urls {
        match connect_async(url.clone()).await {
            Ok((ws, _)) => {
                let (tx, rx) = ws.split();
                relays_tx.push(tx);
                relays_rx.push(rx);
                debug!("Connected to relay: {}", url);
            }
            Err(e) => {
                warn!("Failed to connect to relay {}: {}", url, e);
                continue;
            }
        }
    }

    // Client -> Relays forwarding
    let client_to_relays = async {
        while let Some(Ok(msg)) = client_rx.next().await {
            debug!("Forwarding client message to {} relays", relays_tx.len());
            let send_futures = relays_tx
                .iter_mut()
                .map(|tx| tx.send(msg.clone()));
            future::try_join_all(send_futures).await?;
        }
        Ok::<(), Error>(())
    };

    // Relays -> Client forwarding
    let relays_to_client = async {
        loop {
            let mut messages = Vec::new();
            for rx in &mut relays_rx {
                if let Some(Ok(msg)) = rx.next().await {
                    messages.push(msg);
                }
            }

            for msg in messages {
                client_tx.send(msg).await?;
            }
        }
    };

    tokio::select! {
        res = client_to_relays => res,
        res = relays_to_client => res,
    }?;

    info!("{} disconnected", addr);
    Ok(())
}

