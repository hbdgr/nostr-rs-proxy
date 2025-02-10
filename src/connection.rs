use crate::cache::EventCache;

use tracing::{info, debug, warn};

use futures_util::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};

use tokio::{net::TcpStream, sync::mpsc, time::Duration};
use tokio_tungstenite::{connect_async, tungstenite::{Error,  Message}, WebSocketStream};

use nostr::{Event, JsonUtil};

// ---------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("WebSocket error: {0}")]
    WsError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Event validation failed: {0}")]
    ValidationError(String),

    // #[error("Nostr protocol error: {0}")]
    // NostrError(#[from] nostr::Error)
}

type WsStream = WebSocketStream<TcpStream>;

pub struct Connection {
    client_tx: SplitSink<WsStream, Message>,
    client_rx: SplitStream<WsStream>,
    relay_urls: Vec<String>,
    cache: EventCache,
}

// ---------------------------------------------------------

impl Connection {
    pub async fn new(raw_stream: TcpStream, relay_urls: &[String]) -> Result<Self, Error> {
        info!("New connection from: {}", raw_stream.peer_addr()?);
        let client_ws = tokio_tungstenite::accept_async(raw_stream).await?;
        let (client_tx, client_rx) = client_ws.split();

        let mut relays = Vec::new();
        for url in relay_urls {
            match connect_async(url).await {
                Ok((ws, _)) => relays.push(ws.split()),
                Err(e) => warn!("Failed to connect to relay {}: {}", url, e),
            }
        }

        Ok(Self {
            client_tx,
            client_rx,
            relay_urls: relay_urls.to_vec(),
            cache: EventCache::new(10000),
        })
    }

    pub async fn run(mut self) -> Result<(), Error> {
        while let Some(msg) = self.client_rx.next().await {
            self.handle_message(msg?).await?;
        }

        info!("Client disconnected");
        Ok(())
    }

    async fn handle_message(&mut self, msg: Message) -> Result<(), Error> {
        debug!("handle_message: {}", msg.to_text()?);

        if let Ok(text) = msg.to_text() {
            if let Ok(event) = Event::from_json(text) {

                debug!("1");

                // Validation check
                if let Err(e) = self.validate_event(&event) {
                    warn!("Invalid event from client: {}", e);
                    return Ok(());
                }

                debug!("2");

                // Check cache first
                if let Some(cached) = self.cache.get(&event.id).await {
                    debug!("Cache hit for event: {}", event.id);
                    self.client_tx.send(Message::text(cached.as_json())).await?;
                    return Ok(());
                }

                debug!("3");

                // Handle different event kinds
                match event.kind {
                    nostr::Kind::TextNote => self.handle_text_note(event).await?,

                    // Add other event kinds as needed
                    _ => self.forward_to_relays(event).await?,
                }
            } else {
                warn!("Failed to parse Event from json: {}", text);
            }
        }
        Ok(())
    }

    // Add validation method here
    fn validate_event(&self, event: &Event) -> Result<(), ProxyError> {
        event.verify().map_err(|e| ProxyError::ValidationError(e.to_string()))
    }

    async fn forward_to_relays(&mut self, event: Event) -> Result<(), Error> {
        let event_json = event.as_json();
        let (response_tx, mut response_rx) = mpsc::channel(1);

        // Create new connection for each relay per message
        // TODO connection pool
        for url in &self.relay_urls {
            let url = url.clone();
            let event_json = event_json.clone();
            let response_tx = response_tx.clone();

            tokio::spawn(async move {
                match connect_async(&url).await {
                    Ok((ws, _)) => {
                        let (mut ws_tx, mut ws_rx) = ws.split();
                        if let Err(e) = ws_tx.send(Message::text(&event_json)).await {
                            warn!("Failed to send to {}: {}", url, e);
                            return;
                        }

                        if let Some(Ok(response)) = ws_rx.next().await {
                            let _ = response_tx.send(response).await;
                        }
                    }
                    Err(e) => warn!("Connection failed to {}: {}", url, e),
                }
            });
        }

        // Create timeout for relay response
        let timeout = tokio::time::sleep(Duration::from_secs(2));
        tokio::pin!(timeout);

        // Wait for first response or timeout
        tokio::select! {
            _ = &mut timeout => {
                warn!("Relay response timeout");
                Ok(())
            },
            res = response_rx.recv() => {
                if let Some(response) = res {
                    self.process_response(response).await
                } else {
                    Ok(())
                }
            }
        }
    }

    async fn process_response(&mut self, response: Message) -> Result<(), Error> {
        if let Ok(text) = response.to_text() {
            if let Ok(event) = Event::from_json(text) {
                debug!("Caching response: {}", event.id);

                if EventCache::is_cacheable_event(&event) {
                    self.cache.set(event.clone()).await;
                }
                self.client_tx.send(Message::text(text)).await?;
            }
        }
        Ok(())
    }

    async fn handle_text_note(&mut self, event: Event) -> Result<(), Error> {
        debug!("Handling text note: {}", event.id);
        self.forward_to_relays(event).await
    }
}

/*
pub async fn handle_connection(
    sources: Sources,
    raw_stream: TcpStream,
    addr: SocketAddr
) -> Result<(), Error> {
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
*/
