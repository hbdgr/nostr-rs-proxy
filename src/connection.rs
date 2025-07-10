use crate::cache::EventCache;

use tracing::{info, debug, warn};
use serde_json::Value;

use futures_util::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};

use tokio::{net::TcpStream, sync::mpsc, time::Duration};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Error as WsError, Message},
    WebSocketStream
};

use nostr::{Event, EventId, Filter, ClientMessage, RelayMessage, SubscriptionId, JsonUtil};

// ---------------------------------------------------------

type WsStream = WebSocketStream<TcpStream>;

pub struct Connection {
    client_tx: SplitSink<WsStream, Message>,
    client_rx: SplitStream<WsStream>,

    relay_urls: Vec<String>,

    cache: EventCache,
}

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("WebSocket error: {0}")]
    WsError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Event validation failed: {0}")]
    ValidationError(String),

    #[error("Connection closed")]
    ConnectionClosed,

    // Add other error variants as needed
    // #[error("Nostr protocol error: {0}")]
    // NostrError(#[from] nostr::Error)
}

// ---------------------------------------------------------

// TODO: Implement persistent connection pooling for relays
// - Maintain long-lived connections instead of per-request
// - Use connection reuse with keep-alive
// - Add reconnect logic with backoff
// - Implement connection health checks
// - Add max connection limits

impl Connection {
    pub async fn new(raw_stream: TcpStream, relay_urls: &[String]) -> Result<Self, WsError> {
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

    pub async fn run(&mut self) -> Result<(), ProxyError> {
        while let Some(msg) = self.client_rx.next().await {
            let msg = msg.map_err(ProxyError::WsError)?;

            if let Err(e) = self.handle_message(msg).await {
                match e {
                    ProxyError::ConnectionClosed => break,
                    _ => return Err(e),
                }
                // Handle other errors
            }
        }

        info!("Client disconnected");
        Ok(())
    }

    async fn handle_message(&mut self, msg: Message) -> Result<(), ProxyError> {
        debug!("handle_message:");

        // Log message type and content safely
        match &msg {
            Message::Text(text) => debug!("Received text message: {}", text),
            Message::Binary(data) => debug!("Received binary message ({} bytes)", data.len()),
            Message::Ping(data) => debug!("Received ping ({} bytes)", data.len()),
            Message::Pong(_) => debug!("Received pong"),
            Message::Close(frame) => debug!("Received close frame: {:?}", frame),
            Message::Frame(frame) => debug!("Received raw frame: {:?}", frame),
        }

        match msg {
            // handle Nostr protocol messages (JSON arrays)
            Message::Text(text) => {
                match serde_json::from_str::<Value>(&text) {
                    Ok(Value::Array(arr)) => {
                        self.handle_nostr_command(arr).await?;
                    }
                    _ => {
                        // log invalid JSON format but don't crash connection
                        warn!("Received non-array JSON message: {}", text);
                    }
                }
            }

            // WebSocket control messages
            Message::Ping(data) => {
                // respond to pings to keep connection alive
                self.client_tx.send(Message::Pong(data)).await?;
            }

            Message::Pong(_) => {
                // no action needed for pong responses
            }

            // Handle graceful closure
            Message::Close(frame) => {
                // Send close response if frame exists
                if let Some(frame) = frame {
                    self.client_tx
                        .send(Message::Close(Some(frame)))
                        .await?;
                } else {
                    self.client_tx
                        .send(Message::Close(None))
                        .await?;
                }
                return Err(ProxyError::ConnectionClosed);
            }

            // nostr doesn't use these message types
            Message::Binary(_) | Message::Frame(_) => {
                debug!("Received unsupported binary/frame message");
            }
        }
        Ok(())
    }

    async fn handle_nostr_command(&mut self, command: Vec<Value>) -> Result<(), ProxyError> {
        // Process Nostr commands
        // 1. Parse the command type (EVENT, REQ, CLOSE)
        // 2. Handle locally or forward to relays
        // 3. Send responses back through client_tx

        // Parse command type
        let cmd_type = command.first()
            .and_then(Value::as_str)
            .ok_or_else(|| ProxyError::ValidationError("Invalid command format".into()))?;

        match cmd_type {
            "EVENT" => self.handle_event_command(&command).await,
            "REQ" => self.handle_req_command(&command).await,
            "CLOSE" => self.handle_close_command(&command).await,
            _ => {
                warn!("Unknown command type: {}", cmd_type);
                Ok(())
            }
        }
    }

    async fn send_ok_response(
        &mut self,
        event_id: EventId,
        status: bool,
        message: &str
    ) -> Result<(), ProxyError> {
        let msg = RelayMessage::ok(event_id, status, message.to_string());
        self.client_tx
            .send(Message::text(msg.as_json()))
            .await
            .map_err(ProxyError::WsError)
    }

    async fn send_event(
        &mut self,
        subscription_id: &SubscriptionId,
        event: &Event
    ) -> Result<(), ProxyError> {
        let msg = RelayMessage::event(subscription_id.clone(), event.clone());
        self.client_tx
            .send(Message::text(msg.as_json()))
            .await
            .map_err(ProxyError::WsError)
    }


    async fn handle_event_command(&mut self, command: &[Value]) -> Result<(), ProxyError> {
        // Validate command structure - ["EVENT", event_json]
        if command.len() < 2 {
            return Err(ProxyError::ValidationError("Invalid EVENT command".into()));
        }

        debug!("Handling EVENT command");

        // Extract the event JSON string
        let event_json = command[1].to_string();
        let event = match Event::from_json(&event_json) {
            Ok(event) => event,
            Err(_) => {
                warn!("Failed to parse Event from json: {}", event_json);
                return Err(ProxyError::ValidationError("Invalid event format".into()));
            }
        };

        // 1. Check for duplicate (fixed version)
        if self.cache.get(&event.id).await.is_some() {
            debug!("Duplicate event detected: {}", event.id);
            self.send_ok_response(event.id, true, "duplicate: event already known").await?;
            return Ok(());
        }

        // 2. Cache before forwarding (to prevent concurrent duplicates)
        if EventCache::is_cacheable_event(&event) {
            self.cache.set(event.clone()).await;
        }

        // 3. Forward to relays
        match self.forward_event_to_relays(event.clone()).await {
            Ok(_) => {
                self.send_ok_response(event.id, true, "").await?;
            }
            Err(e) => {
                warn!("Error forwarding event: {}", e);
                // Optionally remove from cache if forwarding fails?
                self.send_ok_response(event.id, false, &e.to_string()).await?;
            }
        }

        Ok(())
    }

    async fn handle_req_command(&mut self, command: &[Value]) -> Result<(), ProxyError> {
        // Validate command structure
        if command.len() < 3 {
            return Err(ProxyError::ValidationError(
                "REQ command requires at least 3 elements".into()
            ));
        }

        let subscription_id: SubscriptionId = SubscriptionId::new(
            command[1]
                .as_str()
                .ok_or_else(|| ProxyError::ValidationError(
                    "Subscription ID must be a string".into()
                ))?
        );

        debug!("Handling REQ for subscription: {}", subscription_id);

       // Parse all filters
        let filters: Vec<Filter> = command[2..]
            .iter()
            .filter_map(|v| {
                let json_str = v.to_string();
                Filter::from_json(&json_str).ok()
            })
            .collect();

        if filters.is_empty() {
            return Err(ProxyError::ValidationError(
                "No valid filters found in REQ command".into()
            ));
        }

        // 1. Serve from cache (all filters)
        for filter in &filters {
            let cached_events = self.cache.query(filter).await;
            for event in cached_events {
                self.send_event(&subscription_id, &event).await?;
            }
        }

        // 2. Forward REQ to relays for each filter
        for filter in filters {
            self.forward_req_to_relays(&subscription_id, &filter).await?;
        }

        Ok(())
    }

    async fn handle_close_command(&mut self, command: &[Value]) -> Result<(), ProxyError> {
        // Validate command structure
        if command.len() < 2 {
            return Err(ProxyError::ValidationError("Invalid CLOSE command".into()));
        }

        // Process subscription close
        let subscription_id = command[1]
            .as_str()
            .ok_or_else(|| ProxyError::ValidationError(
                "Subscription ID must be a string".into()
            ))?;

        debug!("Closing subscription: {}", subscription_id);

        // Terminate subscription
        // self.remove_subscription(subscription_id).await?;

        Ok(())
    }

    async fn process_response(&mut self, response: Message) -> Result<(), ProxyError> {
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

    async fn forward_event_to_relays(&mut self, event: Event) -> Result<(), ProxyError> {
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

    // REQ forwarding to relays
    async fn forward_req_to_relays(
        &mut self,
        subscription_id: &SubscriptionId,
        filter: &Filter
    ) -> Result<(), ProxyError> {
        let req_msg = ClientMessage::req(subscription_id.clone(), filter.clone()).as_json();
        let (response_tx, mut response_rx) = mpsc::channel(1);

        for url in &self.relay_urls {
            let url = url.clone();
            let req_msg = req_msg.clone();
            let response_tx = response_tx.clone();

            tokio::spawn(async move {
                match connect_async(&url).await {
                    Ok((ws, _)) => {
                        let (mut ws_tx, _ws_rx) = ws.split(); // Don't need rx for REQ
                        if let Err(e) = ws_tx.send(Message::text(&req_msg)).await {
                            warn!("Failed to send REQ to {}: {}", url, e);
                        }
                        // For REQ, we don't wait for responses since they come as EVENT streams
                        let _ = response_tx.send(()).await; // Signal completion
                    }
                    Err(e) => warn!("Connection failed to {}: {}", url, e),
                }
            });
        }

        // Just wait for first confirmation (no response content needed)
        let _ = response_rx.recv().await;
        Ok(())
    }
}

// // TODO do we need this?
// async fn handle_relay_message(
//     &mut self,
//     msg: Message
// ) -> Result<(), ProxyError> {
//     match msg {
//         Message::Text(text) => {
//             let text_str = text.as_str();
//
//             match RelayMessage::from_json(text_str) {
//
//                 Ok(RelayMessage::Event { subscription_id, event }) => {
//                     // Cache event
//                     if EventCache::is_cacheable_event(&event) {
//                         self.cache.set(event.as_ref().clone()).await;
//                     }
//
//                     // Forward to client
//                     self.client_tx
//                         .send(Message::text(
//                             RelayMessage::event(
//                                 subscription_id.into_owned(),
//                                 event.as_ref().clone()).as_json()
//                         ))
//                         .await?;
//                 }
//
//                 Ok(RelayMessage::EndOfStoredEvents(sub_id)) => {
//                     self.client_tx
//                         .send(Message::text(
//                             RelayMessage::eose(sub_id.into_owned()).as_json()
//                         ))
//                         .await?;
//                 }
//
//                 Ok(_) => {} // Handle other message types if needed
//                 Err(e) => {
//                     warn!("Failed to parse relay message: {}", e);
//                 }
//             }
//         }
//         _ => {} // Ignore non-text messages
//     }
//     Ok(())
// }
//
//
// // TODO do we need this?
// async fn handle_eose(&mut self, subscription_id: SubscriptionId) -> Result<(), ProxyError> {
//     let msg = RelayMessage::eose(subscription_id);
//     self.client_tx
//         .send(Message::text(msg.as_json()))
//         .await
//         .map_err(ProxyError::WsError)
// }

