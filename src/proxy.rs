use crate::config::Settings;
use crate::relay_client::RelayClient;
use crate::messages::{ClientMessage, Connect, Disconnect, WsMessage};
use actix::prelude::{Actor, Addr, Context, Handler, Recipient};
use uuid::Uuid;

use tracing::{info, debug};


// ------------------ Proxy Server

type Socket = Recipient<WsMessage>;

pub struct Proxy {
    relay_actors: Vec<Addr<RelayClient>>,
}

impl Proxy {
    pub async fn new(settings: &Settings) -> Self {
        debug!("Proxy: new()");

        let mut relay_actors = Vec::new();

        for addr in settings.sources.relays.iter().flatten() {
            info!("rel addr: {addr:?}");
            let mut relay = RelayClient::new(addr);

            // connect to relays
            relay
                .connect()
                .await;

            // start actors
            relay_actors.push(
                relay.start()
            );
        }

        Proxy {
            relay_actors,
        }
    }
}

impl Proxy {
    fn send_message(&self, message: &str, id_to: &Uuid) {
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

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) -> Self::Result {
        debug!("Proxy: handle [ClientMessage], msg: {:?}", msg);
        Ok(true)
    }
}
