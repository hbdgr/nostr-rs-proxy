use crate::config::Settings;
use crate::relay_client::RelayClient;
use crate::messages::{ClientMessage, Connect, Disconnect, WsMessage};
use actix::prelude::{Actor, Addr, Context, Handler, Recipient};
use uuid::Uuid;

use std::sync::{Arc};
use tokio::runtime;
use tokio::sync::{mpsc, Mutex};

use tracing::{info, debug};

// ------------------ Proxy Server

type Socket = Recipient<WsMessage>;

pub struct Proxy {
    relays: Vec<RelayClient>,
}

impl Proxy {
    pub async fn new(settings: &Settings) -> Self {
        debug!("Proxy: new()");

        let mut relays = Vec::new();

        for addr in settings.sources.relays.iter().flatten() {
            info!("rel addr: {addr:?}");

            // Wrap RelayClient in Arc<Mutex>
            let relay = RelayClient::new(addr.to_string());

            println!("Adding new relay to Proxy: {:?}", relay);

            // start actors
            relays.push(
                relay
            );
        }

        Proxy {
            relays,
        }
    }
}

impl Proxy {
    // fn send_message(&self, message: &str, id_to: &Uuid) {
    // }

    fn send_to_relays(&self, msg: ClientMessage, ctx: &mut Context<Self>) {
        debug!("Proxy: send_to_relays, msg: {:?}, ctx: {:?}", msg, ctx);

        for relay in &self.relays {
            let msg_clone = msg.clone();

            // Call the method on the RelayClient
            let _ = relay.schedule_request(msg_clone);
        }
    }
}

impl Actor for Proxy {
    type Context = Context<Self>;
}

// TODO allow only specified address to connect
impl Handler<Connect> for Proxy {
    type Result = Result<bool, std::io::Error>;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        debug!("Proxy: handle [Connect], msg: {:?}", msg);
        Ok(true)
    }
}

impl Handler<Disconnect> for Proxy {
    type Result = Result<bool, std::io::Error>;

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) -> Self::Result {
        debug!("Proxy: handle [Disconnect], msg: {:?}", msg);
        Ok(true)
    }
}

impl Handler<ClientMessage> for Proxy {
    type Result = Result<bool, std::io::Error>;

    fn handle(&mut self, msg: ClientMessage, ctx: &mut Context<Self>) -> Self::Result {
        debug!("Proxy: handle [ClientMessage], msg: {:?}", msg);

        self.send_to_relays(msg.clone(), ctx);

        Ok(true)
    }
}
