use tracing::{info, debug};
use actix_web_actors::ws;

use crate::config::Settings;

use crate::messages::{ClientMessage, Connect, Disconnect, WsMessage};
use actix::prelude::{Actor, Context, Handler, Recipient};
use uuid::Uuid;

// ------------------ Proxy Server

type Socket = Recipient<WsMessage>;

pub struct Proxy {
    relays: Vec<Socket>,
}

impl Proxy {
    pub fn new(settings: &Settings) -> Self {
        debug!("Proxy: new()");

        let relays = Vec::new();
        for r in settings.sources.relays.iter().flatten() {
            info!("rel: {:?}", r);
        }

        Proxy {
            relays,
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
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) {
        debug!("Proxy: handle [Connect], msg: {:?}", msg);
    }
}

impl Handler<Disconnect> for Proxy {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        debug!("Proxy: handle [Disconnect], msg: {:?}", msg);
    }
}

impl Handler<ClientMessage> for Proxy {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        debug!("Proxy: handle [ClientMessage], msg: {:?}", msg);
    }
}
